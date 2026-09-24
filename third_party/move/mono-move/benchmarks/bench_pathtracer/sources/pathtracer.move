// Monte Carlo path tracer over smallpt's nine-sphere Cornell box, in fixed
// point. Every quantity is a world unit scaled by 1e6.
//
// Modeled on smallpt (2008) by Kevin Beason, https://www.kevinbeason.com/smallpt/,
// which the author released under the MIT license. The scene, camera, tent
// filter, materials and the structure of `radiance` follow it. The conversion
// to fixed point is ours.

module bench::pathtracer {
    use std::vector;
    use bench::pathtracer_fixed as fx;
    use bench::pathtracer_fixed::Vec3;

    /// 1.0, and the same value as i256. Repeated from `bench::pathtracer_fixed`
    /// because Move constants do not cross module boundaries.
    const SCALE: i128 = 1000000;
    const SCALE_I256: i256 = 1000000;
    /// SCALE^2, the scale a raw product of two fixed-point values carries.
    const SCALE_SQ: i256 = 1000000000000;
    const SCALE_U64: u64 = 1000000;

    /// Materials.
    const DIFF: u8 = 0;
    const SPEC: u8 = 1;
    const REFR: u8 = 2;

    /// smallpt's 1e-4 offset that keeps a bounce from re-hitting the surface
    /// it left, at SCALE^2 because that is the scale `t` is compared at.
    const EPS_RAW: i256 = 100000000;

    /// Farther than any distance inside the box.
    const T_FAR: i128 = 1000000000000;

    /// Russian roulette starts after this many bounces, as in smallpt.
    const ROULETTE_DEPTH: u64 = 5;

    /// Glass. Refractive index 1.5, and the Schlick reflectance at normal
    /// incidence it implies: (1.5-1)^2 / (1.5+1)^2.
    const GLASS_IOR: i128 = 1500000;
    const SCHLICK_R0: i128 = 40000;

    /// smallpt's camera: origin, tilt of the view direction before it is
    /// normalised, horizontal field of view, and the distance the ray origin
    /// is pushed forward so it starts inside the box.
    const CAM_OX: i128 = 50000000;
    const CAM_OY: i128 = 52000000;
    const CAM_OZ: i128 = 295600000;
    const CAM_TILT_Y: i128 = -42612;
    const FOV: i128 = 513500;
    const FOCAL: i128 = 140000000;

    /// Image size assumed by `bench_trace_pixel`, which takes a pixel but not
    /// a frame. `bench_render` takes both.
    const REF_WIDTH: u64 = 1024;
    const REF_HEIGHT: u64 = 768;

    /// House LCG and checksum-fold parameters, shared across the benchmark
    /// suite.
    const LCG_MUL: u64 = 1103515245;
    const LCG_INC: u64 = 12345;
    const LCG_MOD: u64 = 1000003;
    const CHECKSUM_MOD: u64 = 1000000007;
    /// Knuth's multiplicative constant, used to spread neighbouring pixel
    /// indices apart before they seed a stream.
    const SEED_SPREAD: u64 = 2654435761;

    const E_UNEXPECTED: u64 = 1;

    struct Sphere has copy, drop, store {
        rad: i128,
        pos: Vec3,
        emission: Vec3,
        colour: Vec3,
        refl: u8,
    }

    struct Ray has copy, drop, store {
        origin: Vec3,
        dir: Vec3,
    }

    //
    // Scene.
    //

    fun sphere(rad: i128, pos: Vec3, emission: Vec3, colour: Vec3, refl: u8): Sphere {
        Sphere { rad, pos, emission, colour, refl }
    }

    /// smallpt's Cornell box. Six spheres of radius 1e5 stand in for the
    /// walls, a mirror ball and a glass ball sit on the floor, and a sphere
    /// of radius 600 pokes an 18-unit disc of light through the ceiling.
    public fun scene(): vector<Sphere> {
        let s = vector::empty<Sphere>();
        let black = fx::zero();
        let grey = fx::vec3(750000, 750000, 750000);
        let near_white = fx::vec3(999000, 999000, 999000);

        // Left, red.
        vector::push_back(&mut s, sphere(
            100000000000,
            fx::vec3(100001000000, 40800000, 81600000),
            black,
            fx::vec3(750000, 250000, 250000),
            DIFF,
        ));
        // Right, blue.
        vector::push_back(&mut s, sphere(
            100000000000,
            fx::vec3(-99901000000, 40800000, 81600000),
            black,
            fx::vec3(250000, 250000, 750000),
            DIFF,
        ));
        // Back.
        vector::push_back(&mut s, sphere(
            100000000000,
            fx::vec3(50000000, 40800000, 100000000000),
            black,
            grey,
            DIFF,
        ));
        // Front. Black, so paths leaving through the camera side die.
        vector::push_back(&mut s, sphere(
            100000000000,
            fx::vec3(50000000, 40800000, -99830000000),
            black,
            black,
            DIFF,
        ));
        // Floor.
        vector::push_back(&mut s, sphere(
            100000000000,
            fx::vec3(50000000, 100000000000, 81600000),
            black,
            grey,
            DIFF,
        ));
        // Ceiling.
        vector::push_back(&mut s, sphere(
            100000000000,
            fx::vec3(50000000, -99918400000, 81600000),
            black,
            grey,
            DIFF,
        ));
        // Mirror ball.
        vector::push_back(&mut s, sphere(
            16500000,
            fx::vec3(27000000, 16500000, 47000000),
            black,
            near_white,
            SPEC,
        ));
        // Glass ball.
        vector::push_back(&mut s, sphere(
            16500000,
            fx::vec3(73000000, 16500000, 78000000),
            black,
            near_white,
            REFR,
        ));
        // Light.
        vector::push_back(&mut s, sphere(
            600000000,
            fx::vec3(50000000, 681330000, 81600000),
            fx::vec3(12000000, 12000000, 12000000),
            black,
            DIFF,
        ));
        s
    }

    //
    // Intersection.
    //

    /// Distance along `r` to the near surface of `s`, or 0 for a miss.
    ///
    /// The discriminant is built at SCALE^4 in i256 and never narrowed. For
    /// the radius-1e5 wall spheres it is a difference of two terms near 1e34
    /// whose leading four digits agree, so rounding either one to SCALE first
    /// would throw away most of the answer.
    fun intersect_sphere(s: &Sphere, r: &Ray): i128 {
        let op = fx::sub(&s.pos, &r.origin);
        let b = fx::dot_raw(&op, &r.dir);
        let oo = fx::dot_raw(&op, &op);
        let rr = (s.rad as i256) * (s.rad as i256);
        let det = b * b - (oo - rr) * SCALE_SQ;
        if (det < 0) {
            return 0
        };
        let sdet = fx::isqrt_i256(det);
        let t = b - sdet;
        if (t > EPS_RAW) {
            return ((t / SCALE_I256) as i128)
        };
        t = b + sdet;
        if (t > EPS_RAW) {
            return ((t / SCALE_I256) as i128)
        };
        0
    }

    /// Nearest hit over the whole scene, as (hit, distance, sphere index).
    fun intersect(spheres: &vector<Sphere>, r: &Ray): (bool, i128, u64) {
        let n = vector::length(spheres);
        let best_t = T_FAR;
        let best_id = 0;
        let hit = false;
        let i = 0;
        while (i < n) {
            let d = intersect_sphere(vector::borrow(spheres, i), r);
            if (d != 0 && d < best_t) {
                best_t = d;
                best_id = i;
                hit = true;
            };
            i = i + 1;
        };
        (hit, best_t, best_id)
    }

    //
    // Randomness.
    //

    fun lcg_next(state: &mut u64): u64 {
        *state = ((*state * LCG_MUL) + LCG_INC) % LCG_MOD;
        *state
    }

    /// Uniform fixed-point value in [0, 1).
    fun rand01(state: &mut u64): i128 {
        (((lcg_next(state) * SCALE_U64) / LCG_MOD) as i128)
    }

    /// Per-pixel stream. The LCG has a short period and neighbouring seeds
    /// produce visibly correlated streams, so the pixel index is spread out
    /// and the state is stepped a few times before any sample is drawn.
    fun pixel_seed(seed: u64, px: u64, py: u64, width: u64): u64 {
        let idx = (py * width + px) % LCG_MOD;
        let s = (seed % LCG_MOD + idx * SEED_SPREAD) % LCG_MOD;
        lcg_next(&mut s);
        lcg_next(&mut s);
        lcg_next(&mut s);
        s
    }

    //
    // Radiance.
    //

    /// smallpt's `radiance`, recursive over the three materials. `depth`
    /// counts bounces already taken. `max_depth` is a hard cap on top of the
    /// Russian roulette, so the recursion tree is finite whatever the LCG
    /// draws and the benchmark's cost is a knob rather than a distribution.
    fun radiance(
        spheres: &vector<Sphere>,
        r: &Ray,
        depth: u64,
        max_depth: u64,
        rng: &mut u64,
    ): Vec3 {
        let (hit, t, id) = intersect(spheres, r);
        if (!hit) {
            return fx::zero()
        };
        let obj = *vector::borrow(spheres, id);
        let x = fx::add(&r.origin, &fx::scale_by(&r.dir, t));
        let n = fx::norm(&fx::sub(&x, &obj.pos));
        // The shading normal always faces the incoming ray; `n` keeps the
        // geometric orientation that the refraction branch needs.
        let nl = if (fx::dot(&n, &r.dir) < 0) { n } else { fx::neg(&n) };
        let f = obj.colour;

        let next_depth = depth + 1;
        if (next_depth >= max_depth) {
            return obj.emission
        };
        if (next_depth > ROULETTE_DEPTH) {
            let p = fx::max_component(&f);
            // A black surface has p == 0, and `rand01` is never negative, so
            // the reweighting below cannot divide by zero.
            if (rand01(rng) >= p) {
                return obj.emission
            };
            f = fx::scale_by(&f, fx::fp_div(SCALE, p));
        };

        if (obj.refl == DIFF) {
            let dir = diffuse_dir(&nl, rng);
            let next = Ray { origin: x, dir };
            let incoming = radiance(spheres, &next, next_depth, max_depth, rng);
            return fx::add(&obj.emission, &fx::mul(&f, &incoming))
        };

        let refl_ray = Ray {
            origin: x,
            dir: fx::sub(&r.dir, &fx::scale_by(&n, 2 * fx::dot(&n, &r.dir))),
        };

        if (obj.refl == SPEC) {
            let incoming = radiance(spheres, &refl_ray, next_depth, max_depth, rng);
            return fx::add(&obj.emission, &fx::mul(&f, &incoming))
        };

        // REFR. Snell for the transmitted direction, Schlick for the split
        // between the two rays.
        let into = fx::dot(&n, &nl) > 0;
        let nnt = if (into) { fx::fp_div(SCALE, GLASS_IOR) } else { GLASS_IOR };
        let ddn = fx::dot(&r.dir, &nl);
        let cos2t = SCALE - fx::fp_mul(fx::fp_mul(nnt, nnt), SCALE - fx::fp_mul(ddn, ddn));
        if (cos2t < 0) {
            // Total internal reflection.
            let incoming = radiance(spheres, &refl_ray, next_depth, max_depth, rng);
            return fx::add(&obj.emission, &fx::mul(&f, &incoming))
        };

        let k = fx::fp_mul(ddn, nnt) + fx::fp_sqrt(cos2t);
        let tdir = fx::norm(&fx::sub(
            &fx::scale_by(&r.dir, nnt),
            &fx::scale_by(&n, if (into) { k } else { -k }),
        ));
        let refr_ray = Ray { origin: x, dir: tdir };

        let c = if (into) { SCALE + ddn } else { SCALE - fx::dot(&tdir, &n) };
        let c2 = fx::fp_mul(c, c);
        let c5 = fx::fp_mul(fx::fp_mul(c2, c2), c);
        let re = SCHLICK_R0 + fx::fp_mul(SCALE - SCHLICK_R0, c5);
        let tr = SCALE - re;
        // Pick one of the two rays once the path is deep enough to make
        // tracing both wasteful, and reweight by the pick probability.
        let p = SCALE / 4 + re / 2;
        let split = if (next_depth > 2) {
            if (rand01(rng) < p) {
                let refl_in = radiance(spheres, &refl_ray, next_depth, max_depth, rng);
                fx::scale_by(&refl_in, fx::fp_div(re, p))
            } else {
                let refr_in = radiance(spheres, &refr_ray, next_depth, max_depth, rng);
                fx::scale_by(&refr_in, fx::fp_div(tr, SCALE - p))
            }
        } else {
            let refl_in = radiance(spheres, &refl_ray, next_depth, max_depth, rng);
            let refr_in = radiance(spheres, &refr_ray, next_depth, max_depth, rng);
            fx::add(&fx::scale_by(&refl_in, re), &fx::scale_by(&refr_in, tr))
        };
        fx::add(&obj.emission, &fx::mul(&f, &split))
    }

    /// Cosine-weighted direction about `nl`, in an orthonormal basis built
    /// from whichever world axis is least aligned with it.
    fun diffuse_dir(nl: &Vec3, rng: &mut u64): Vec3 {
        let (cos_a, sin_a) = fx::cos_sin_turn(rand01(rng));
        let r2 = rand01(rng);
        let r2s = fx::fp_sqrt(r2);
        let (wx, _, _) = fx::parts(nl);
        let axis = if (fx::abs(wx) > SCALE / 10) {
            fx::vec3(0, SCALE, 0)
        } else {
            fx::vec3(SCALE, 0, 0)
        };
        let u = fx::norm(&fx::cross(&axis, nl));
        let v = fx::cross(nl, &u);
        let tangential = fx::add(
            &fx::scale_by(&u, fx::fp_mul(cos_a, r2s)),
            &fx::scale_by(&v, fx::fp_mul(sin_a, r2s)),
        );
        fx::norm(&fx::add(&tangential, &fx::scale_by(nl, fx::fp_sqrt(SCALE - r2))))
    }

    //
    // Camera and tone mapping.
    //

    /// Origin, view direction, and the two image-plane axes already scaled by
    /// the field of view. `cy` points up, so pixel row 0 is the bottom of the
    /// frame, matching smallpt.
    fun camera(width: u64, height: u64): (Vec3, Vec3, Vec3, Vec3) {
        let origin = fx::vec3(CAM_OX, CAM_OY, CAM_OZ);
        let dir = fx::norm(&fx::vec3(0, CAM_TILT_Y, -SCALE));
        let cx = fx::vec3(((width as i128) * FOV) / (height as i128), 0, 0);
        let cy = fx::scale_by(&fx::norm(&fx::cross(&cx, &dir)), FOV);
        (origin, dir, cx, cy)
    }

    /// smallpt's tent filter: two uniform draws folded into a triangular
    /// distribution over [-1, 1].
    fun tent(rng: &mut u64): i128 {
        let r = 2 * rand01(rng);
        if (r < SCALE) {
            fx::fp_sqrt(r) - SCALE
        } else {
            SCALE - fx::fp_sqrt(2 * SCALE - r)
        }
    }

    /// One 8-bit channel. smallpt applies gamma 2.2; this uses gamma 2 so the
    /// transfer function is the square root already needed elsewhere.
    fun to_byte(v: i128): u64 {
        ((fx::fp_sqrt(fx::clamp01(v)) * 255 / SCALE) as u64)
    }

    fun pack(c: &Vec3): u64 {
        let (r, g, b) = fx::parts(c);
        (to_byte(r) << 16) | (to_byte(g) << 8) | to_byte(b)
    }

    fun trace_pixel(
        spheres: &vector<Sphere>,
        origin: &Vec3,
        dir: &Vec3,
        cx: &Vec3,
        cy: &Vec3,
        width: u64,
        height: u64,
        px: u64,
        py: u64,
        spp: u64,
        max_depth: u64,
        rng: &mut u64,
    ): u64 {
        let acc = fx::zero();
        let s = 0;
        while (s < spp) {
            let sx = (px as i128) * SCALE + SCALE / 2 + tent(rng) / 2;
            let sy = (py as i128) * SCALE + SCALE / 2 + tent(rng) / 2;
            let u = fx::fp_div(sx, (width as i128) * SCALE) - SCALE / 2;
            let v = fx::fp_div(sy, (height as i128) * SCALE) - SCALE / 2;
            let d = fx::add(
                &fx::add(&fx::scale_by(cx, u), &fx::scale_by(cy, v)),
                dir,
            );
            let ray = Ray {
                origin: fx::add(origin, &fx::scale_by(&d, FOCAL)),
                dir: fx::norm(&d),
            };
            acc = fx::add(&acc, &radiance(spheres, &ray, 0, max_depth, rng));
            s = s + 1;
        };
        pack(&fx::div_by(&acc, (spp as i128) * SCALE))
    }

    //
    // Benchmark surface.
    //

    /// One pixel of a 1024x768 frame, packed as (r << 16) | (g << 8) | b.
    public fun bench_trace_pixel(x: u64, y: u64, spp: u64, max_depth: u64, seed: u64): u64 {
        let spheres = scene();
        let (origin, dir, cx, cy) = camera(REF_WIDTH, REF_HEIGHT);
        let rng = pixel_seed(seed, x, y, REF_WIDTH);
        trace_pixel(
            &spheres, &origin, &dir, &cx, &cy,
            REF_WIDTH, REF_HEIGHT, x, y, spp, max_depth, &mut rng,
        )
    }

    /// Checksum over every pixel of a `width` x `height` frame.
    public fun bench_render(width: u64, height: u64, spp: u64, max_depth: u64, seed: u64): u64 {
        let spheres = scene();
        let (origin, dir, cx, cy) = camera(width, height);
        let acc = 0;
        let py = 0;
        while (py < height) {
            let px = 0;
            while (px < width) {
                let rng = pixel_seed(seed, px, py, width);
                let packed = trace_pixel(
                    &spheres, &origin, &dir, &cx, &cy,
                    width, height, px, py, spp, max_depth, &mut rng,
                );
                acc = (acc * LCG_MOD + packed) % CHECKSUM_MOD;
                px = px + 1;
            };
            py = py + 1;
        };
        acc
    }

    public entry fun run(
        _s: &signer,
        x: u64,
        y: u64,
        spp: u64,
        max_depth: u64,
        seed: u64,
        expected: u64,
    ) {
        assert!(bench_trace_pixel(x, y, spp, max_depth, seed) == expected, E_UNEXPECTED);
    }

    //
    // Tests.
    //

    // Two pixels of the reference frame that land on the side walls level
    // with the light, about 50 units from it. A bounce there reaches the
    // light roughly 2.5% of the time, against 0.6% for a pixel at the middle
    // of the same column, so a colour assertion needs a quarter of the
    // samples to stop being a coin flip.
    #[test_only]
    const LIT_LEFT_X: u64 = 168;
    #[test_only]
    const LIT_RIGHT_X: u64 = 856;
    #[test_only]
    const LIT_Y: u64 = 574;

    #[test_only]
    fun unit_sphere_at(z: i128): Sphere {
        sphere(
            SCALE,
            fx::vec3(0, 0, z),
            fx::zero(),
            fx::vec3(SCALE, SCALE, SCALE),
            DIFF,
        )
    }

    #[test_only]
    fun ray_from_origin(dx: i128, dy: i128, dz: i128): Ray {
        Ray { origin: fx::zero(), dir: fx::norm(&fx::vec3(dx, dy, dz)) }
    }

    #[test_only]
    /// Index of the sphere the primary ray for a pixel lands on.
    fun primary_hit_id(px: u64, py: u64): u64 {
        let spheres = scene();
        let (origin, dir, cx, cy) = camera(REF_WIDTH, REF_HEIGHT);
        let u = fx::fp_div((px as i128) * SCALE + SCALE / 2, (REF_WIDTH as i128) * SCALE)
            - SCALE / 2;
        let v = fx::fp_div((py as i128) * SCALE + SCALE / 2, (REF_HEIGHT as i128) * SCALE)
            - SCALE / 2;
        let d = fx::add(&fx::add(&fx::scale_by(&cx, u), &fx::scale_by(&cy, v)), &dir);
        let ray = Ray {
            origin: fx::add(&origin, &fx::scale_by(&d, FOCAL)),
            dir: fx::norm(&d),
        };
        let (hit, _, id) = intersect(&spheres, &ray);
        assert!(hit, 999);
        id
    }

    #[test]
    fun test_ray_hits_sphere_at_analytic_t() {
        // Unit sphere ten units down +z: the near surface is at exactly 9.
        let s = unit_sphere_at(10 * SCALE);
        let r = ray_from_origin(0, 0, SCALE);
        assert!(intersect_sphere(&s, &r) == 9 * SCALE, 1);

        // Grazing the sphere off-axis: t = 10 - sqrt(1 - 0.6^2) = 9.2.
        let off = Sphere {
            rad: SCALE,
            pos: fx::vec3(600000, 0, 10 * SCALE),
            emission: fx::zero(),
            colour: fx::vec3(SCALE, SCALE, SCALE),
            refl: DIFF,
        };
        let t = intersect_sphere(&off, &r);
        assert!(t >= 9200000 - 4 && t <= 9200000 + 4, 2);

        // From inside, the only root ahead of the ray is the far one.
        let inside = Ray { origin: fx::vec3(0, 0, 10 * SCALE), dir: fx::vec3(0, 0, SCALE) };
        assert!(intersect_sphere(&s, &inside) == SCALE, 3);
    }

    #[test]
    fun test_ray_misses_sphere() {
        let s = unit_sphere_at(10 * SCALE);
        // Perpendicular to the sphere's direction.
        assert!(intersect_sphere(&s, &ray_from_origin(SCALE, 0, 0)) == 0, 1);
        // Aimed just outside the silhouette.
        assert!(intersect_sphere(&s, &ray_from_origin(1100000, 0, 10 * SCALE)) == 0, 2);
        // Behind the ray.
        let behind = unit_sphere_at(-10 * SCALE);
        assert!(intersect_sphere(&behind, &ray_from_origin(0, 0, SCALE)) == 0, 3);
    }

    #[test]
    fun test_wall_spheres_bound_the_box() {
        // The huge wall spheres have to intersect accurately despite the
        // discriminant being a difference of near-equal 1e34 terms.
        let spheres = scene();
        let origin = fx::vec3(50000000, 40000000, 100000000);
        // Straight at the left (red) wall, which stands at x = 1.
        let left = Ray { origin, dir: fx::vec3(-SCALE, 0, 0) };
        let (hit, t, id) = intersect(&spheres, &left);
        assert!(hit, 1);
        assert!(id == 0, 2);
        // 49 units, less the sag of a 1e5-radius sphere over the 18 units the
        // ray is off its axis.
        assert!(t >= 48998000 && t <= 48999000, 3);
        // Straight at the right (blue) wall, at x = 99.
        let right = Ray { origin, dir: fx::vec3(SCALE, 0, 0) };
        let (hit, t, id) = intersect(&spheres, &right);
        assert!(hit, 4);
        assert!(id == 1, 5);
        assert!(t >= 48998000 && t <= 48999000, 6);
        // Straight down at the floor, which is the plane y = 0.
        let down = Ray { origin, dir: fx::vec3(0, -SCALE, 0) };
        let (hit, t, id) = intersect(&spheres, &down);
        assert!(hit, 7);
        assert!(id == 4, 8);
        assert!(t >= 39998000 && t <= 39999000, 9);
    }

    #[test]
    fun test_light_returns_its_emission() {
        // The light is the only emitter and its albedo is black, so a ray
        // that lands on it returns the emission exactly, at any depth.
        let spheres = scene();
        let rng = 7;
        let up = Ray {
            origin: fx::vec3(50000000, 40000000, 81600000),
            dir: fx::vec3(0, SCALE, 0),
        };
        let (hit, _, id) = intersect(&spheres, &up);
        assert!(hit, 1);
        assert!(id == 8, 2);
        let l = radiance(&spheres, &up, 0, 8, &mut rng);
        let (r, g, b) = fx::parts(&l);
        assert!(r == 12000000, 3);
        assert!(g == 12000000, 4);
        assert!(b == 12000000, 5);
    }

    #[test]
    fun test_side_walls_are_red_and_blue() {
        // A t-value test cannot tell a mirrored scene from a correct one.
        // The leftmost column must land on the red wall and the rightmost on
        // the blue one.
        let spheres = scene();
        assert!(primary_hit_id(0, REF_HEIGHT / 2) == 0, 1);
        assert!(primary_hit_id(REF_WIDTH - 1, REF_HEIGHT / 2) == 1, 2);
        assert!(primary_hit_id(LIT_LEFT_X, LIT_Y) == 0, 5);
        assert!(primary_hit_id(LIT_RIGHT_X, LIT_Y) == 1, 6);

        let left = *vector::borrow(&spheres, 0);
        let (lr, lg, lb) = fx::parts(&left.colour);
        assert!(lr > lg && lr > lb, 3);

        let right = *vector::borrow(&spheres, 1);
        let (rr, rg, rb) = fx::parts(&right.colour);
        assert!(rb > rr && rb > rg, 4);
    }

    #[test]
    fun test_lit_left_wall_is_red_dominant() {
        // End to end: camera, sampling, transport and tone mapping all have
        // to be right for this pixel to come back red-dominant. The pixel is
        // the one at LIT_LEFT, chosen because it lands on the left wall level
        // with the light, where a bounce finds the light often enough that 128
        // samples resolve the colour instead of returning black.
        let packed = bench_trace_pixel(LIT_LEFT_X, LIT_Y, 128, 6, 11);
        let r = packed >> 16;
        let g = (packed >> 8) & 255;
        let b = packed & 255;
        assert!(r > 0, 1);
        assert!(r > g, 2);
        assert!(r > b, 3);
    }

    #[test]
    fun test_lit_right_wall_is_blue_dominant() {
        let packed = bench_trace_pixel(LIT_RIGHT_X, LIT_Y, 128, 6, 11);
        let r = packed >> 16;
        let g = (packed >> 8) & 255;
        let b = packed & 255;
        assert!(b > 0, 1);
        assert!(b > r, 2);
        assert!(b > g, 3);
    }

    #[test]
    fun test_deterministic() {
        let a = bench_trace_pixel(400, 300, 4, 5, 42);
        let b = bench_trace_pixel(400, 300, 4, 5, 42);
        assert!(a == b, 1);
        let c = bench_render(3, 2, 2, 3, 5);
        let d = bench_render(3, 2, 2, 3, 5);
        assert!(c == d, 3);
    }

    #[test]
    fun test_knobs_are_live() {
        // Each knob has to reach the kernel, or the benchmark measures the
        // wrong thing. Both baselines are lit, so a knob that went nowhere
        // would show up as equality rather than as two black results.
        let base = bench_trace_pixel(LIT_LEFT_X, LIT_Y, 32, 6, 3);
        assert!(base != 0, 1);
        assert!(bench_trace_pixel(LIT_LEFT_X, LIT_Y, 64, 6, 3) != base, 2);
        assert!(bench_trace_pixel(LIT_LEFT_X, LIT_Y, 32, 3, 3) != base, 3);
        assert!(bench_trace_pixel(LIT_LEFT_X, LIT_Y, 32, 6, 9) != base, 4);
        assert!(bench_trace_pixel(LIT_LEFT_X + 1, LIT_Y, 32, 6, 3) != base, 5);

        let frame = bench_render(3, 2, 8, 6, 1);
        assert!(frame != 0, 6);
        assert!(bench_render(4, 2, 8, 6, 1) != frame, 7);
        assert!(bench_render(3, 3, 8, 6, 1) != frame, 8);
        assert!(bench_render(3, 2, 16, 6, 1) != frame, 9);
        assert!(bench_render(3, 2, 8, 3, 1) != frame, 10);
    }

    #[test]
    fun test_channels_stay_in_range() {
        // Three 8-bit channels and nothing above them.
        let packed = bench_trace_pixel(512, 700, 8, 6, 2);
        assert!(packed >> 24 == 0, 1);
    }

    #[test]
    fun test_golden_pixel() {
        // Self-generated: there is no external reference for this scene in
        // fixed point. Pinned so a change in the numerics is not silent.
        assert!(bench_trace_pixel(LIT_LEFT_X, LIT_Y, 32, 6, 1) == 8867406, 1);
    }

    #[test]
    fun test_golden_render() {
        assert!(bench_render(6, 4, 4, 5, 7) == 200530231, 1);
    }

    #[test(s = @bench)]
    fun test_run_accepts_the_expected_value(s: &signer) {
        // A lit pixel, so the round trip is over a value that a change in the
        // numerics would move.
        let value = bench_trace_pixel(LIT_LEFT_X, LIT_Y, 32, 6, 5);
        assert!(value != 0, 100);
        run(s, LIT_LEFT_X, LIT_Y, 32, 6, 5, value);
    }

    #[test(s = @bench)]
    #[expected_failure(abort_code = E_UNEXPECTED)]
    fun test_run_rejects_a_wrong_value(s: &signer) {
        let value = bench_trace_pixel(LIT_LEFT_X, LIT_Y, 32, 6, 5);
        run(s, LIT_LEFT_X, LIT_Y, 32, 6, 5, value + 1);
    }
}

// Fixed-point scalars and 3-vectors for the path tracer. A value is an
// integer scaled by 1e6. Products widen to i256 before they are narrowed
// back, so multiplying two world-space quantities cannot overflow.

module bench::pathtracer_fixed {
    /// 1.0. `bench::pathtracer` repeats this constant; Move constants do not
    /// cross module boundaries.
    const SCALE: i128 = 1000000;
    const SCALE_I256: i256 = 1000000;

    /// 2*pi and pi/2. Note 4 * HALF_PI is one unit short of TWO_PI, which is
    /// why the quadrant index is taken modulo 4 rather than assumed to be
    /// in range.
    const TWO_PI: i128 = 6283185;
    const HALF_PI: i128 = 1570796;

    /// Newton steps in `isqrt`. The seed is within a factor of two of the
    /// root and every step doubles the number of correct bits, so eight
    /// covers a full-width u256.
    const SQRT_ITERS: u64 = 8;

    /// A negative radicand means a caller lost a sign, not that a value
    /// rounded below zero.
    const E_NEGATIVE_SQRT: u64 = 1;

    struct Vec3 has copy, drop, store {
        x: i128,
        y: i128,
        z: i128,
    }

    //
    // Scalars.
    //

    public fun abs(a: i128): i128 {
        if (a < 0) { -a } else { a }
    }

    /// Clamp to [0, 1] for tone mapping.
    public fun clamp01(a: i128): i128 {
        if (a < 0) {
            0
        } else if (a > SCALE) {
            SCALE
        } else {
            a
        }
    }

    public fun fp_mul(a: i128, b: i128): i128 {
        ((((a as i256) * (b as i256)) / SCALE_I256) as i128)
    }

    public fun fp_div(a: i128, b: i128): i128 {
        ((((a as i256) * SCALE_I256) / (b as i256)) as i128)
    }

    /// floor(sqrt(n)). The seed is a power of two straddling the root, so
    /// the iteration count is fixed instead of depending on the input.
    public fun isqrt(n: u256): u256 {
        if (n == 0) {
            return 0
        };
        let x = n;
        let r = 1u256;
        if (x >= 1u256 << 128) { x = x >> 128; r = r << 64; };
        if (x >= 1u256 << 64) { x = x >> 64; r = r << 32; };
        if (x >= 1u256 << 32) { x = x >> 32; r = r << 16; };
        if (x >= 1u256 << 16) { x = x >> 16; r = r << 8; };
        if (x >= 1u256 << 8) { x = x >> 8; r = r << 4; };
        if (x >= 1u256 << 4) { x = x >> 4; r = r << 2; };
        if (x >= 1u256 << 2) { r = r << 1; };

        let i = 0;
        while (i < SQRT_ITERS) {
            r = (r + n / r) / 2;
            i = i + 1;
        };
        // Newton overshoots by at most one from this seed.
        let q = n / r;
        if (r < q) { r } else { q }
    }

    /// `isqrt` on a non-negative i256. A radicand at scale S^2 yields a root
    /// at scale S, which is how the sphere discriminant stays exact.
    public fun isqrt_i256(n: i256): i256 {
        assert!(n >= 0, E_NEGATIVE_SQRT);
        (isqrt((n as u256)) as i256)
    }

    /// Square root of a fixed-point value, in fixed point.
    public fun fp_sqrt(a: i128): i128 {
        assert!(a >= 0, E_NEGATIVE_SQRT);
        ((isqrt(((a as i256) * SCALE_I256) as u256) as i128))
    }

    /// Cosine and sine of `u` turns, `u` in fixed point. One call rather than
    /// two because the quadrant fold is shared.
    public fun cos_sin_turn(u: i128): (i128, i128) {
        let frac = u % SCALE;
        if (frac < 0) {
            frac = frac + SCALE;
        };
        let theta = fp_mul(frac, TWO_PI);
        let q = (theta / HALF_PI) % 4;
        let x = theta % HALF_PI;
        let s = sin_series(x);
        let c = cos_series(x);
        if (q == 0) {
            (c, s)
        } else if (q == 1) {
            (-s, c)
        } else if (q == 2) {
            (-c, -s)
        } else {
            (s, -c)
        }
    }

    // Taylor series for x in [0, pi/2], where the last term retained is
    // already below the 1e-6 resolution. Successive terms differ by
    // -x^2 / ((2k)(2k+1)), which is where the divisors come from.
    fun sin_series(x: i128): i128 {
        let x2 = fp_mul(x, x);
        let term = x;
        let acc = term;
        term = -fp_mul(term, x2) / 6;
        acc = acc + term;
        term = -fp_mul(term, x2) / 20;
        acc = acc + term;
        term = -fp_mul(term, x2) / 42;
        acc = acc + term;
        term = -fp_mul(term, x2) / 72;
        acc = acc + term;
        term = -fp_mul(term, x2) / 110;
        acc + term
    }

    fun cos_series(x: i128): i128 {
        let x2 = fp_mul(x, x);
        let term = SCALE;
        let acc = term;
        term = -fp_mul(term, x2) / 2;
        acc = acc + term;
        term = -fp_mul(term, x2) / 12;
        acc = acc + term;
        term = -fp_mul(term, x2) / 30;
        acc = acc + term;
        term = -fp_mul(term, x2) / 56;
        acc = acc + term;
        term = -fp_mul(term, x2) / 90;
        acc + term
    }

    //
    // Vectors.
    //

    public fun vec3(x: i128, y: i128, z: i128): Vec3 {
        Vec3 { x, y, z }
    }

    public fun zero(): Vec3 {
        Vec3 { x: 0, y: 0, z: 0 }
    }

    public fun parts(v: &Vec3): (i128, i128, i128) {
        (v.x, v.y, v.z)
    }

    public fun add(a: &Vec3, b: &Vec3): Vec3 {
        Vec3 { x: a.x + b.x, y: a.y + b.y, z: a.z + b.z }
    }

    public fun sub(a: &Vec3, b: &Vec3): Vec3 {
        Vec3 { x: a.x - b.x, y: a.y - b.y, z: a.z - b.z }
    }

    public fun neg(v: &Vec3): Vec3 {
        Vec3 { x: -v.x, y: -v.y, z: -v.z }
    }

    /// Componentwise product, used to attenuate a colour by a surface.
    public fun mul(a: &Vec3, b: &Vec3): Vec3 {
        Vec3 {
            x: fp_mul(a.x, b.x),
            y: fp_mul(a.y, b.y),
            z: fp_mul(a.z, b.z),
        }
    }

    public fun scale_by(v: &Vec3, s: i128): Vec3 {
        Vec3 {
            x: fp_mul(v.x, s),
            y: fp_mul(v.y, s),
            z: fp_mul(v.z, s),
        }
    }

    public fun div_by(v: &Vec3, s: i128): Vec3 {
        Vec3 {
            x: fp_div(v.x, s),
            y: fp_div(v.y, s),
            z: fp_div(v.z, s),
        }
    }

    public fun dot(a: &Vec3, b: &Vec3): i128 {
        ((dot_raw(a, b) / SCALE_I256) as i128)
    }

    /// The three products summed without narrowing, so the result carries
    /// S^2. The sphere discriminant subtracts terms that agree to four
    /// decimal digits, and narrowing either one first would leave almost no
    /// precision behind.
    public fun dot_raw(a: &Vec3, b: &Vec3): i256 {
        (a.x as i256) * (b.x as i256)
            + (a.y as i256) * (b.y as i256)
            + (a.z as i256) * (b.z as i256)
    }

    public fun cross(a: &Vec3, b: &Vec3): Vec3 {
        Vec3 {
            x: fp_mul(a.y, b.z) - fp_mul(a.z, b.y),
            y: fp_mul(a.z, b.x) - fp_mul(a.x, b.z),
            z: fp_mul(a.x, b.y) - fp_mul(a.y, b.x),
        }
    }

    /// Unit vector. The zero vector has no direction to report, so it comes
    /// back unchanged rather than aborting on the division.
    public fun norm(v: &Vec3): Vec3 {
        let len = isqrt_i256(dot_raw(v, v));
        if (len == 0) {
            return *v
        };
        Vec3 {
            x: (((v.x as i256) * SCALE_I256 / len) as i128),
            y: (((v.y as i256) * SCALE_I256 / len) as i128),
            z: (((v.z as i256) * SCALE_I256 / len) as i128),
        }
    }

    /// Largest component. Russian roulette uses the brightest colour channel
    /// as the survival probability.
    public fun max_component(v: &Vec3): i128 {
        let m = v.x;
        if (v.y > m) {
            m = v.y;
        };
        if (v.z > m) {
            m = v.z;
        };
        m
    }

    //
    // Tests.
    //

    #[test_only]
    fun assert_near(a: i128, b: i128, tol: i128, code: u64) {
        assert!(abs(a - b) <= tol, code);
    }

    #[test]
    fun test_mul_round_trip() {
        assert!(fp_mul(SCALE, SCALE) == SCALE, 1);
        assert!(fp_mul(7 * SCALE, SCALE) == 7 * SCALE, 2);
        assert!(fp_mul(2 * SCALE, 3 * SCALE) == 6 * SCALE, 3);
        assert!(fp_mul(1500000, 1500000) == 2250000, 4);
        assert_near(fp_div(fp_mul(1234567, 7654321), 7654321), 1234567, 2, 5);
        assert!(fp_div(6 * SCALE, 3 * SCALE) == 2 * SCALE, 6);
    }

    #[test]
    fun test_mul_signs() {
        assert!(fp_mul(2 * SCALE, 3 * SCALE) == 6 * SCALE, 1);
        assert!(fp_mul(-2 * SCALE, 3 * SCALE) == -6 * SCALE, 2);
        assert!(fp_mul(2 * SCALE, -3 * SCALE) == -6 * SCALE, 3);
        assert!(fp_mul(-2 * SCALE, -3 * SCALE) == 6 * SCALE, 4);
        assert!(fp_div(2 * SCALE, -4 * SCALE) == -SCALE / 2, 5);
        assert!(fp_div(-2 * SCALE, -4 * SCALE) == SCALE / 2, 6);
    }

    #[test]
    fun test_division_truncates_toward_zero() {
        // The port relies on this: a shift would floor, and there is no
        // signed shift to reach for anyway.
        assert!((-7 * SCALE) / (2 * SCALE) == -3, 1);
        assert!((7 * SCALE) / (-2 * SCALE) == -3, 2);
        assert!((-7 * SCALE) % (2 * SCALE) == -1000000, 3);
        // -1e-6 * 1e-6 rounds to zero, not to the next value below.
        assert!(fp_mul(-1, 1) == 0, 4);
        assert!(fp_div(-10, 3 * SCALE) == -3, 5);
        assert!(fp_div(10, -3 * SCALE) == -3, 6);
    }

    #[test]
    fun test_abs_and_clamp() {
        assert!(abs(-5) == 5, 1);
        assert!(abs(5) == 5, 2);
        assert!(abs(0) == 0, 3);
        assert!(clamp01(-1) == 0, 4);
        assert!(clamp01(SCALE + 1) == SCALE, 5);
        assert!(clamp01(SCALE / 3) == SCALE / 3, 6);
    }

    #[test]
    fun test_isqrt_known_squares() {
        assert!(isqrt(0) == 0, 1);
        assert!(isqrt(1) == 1, 2);
        assert!(isqrt(4) == 2, 3);
        assert!(isqrt(1000000) == 1000, 4);
        assert!(isqrt(1000000000000) == 1000000, 5);
        assert!(isqrt(1u256 << 200) == 1u256 << 100, 6);
        // 2^255 - 1 is the largest value `isqrt_i256` can be handed.
        let big = (1u256 << 127) - 1;
        assert!(isqrt(big * big) == big, 7);
    }

    #[test]
    fun test_isqrt_floors_and_is_monotone() {
        let n = 0u256;
        let prev = 0u256;
        while (n < 400) {
            let r = isqrt(n);
            assert!(r * r <= n, 1);
            assert!((r + 1) * (r + 1) > n, 2);
            assert!(r >= prev, 3);
            prev = r;
            n = n + 1;
        }
    }

    #[test]
    fun test_fp_sqrt() {
        assert!(fp_sqrt(0) == 0, 1);
        assert!(fp_sqrt(SCALE) == SCALE, 2);
        assert!(fp_sqrt(4 * SCALE) == 2 * SCALE, 3);
        assert!(fp_sqrt(2 * SCALE) == 1414213, 4);
        assert!(fp_sqrt(SCALE / 4) == SCALE / 2, 5);
        // Small radicands still resolve: 1e-6 -> 1e-3.
        assert!(fp_sqrt(1) == 1000, 6);
    }

    #[test]
    #[expected_failure(abort_code = E_NEGATIVE_SQRT)]
    fun test_fp_sqrt_rejects_negative() {
        fp_sqrt(-1);
    }

    #[test]
    fun test_trig_axes() {
        let (c, s) = cos_sin_turn(0);
        assert_near(c, SCALE, 2, 1);
        assert_near(s, 0, 2, 2);
        let (c, s) = cos_sin_turn(SCALE / 4);
        assert_near(c, 0, 8, 3);
        assert_near(s, SCALE, 8, 4);
        let (c, s) = cos_sin_turn(SCALE / 2);
        assert_near(c, -SCALE, 8, 5);
        assert_near(s, 0, 8, 6);
        let (c, s) = cos_sin_turn(3 * SCALE / 4);
        assert_near(c, 0, 8, 7);
        assert_near(s, -SCALE, 8, 8);
    }

    #[test]
    fun test_trig_pythagorean() {
        // The identity holds everywhere on the circle only if the quadrant
        // fold and both series are right.
        let u = 0;
        while (u < SCALE) {
            let (c, s) = cos_sin_turn(u);
            assert_near(fp_mul(c, c) + fp_mul(s, s), SCALE, 50, 1);
            u = u + 6151;
        }
    }

    #[test]
    fun test_trig_known_angle() {
        // 1/8 turn is 45 degrees; both components are sqrt(2)/2.
        let (c, s) = cos_sin_turn(SCALE / 8);
        assert_near(c, 707107, 20, 1);
        assert_near(s, 707107, 20, 2);
        // 1/6 turn is 60 degrees. SCALE / 6 truncates, so the angle itself is
        // a few units short and the tolerance has to absorb that.
        let (c, s) = cos_sin_turn(SCALE / 6);
        assert_near(c, 500000, 32, 3);
        assert_near(s, 866025, 32, 4);
    }

    #[test]
    fun test_vec_arithmetic() {
        let a = vec3(1 * SCALE, 2 * SCALE, 3 * SCALE);
        let b = vec3(4 * SCALE, -5 * SCALE, 6 * SCALE);
        let s = add(&a, &b);
        assert!(dot(&s, &vec3(SCALE, 0, 0)) == 5 * SCALE, 1);
        let d = sub(&a, &b);
        assert!(dot(&d, &vec3(0, SCALE, 0)) == 7 * SCALE, 2);
        assert!(dot(&a, &b) == (4 - 10 + 18) * SCALE, 3);
        let m = mul(&a, &b);
        assert!(dot(&m, &vec3(0, 0, SCALE)) == 18 * SCALE, 4);
        let n = neg(&a);
        assert!(dot(&n, &vec3(SCALE, 0, 0)) == -SCALE, 5);
        assert!(max_component(&b) == 6 * SCALE, 6);
        assert!(max_component(&neg(&b)) == 5 * SCALE, 7);
    }

    #[test]
    fun test_cross_is_right_handed() {
        let x = vec3(SCALE, 0, 0);
        let y = vec3(0, SCALE, 0);
        let z = cross(&x, &y);
        assert!(dot(&z, &vec3(0, 0, SCALE)) == SCALE, 1);
        // Anti-commutative, and orthogonal to both inputs.
        let w = cross(&y, &x);
        assert!(dot(&w, &vec3(0, 0, SCALE)) == -SCALE, 2);
        let a = norm(&vec3(3 * SCALE, -4 * SCALE, 5 * SCALE));
        let b = norm(&vec3(-2 * SCALE, 7 * SCALE, SCALE));
        let c = cross(&a, &b);
        assert_near(dot(&c, &a), 0, 32, 3);
        assert_near(dot(&c, &b), 0, 32, 4);
    }

    #[test]
    fun test_norm() {
        let n = norm(&vec3(3 * SCALE, 4 * SCALE, 0));
        let (x, y, z) = parts(&n);
        assert!(x == 600000, 1);
        assert!(y == 800000, 2);
        assert!(z == 0, 3);
        // Unit length survives a vector spanning five orders of magnitude.
        let big = norm(&vec3(100000000000, -40800000, 81600000));
        assert_near(dot(&big, &big), SCALE, 16, 4);
        let zero = norm(&zero());
        assert!(dot(&zero, &zero) == 0, 5);
    }

    #[test]
    fun test_dot_raw_is_exact() {
        // The wall spheres square quantities near 1e5; the raw form keeps
        // every digit, whereas `dot` has already dropped six of them.
        let v = vec3(100000000000, 0, 0);
        assert!(dot_raw(&v, &v) == 10000000000000000000000, 1);
        assert!(dot(&v, &v) == 10000000000000000, 2);
        let a = vec3(3, 5, 7);
        assert!(dot_raw(&a, &a) == 83, 3);
        assert!(dot(&a, &a) == 0, 4);
    }

    #[test]
    fun test_div_by() {
        let v = div_by(&vec3(6 * SCALE, -9 * SCALE, 0), 3 * SCALE);
        let (x, y, z) = parts(&v);
        assert!(x == 2 * SCALE, 1);
        assert!(y == -3 * SCALE, 2);
        assert!(z == 0, 3);
    }
}

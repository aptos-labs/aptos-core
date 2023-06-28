
// An example taken from:
// https://move-language.github.io/move/structs-and-resources.html#example-2-geometry
// Unmodified other than adding a driver script.

module 0x100::point {
    struct Point has copy, drop, store {
        x: u64,
        y: u64,
    }

    public fun new(x: u64, y: u64): Point {
        Point {
            x, y
        }
    }

    public fun x(p: &Point): u64 {
        p.x
    }

    public fun y(p: &Point): u64 {
        p.y
    }

    fun abs_sub(a: u64, b: u64): u64 {
        if (a < b) {
            b - a
        }
        else {
            a - b
        }
    }

    public fun dist_squared(p1: &Point, p2: &Point): u64 {
        let dx = abs_sub(p1.x, p2.x);
        let dy = abs_sub(p1.y, p2.y);
        dx*dx + dy*dy
    }
}

module 0x100::circle {
    use 0x100::point::{Self, Point};

    struct Circle has copy, drop, store {
        center: Point,
        radius: u64,
    }

    public fun new(center: Point, radius: u64): Circle {
        Circle { center, radius }
    }

    public fun overlaps(c1: &Circle, c2: &Circle): bool {
        let d = point::dist_squared(&c1.center, &c2.center);
        let r1 = c1.radius;
        let r2 = c2.radius;
        d*d <= r1*r1 + 2*r1*r2 + r2*r2
    }
}


script {
    use 0x100::circle;
    use 0x100::point;

    fun main() {
        let p1 = point::new(12, 34);
        assert!(point::x(&p1) == 12, 0xf00);
        assert!(point::y(&p1) == 34, 0xf01);

        let p2 = point::new(0, 10);
        let p3 = point::new(10, 0);
        assert!(point::dist_squared(&p2, &p3) == 200, 0xf02);

        let c1 = circle::new(point::new(0, 0), 30);
        let c2 = circle::new(point::new(10, 0), 100);
        assert!(circle::overlaps(&c1, &c2), 0xf03);
    }
}

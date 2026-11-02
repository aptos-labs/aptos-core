// Values without `copy`: passed and returned by value, their fields read in
// place, moved between locals, destructured, tested for a variant, and
// matched.

// RUN: publish
module 0x4e::moves {
    struct Inner has drop {
        v: u64,
    }

    struct Outer has drop {
        inner: Inner,
        w: u64,
    }

    enum Shape has drop {
        Circle { r: u64 },
        Square { side: u64 },
    }

    fun make(v: u64, w: u64): Outer {
        Outer { inner: Inner { v }, w }
    }

    fun take(s: Outer): u64 {
        s.inner.v + s.w
    }

    fun area(shape: Shape): u64 {
        match (shape) {
            Shape::Circle { r } => 3 * r * r,
            Shape::Square { side } => side * side,
        }
    }

    public fun pass(v: u64, w: u64): u64 {
        let s = make(v, w);
        take(s)
    }

    public fun fields(v: u64, w: u64): u64 {
        let s = make(v, w);
        let a = s.inner.v;
        let b = s.w;
        a * b + s.inner.v
    }

    public fun moved(v: u64): u64 {
        let s = make(v, 1);
        let t = s;
        take(t)
    }

    public fun unpacked(v: u64, w: u64): u64 {
        let Outer { inner: Inner { v: a }, w: b } = make(v, w);
        a - b
    }

    public fun shapes(r: u64, side: u64): u64 {
        let c = Shape::Circle { r };
        let s = Shape::Square { side };
        if (c is Shape::Circle) area(c) + area(s) else 0
    }
}

// RUN: execute 0x4e::moves::pass --args 3, 4
// RUN: execute 0x4e::moves::fields --args 3, 4
// RUN: execute 0x4e::moves::moved --args 5
// RUN: execute 0x4e::moves::unpacked --args 9, 4
// RUN: execute 0x4e::moves::shapes --args 2, 5

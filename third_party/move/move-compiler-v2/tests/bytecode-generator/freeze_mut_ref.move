module 0x42::freeze_mut_ref {
    use std::vector;

    public fun borrow_mut<Element>(
        map: &mut vector<Element>,
    ): &Element {
        vector::borrow_mut(map, 0)
    }

    public fun borrow_mut2<Element>(
        v: &mut Element,
    ): &Element {
        v
    }

    public fun borrow_mut3<Element>(
        v1: &mut Element,
        v2: & Element
    ): &Element {
        if (true)
            v1
        else
            v2
    }

    public fun borrow_mut4<Element>(
        v: &mut Element,
    ): &Element {
        return v
    }

    struct S has drop {

    }

    fun t0() {
        let x: &u64 = &mut 0; x;
    }

    fun t1(s: &mut S): &S {
        s
    }

    fun t2(u1: &mut u64, u2: &mut u64): (&u64, &mut u64) {
        (u1, u2)
    }

    // TODO: this case is not handled
    // fun t3() {
    //     let x: &u64;
    //     let y: &u64;
    //     (x, y) = t2(&mut 3, &mut 4);
    // }

    public fun t4() {
        let x: &u64;
        let y: &u64;
        (x, y) = (&mut 0, &mut 0);
    }

    struct G { f: u64 }

    public fun t5(s: &mut G) {
        let x = 0;
        let f = &mut ({x = x + 1; s}).f;
        let g = &mut ({x = x + 1; s}).f;
        let y = &mut 2;
        let z: &u64;
        *({*f = 0; z = y; g}) = 2;
    }

    fun t6(cond: bool, s: &mut S, other: &S) {
        let x: &S;
        if (cond) x = copy s else x = other;
    }

    fun t7(cond: bool, s: &mut S, other: &S) {
        let _x;
        _x = if (cond) s else other;
    }

    fun t8(cond: bool, s: &mut S, other: &S) {
        let _x: &S = if (cond) s else other;
    }

}

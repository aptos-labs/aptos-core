module 0xcafe::m {

    struct S {
        x: u64
    }

    /// A higher order function on ints
    fun map(x: u64, f: |u64|u64): u64 {
        f(x)
    }

    fun assigns_param(x: u64, c: u64): u64 {
        map(x, |y| {
            x = 2;
            y + c
        })
    }

    fun borrows_param(x: u64, c: u64): u64 {
        map(x, |y| {
            let r = &mut c;
            y + *r
        })
    }

    fun assigns_local(x: u64, c: u64): u64 {
        let z = 1;
        map(x, |y| {
            z = 2;
            y + c
        })
    }

    fun borrows_local(x: u64): u64 {
        let z = 1;
        map(x, |y| {
            let r = &mut z;
            y + *r
        })
    }

    fun immutable_borrow_ok(x: u64): u64 {
        let z = 1;
        map(x, |y| {
            let r = &z;
            y + *r
        })
    }
}

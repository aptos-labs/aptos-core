module 0xcafe::m {

    public fun test1(): u64 {
        let x = 0;
        let r1 = &mut x;
        let r2 = &mut x;
        *r1 + *r2
    }

    public fun test2(): u64 {
        let x = 0;
        let r1 = &mut x;
        let r2 = &mut x;
        *r2 + *r1 // <- changed order here
    }
}

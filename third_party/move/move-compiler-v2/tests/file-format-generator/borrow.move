module 0x42::borrow {

    struct S {
        f: u64
    }

    fun param(param: u64): u64 {
        let r = &param;
        *r
    }

    fun local(param: u64): u64 {
        let local: u64 = 33;
        let r = &local;
        *r
    }

    fun field(s: &S): u64 {
        let r = &s.f;
        *r
    }

    fun mut_param(param: u64): u64 {
        let r = &mut param;
        *r = 22;
        *r
    }

    fun mut_local(param: u64): u64 {
        let local: u64 = 33;
        let r = &mut local;
        *r = 22;
        *r
    }

    fun mut_field(s: &mut S): u64 {
        let r = &mut s.f;
        *r = 22;
        *r
    }
}

module 0x42::reference_conversion {

    fun deref(r: &u64): u64 {
        *r
    }

    fun use_it(): u64 {
        let x = 42;
        let r = &mut x;
        *r = 43;
        deref(r)
    }


}

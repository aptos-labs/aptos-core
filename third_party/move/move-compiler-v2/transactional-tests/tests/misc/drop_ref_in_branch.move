//# publish
module 0x42::m {

    struct R has key { v: u64 }

    fun do() acquires R {
        let r = borrow_global_mut<R>(@0x42);
        let vr = get_vmut(r);
        if (*vr > 0) {
            *vr = 0;
        } else {
            // vr alive and should be dropped here, so we can use r
            some(r)
        }
    }

    fun some(_r: &mut R) {
    }

    fun get_vmut(r: &mut R): &mut u64 {
        &mut r.v
    }
}

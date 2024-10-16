module 0xcafe::m {

    struct R<phantom T> has key, copy, drop { }

    fun reads_any_R(addr: address) reads R {
        let _x = borrow_global<R<u64>>(addr);
    }

    fun reads_any_cafe(addr: address) reads 0xcafe::*::* {
        let _x = borrow_global<R<u64>>(addr);
    }

    fun reads_any_m(addr: address) reads 0xcafe::m::* {
        let _x = borrow_global<R<u64>>(addr);
    }

    fun reads_not_any_m(addr: address) !reads 0xcafe::m::* {
       let _x = borrow_global<R<u64>>(addr);
    }

    fun writes_any_R_u64(addr: address) writes R<u64>(addr) {
        let _x = borrow_global_mut<R<u64>>(addr);
    }

    fun t0_invalid(addr: address) acquires R {
        let _r1 = borrow_global_mut<R<u64>>(addr);
        reads_any_R(addr);
        reads_any_cafe(addr);
        reads_any_m(addr);
        reads_not_any_m(addr); // no error
        *_r1;
    }

    fun t1_valid(addr: address) acquires R {
        let _r1 = borrow_global<R<u64>>(addr);
        reads_any_R(addr);
        reads_any_cafe(addr);
        reads_any_m(addr);
        reads_not_any_m(addr);
        *_r1;
    }

    fun t2_invalid(addr: address) acquires R {
        let _r1 = borrow_global<R<u64>>(addr);
        writes_any_R_u64(addr);
        *_r1;
    }

    fun t3_valid(addr: address) acquires R {
        let _r1 = borrow_global<R<u128>>(addr);
        writes_any_R_u64(addr);
        *_r1;
    }

}

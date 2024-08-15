module 0x8675309::Tester {

    fun foo(_other: &mut u64): &mut u64 {
        _other
    }

    fun test(result: &mut u64): &u64 {
        let returned_ref = foo(result);
        freeze(result);
        returned_ref
    }

    fun test_return_ref_no_use(result: &mut u64) {
        let _returned_ref = foo(result);
        freeze(result);
    }

}

module 0x8675309::Tester {

    fun foo(_other: &mut u64): &mut u64 {
        _other
    }

    fun test(result: &mut u64): &u64 {
        let returned_ref = foo(result);
        freeze(result);
        returned_ref
    }
}

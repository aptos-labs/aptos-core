// TODO(#13927): after fix, rename file to reflect issue (`bug_nnn.move` to `bug_nnn_<issue>.move`)
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

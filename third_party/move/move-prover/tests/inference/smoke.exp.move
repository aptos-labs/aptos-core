/// Smoke test for the inference end-to-end test harness.
module 0x1::smoke {

    fun add(a: u64, b: u64): u64 {
        a + b
    }
    spec add(a: u64, b: u64): u64 {
        ensures [inferred] result == a + b;
        aborts_if [inferred] a + b > MAX_U64;
    }


    fun id(x: u64): u64 {
        x
    }
    spec id(x: u64): u64 {
        ensures [inferred] result == x;
    }

}
/*
Verification: Succeeded.
*/

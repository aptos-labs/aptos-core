module 0x42::test {


    fun exec(f: |u64|u64, x: u64): u64 {
        f(x)
    }

    // We can prove the functions below because `exec` is inlined in Boogie and
    // the function value is known in the inlining context.

    fun call_exec1_ok(x: u64): u64 {
        exec(|y| x + y, 3)
    }
    spec call_exec1_ok {
        ensures result == x + 3;
    }

    fun call_exec2_ok(x: u64): u64 {
        exec(|y| y + 1, x)
    }
    spec call_exec2_ok {
        ensures result == x + 1;
    }

    fun call_exec3_fail(x: u64): u64 {
        exec(|y| y + 2, x)
    }
    spec call_exec3_fail {
        // Expected to fail since result == x + 2
        ensures result == x + 1;
    }

    fun call_exec4_ok(x: u64): u64 {
        // Let doesn't matter
        let f = |y| y + 1;
        exec(f, x)
    }
    spec call_exec4_ok {
        ensures result == x + 1;
    }
}

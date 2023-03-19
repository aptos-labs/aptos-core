module 0x42::loop_unroll {
    fun t1_success(): u64 {
        let i = 0;
        while ({
            spec {
                invariant [unroll = 3] true;
            };
            (i < 5)
        }) {
            i = i + 1;
        };
        i
    }
    spec t1_success {
        // this is expecteds, as the loop never terminutes within 3 unrolling
        ensures false;
    }

    fun t2_success(): u64 {
        let i = 0;
        while ({
            // it is okay to unroll more than the loop iterations, it won't
            // affect the correctness
            spec {
                invariant [unroll = 6] true;
            };
            (i < 5)
        }) {
            i = i + 1;
        };
        i
    }
    spec t2_success {
        ensures result == 5;
    }

    fun t3_success(n: u64): u64 {
        let i = 0;
        while ({
            spec {
                invariant [unroll = 3] true;
            };
            (i < n)
        }) {
            i = i + 1;
        };
        i
    }
    spec t3_success {
        ensures result == n;
    }

    fun t4_success(n: u64): u64 {
        let i = 0;
        while ({
            spec {
                invariant [unroll = 3] true;
            };
            (i < n)
        }) {
            i = i + 1;
        };
        i
    }
    spec t4_success {
        // this is expected, as we only unroll the loop 3 times
        ensures result <= 3;
    }

    fun t5_success(n: u64): u64 {
        let i = 0;
        while ({
            spec {
                invariant [unroll = 3] true;
            };
            (i < n)
        }) {
            i = i + 1;
            assert!(i != 5, 0);
        };
        i
    }
    spec t5_success {
        // this is expected, as we only unroll the loop 3 times
        aborts_if false;
    }

    fun t6_failure(n: u64): u64 {
        let i = 0;
        while ({
            spec {
                invariant [unroll = 6] true;
            };
            (i < n)
        }) {
            i = i + 1;
            assert!(i != 5, 0);
        };
        i
    }
    spec t6_failure {
        // this will not hold when we increase the unroll count
        aborts_if false;
    }
}

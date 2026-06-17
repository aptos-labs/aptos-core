// Tests an inline function whose loop-based summation is abstracted by an
// opaque spec with the closed form (Gauss formula): the body is verified once
// against the formula, using loop invariants inside the inline function, and
// callers reason with the formula only. `sum_trusted` opts out of the body
// verification with `pragma verify = false`; its spec is trusted.
module 0x42::opaque_inline_loop_sum {

    inline fun sum(n: u64): u64 {
        let s = 0;
        let i = 0;
        while (i <= n) {
            s = s + i;
            i = i + 1;
        } spec {
            invariant i <= n + 1;
            invariant s == i * (i - 1) / 2;
        };
        s
    }
    spec sum {
        pragma opaque;
        requires n < (1 << 32);
        aborts_if false;
        ensures result == n * (n + 1) / 2;
    }

    inline fun sum_trusted(n: u64): u64 {
        let s = 0;
        let i = 0;
        while (i <= n) {
            s = s + i;
            i = i + 1;
        };
        s
    }
    spec sum_trusted {
        pragma opaque;
        pragma verify = false;
        requires n < (1 << 32);
        aborts_if false;
        ensures result == n * (n + 1) / 2;
    }

    fun test_sum_concrete(): u64 {
        sum(10)
    }
    spec test_sum_concrete {
        ensures result == 55;
    }

    fun test_sum_symbolic(n: u64): u64 {
        sum(n)
    }
    spec test_sum_symbolic {
        requires n < 1000;
        ensures result == n * (n + 1) / 2;
    }

    fun test_sum_twice(n: u64): u64 {
        sum(n) + sum(n)
    }
    spec test_sum_twice {
        requires n < 1000;
        ensures result == n * (n + 1);
    }

    fun test_sum_trusted(n: u64): u64 {
        sum_trusted(n)
    }
    spec test_sum_trusted {
        requires n < 1000;
        ensures result == n * (n + 1) / 2;
    }

    fun test_sum_wrong(n: u64): u64 {
        sum(n)
    }
    spec test_sum_wrong {
        requires n < 1000;
        ensures result == n * (n + 1); // error: post-condition does not hold
    }
}

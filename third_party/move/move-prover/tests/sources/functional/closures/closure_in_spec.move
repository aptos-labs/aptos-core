module 0x42::test {
    fun simple(x: u64, p: |u64|bool): bool {
        p(x)
    }
    spec simple {
        ensures result == p(x);
    }

    fun using_simple(): bool {
        simple(1, |x| x == 1)
    }
    spec using_simple {
        ensures result;
    }

    fun using_simple_fail(): bool {
        simple(1, |x| x != 1)
    }
    spec using_simple_fail {
        ensures result; // expected to fail
    }
}

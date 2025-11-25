module 0xc0ffee::m {
    fun test1_warn() {
        test1_warn();
    }



    fun test2_a_warn() {
        test2_b_warn();
    }

    fun test2_b_warn() {
        test2_a_warn();
    }



    fun test3_a_warn() {
        test3_b_warn();
    }

    fun test3_b_warn() {
        test3_c_warn();
    }

    fun test3_c_warn() {
        test3_d_warn();
    }

    fun test3_d_warn() {
        test3_a_warn();
    }



    fun test4_a_warn(x: u64, limit: u64) {
        if (limit < 1 || limit < 2) {
            abort 0
        };
        test4_a_warn(collatz(x), limit - 1);
    }

    fun collatz(n: u64): u64 {
        if (n % 2 == 0) {
            n / 2
        } else {
            n * 3 + 1
        }
    }
}

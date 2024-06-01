module 0xc0ffee::m {
    fun always_abort() {
        abort 0
    }

    fun test(): u64 {
        always_abort();
        let i = 0; // intraprocedural analysis cannot prove this is unreachable
        i = i + 1;
        i
    }

}

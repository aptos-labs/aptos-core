// 1-arg form `debug_assert!(cond)`. In non-test mode the arguments must not
// be evaluated, so `touched` is never referenced (see paired `no_test.exp`).
module 0x42::m {
    fun touched(b: &mut bool) {
        *b = true;
    }

    public fun check_active(b: bool) {
        debug_assert!(b);
    }

    public fun check_args_not_evaluated() {
        let flag = false;
        let dummy = true;
        debug_assert!(dummy && { touched(&mut flag); flag });
    }
}

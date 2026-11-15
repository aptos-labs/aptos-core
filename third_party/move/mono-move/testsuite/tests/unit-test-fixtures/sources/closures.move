/// Stores a function value, which MonoMove cannot serialize yet.
module fixtures::closures {
    struct Holder has key {
        apply: |u64| u64 has copy + drop + store,
    }

    public fun double(value: u64): u64 {
        value * 2
    }

    #[test(account = @fixtures)]
    fun stores_a_closure(account: &signer) {
        move_to(account, Holder { apply: double });
    }

    #[test(account = @fixtures)]
    #[expected_failure(abort_code = 7, location = Self)]
    fun aborts_after_storing_a_closure(account: &signer) {
        move_to(account, Holder { apply: double });
        abort 7
    }
}

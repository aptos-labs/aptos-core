module 0xABCD::scheduled_txns_perf {
    use std::signer;
    use std::option::Option;
    use aptos_std::debug;
    use std::string;
    use aptos_framework::scheduled_txns;

    struct State has copy, store, drop {
        value: u64
    }

    #[persistent]
    fun step(state: State, _s: Option<signer>) {
        //debug::print(&string::utf8(b"Move: in the func step with value"));
        //debug::print(&state.value);
        if (state.value < 10) {
            state.value = state.value + 1;
        }
    }

    #[persistent]
    fun no_op(_s: Option<signer>) {
        // This function does nothing, it is just a placeholder for testing purposes.
    }

    public entry fun test_insert_transactions(user: &signer, current_time_ms: u64) {
        //debug::print(&string::utf8(b"test_insert_transactions"));

        //let state1 = State { value: 1 };
        //let foo1 = |s: Option<signer>| step(state1, s);
        let foo_no_op = |s: Option<signer>| no_op(s);
        let txn1 = scheduled_txns::new_scheduled_transaction(
            signer::address_of(user),
            current_time_ms,
            10000,
            150,
            false,
            foo_no_op
        );

        scheduled_txns::insert(user, txn1);
    }
}

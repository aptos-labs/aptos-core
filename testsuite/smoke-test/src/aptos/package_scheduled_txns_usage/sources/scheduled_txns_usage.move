module 0xA550C18::scheduled_txns_usage {
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
        debug::print(&string::utf8(b"Move: in the func step with value"));
        debug::print(&state.value);
        if (state.value < 10) {
            state.value = state.value + 1;
        }
    }

    public entry fun test_insert_transactions(user: &signer, current_time_ms: u64) {
        debug::print(&string::utf8(b"test_insert_transactions"));

        let state1 = State { value: 1 };
        let foo1 = |s: Option<signer>| step(state1, s);
        let txn1 = scheduled_txns::new_scheduled_transaction(
            signer::address_of(user),
            current_time_ms + 2000,
            10000,
            200,
            false,
            foo1
        );

        let state2 = State { value: 2 };
        let foo2 = |s: Option<signer>| step(state2, s);
        let txn2 = scheduled_txns::new_scheduled_transaction(
            signer::address_of(user),
            current_time_ms + 2000,
            10000,
            300,
            false,
            foo2
        );

        let state3 = State { value: 3 };
        let foo3 = |s: Option<signer>| step(state3, s);
        let txn3 = scheduled_txns::new_scheduled_transaction(
            signer::address_of(user),
            current_time_ms + 2000,
            10000,
            200,
            false,
            foo3
        );

        scheduled_txns::insert(user, txn1);
        scheduled_txns::insert(user, txn2);
        scheduled_txns::insert(user, txn3);
    }

    public entry fun test_cancel_transaction(_user: &signer) {
        //scheduled_txns::cancel(user, key);
    }

    public entry fun test_shutdown(aptos: &signer) {
        scheduled_txns::shutdown(aptos);
    }
}

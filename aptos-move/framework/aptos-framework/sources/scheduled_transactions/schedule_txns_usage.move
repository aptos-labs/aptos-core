module aptos_framework::schedule_txns_usage {
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
        if (state.value < 10) {
            state.value = state.value + 1;
        }
    }

    public entry fun test_initialize(aptos: &signer) {
        scheduled_txns::initialize(aptos);
    }

    public entry fun test_insert_transactions(user: &signer, current_time_ms: u64) {
        debug::print(&string::utf8(b"test_insert_transactions"));
        let state = State { value: 8 };
        let foo = |s: Option<signer>| step(state, s);

        let txn1 = scheduled_txns::new_scheduled_transaction(
            signer::address_of(user),
            current_time_ms + 100000,
            0,
            20,
            false,
            foo
        );
        let txn2 = scheduled_txns::new_scheduled_transaction(
            signer::address_of(user),
            current_time_ms + 200000,
            0,
            20,
            false,
            foo
        );
        let txn3 = scheduled_txns::new_scheduled_transaction(
            signer::address_of(user),
            current_time_ms + 300000,
            0,
            20,
            false,
            foo
        );

        scheduled_txns::insert(user, txn1);
        scheduled_txns::insert(user, txn2);
        scheduled_txns::insert(user, txn3);

        //assert!(3 == scheduled_txns::get_num_txns(), scheduled_txns::get_num_txns());
    }

    public entry fun test_cancel_transaction(user: &signer) {
        //scheduled_txns::cancel(user, key);
    }

    public entry fun test_shutdown(aptos: &signer) {
        scheduled_txns::shutdown(aptos);
    }
}

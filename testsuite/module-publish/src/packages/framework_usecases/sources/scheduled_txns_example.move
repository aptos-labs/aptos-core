module 0xABCD::scheduled_txns_example {
    use std::signer;
    use std::option::Option;
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
        let state = State { value: 8 };
        let foo = |s: Option<signer>| step(state, s);

        let txn1 = scheduled_txns::new_scheduled_transaction(
            signer::address_of(user),
            current_time_ms + 1000,
            0,
            20,
            false,
            foo
        );
        let txn2 = scheduled_txns::new_scheduled_transaction(
            signer::address_of(user),
            current_time_ms + 2000,
            0,
            20,
            false,
            foo
        );
        let txn3 = scheduled_txns::new_scheduled_transaction(
            signer::address_of(user),
            current_time_ms + 3000,
            0,
            20,
            false,
            foo
        );

        scheduled_txns::insert(user, txn1);
        scheduled_txns::insert(user, txn2);
        scheduled_txns::insert(user, txn3);
    }

    public entry fun test_cancel_transaction(user: &signer) {
        //scheduled_txns::cancel(user, key);
    }

    public entry fun test_shutdown(aptos: &signer) {
        scheduled_txns::shutdown(aptos);
    }
}

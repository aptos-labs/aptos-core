module 0xA550C18::scheduled_txns_usage {
    use std::signer;
    use std::option::Option;
    use aptos_std::debug;
    use std::string;
    use aptos_framework::scheduled_txns;
    use aptos_std::big_ordered_map::{Self, BigOrderedMap};

    struct State has copy, store, drop {
        value: u64
    }

    /// Resource to store the scheduled transaction keys
    struct StoredScheduledTxns has key {
        txns: BigOrderedMap<scheduled_txns::ScheduleMapKey, ScheduledTransactionInfo>
    }

    struct ScheduledTransactionInfo has copy, drop, store {
        sender_addr: address,
        max_gas_amount: u64,
        gas_unit_price: u64
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

        let user_addr = signer::address_of(user);
        let txn_map = big_ordered_map::new<scheduled_txns::ScheduleMapKey, ScheduledTransactionInfo>();

        let state1 = State { value: 1 };
        let foo1 = |s: Option<signer>| step(state1, s);
        let txn1 = scheduled_txns::new_scheduled_transaction(
            user_addr,
            current_time_ms + 1500,
            10000,
            200,
            false,
            foo1
        );
        let key1 = scheduled_txns::insert(user, txn1);
        let txn_info1 = ScheduledTransactionInfo {
            sender_addr: user_addr,
            max_gas_amount: 10000,
            gas_unit_price: 200
        };
        big_ordered_map::add(&mut txn_map, key1, txn_info1);

        let state2 = State { value: 2 };
        let foo2 = |s: Option<signer>| step(state2, s);
        let txn2 = scheduled_txns::new_scheduled_transaction(
            user_addr,
            current_time_ms + 1500,
            10000,
            300,
            false,
            foo2
        );
        let key2 = scheduled_txns::insert(user, txn2);
        let txn_info2 = ScheduledTransactionInfo {
            sender_addr: user_addr,
            max_gas_amount: 10000,
            gas_unit_price: 300
        };
        big_ordered_map::add(&mut txn_map, key2, txn_info2);

        let state3 = State { value: 3 };
        let foo3 = |s: Option<signer>| step(state3, s);
        let txn3 = scheduled_txns::new_scheduled_transaction(
            user_addr,
            current_time_ms + 1500,
            10000,
            200,
            false,
            foo3
        );
        let key3 = scheduled_txns::insert(user, txn3);
        let txn_info3 = ScheduledTransactionInfo {
            sender_addr: user_addr,
            max_gas_amount: 10000,
            gas_unit_price: 200
        };
        big_ordered_map::add(&mut txn_map, key3, txn_info3);

        // Store the transaction infos in StoredScheduledTxns
        move_to(user, StoredScheduledTxns { txns: txn_map });
    }

    public entry fun test_cancel_transaction(user: &signer) acquires StoredScheduledTxns {
        // Get the first key from the BigOrderedMap
        let user_addr = signer::address_of(user);
        let stored_txns = borrow_global<StoredScheduledTxns>(user_addr);
        let (first_key, _value) = stored_txns.txns.borrow_front();

        // Cancel the scheduled transaction using the first key
        scheduled_txns::cancel(user, first_key);
    }

    public entry fun test_shutdown(core_resources: &signer) {
        // Use governance to get the framework signer
        let framework_signer = aptos_framework::aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        scheduled_txns::start_shutdown(&framework_signer);
    }
}

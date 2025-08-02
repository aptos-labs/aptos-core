module 0xCAFE::scheduled_txns_usage {
    use std::signer;
    use std::option::Option;
    use aptos_std::debug;
    use std::string;
    use std::vector;
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

    struct ScheduledTransactionInfoWithKey has copy, drop, store {
        sender_addr: address,
        max_gas_amount: u64,
        gas_unit_price: u64,
        key: scheduled_txns::ScheduleMapKey
    }

    #[persistent]
    fun step(state: State, _s: Option<signer>) {
        debug::print(&string::utf8(b"Move: in the func step with val"));
        debug::print(&state.value);
        if (state.value < 10) {
            state.value = state.value + 1;
        }
    }

    #[persistent]
    fun user_func_abort(_s: Option<signer>) {
        debug::print(&string::utf8(b"Move: in the user_func_abort"));
        assert!(false, 0x1);
    }

    public entry fun create_and_add_transactions(
        user: &signer,
        current_time_ms: u64,
        values: vector<u64>,
        gas_amounts: vector<u64>,
        gas_prices: vector<u64>
    ) {
        let user_addr = signer::address_of(user);
        let num_txns = values.length();

        assert!(gas_amounts.length() == num_txns, 0x1);
        assert!(gas_prices.length() == num_txns, 0x2);

        let txn_map =
            big_ordered_map::new<scheduled_txns::ScheduleMapKey, ScheduledTransactionInfo>();

        let i = 0;
        while (i < num_txns) {
            let state = State { value: values[i] };
            let foo = |s: Option<signer>| step(state, s);
            let txn_time = current_time_ms + 1500 * (i + 1);
            let gas_unit_price = *vector::borrow(&gas_prices, i);
            let txn =
                scheduled_txns::new_scheduled_transaction(
                    user_addr,
                    txn_time,
                    *vector::borrow(&gas_amounts, i),
                    gas_unit_price,
                    false,
                    foo
                );

            let key = scheduled_txns::insert(user, txn);

            let txn_info = ScheduledTransactionInfo {
                sender_addr: user_addr,
                max_gas_amount: *vector::borrow(&gas_amounts, i),
                gas_unit_price: *vector::borrow(&gas_prices, i)
            };
            big_ordered_map::add(&mut txn_map, key, txn_info);

            i = i + 1;
        };

        // Store the transaction infos in StoredScheduledTxns
        move_to(user, StoredScheduledTxns { txns: txn_map });
    }

    public entry fun add_txn_with_user_func_abort(
        user: &signer, current_time_ms: u64
    ) {
        let gas_amount = 10000;
        let gas_unit_price = 200;
        let user_addr = signer::address_of(user);

        let foo_abort = |s: Option<signer>| user_func_abort(s);
        let txn_time = current_time_ms + 1500;

        let txn =
            scheduled_txns::new_scheduled_transaction(
                user_addr,
                txn_time,
                gas_amount,
                gas_unit_price,
                false,
                foo_abort
            );

        let key = scheduled_txns::insert(user, txn);

        // Create a ScheduledTransactionInfo and store it
        let txn_info = ScheduledTransactionInfo {
            sender_addr: user_addr,
            max_gas_amount: gas_amount,
            gas_unit_price: gas_unit_price
        };

        // Store the transaction info in StoredScheduledTxns
        let txn_map =
            big_ordered_map::new<scheduled_txns::ScheduleMapKey, ScheduledTransactionInfo>();
        big_ordered_map::add(&mut txn_map, key, txn_info);
        move_to(user, StoredScheduledTxns { txns: txn_map });
    }

    public entry fun cancel_txn(user: &signer) acquires StoredScheduledTxns {
        // Get the first key from the BigOrderedMap
        let user_addr = signer::address_of(user);
        let stored_txns = borrow_global<StoredScheduledTxns>(user_addr);
        let (first_key, _value) = stored_txns.txns.borrow_front();

        // Cancel the scheduled transaction using the first key
        scheduled_txns::cancel(user, first_key);
    }

    #[view]
    public fun get_stored_sched_txns(
        addr: address
    ): vector<ScheduledTransactionInfoWithKey> acquires StoredScheduledTxns {
        let stored_txns = borrow_global<StoredScheduledTxns>(addr);
        let result = vector::empty<ScheduledTransactionInfoWithKey>();

        stored_txns.txns.for_each_ref(
            |key, value| {
                let txn_info_with_key =
                    ScheduledTransactionInfoWithKey {
                        sender_addr: value.sender_addr,
                        max_gas_amount: value.max_gas_amount,
                        gas_unit_price: value.gas_unit_price,
                        key: *key
                    };
                result.push_back(txn_info_with_key);
            }
        );

        result
    }
}

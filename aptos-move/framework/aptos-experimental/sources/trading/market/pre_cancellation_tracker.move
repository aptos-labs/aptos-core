module aptos_experimental::pre_cancellation_tracker {
    use std::signer;
    use aptos_std::big_ordered_map;
    use aptos_std::big_ordered_map::BigOrderedMap;
    use aptos_experimental::order_book_types::{
        AccountClientOrderId,
        new_account_client_order_id
    };
    #[test_only]
    use std::vector;
    #[test_only]
    use aptos_framework::timestamp;

    const DUPLICATE_ORDER_PLACEMENT: u64 = 1;

    const MAX_ORDERS_GARBAGE_COLLECTED_PER_CALL: u64 = 10;

    struct PreCancellationTracker has store {
        pre_cancellation_window_micros: u64,
        // Map of order IDs with expiration time to a boolean indicating if the order is active.
        order_ids_with_expiration: BigOrderedMap<OrderIdWithExpiration, bool>,
        // Map of order Ids to their corresponding expiration time.
        account_order_ids: BigOrderedMap<AccountClientOrderId, u64>
    }

    struct OrderIdWithExpiration has copy, drop, store {
        expiration_time: u64,
        account_order_id: AccountClientOrderId
    }

    public fun new_pre_cancellation_tracker(expiration_time_micros: u64): PreCancellationTracker {
        PreCancellationTracker {
            pre_cancellation_window_micros: expiration_time_micros,
            order_ids_with_expiration: big_ordered_map::new_with_reusable(),
            account_order_ids: big_ordered_map::new_with_reusable()
        }
    }

    public fun pre_cancel_order_for_tracker(
        tracker: &mut PreCancellationTracker,
        account: &signer,
        client_order_id: u64
    ) {
        garbage_collect(tracker);
        let account_order_id = new_account_client_order_id(signer::address_of(account), client_order_id);
        if (tracker.account_order_ids.contains(&account_order_id)) {
            // If the account_order_id already exists with a previously set expiration time,
            // we update the expiration time.
            let expiration_time = tracker.account_order_ids.remove(&account_order_id);
            let order_id_with_expiration =
                OrderIdWithExpiration { expiration_time, account_order_id };
            // If the mapping exists, then we remove the order ID with its expiration time.
            tracker.order_ids_with_expiration.remove(&order_id_with_expiration);
        };
        let current_time = aptos_std::timestamp::now_microseconds();
        let expiration_time = current_time + tracker.pre_cancellation_window_micros;
        let order_id_with_expiration = OrderIdWithExpiration {
            expiration_time,
            account_order_id
        };
        tracker.account_order_ids.add(account_order_id, expiration_time);
        tracker.order_ids_with_expiration.add(order_id_with_expiration, true);
    }

    public fun is_pre_cancelled(
        tracker: &mut PreCancellationTracker,
        account: address,
        client_order_id: u64
    ): bool {
        garbage_collect(tracker);
        let account_order_id = new_account_client_order_id(account, client_order_id);
        let current_time = aptos_std::timestamp::now_microseconds();
        if (tracker.account_order_ids.contains(&account_order_id)) {
            let expiration_time =
                tracker.account_order_ids.get(&account_order_id).destroy_some();
            if (current_time > expiration_time) {
                // This is possible as garbage collection may not be able to garbage collect all expired orders
                // in a single call.
                tracker.account_order_ids.remove(&account_order_id);
                let order_id_with_expiration =
                    OrderIdWithExpiration { expiration_time, account_order_id };
                tracker.order_ids_with_expiration.remove(&order_id_with_expiration);
            } else {
                return true; // Order ID already exists with a valid expiration time.
            }
        };
        return false
    }

    public fun garbage_collect(tracker: &mut PreCancellationTracker) {
        let i = 0;
        let current_time = aptos_std::timestamp::now_microseconds();
        while (i < MAX_ORDERS_GARBAGE_COLLECTED_PER_CALL
            && !tracker.order_ids_with_expiration.is_empty()) {
            let (front_k, _) = tracker.order_ids_with_expiration.borrow_front();
            // We garbage collect a nonce after it has expired and the NONCE_REPLAY_PROTECTION_OVERLAP_INTERVAL_SECS
            // seconds have passed.
            if (front_k.expiration_time < current_time) {
                tracker.order_ids_with_expiration.pop_front();
                tracker.account_order_ids.remove(&front_k.account_order_id);
            } else {
                break;
            };
            i += 1;
        };
    }

    #[test_only]
    public fun destroy_tracker(tracker: PreCancellationTracker) {
        // This function is used to destroy the tracker in tests.
        // In production, the tracker will be garbage collected automatically.
        let PreCancellationTracker {
            pre_cancellation_window_micros: _,
            order_ids_with_expiration,
            account_order_ids
        } = tracker;
        order_ids_with_expiration.destroy(|_v| {});
        account_order_ids.destroy(|_v| {});
    }

    #[test(account = @0x456, aptos_framework = @0x1)]
    public fun test_order_id_tracking_flow(
        account: &signer, aptos_framework: &signer
    ) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        // Set short expiration for test purposes
        let expiration_window = 100_000; // 100 milliseconds
        let tracker = new_pre_cancellation_tracker(expiration_window);

        let client_order_id = 42;

        // Initially, order should not be pre-cancelled
        let is_cancelled = is_pre_cancelled(&mut tracker, signer::address_of(account), client_order_id);
        assert!(!is_cancelled);

        // Pre-cancel the order
        pre_cancel_order_for_tracker(&mut tracker, account, client_order_id);

        // Now it should be marked as pre-cancelled
        let is_cancelled = is_pre_cancelled(&mut tracker, signer::address_of(account), client_order_id);
        assert!(is_cancelled);

        destroy_tracker(tracker);
    }

    #[test(account = @0x456, aptos_framework = @0x1)]
    public fun test_order_expiration(
        account: &signer, aptos_framework: &signer
    ) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        // Set very short expiration for test
        let expiration_window = 10_000; // 10 milliseconds
        let tracker = new_pre_cancellation_tracker(expiration_window);

        let addr = signer::address_of(account);
        let client_order_id = 99;

        // Pre-cancel the order
        pre_cancel_order_for_tracker(&mut tracker, account, client_order_id);
        let initial_time = timestamp::now_microseconds();
        timestamp::update_global_time_for_test(initial_time + 5000);
        // Should be considered pre-cancelled before expiration
        let is_cancelled = is_pre_cancelled(&mut tracker, addr, client_order_id);
        assert!(is_cancelled);

        // Wait for expiration
        timestamp::update_global_time_for_test(initial_time + expiration_window + 1);

        // Should be considered not pre-cancelled after expiration
        let is_cancelled = is_pre_cancelled(&mut tracker, addr, client_order_id);
        assert!(!is_cancelled, 100);
        destroy_tracker(tracker);
    }

    #[test(account = @0x456, aptos_framework = @0x1)]
    public fun test_garbage_collection(
        account: &signer, aptos_framework: &signer
    ) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        let expiration_window = 5_000;
        let tracker = new_pre_cancellation_tracker(expiration_window);
        let addr = signer::address_of(account);

        let ids = vector::empty<u64>();
        ids.push_back(1);
        ids.push_back(2);
        ids.push_back(3);

        // Pre-cancel multiple orders
        let i = 0;
        while (i < ids.length()) {
            let id = ids[i];
            pre_cancel_order_for_tracker(&mut tracker, account, id);
            i += 1;
        };

        // Wait to let them expire
        let initial_time = timestamp::now_microseconds();
        timestamp::update_global_time_for_test(initial_time + expiration_window + 1);

        let j = 0;
        // Before garbage collection, we should still have them in account_order_ids
        while (j < ids.length()) {
            let id = ids[j];
            assert!(
                tracker.account_order_ids.contains(&new_account_client_order_id(addr, id))
            );
            j += 1;
        };

        // Trigger garbage collection
        garbage_collect(&mut tracker);

        // All should now be considered not pre-cancelled
        let j = 0;
        while (j < ids.length()) {
            let id = ids[j];
            assert!(
                !tracker.account_order_ids.contains(&new_account_client_order_id(addr, id))
            );
            let is_cancelled = is_pre_cancelled(&mut tracker, addr, id);
            assert!(!is_cancelled, 200 + j);
            j += 1;
        };
        destroy_tracker(tracker);
    }
}

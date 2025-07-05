module aptos_experimental::client_order_id_tracker {
    use std::option;
    use std::option::Option;
    use aptos_std::big_ordered_map;
    use aptos_std::big_ordered_map::BigOrderedMap;
    use aptos_experimental::order_book_types::{OrderIdType, AccountClientOrderId, new_account_client_order_id};

    const DUPLICATE_ORDER_PLACEMENT: u64 = 1;

    const MAX_ORDERS_GARBAGE_COLLECTED_PER_CALL: u64 = 10;

    struct OrderIdTracker has store {
        pre_cancellation_window_micros: u64,
        // Map of order IDs with expiration time to a boolean indicating if the order is active.
        order_ids_with_expiration: BigOrderedMap<OrderIdWithExpiration, bool>,
        // Map of order Ids to their corresponding expiration time.
        account_order_ids: BigOrderedMap<AccountClientOrderId, u64>,
    }

    struct OrderIdWithExpiration has store {
        expiration_time: u64,
        account_order_id: AccountClientOrderId,
    }

    public fun new_order_id_tracker(expiration_time_micros: u64): OrderIdTracker {
        OrderIdTracker {
            pre_cancellation_window_micros: expiration_time_micros,
            order_ids_with_expiration: big_ordered_map::new_with_reusable(),
            account_order_ids: big_ordered_map::new_with_reusable(),
        }
    }

    public fun pre_cancel_order(
        tracker: &mut OrderIdTracker,
        account: address,
        client_order_id: u64,
    ): Option<OrderIdType> {
        garbage_collect(tracker);
        let account_order_id = new_account_client_order_id(
            account,
            client_order_id
        );
        if (!tracker.account_order_ids.contains(&account_order_id)) {
            // If the account_order_id already exists with a previously set expiration time,
            // we update the expiration time.
            let expiration_time = tracker.account_order_ids.remove(&account_order_id);
            let order_id_with_expiration = OrderIdWithExpiration {
                expiration_time,
                account_order_id,
            };
            // If the mapping exists, then we remove the order ID with its expiration time.
            tracker.order_ids_with_expiration.remove(&order_id_with_expiration);
        };
        let current_time = aptos_std::timestamp::now_microseconds();
        let expiration_time = current_time + tracker.pre_cancellation_window_micros;
        let order_id_with_expiration = OrderIdWithExpiration {
            expiration_time,
            account_order_id,
        };
        tracker.account_order_ids.add(
            account_order_id,
            expiration_time
        );
        tracker.order_ids_with_expiration.add(
            order_id_with_expiration,
            true
        );
        option::none()
    }

    public fun is_pre_cancelled(
        tracker: &mut OrderIdTracker,
        account: address,
        client_order_id: u64,
    ) : bool {
        garbage_collect(tracker);
        let account_order_id = new_account_client_order_id(
            account,
            client_order_id
        );
        let current_time = aptos_std::timestamp::now_microseconds();
        if (tracker.account_order_ids.contains(&account_order_id)) {
            let expiration_time = tracker.account_order_ids.get(&account_order_id).destroy_some();
            if (current_time > expiration_time) {
                // This is possible as garbage collection may not be able to garbage collect all expired orders
                // in a single call.
                tracker.account_order_ids.remove(&account_order_id);
                let order_id_with_expiration = OrderIdWithExpiration {
                    expiration_time,
                    account_order_id,
                };
                tracker.order_ids_with_expiration.remove(&order_id_with_expiration);
            } else {
                return true; // Order ID already exists with a valid expiration time.
            }
        };
        return false
    }

    public fun garbage_collect(tracker: &mut OrderIdTracker) {
        let i = 0;
        let current_time = aptos_std::timestamp::now_microseconds();
        while (i < MAX_ORDERS_GARBAGE_COLLECTED_PER_CALL && !tracker.order_ids_with_expiration.is_empty()) {
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

}

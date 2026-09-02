/// (work in progress)
module aptos_experimental::pending_order_book_index {
    friend aptos_experimental::single_order_book;

    use std::vector;
    use aptos_std::math64;
    use aptos_framework::timestamp;
    use aptos_framework::big_ordered_map::BigOrderedMap;
    use aptos_trading::order_book_types::{
        OrderId,
        IncreasingIdx,
        TriggerCondition,
        DecreasingIdx
    };
    use aptos_experimental::order_book_utils;

    struct PendingUpOrderKey has store, copy, drop {
        price: u64,
        tie_breaker: IncreasingIdx
    }

    struct PendingDownOrderKey has store, copy, drop {
        price: u64,
        tie_breaker: DecreasingIdx
    }

    struct PendingTimeKey has store, copy, drop {
        time: u64,
        tie_breaker: IncreasingIdx
    }

    enum PendingOrderBookIndex has store {
        V1 {
            // Order to trigger when the oracle price move less than
            price_move_down_index: BigOrderedMap<PendingDownOrderKey, OrderId>,
            // Orders to trigger whem the oracle price move greater than
            price_move_up_index: BigOrderedMap<PendingUpOrderKey, OrderId>,
            // Orders to trigger when the time is greater than
            time_based_index: BigOrderedMap<PendingTimeKey, OrderId>
        }
    }

    public(friend) fun new_pending_order_book_index(): PendingOrderBookIndex {
        PendingOrderBookIndex::V1 {
            price_move_up_index: order_book_utils::new_default_big_ordered_map(),
            price_move_down_index: order_book_utils::new_default_big_ordered_map(),
            time_based_index: order_book_utils::new_default_big_ordered_map()
        }
    }

    public(friend) fun cancel_pending_order(
        self: &mut PendingOrderBookIndex,
        trigger_condition: TriggerCondition,
        unique_priority_idx: IncreasingIdx
    ) {
        let (price_move_down_index, price_move_up_index, time_based_index) =
            trigger_condition.get_trigger_condition_indices();
        if (price_move_up_index.is_some()) {
            self.price_move_up_index.remove(
                &PendingUpOrderKey {
                    price: price_move_up_index.destroy_some(),
                    tie_breaker: unique_priority_idx
                }
            );
        };
        if (price_move_down_index.is_some()) {
            self.price_move_down_index.remove(
                &PendingDownOrderKey {
                    price: price_move_down_index.destroy_some(),
                    tie_breaker: unique_priority_idx.into_decreasing_idx_type()
                }
            );
        };
        if (time_based_index.is_some()) {
            self.time_based_index.remove(
                &PendingTimeKey {
                    time: time_based_index.destroy_some(),
                    tie_breaker: unique_priority_idx
                }
            );
        };
    }

    public(friend) fun place_pending_order(
        self: &mut PendingOrderBookIndex,
        order_id: OrderId,
        trigger_condition: TriggerCondition,
        unique_priority_idx: IncreasingIdx
    ) {
        // Add this order to the pending order book index
        let (price_move_down_index, price_move_up_index, time_based_index) =
            trigger_condition.get_trigger_condition_indices();
        if (price_move_up_index.is_some()) {
            self.price_move_up_index.add(
                PendingUpOrderKey {
                    price: price_move_up_index.destroy_some(),
                    tie_breaker: unique_priority_idx
                },
                order_id
            );
        } else if (price_move_down_index.is_some()) {
            self.price_move_down_index.add(
                PendingDownOrderKey {
                    price: price_move_down_index.destroy_some(),
                    // Use a descending tie breaker to ensure that for price move down orders,
                    // orders with the same price are processed in FIFO order
                    tie_breaker: unique_priority_idx.into_decreasing_idx_type()
                },
                order_id
            );
        } else if (time_based_index.is_some()) {
            self.time_based_index.add(
                PendingTimeKey {
                    time: time_based_index.destroy_some(),
                    tie_breaker: unique_priority_idx
                },
                order_id
            );
        };
    }

    inline fun take_ready_price_move_up_orders(
        self: &mut PendingOrderBookIndex,
        current_price: u64,
        orders: &mut vector<OrderId>,
        limit: u64
    ) {
        while (!self.price_move_up_index.is_empty() && orders.length() < limit) {
            let (key, order_id) = self.price_move_up_index.borrow_front();
            if (current_price >= key.price) {
                orders.push_back(*order_id);
                self.price_move_up_index.remove(&key);
            } else {
                break;
            }
        };
    }

    inline fun take_ready_price_move_down_orders(
        self: &mut PendingOrderBookIndex,
        current_price: u64,
        orders: &mut vector<OrderId>,
        limit: u64
    ) {
        while (!self.price_move_down_index.is_empty() && orders.length() < limit) {
            let (key, order_id) = self.price_move_down_index.borrow_back();
            if (current_price <= key.price) {
                orders.push_back(*order_id);
                self.price_move_down_index.remove(&key);
            } else {
                break;
            }
        };
    }

    public(friend) fun take_ready_price_based_orders(
        self: &mut PendingOrderBookIndex, current_price: u64, order_limit: u64
    ): vector<OrderId> {
        let orders = vector::empty();
        self.take_ready_price_move_up_orders(
            current_price,
            &mut orders,
            math64::ceil_div(order_limit, 2)
        );
        self.take_ready_price_move_down_orders(current_price, &mut orders, order_limit);
        // Try to fill the rest of the space if available.
        self.take_ready_price_move_up_orders(current_price, &mut orders, order_limit);
        orders
    }

    public(friend) fun take_ready_time_based_orders(
        self: &mut PendingOrderBookIndex, order_limit: u64
    ): vector<OrderId> {
        let orders = vector::empty();
        while (!self.time_based_index.is_empty() && orders.length() < order_limit) {
            let current_time = timestamp::now_seconds();
            let (time, order_id) = self.time_based_index.borrow_front();
            if (current_time >= time.time) {
                orders.push_back(*order_id);
                self.time_based_index.remove(&time);
            } else {
                break;
            }
        };
        orders
    }

    #[test_only]
    public(friend) fun destroy_pending_order_book_index(
        self: PendingOrderBookIndex
    ) {
        let PendingOrderBookIndex::V1 {
            price_move_up_index,
            price_move_down_index,
            time_based_index
        } = self;
        price_move_up_index.destroy(|_v| {});
        price_move_down_index.destroy(|_v| {});
        time_based_index.destroy(|_v| {});
    }

    #[test_only]
    public(friend) fun get_price_move_down_index(
        self: &PendingOrderBookIndex
    ): &BigOrderedMap<PendingDownOrderKey, OrderId> {
        &self.price_move_down_index
    }

    #[test_only]
    public(friend) fun get_price_move_up_index(
        self: &PendingOrderBookIndex
    ): &BigOrderedMap<PendingUpOrderKey, OrderId> {
        &self.price_move_up_index
    }

    #[test_only]
    public(friend) fun get_time_based_index(
        self: &PendingOrderBookIndex
    ): &BigOrderedMap<PendingTimeKey, OrderId> {
        &self.time_based_index
    }
}

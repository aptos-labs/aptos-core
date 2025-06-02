/// (work in progress)
module aptos_experimental::pending_order_book_index {
    use std::vector;
    use aptos_framework::timestamp;
    use aptos_framework::big_ordered_map::BigOrderedMap;
    use aptos_experimental::order_book_types::{
        OrderIdType,
        UniqueIdxType,
        TriggerCondition,
        new_default_big_ordered_map
    };

    friend aptos_experimental::order_book;

    struct PendingOrderKey has store, copy, drop {
        price: u64,
        tie_breaker: UniqueIdxType
    }

    enum PendingOrderBookIndex has store {
        V1 {
            // Order to trigger when the oracle price move less than
            price_move_down_index: BigOrderedMap<PendingOrderKey, OrderIdType>,
            // Orders to trigger whem the oracle price move greater than
            price_move_up_index: BigOrderedMap<PendingOrderKey, OrderIdType>,
            //time_based_index: BigOrderedMap<BidKey<TimestampType>, ActiveBidData>,
            // Orders to trigger when the time is greater than
            time_based_index: BigOrderedMap<u64, OrderIdType>
        }
    }

    public(friend) fun new_pending_order_book_index(): PendingOrderBookIndex {
        PendingOrderBookIndex::V1 {
            price_move_up_index: new_default_big_ordered_map(),
            price_move_down_index: new_default_big_ordered_map(),
            time_based_index: new_default_big_ordered_map()
        }
    }

    public(friend) fun cancel_pending_order(
        self: &mut PendingOrderBookIndex,
        trigger_condition: TriggerCondition,
        unique_priority_idx: UniqueIdxType,
        is_buy: bool
    ) {
        let (price_move_up_index, price_move_down_index, time_based_index) =
            trigger_condition.index(is_buy);
        if (price_move_up_index.is_some()) {
            self.price_move_up_index.remove(
                &PendingOrderKey {
                    price: price_move_up_index.destroy_some(),
                    tie_breaker: unique_priority_idx
                }
            );
        };
        if (price_move_down_index.is_some()) {
            self.price_move_down_index.remove(
                &PendingOrderKey {
                    price: price_move_down_index.destroy_some(),
                    tie_breaker: unique_priority_idx
                }
            );
        };
        if (time_based_index.is_some()) {
            self.time_based_index.remove(&time_based_index.destroy_some());
        };
    }

    public(friend) fun place_pending_maker_order(
        self: &mut PendingOrderBookIndex,
        order_id: OrderIdType,
        trigger_condition: TriggerCondition,
        unique_priority_idx: UniqueIdxType,
        is_buy: bool
    ) {
        // Add this order to the pending order book index
        let (price_move_down_index, price_move_up_index, time_based_index) =
            trigger_condition.index(is_buy);

        if (price_move_up_index.is_some()) {
            self.price_move_up_index.add(
                PendingOrderKey {
                    price: price_move_up_index.destroy_some(),
                    tie_breaker: unique_priority_idx
                },
                order_id
            );
        } else if (price_move_down_index.is_some()) {
            self.price_move_down_index.add(
                PendingOrderKey {
                    price: price_move_down_index.destroy_some(),
                    tie_breaker: unique_priority_idx
                },
                order_id
            );
        } else if (time_based_index.is_some()) {
            self.time_based_index.add(time_based_index.destroy_some(), order_id);
        };
    }

    public fun take_ready_price_based_orders(
        self: &mut PendingOrderBookIndex, current_price: u64
    ): vector<OrderIdType> {
        let orders = vector::empty();
        while (!self.price_move_up_index.is_empty()) {
            let (key, order_id) = self.price_move_up_index.borrow_front();
            if (current_price >= key.price) {
                orders.push_back(*order_id);
                self.price_move_up_index.remove(&key);
            } else {
                break;
            }
        };
        while (!self.price_move_down_index.is_empty()) {
            let (key, order_id) = self.price_move_down_index.borrow_back();
            if (current_price <= key.price) {
                orders.push_back(*order_id);
                self.price_move_down_index.remove(&key);
            } else {
                break;
            }
        };
        orders
    }

    public fun take_time_time_based_orders(
        self: &mut PendingOrderBookIndex
    ): vector<OrderIdType> {
        let orders = vector::empty();
        while (!self.time_based_index.is_empty()) {
            let current_time = timestamp::now_seconds();
            let (time, order_id) = self.time_based_index.borrow_front();
            if (current_time >= time) {
                orders.push_back(*order_id);
                self.time_based_index.remove(&time);
            } else {
                break;
            }
        };
        orders
    }

    #[test_only]
    public(friend) fun get_price_move_down_index(
        self: &PendingOrderBookIndex
    ): &BigOrderedMap<PendingOrderKey, OrderIdType> {
        &self.price_move_down_index
    }

    #[test_only]
    public(friend) fun get_price_move_up_index(
        self: &PendingOrderBookIndex
    ): &BigOrderedMap<PendingOrderKey, OrderIdType> {
        &self.price_move_up_index
    }

    #[test_only]
    public(friend) fun get_time_based_index(
        self: &PendingOrderBookIndex
    ): &BigOrderedMap<u64, OrderIdType> {
        &self.time_based_index
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
}

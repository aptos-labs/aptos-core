/// ActiveOrderBook: This is the main order book that keeps track of active orders and their states. The active order
/// book is backed by a BigOrderedMap, which is a data structure that allows for efficient insertion, deletion, and matching of the order
/// The orders are matched based on time-price priority.
///
/// This is internal module, which cannot be used directly, use OrderBook instead.
module aptos_experimental::active_order_book {
    use std::option::{Self, Option};
    use aptos_std::math64::mul_div;
    use aptos_framework::big_ordered_map::BigOrderedMap;
    use aptos_experimental::order_book_types::{
        OrderIdType,
        UniqueIdxType,
        new_active_matched_order,
        ActiveMatchedOrder,
        get_slippage_pct_precision,
        new_default_big_ordered_map
    };
    #[test_only]
    use std::vector;
    #[test_only]
    use aptos_experimental::order_book_types::{new_order_id_type, new_unique_idx_type};

    const EINVALID_MAKER_ORDER: u64 = 1;
    /// There is a code bug that breaks internal invariant
    const EINTERNAL_INVARIANT_BROKEN: u64 = 2;

    friend aptos_experimental::order_book;

    /// ========= Active OrderBook ===========

    // Active Order Book:
    // bids: (order_id, price, unique_priority_idx, volume)

    // (price, unique_priority_idx) -> (volume, order_id)

    const U64_MAX: u64 = 0xffffffffffffffff;

    struct ActiveBidKey has store, copy, drop {
        price: u64,
        tie_breaker: UniqueIdxType
    }

    struct ActiveBidData has store, copy, drop {
        order_id: OrderIdType,
        size: u64
    }

    /// OrderBook tracking active (i.e. unconditional, immediately executable) limit orders.
    ///
    /// - invariant - all buys are smaller than sells, at all times.
    /// - tie_breaker in sells is U128_MAX-value, to make sure largest value in the book
    ///   that is taken first, is the one inserted first, amongst those with same bid price.
    enum ActiveOrderBook has store {
        V1 {
            buys: BigOrderedMap<ActiveBidKey, ActiveBidData>,
            sells: BigOrderedMap<ActiveBidKey, ActiveBidData>
        }
    }

    public(friend) fun new_active_order_book(): ActiveOrderBook {
        // potentially add max value to both sides (that will be skipped),
        // so that max_key never changes, and doesn't create conflict.
        ActiveOrderBook::V1 {
            buys: new_default_big_ordered_map(),
            sells: new_default_big_ordered_map()
        }
    }

    /// Picks the best (i.e. highest) bid (i.e. buy) price from the active order book.
    /// aborts if there are no buys
    public(friend) fun best_bid_price(self: &ActiveOrderBook): Option<u64> {
        if (self.buys.is_empty()) {
            option::none()
        } else {
            let (back_key, _back_value) = self.buys.borrow_back();
            option::some(back_key.price)
        }
    }

    /// Picks the best (i.e. lowest) ask (i.e. sell) price from the active order book.
    /// aborts if there are no sells
    public(friend) fun best_ask_price(self: &ActiveOrderBook): Option<u64> {
        if (self.sells.is_empty()) {
            option::none()
        } else {
            let (front_key, _front_value) = self.sells.borrow_front();
            option::some(front_key.price)
        }
    }

    public(friend) fun get_mid_price(self: &ActiveOrderBook): Option<u64> {
        let best_bid = self.best_bid_price();
        let best_ask = self.best_ask_price();
        if (best_bid.is_none() || best_ask.is_none()) {
            option::none()
        } else {
            option::some(
                (best_bid.destroy_some() + best_ask.destroy_some()) / 2
            )
        }
    }

    public(friend) fun get_slippage_price(
        self: &ActiveOrderBook, is_bid: bool, slippage_pct: u64
    ): Option<u64> {
        let mid_price = self.get_mid_price();
        if (mid_price.is_none()) {
            return option::none();
        };
        let mid_price = mid_price.destroy_some();
        let slippage = mul_div(
            mid_price, slippage_pct, get_slippage_pct_precision() * 100
        );
        if (is_bid) {
            option::some(mid_price + slippage)
        } else {
            option::some(mid_price - slippage)
        }
    }

    // TODO check if keeping depth book is more efficient than computing impact prices manually

    fun get_impact_bid_price(self: &ActiveOrderBook, impact_size: u64): Option<u64> {
        let total_value = (0 as u128);
        let total_size = 0;
        let orders = &self.buys;
        if (orders.is_empty()) {
            return option::none();
        };
        let (front_key, front_value) = orders.borrow_back();
        while (total_size < impact_size) {
            let matched_size =
                if (total_size + front_value.size > impact_size) {
                    impact_size - total_size
                } else {
                    front_value.size
                };
            total_value +=(matched_size as u128) * (front_key.price as u128);
            total_size += matched_size;
            let next_key = orders.prev_key(&front_key);
            if (next_key.is_none()) {
                // TODO maybe we should return none if there is not enough depth?
                break;
            };
            front_key = next_key.destroy_some();
            front_value = orders.borrow(&front_key);
        };
        option::some((total_value / (total_size as u128)) as u64)
    }

    fun get_impact_ask_price(self: &ActiveOrderBook, impact_size: u64): Option<u64> {
        let total_value = 0 as u128;
        let total_size = 0;
        let orders = &self.sells;
        if (orders.is_empty()) {
            return option::none();
        };
        let (front_key, front_value) = orders.borrow_front();
        while (total_size < impact_size) {
            let matched_size =
                if (total_size + front_value.size > impact_size) {
                    impact_size - total_size
                } else {
                    front_value.size
                };
            total_value +=(matched_size as u128) * (front_key.price as u128);
            total_size += matched_size;
            let next_key = orders.next_key(&front_key);
            if (next_key.is_none()) {
                break;
            };
            front_key = next_key.destroy_some();
            front_value = orders.borrow(&front_key);
        };
        option::some((total_value / (total_size as u128)) as u64)
    }

    inline fun get_tie_breaker(
        unique_priority_idx: UniqueIdxType, is_bid: bool
    ): UniqueIdxType {
        if (is_bid) {
            unique_priority_idx
        } else {
            unique_priority_idx.descending_idx()
        }
    }

    public(friend) fun cancel_active_order(
        self: &mut ActiveOrderBook,
        price: u64,
        unique_priority_idx: UniqueIdxType,
        is_bid: bool
    ): u64 {
        let tie_breaker = get_tie_breaker(unique_priority_idx, is_bid);
        let key = ActiveBidKey { price: price, tie_breaker };
        let value =
            if (is_bid) {
                self.buys.remove(&key)
            } else {
                self.sells.remove(&key)
            };
        value.size
    }

    public(friend) fun is_active_order(
        self: &ActiveOrderBook,
        price: u64,
        unique_priority_idx: UniqueIdxType,
        is_bid: bool
    ): bool {
        let tie_breaker = get_tie_breaker(unique_priority_idx, is_bid);
        let key = ActiveBidKey { price: price, tie_breaker };
        if (is_bid) {
            self.buys.contains(&key)
        } else {
            self.sells.contains(&key)
        }
    }

    /// Check if the order is a taker order - i.e. if it can be immediately matched with the order book fully or partially.
    public fun is_taker_order(
        self: &ActiveOrderBook, price: Option<u64>, is_bid: bool
    ): bool {
        if (is_bid) {
            let best_ask_price = self.best_ask_price();
            best_ask_price.is_some() && (price.is_none() || price.destroy_some() >= best_ask_price.destroy_some())
        } else {
            let best_bid_price = self.best_bid_price();
            best_bid_price.is_some() && (price.is_none() || price.destroy_some() <= best_bid_price.destroy_some())
        }
    }

    fun single_match_with_current_active_order(
        remaining_size: u64,
        cur_key: ActiveBidKey,
        cur_value: ActiveBidData,
        orders: &mut BigOrderedMap<ActiveBidKey, ActiveBidData>
    ): ActiveMatchedOrder {
        let is_cur_match_fully_consumed = cur_value.size <= remaining_size;

        let matched_size_for_this_order =
            if (is_cur_match_fully_consumed) {
                cur_value.size
            } else {
                remaining_size
            };

        let result =
            new_active_matched_order(
                cur_value.order_id,
                matched_size_for_this_order, // Matched size on the maker order
                cur_value.size - matched_size_for_this_order // Remaining size on the maker order
            );

        if (is_cur_match_fully_consumed) {
            orders.remove(&cur_key);
        } else {
            orders.borrow_mut(&cur_key).size -= matched_size_for_this_order;
        };
        result
    }

    fun get_single_match_for_buy_order(
        self: &mut ActiveOrderBook, price: Option<u64>, size: u64
    ): ActiveMatchedOrder {
        let (smallest_key, smallest_value) = self.sells.borrow_front();
        if (price.is_some()) {
            assert!(price.destroy_some() >= smallest_key.price, EINTERNAL_INVARIANT_BROKEN);
        };
        single_match_with_current_active_order(
            size,
            smallest_key,
            *smallest_value,
            &mut self.sells
        )
    }

    fun get_single_match_for_sell_order(
        self: &mut ActiveOrderBook, price: Option<u64>, size: u64
    ): ActiveMatchedOrder {
        let (largest_key, largest_value) = self.buys.borrow_back();
        if (price.is_some()) {
            assert!(price.destroy_some() <= largest_key.price, EINTERNAL_INVARIANT_BROKEN);
        };
        single_match_with_current_active_order(
            size,
            largest_key,
            *largest_value,
            &mut self.buys
        )
    }

    public(friend) fun get_single_match_result(
        self: &mut ActiveOrderBook,
        price: Option<u64>,
        size: u64,
        is_bid: bool
    ): ActiveMatchedOrder {
        if (is_bid) {
            self.get_single_match_for_buy_order(price, size)
        } else {
            self.get_single_match_for_sell_order(price, size)
        }
    }

    /// Increase the size of the order in the orderbook without altering its position in the price-time priority.
    public(friend) fun increase_order_size(
        self: &mut ActiveOrderBook,
        price: u64,
        unique_priority_idx: UniqueIdxType,
        size_delta: u64,
        is_bid: bool
    ) {
        let tie_breaker = get_tie_breaker(unique_priority_idx, is_bid);
        let key = ActiveBidKey { price, tie_breaker };
        if (is_bid) {
            self.buys.borrow_mut(&key).size += size_delta;
        } else {
            self.sells.borrow_mut(&key).size += size_delta;
        };
    }

    /// Decrease the size of the order in the order book without altering its position in the price-time priority.
    public(friend) fun decrease_order_size(
        self: &mut ActiveOrderBook,
        price: u64,
        unique_priority_idx: UniqueIdxType,
        size_delta: u64,
        is_bid: bool
    ) {
        let tie_breaker = get_tie_breaker(unique_priority_idx, is_bid);
        let key = ActiveBidKey { price, tie_breaker };
        if (is_bid) {
            self.buys.borrow_mut(&key).size -= size_delta;
        } else {
            self.sells.borrow_mut(&key).size -= size_delta;
        };
    }

    public(friend) fun place_maker_order(
        self: &mut ActiveOrderBook,
        order_id: OrderIdType,
        price: u64,
        unique_priority_idx: UniqueIdxType,
        size: u64,
        is_bid: bool
    ) {
        let tie_breaker = get_tie_breaker(unique_priority_idx, is_bid);
        let key = ActiveBidKey { price, tie_breaker };
        let value = ActiveBidData { order_id, size };
        // Assert that this is not a taker order
        assert!(!self.is_taker_order(option::some(price), is_bid), EINVALID_MAKER_ORDER);
        if (is_bid) {
            self.buys.add(key, value);
        } else {
            self.sells.add(key, value);
        };
    }

    #[test_only]
    public fun destroy_active_order_book(self: ActiveOrderBook) {
        let ActiveOrderBook::V1 { sells, buys } = self;
        sells.destroy(|_v| {});
        buys.destroy(|_v| {});
    }

    #[test_only]
    struct TestOrder has copy, drop {
        account: address,
        order_id: OrderIdType,
        price: u64,
        size: u64,
        unique_idx: UniqueIdxType,
        is_bid: bool
    }

    #[test_only]
    fun place_test_order(self: &mut ActiveOrderBook, order: TestOrder):
        vector<ActiveMatchedOrder> {
        let result = vector::empty();
        let remaining_size = order.size;
        while (remaining_size > 0) {
            if (!self.is_taker_order(option::some(order.price), order.is_bid)) {
                self.place_maker_order(
                    order.order_id, order.price, order.unique_idx, order.size, order.is_bid
                );
                return result;
            };
            let match_result =
                self.get_single_match_result(option::some(order.price), remaining_size, order.is_bid);
            remaining_size -= match_result.get_active_matched_size();
            result.push_back(match_result);
        };
        result
    }

    #[test]
    // TODO (skedia) Add more comprehensive tests for the acive order book
    fun test_active_order_book() {
        let active_order_book = new_active_order_book();

        assert!(active_order_book.best_bid_price().is_none());
        assert!(active_order_book.best_ask_price().is_none());

        // $200 - 10000
        // --
        let match_result =
            active_order_book.place_test_order(
                TestOrder {
                    account: @0xAA,
                    order_id: new_order_id_type(0),
                    price: 200,
                    size: 1000,
                    unique_idx: new_unique_idx_type(0),
                    is_bid: false
                }
            );
        assert!(match_result.is_empty());

        // $200 - 10000
        // --
        // $100 - 1000
        let match_result =
            active_order_book.place_test_order(
                TestOrder {
                    account: @0xAA,
                    order_id: new_order_id_type(1),
                    price: 100,
                    size: 1000,
                    unique_idx: new_unique_idx_type(1),
                    is_bid: true
                }
            );
        assert!(match_result.is_empty());

        assert!(active_order_book.best_bid_price().destroy_some() == 100);
        assert!(active_order_book.best_ask_price().destroy_some() == 200);

        // $200 - 10000
        // $150 - 100
        // --
        // $100 - 1000
        let match_result =
            active_order_book.place_test_order(
                TestOrder {
                    account: @0xAA,
                    order_id: new_order_id_type(2),
                    price: 150,
                    size: 100,
                    unique_idx: new_unique_idx_type(2),
                    is_bid: false
                }
            );
        assert!(match_result.is_empty());

        // $200 - 10000
        // $175 - 100
        // $150 - 100
        // --
        // $100 - 1000
        let match_result =
            active_order_book.place_test_order(
                TestOrder {
                    account: @0xAA,
                    order_id: new_order_id_type(3),
                    price: 175,
                    size: 100,
                    unique_idx: new_unique_idx_type(3),
                    is_bid: false
                }
            );
        assert!(match_result.is_empty());

        assert!(active_order_book.best_bid_price().destroy_some() == 100);
        assert!(active_order_book.best_ask_price().destroy_some() == 150);

        // $200 - 10000
        // $175 - 100
        // $150 - 50 <-- match 50 units
        // --
        // $100 - 1000
        let match_result =
            active_order_book.place_test_order(
                TestOrder {
                    account: @0xAA,
                    order_id: new_order_id_type(4),
                    price: 160,
                    size: 50,
                    unique_idx: new_unique_idx_type(4),
                    is_bid: true
                }
            );
        assert!(match_result.length() == 1);
        // TODO - seems like we have no match price in ActiveMatchResult any more
        // we need to add it back, and assert?
        // Maker ask order was partially filled 100 -> 50
        assert!(
            match_result
                == vector[
                    new_active_matched_order(
                        new_order_id_type(2),
                        50, // matched size
                        50 // remaining size
                    )
                ],
            7
        );
        active_order_book.destroy_active_order_book();
    }

    #[test]
    fun test_get_impact_sell_price() {
        let active_order_book = new_active_order_book();

        // Add sell orders at different prices
        active_order_book.place_test_order(
            TestOrder {
                account: @0xAA,
                order_id: new_order_id_type(1),
                price: 100,
                size: 50,
                unique_idx: new_unique_idx_type(1),
                is_bid: false
            }
        );

        active_order_book.place_test_order(
            TestOrder {
                account: @0xAA,
                order_id: new_order_id_type(2),
                price: 150,
                size: 100,
                unique_idx: new_unique_idx_type(2),
                is_bid: false
            }
        );

        active_order_book.place_test_order(
            TestOrder {
                account: @0xAA,
                order_id: new_order_id_type(3),
                price: 200,
                size: 150,
                unique_idx: new_unique_idx_type(3),
                is_bid: false
            }
        );

        // Test impact price calculations
        // Impact size 50 should give price of lowest order (100)
        assert!(active_order_book.get_impact_ask_price(50).destroy_some() == 100, 1);

        // Impact size 100 should give weighted average of first two orders
        // (50 * 100 + 50 * 150) / 100 = 125
        assert!(active_order_book.get_impact_ask_price(100).destroy_some() == 125, 2);

        // Impact size 200 should give weighted average of all orders
        // (50 * 100 + 100 * 150 + 50 * 200) / 200 = 150
        assert!(active_order_book.get_impact_ask_price(200).destroy_some() == 150, 3);

        // Impact size larger than total available should still use all orders
        // (50 * 100 + 100 * 150 + 150 * 200) / 300 = 166
        assert!(active_order_book.get_impact_ask_price(1000).destroy_some() == 166, 4);

        active_order_book.destroy_active_order_book();
    }

    #[test]
    fun test_get_impact_bid_price() {
        let active_order_book = new_active_order_book();

        // Place test buy orders at different prices
        active_order_book.place_test_order(
            TestOrder {
                account: @0xAA,
                order_id: new_order_id_type(1),
                price: 200,
                size: 50,
                unique_idx: new_unique_idx_type(1),
                is_bid: true
            }
        );

        active_order_book.place_test_order(
            TestOrder {
                account: @0xAA,
                order_id: new_order_id_type(2),
                price: 150,
                size: 100,
                unique_idx: new_unique_idx_type(2),
                is_bid: true
            }
        );

        active_order_book.place_test_order(
            TestOrder {
                account: @0xAA,
                order_id: new_order_id_type(3),
                price: 100,
                size: 150,
                unique_idx: new_unique_idx_type(3),
                is_bid: true
            }
        );

        // Test impact price calculations
        // Impact size 50 should give price of first order (200)
        assert!(active_order_book.get_impact_bid_price(50).destroy_some() == 200, 1);

        // Impact size 100 should give weighted average of first two orders
        // (50 * 200 + 50 * 150) / 100 = 175
        assert!(active_order_book.get_impact_bid_price(100).destroy_some() == 175, 2);

        // Impact size 200 should give weighted average of all orders
        // (50 * 200 + 100 * 150 + 50 * 100) / 200 = 150
        assert!(active_order_book.get_impact_bid_price(200).destroy_some() == 150, 3);

        // Impact size larger than total available should still use all orders
        // (50 * 200 + 100 * 150 + 150 * 100) / 300 = 133
        assert!(active_order_book.get_impact_bid_price(1000).destroy_some() == 133, 4);

        active_order_book.destroy_active_order_book();
    }

    #[test]
    fun test_get_slippage_price() {
        let active_order_book = new_active_order_book();

        // Add sell orders at different prices
        active_order_book.place_test_order(
            TestOrder {
                account: @0xAA,
                order_id: new_order_id_type(1),
                price: 101,
                size: 50,
                unique_idx: new_unique_idx_type(1),
                is_bid: false
            }
        );

        active_order_book.place_test_order(
            TestOrder {
                account: @0xAA,
                order_id: new_order_id_type(2),
                price: 102,
                size: 100,
                unique_idx: new_unique_idx_type(2),
                is_bid: false
            }
        );

        active_order_book.place_test_order(
            TestOrder {
                account: @0xAA,
                order_id: new_order_id_type(3),
                price: 103,
                size: 150,
                unique_idx: new_unique_idx_type(3),
                is_bid: false
            }
        );

        // Add some buy orders
        active_order_book.place_test_order(
            TestOrder {
                account: @0xAA,
                order_id: new_order_id_type(4),
                price: 99,
                size: 50,
                unique_idx: new_unique_idx_type(4),
                is_bid: true
            }
        );

        active_order_book.place_test_order(
            TestOrder {
                account: @0xAA,
                order_id: new_order_id_type(5),
                price: 98,
                size: 100,
                unique_idx: new_unique_idx_type(5),
                is_bid: true
            }
        );

        // Test slippage price calculations
        assert!(active_order_book.get_mid_price().destroy_some() == 100);
        // Slippage 10% for buy order should give price of mid price (100) + 10% = 110
        assert!(active_order_book.get_slippage_price(true, 1000).destroy_some() == 110);
        assert!(active_order_book.get_slippage_price(true, 100).destroy_some() == 101);
        assert!(active_order_book.get_slippage_price(true, 10).destroy_some() == 100);

        assert!(active_order_book.get_slippage_price(false, 1500).destroy_some() == 85);
        assert!(active_order_book.get_slippage_price(false, 100).destroy_some() == 99);
        assert!(active_order_book.get_slippage_price(false, 10).destroy_some() == 100);
        assert!(active_order_book.get_slippage_price(false, 0).destroy_some() == 100);

        active_order_book.destroy_active_order_book();

    }
}

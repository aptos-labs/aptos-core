module aptos_experimental::market {

    use std::option;
    use std::option::Option;
    use std::signer;
    use aptos_framework::event;
    use aptos_experimental::order_book::{OrderBook, new_order_book, new_order_request};
    use aptos_experimental::order_book_types::{TriggerCondition, UniqueIdxType, Order};
    use aptos_experimental::market_types::MarketClearinghouseCallbacks;

    // Error codes
    const EINVALID_ORDER: u64 = 1;
    const EORDER_BOOK_FULL: u64 = 2;
    const EMARKET_NOT_FOUND: u64 = 3;
    const ENOT_ADMIN: u64 = 4;
    const EINVALID_FEE_TIER: u64 = 5;
    const EORDER_DOES_NOT_EXIST: u64 = 6;
    const EINVALID_TIME_IN_FORCE_FOR_MAKER: u64 = 7;
    const EINVALID_TIME_IN_FORCE_FOR_TAKER: u64 = 8;
    const EINVALID_MATCHING_FOR_MAKER_REINSERT: u64 = 9;
    const EINVALID_TAKER_POSITION_UPDATE: u64 = 10;
    const EINVALID_LIQUIDATION: u64 = 11;

    // Order time in force
    const TIME_IN_FORCE_GTC: u8 = 0;
    const TIME_IN_FORCE_POST_ONLY: u8 = 1;
    const TIME_IN_FORCE_IOC: u8 = 2;

    public fun good_till_cancelled(): u8 {
        TIME_IN_FORCE_GTC
    }

    public fun post_only(): u8 {
        TIME_IN_FORCE_POST_ONLY
    }

    public fun immediate_or_cancel(): u8 {
        TIME_IN_FORCE_IOC
    }

    // TODO(skedia): Revisit this slippage tolerance for twap
    const SLIPPAGE_TOLERANCE_FOR_TWAP: u64 = 300; // 3%

    struct Market<M: store + copy + drop> has store {
        /// Address of the parent object that created this market
        /// Purely for grouping events based on the source DEX, not used otherwise
        parent: address,
        /// Address of the market object of this market.
        market: address,

        // TODO: remove sequential order id generation
        last_order_id: u64,
        order_book: OrderBook<M>
    }

    /// Order has been accepted by the engine.
    const ORDER_STATUS_OPEN: u8 = 0;
    /// Order has been fully or partially filled.
    const ORDER_STATUS_FILLED: u8 = 1;
    /// Order has been cancelled by the user or engine.
    const ORDER_STATUS_CANCELLED: u8 = 2;
    /// Order has been rejected by the engine. Unlike cancelled orders, rejected
    /// orders are invalid orders. Rejection reasons:
    /// 1. Insufficient margin
    /// 2. Order is reduce_only but does not reduce
    const ORDER_STATUS_REJECTED: u8 = 3;

    public fun order_status_open(): u8 {
        ORDER_STATUS_OPEN
    }

    public fun order_status_filled(): u8 {
        ORDER_STATUS_FILLED
    }

    public fun order_status_cancelled(): u8 {
        ORDER_STATUS_CANCELLED
    }

    public fun order_status_rejected(): u8 {
        ORDER_STATUS_REJECTED
    }

    #[event]
    struct OrderEvent has drop, copy, store {
        parent: address,
        market: address,
        order_id: u64,
        user: address,
        orig_size: u64,
        remaining_size: u64,
        // TODO(bl): Brian and Sean will revisit to see if we should have split
        // into multiple events for OrderEvent
        /// OPEN - size_delta will be amount of size added
        /// CANCELLED - size_delta will be amount of size removed
        /// FILLED - size_delta will be amount of size filled
        /// REJECTED - size_delta will always be 0
        size_delta: u64,
        price: u64,
        is_buy: bool,
        // Whether the order crosses the orderbook.
        is_taker: bool,
        status: u8,
        details: std::string::String
    }

    enum OrderCancellationReason has drop, copy {
        PostOnlyViolation,
        IOCViolation,
        PositionUpdateViolation,
        ReduceOnlyViolation,
        ClearinghouseSettleViolation,
        MaxFillLimitViolation
    }

    struct OrderMatchResult has drop {
        order_id: u64,
        remaining_size: u64,
        cancel_reason: Option<OrderCancellationReason>,
        num_fills: u64
    }

    public fun destroy_order_match_result(
        self: OrderMatchResult
    ): (u64, u64, Option<OrderCancellationReason>, u64) {
        let OrderMatchResult { order_id, remaining_size, cancel_reason, num_fills } =
            self;
        (order_id, remaining_size, cancel_reason, num_fills)
    }

    public fun number_of_fills(self: &OrderMatchResult): u64 {
        self.num_fills
    }

    public fun get_cancel_reason(self: &OrderMatchResult): Option<OrderCancellationReason> {
        self.cancel_reason
    }

    public fun get_remaining_size_from_result(self: &OrderMatchResult): u64 {
        self.remaining_size
    }

    public fun is_ioc_violation(self: OrderCancellationReason): bool {
        return self == OrderCancellationReason::IOCViolation
    }

    public fun is_fill_limit_violation(
        cancel_reason: OrderCancellationReason
    ): bool {
        return cancel_reason == OrderCancellationReason::MaxFillLimitViolation
    }

    public fun get_order_id(self: OrderMatchResult): u64 {
        self.order_id
    }

    #[test_only]
    public fun get_order_id_from_event(self: OrderEvent): u64 {
        self.order_id
    }

    #[test_only]
    public fun verify_order_event(
        self: OrderEvent,
        order_id: u64,
        market: address,
        user: address,
        orig_size: u64,
        remaining_size: u64,
        size_delta: u64,
        price: u64,
        is_buy: bool,
        is_taker: bool,
        status: u8
    ) {
        assert!(self.order_id == order_id);
        assert!(self.market == market);
        assert!(self.user == user);
        assert!(self.orig_size == orig_size);
        assert!(self.remaining_size == remaining_size);
        assert!(self.size_delta == size_delta);
        assert!(self.price == price);
        assert!(self.is_buy == is_buy);
        assert!(self.is_taker == is_taker);
        assert!(self.status == status);
    }

    #[test_only]
    public fun is_clearinghouse_settle_violation(
        cancellation_reason: OrderCancellationReason
    ): bool {
        if (cancellation_reason
            == OrderCancellationReason::ClearinghouseSettleViolation) {
            return true;
        };
        false
    }

    public fun new_market<M: store + copy + drop>(
        parent: &signer, market: &signer
    ): Market<M> {
        // requiring signers, and not addresses, purely to guarantee different dexes
        // cannot polute events to each other, accidentally or maliciously.
        Market {
            parent: signer::address_of(parent),
            market: signer::address_of(market),
            last_order_id: 0,
            order_book: new_order_book()
        }
    }

    #[test_only]
    public fun destroy_market<M: store + copy + drop>(self: Market<M>) {
        let Market {
            parent: _parent,
            market: _market,
            last_order_id: _last_order_id,
            order_book
        } = self;
        order_book.destroy_order_book()
    }

    public fun get_market<M: store + copy + drop>(self: &Market<M>): address {
        self.market
    }

    public fun get_order_book<M: store + copy + drop>(self: &Market<M>): &OrderBook<M> {
        &self.order_book
    }

    public fun get_order_book_mut<M: store + copy + drop>(
        self: &mut Market<M>
    ): &mut OrderBook<M> {
        &mut self.order_book
    }

    public fun best_bid_price<M: store + copy + drop>(self: &Market<M>): Option<u64> {
        self.order_book.best_bid_price()
    }

    public fun best_ask_price<M: store + copy + drop>(self: &Market<M>): Option<u64> {
        self.order_book.best_ask_price()
    }

    public fun is_taker_order<M: store + copy + drop>(
        self: &Market<M>, price: u64, is_buy: bool
    ): bool {
        self.order_book.is_taker_order(price, is_buy, option::none())
    }

    public fun place_order<M: store + copy + drop>(
        self: &mut Market<M>,
        user: &signer,
        price: u64,
        orig_size: u64,
        is_buy: bool,
        time_in_force: u8,
        trigger_condition: Option<TriggerCondition>,
        metadata: M,
        max_fill_limit: u64,
        cancel_on_fill_limit: bool,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): OrderMatchResult {
        let order_id = self.next_order_id();
        self.place_order_with_order_id(
            signer::address_of(user),
            price,
            orig_size,
            orig_size,
            is_buy,
            time_in_force,
            trigger_condition,
            metadata,
            order_id,
            option::none(),
            max_fill_limit,
            cancel_on_fill_limit,
            true,
            callbacks
        )
    }

    public fun next_order_id<M: store + copy + drop>(self: &mut Market<M>): u64 {
        self.last_order_id += 1;
        self.last_order_id
    }

    public fun place_order_with_user_addr<M: store + copy + drop>(
        self: &mut Market<M>,
        user_addr: address,
        price: u64,
        orig_size: u64,
        is_buy: bool,
        time_in_force: u8,
        trigger_condition: Option<TriggerCondition>,
        metadata: M,
        max_fill_limit: u64,
        cancel_on_fill_limit: bool,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): OrderMatchResult {
        let order_id = self.next_order_id();
        self.place_order_with_order_id(
            user_addr,
            price,
            orig_size,
            orig_size,
            is_buy,
            time_in_force,
            trigger_condition,
            metadata,
            order_id,
            option::none(),
            max_fill_limit,
            cancel_on_fill_limit,
            true,
            callbacks
        )
    }

    fun place_maker_order<M: store + copy + drop>(
        self: &mut Market<M>,
        user_addr: address,
        price: u64,
        orig_size: u64,
        remaining_size: u64,
        is_buy: bool,
        time_in_force: u8,
        trigger_condition: Option<TriggerCondition>,
        metadata: M,
        order_id: u64,
        unique_priority_idx: Option<UniqueIdxType>,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): OrderMatchResult {
        // Validate that the order is valid from position management perspective
        if (time_in_force == TIME_IN_FORCE_IOC) {
            event::emit(
                OrderEvent {
                    parent: self.parent,
                    market: self.market,
                    order_id,
                    user: user_addr,
                    orig_size,
                    remaining_size: orig_size,
                    size_delta: orig_size,
                    price,
                    is_buy,
                    is_taker: false,
                    status: ORDER_STATUS_OPEN,
                    details: std::string::utf8(b"")
                }
            );
            event::emit(
                OrderEvent {
                    parent: self.parent,
                    market: self.market,
                    order_id,
                    user: user_addr,
                    orig_size,
                    remaining_size: orig_size,
                    size_delta: orig_size,
                    price,
                    is_buy,
                    is_taker: false,
                    status: ORDER_STATUS_CANCELLED,
                    details: std::string::utf8(b"IOC_VIOLATION")
                }
            );
            return OrderMatchResult {
                order_id,
                remaining_size,
                cancel_reason: option::some(OrderCancellationReason::IOCViolation),
                num_fills: 0
            };
        };

        if (
            !callbacks.validate_settlement_update(
                user_addr, false, // is_taker
                is_buy, price, orig_size
            )) {
            event::emit(
                OrderEvent {
                    parent: self.parent,
                    market: self.market,
                    order_id,
                    user: user_addr,
                    orig_size,
                    remaining_size,
                    size_delta: 0, // 0 because order was never placed
                    price,
                    is_buy,
                    is_taker: false,
                    status: ORDER_STATUS_REJECTED,
                    details: std::string::utf8(b"Position Update violation")
                }
            );
            return OrderMatchResult {
                order_id,
                remaining_size,
                cancel_reason: option::some(
                    OrderCancellationReason::PositionUpdateViolation
                ),
                num_fills: 0
            };
        };
        self.order_book.place_maker_order(
            new_order_request(
                user_addr,
                order_id,
                unique_priority_idx,
                price,
                orig_size,
                remaining_size,
                is_buy,
                trigger_condition,
                metadata
            )
        );
        // Order was successfully placed
        event::emit(
            OrderEvent {
                parent: self.parent,
                market: self.market,
                order_id,
                user: user_addr,
                orig_size,
                remaining_size,
                size_delta: orig_size,
                price,
                is_buy,
                is_taker: false,
                status: ORDER_STATUS_OPEN,
                details: std::string::utf8(b"")
            }
        );
        return OrderMatchResult {
            order_id,
            remaining_size,
            cancel_reason: option::none(),
            num_fills: 0
        }
    }

    public fun place_order_with_order_id<M: store + copy + drop>(
        self: &mut Market<M>,
        user_addr: address,
        price: u64,
        orig_size: u64,
        remaining_size: u64,
        is_buy: bool,
        time_in_force: u8,
        trigger_condition: Option<TriggerCondition>,
        metadata: M,
        order_id: u64,
        unique_priority_idx: Option<UniqueIdxType>,
        max_fill_limit: u64,
        emit_cancel_on_fill_limit: bool,
        emit_taker_order_open: bool,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): OrderMatchResult {
        assert!(orig_size > 0, EINVALID_ORDER);
        // TODO(skedia) add support for trigger condition
        // TODO(skedia) is_taker_order API can actually return false positive as the maker orders might not be valid.
        // Changes are needed to ensure the maker order is valid for this order to be a valid taker order.
        // TODO(skedia) reconsile the semantics around global order id vs account local id.
        let settlement_size =
            callbacks.max_settlement_size(
                user_addr, is_buy, remaining_size, metadata
            );
        if (settlement_size.is_none()) {
            event::emit(
                OrderEvent {
                    parent: self.parent,
                    market: self.market,
                    order_id,
                    user: user_addr,
                    orig_size,
                    remaining_size,
                    size_delta: 0, // 0 because order was never placed
                    price,
                    is_buy,
                    is_taker: false,
                    status: ORDER_STATUS_REJECTED,
                    details: std::string::utf8(b"Max settlement size violation")
                }
            );
            return OrderMatchResult {
                order_id,
                remaining_size,
                cancel_reason: option::some(
                    OrderCancellationReason::ReduceOnlyViolation
                ),
                num_fills: 0
            };
        };
        let remaining_size = settlement_size.destroy_some();
        if (
            !callbacks.validate_settlement_update(
                user_addr, true, // is_taker
                is_buy, price, remaining_size
            )) {
            event::emit(
                OrderEvent {
                    parent: self.parent,
                    market: self.market,
                    order_id,
                    user: user_addr,
                    orig_size,
                    remaining_size,
                    size_delta: 0, // 0 because order was never placed
                    price,
                    is_buy,
                    is_taker: false,
                    status: ORDER_STATUS_REJECTED,
                    details: std::string::utf8(b"Position Update violation")
                }
            );
            return OrderMatchResult {
                order_id,
                remaining_size: orig_size,
                cancel_reason: option::some(
                    OrderCancellationReason::PositionUpdateViolation
                ),
                num_fills: 0
            };
        };

        let is_taker_order = self.order_book.is_taker_order(price, is_buy, option::none());
        if (!is_taker_order) {
            return self.place_maker_order(
                user_addr,
                price,
                orig_size,
                remaining_size,
                is_buy,
                time_in_force,
                trigger_condition,
                metadata,
                order_id,
                unique_priority_idx,
                callbacks
            );
        };

        // NOTE: We should always use is_taker: true for this order past this
        // point so that indexer can consistently track the order's status
        if (emit_taker_order_open) {
            event::emit(
                OrderEvent {
                    parent: self.parent,
                    market: self.market,
                    order_id,
                    user: user_addr,
                    orig_size,
                    remaining_size,
                    size_delta: orig_size,
                    price,
                    is_buy,
                    is_taker: true,
                    status: ORDER_STATUS_OPEN,
                    details: std::string::utf8(b"")
                }
            );
        };
        if (time_in_force == TIME_IN_FORCE_POST_ONLY) {
            event::emit(
                OrderEvent {
                    parent: self.parent,
                    market: self.market,
                    order_id,
                    user: user_addr,
                    orig_size,
                    remaining_size,
                    size_delta: remaining_size,
                    price,
                    is_buy,
                    is_taker: true,
                    status: ORDER_STATUS_CANCELLED,
                    details: std::string::utf8(b"Post only violation")
                }
            );
            return OrderMatchResult {
                order_id,
                remaining_size: orig_size,
                cancel_reason: option::some(OrderCancellationReason::PostOnlyViolation),
                num_fills: 0
            };
        };
        let num_fills = 0;
        loop {
            let result =
                self.order_book.get_single_match_for_taker(price, remaining_size, is_buy);
            let (maker_order, maker_matched_size) = result.destroy_single_order_match();
            // let maker_reduce_only = maker_order.get_metadata_from_order().is_reduce_only;
            let (maker_address, maker_order_id) =
                maker_order.get_order_id().destroy_order_id_type();

            let expected_settlement_size = {
                let maker_settlement_size =
                    callbacks.max_settlement_size(
                        maker_address,
                        !is_buy,
                        maker_matched_size,
                        maker_order.get_metadata_from_order()
                    );
                // TODO(skedia) emit event for partial order cancellation
                // if maker settlement size is less than maker matched size
                if (maker_settlement_size.is_none()) {
                    let remaining_size = maker_order.get_remaining_size();
                    event::emit(
                        OrderEvent {
                            parent: self.parent,
                            market: self.market,
                            order_id,
                            user: maker_address,
                            orig_size: maker_order.get_orig_size(),
                            remaining_size: 0,
                            size_delta: remaining_size,
                            price,
                            is_buy: !is_buy,
                            is_taker: true,
                            status: ORDER_STATUS_CANCELLED,
                            details: std::string::utf8(b"Max settlement size violation")
                        }
                    );
                    self.order_book.cancel_order(maker_address, maker_order_id);
                };
                maker_settlement_size.destroy_some()
            };

            let settle_result =
                callbacks.settle_trade(
                    user_addr,
                    maker_address,
                    is_buy,
                    maker_order.get_price(), // Order is always matched at the price of the maker
                    expected_settlement_size
                );

            let maker_remaining_settled_size = expected_settlement_size;
            let settled_size = settle_result.get_settled_size();
            if (settled_size > 0) {
                remaining_size -= settled_size;
                maker_remaining_settled_size -= settled_size;
                num_fills += 1;
                // Event for taker fill
                event::emit(
                    OrderEvent {
                        parent: self.parent,
                        market: self.market,
                        order_id,
                        user: user_addr,
                        orig_size,
                        remaining_size,
                        size_delta: settled_size,
                        price: maker_order.get_price(),
                        is_buy,
                        is_taker: true,
                        status: ORDER_STATUS_FILLED,
                        details: std::string::utf8(b"")
                    }
                );
                // Event for maker fill
                event::emit(
                    OrderEvent {
                        parent: self.parent,
                        market: self.market,
                        order_id: maker_order_id,
                        user: maker_address,
                        orig_size: maker_order.get_orig_size(),
                        remaining_size: maker_order.get_remaining_size(),
                        size_delta: settled_size,
                        price: maker_order.get_price(),
                        is_buy: !is_buy,
                        is_taker: false,
                        status: ORDER_STATUS_FILLED,
                        details: std::string::utf8(b"")
                    }
                );
            };

            let maker_cancellation_reason = settle_result.get_maker_cancellation_reason();
            if (maker_cancellation_reason.is_some()) {
                let maker_cancel_size =
                    maker_remaining_settled_size + maker_order.get_remaining_size();
                event::emit(
                    OrderEvent {
                        parent: self.parent,
                        market: self.market,
                        order_id: maker_order_id,
                        user: maker_address,
                        orig_size: maker_order.get_orig_size(),
                        remaining_size: 0,
                        size_delta: maker_cancel_size,
                        price: maker_order.get_price(),
                        is_buy: !is_buy,
                        is_taker: false,
                        status: ORDER_STATUS_CANCELLED,
                        details: maker_cancellation_reason.destroy_some()
                    }
                );
                // If the maker is invalid cancel the maker order and continue to the next maker order
                self.order_book.cancel_order(maker_address, maker_order_id);
            };

            let taker_cancellation_reason = settle_result.get_taker_cancellation_reason();
            if (taker_cancellation_reason.is_some()) {
                event::emit(
                    OrderEvent {
                        parent: self.parent,
                        market: self.market,
                        order_id,
                        user: user_addr,
                        orig_size,
                        remaining_size,
                        size_delta: remaining_size,
                        price,
                        is_buy,
                        is_taker: true,
                        status: ORDER_STATUS_CANCELLED,
                        details: taker_cancellation_reason.destroy_some()
                    }
                );
                return OrderMatchResult {
                    order_id,
                    remaining_size,
                    cancel_reason: option::some(
                        OrderCancellationReason::ClearinghouseSettleViolation
                    ),
                    num_fills
                };
            };

            if (remaining_size == 0) {
                break;
            };

            // Check if the next iteration will still match
            let is_taker_order =
                self.order_book.is_taker_order(price, is_buy, option::none());
            if (!is_taker_order) {
                if (time_in_force == TIME_IN_FORCE_IOC) {
                    event::emit(
                        OrderEvent {
                            parent: self.parent,
                            market: self.market,
                            order_id,
                            user: user_addr,
                            orig_size,
                            remaining_size,
                            size_delta: remaining_size,
                            price,
                            is_buy,
                            // NOTE: Keep consistent with all the logs we've
                            // emitted for this taker order
                            is_taker: true,
                            status: ORDER_STATUS_CANCELLED,
                            details: std::string::utf8(b"IOC_VIOLATION")
                        }
                    );
                    return OrderMatchResult {
                        order_id,
                        remaining_size,
                        cancel_reason: option::some(OrderCancellationReason::IOCViolation),
                        num_fills
                    };
                };
                event::emit(
                    OrderEvent {
                        parent: self.parent,
                        market: self.market,
                        order_id,
                        user: user_addr,
                        orig_size,
                        remaining_size,
                        size_delta: orig_size,
                        price,
                        is_buy,
                        is_taker: false,
                        status: ORDER_STATUS_OPEN,
                        details: std::string::utf8(b"")
                    }
                );
                self.order_book.place_maker_order(
                    new_order_request(
                        user_addr,
                        order_id,
                        unique_priority_idx,
                        price,
                        orig_size,
                        remaining_size,
                        is_buy,
                        trigger_condition,
                        metadata
                    )
                );
                break;
            };

            if (num_fills >= max_fill_limit) {
                if (emit_cancel_on_fill_limit) {
                    event::emit(
                        OrderEvent {
                            parent: self.parent,
                            market: self.market,
                            order_id,
                            user: user_addr,
                            orig_size,
                            remaining_size,
                            size_delta: remaining_size,
                            price,
                            is_buy,
                            is_taker: true,
                            status: ORDER_STATUS_CANCELLED,
                            details: std::string::utf8(b"Fill limit reached")
                        }
                    );
                };
                return OrderMatchResult {
                    order_id,
                    remaining_size,
                    cancel_reason: option::some(
                        OrderCancellationReason::MaxFillLimitViolation
                    ),
                    num_fills
                };
            };
        };
        OrderMatchResult {
            order_id,
            remaining_size,
            cancel_reason: option::none(),
            num_fills
        }
    }

    public fun cancel_order<M: store + copy + drop>(
        self: &mut Market<M>, user: &signer, order_id: u64
    ) {
        let account = signer::address_of(user);
        let maybe_order = self.order_book.cancel_order(account, order_id);
        if (maybe_order.is_some()) {
            let order = maybe_order.destroy_some();
            let (
                order_id_type,
                _unique_priority_idx,
                price,
                orig_size,
                remaining_size,
                is_buy,
                _trigger_condition,
                _metadata
            ) = order.destroy_order();
            let (user, order_id) = order_id_type.destroy_order_id_type();
            event::emit(
                OrderEvent {
                    parent: self.parent,
                    market: self.market,
                    order_id,
                    user,
                    orig_size,
                    remaining_size,
                    size_delta: remaining_size,
                    price,
                    is_buy,
                    is_taker: false,
                    status: ORDER_STATUS_CANCELLED,
                    details: std::string::utf8(b"Order cancelled")
                }
            )
        }
    }

    public fun get_remaining_size<M: store + copy + drop>(
        self: &Market<M>, user: address, order_id: u64
    ): u64 {
        self.order_book.get_remaining_size(user, order_id)
    }

    public fun take_ready_price_based_orders<M: store + copy + drop>(
        self: &mut Market<M>, oracle_price: u64
    ): vector<Order<M>> {
        self.order_book.take_ready_price_based_orders(oracle_price)
    }

    public fun trigger_price_based_orders<M: store + copy + drop>(
        self: &mut Market<M>,
        oracle_price: u64,
        callbacks: &MarketClearinghouseCallbacks<M>
    ) {
        let ready_orders = self.order_book.take_ready_price_based_orders(oracle_price);
        let i = 0;
        while (i < ready_orders.length()) {
            let order = ready_orders[i];
            let (order_id, unique_priority_idx, price, orig_size, _, is_buy, _, metadata) =

                order.destroy_order();
            let (user_addr, order_id) = order_id.destroy_order_id_type();
            self.place_order_with_order_id(
                user_addr,
                price,
                orig_size,
                orig_size,
                is_buy,
                TIME_IN_FORCE_GTC, // TODO(skedia): Add time in force to the Order metadata and retrieve it here
                option::none(),
                metadata,
                order_id,
                option::some(unique_priority_idx),
                1000, // TODO(skedia): Add support for fill limit here.
                false,
                true,
                callbacks
            );
            i += 1;
        };
    }

    public fun take_ready_time_based_orders<M: store + copy + drop>(
        self: &mut Market<M>
    ): vector<Order<M>> {
        self.order_book.take_ready_time_based_orders()
    }
}

/// This module provides a generic trading engine implementation for a market. On a high level, its a data structure,
/// that stores an order book and provides APIs to place orders, cancel orders, and match orders. The market also acts
/// as a wrapper around the order book and pluggable clearinghouse implementation.
/// A clearing house implementation is expected to implement the following APIs
///  - settle_trade(taker, maker, taker_order_id, maker_order_id, fill_id, is_taker_long, price, size): SettleTradeResult ->
/// Called by the market when there is an match between taker and maker. The clearinghouse is expected to settle the trade
/// and return the result. Please note that the clearing house settlment size might not be the same as the order match size and
/// the settlement might also fail. The fill_id is an incremental counter for matched orders and can be used to track specific fills
///  - validate_order_placement(account, is_taker, is_long, price, size): bool -> Called by the market to validate
///  an order when its placed. The clearinghouse is expected to validate the order and return true if the order is valid.
///  Checkout clearinghouse_test as an example of the simplest form of clearing house implementation that just tracks
///  the position size of the user and does not do any validation.
///
/// - place_maker_order(account, order_id, is_bid, price, size, metadata) -> Called by the market before placing the
/// maker order in the order book. The clearinghouse can use this to track pending orders in the order book and perform
/// any other book keeping operations.
///
/// - cleanup_order(account, order_id, is_bid, remaining_size) -> Called by the market when an order is cancelled or fully filled
/// The clearinhouse can perform any cleanup operations like removing the order from the pending orders list. For every order placement
/// that passes the validate_order_placement check,
/// the market guarantees that the cleanup_order API will be called once and only once with the remaining size of the order.
///
/// - decrease_order_size(account, order_id, is_bid, price, size) -> Called by the market when a maker order is decreased
/// in size by the user. Please note that this API will only be called after place_maker_order is called and the order is
/// already in the order book. Size in this case is the remaining size of the order after the decrease.
///
/// Following are some valid sequence of API calls that the market makes to the clearinghouse:
/// 1. validate_order_placement(10)
/// 2. settle_trade(2)
/// 3. settle_trade(3)
/// 4. place_maker_order(5)
/// 5. decrease_order_size(2)
/// 6. decrease_order_size(1)
/// 7. cleanup_order(2)
/// or
/// 1. validate_order_placement(10)
/// 2. cleanup_order(10)
///
/// Upon placement of an order, the market generates an order id and emits an event with the order details - the order id
/// is a unique id for the order that can be used to later get the status of the order or cancel the order.
///
/// Market also supports various conditions for order matching like Good Till Cancelled (GTC), Post Only, Immediate or Cancel (IOC).
/// GTC orders are orders that are valid until they are cancelled or filled. Post Only orders are orders that are valid only if they are not
/// taker orders. IOC orders are orders that are valid only if they are taker orders.
///
/// In addition, the market also supports trigger conditions for orders. An order with trigger condition is not put
/// on the order book until its trigger conditions are met. Following trigger conditions are supported:
/// TakeProfit(price): If its a buy order its triggered when the market price is greater than or equal to the price. If
/// its a sell order its triggered when the market price is less than or equal to the price.
/// StopLoss(price): If its a buy order its triggered when the market price is less than or equal to the price. If its
/// a sell order its triggered when the market price is greater than or equal to the price.
/// TimeBased(time): The order is triggered when the current time is greater than or equal to the time.
///
module aptos_experimental::market {

    use std::option;
    use std::option::Option;
    use std::signer;
    use std::string::String;
    use std::vector;
    use aptos_framework::event;
    use aptos_experimental::order_book::{OrderBook, new_order_book, new_order_request};
    use aptos_experimental::order_book_types::{TriggerCondition, Order};
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

    /// Order time in force
    /// Good till cancelled order type
    const TIME_IN_FORCE_GTC: u8 = 0;
    /// Post Only order type - ensures that the order is not a taker order
    const TIME_IN_FORCE_POST_ONLY: u8 = 1;
    /// Immediate or Cancel order type - ensures that the order is a taker order. Try to match as much of the
    /// order as possible as taker order and cancel the rest.
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

    struct Market<M: store + copy + drop> has store {
        /// Address of the parent object that created this market
        /// Purely for grouping events based on the source DEX, not used otherwise
        parent: address,
        /// Address of the market object of this market.
        market: address,
        // TODO: remove sequential order id generation
        last_order_id: u64,
        // Incremental fill id for matched orders
        next_fill_id: u64,
        config: MarketConfig,
        order_book: OrderBook<M>
    }

    struct MarketConfig has store {
        /// Weather to allow self matching orders
        allow_self_trade: bool,
        /// Whether to allow sending all events for the markett
        allow_events_emission: bool
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
    const ORDER_SIZE_REDUCED: u8 = 4;

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
        /// Original size of the order
        orig_size: u64,
        /// Remaining size of the order in the order book
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
        /// Whether the order crosses the orderbook.
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
        fill_sizes: vector<u64>
    }

    public fun destroy_order_match_result(
        self: OrderMatchResult
    ): (u64, u64, Option<OrderCancellationReason>, vector<u64>) {
        let OrderMatchResult { order_id, remaining_size, cancel_reason, fill_sizes } =
            self;
        (order_id, remaining_size, cancel_reason, fill_sizes)
    }

    public fun number_of_fills(self: &OrderMatchResult): u64 {
        self.fill_sizes.length()
    }

    public fun total_fill_size(self: &OrderMatchResult): u64 {
        self.fill_sizes.fold(0, |acc, fill_size| acc + fill_size)
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

    public fun new_market_config(
        allow_self_matching: bool, allow_events_emission: bool
    ): MarketConfig {
        MarketConfig { allow_self_trade: allow_self_matching, allow_events_emission: allow_events_emission }
    }

    public fun new_market<M: store + copy + drop>(
        parent: &signer, market: &signer, config: MarketConfig
    ): Market<M> {
        // requiring signers, and not addresses, purely to guarantee different dexes
        // cannot polute events to each other, accidentally or maliciously.
        Market {
            parent: signer::address_of(parent),
            market: signer::address_of(market),
            last_order_id: 0,
            next_fill_id: 0,
            config,
            order_book: new_order_book()
        }
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
        self: &Market<M>,
        price: u64,
        is_buy: bool,
        trigger_condition: Option<TriggerCondition>
    ): bool {
        self.order_book.is_taker_order(price, is_buy, trigger_condition)
    }

    /// Places an order - If its a taker order, it will be matched immediately and if its a maker order, it will simply
    /// be placed in the order book. An order id is generated when the order is placed and this id can be used to
    /// uniquely identify the order for this market and can also be used to get the status of the order or cancel the order.
    /// The order is placed with the following parameters:
    /// - user: The user who is placing the order
    /// - price: The price at which the order is placed
    /// - orig_size: The original size of the order
    /// - is_buy: Whether the order is a buy order or a sell order
    /// - time_in_force: The time in force for the order. This can be one of the following:
    ///  - TIME_IN_FORCE_GTC: Good till cancelled order type
    /// - TIME_IN_FORCE_POST_ONLY: Post Only order type - ensures that the order is not a taker order
    /// - TIME_IN_FORCE_IOC: Immediate or Cancel order type - ensures that the order is a taker order. Try to match as much of the
    /// order as possible as taker order and cancel the rest.
    /// - trigger_condition: The trigger condition
    /// - metadata: The metadata for the order. This can be any type that the clearing house implementation supports.
    /// - max_fill_limit: The maximum fill limit for the order. This is the maximum number of fills to trigger for this order.
    /// This knob is present to configure maximum amount of gas any order placement transaction might consume and avoid
    /// hitting the maximum has limit of the blockchain.
    /// - emit_cancel_on_fill_limit: bool,: Whether to emit an order cancellation event when the fill limit is reached.
    /// This is used ful as the caller might not want to cancel the order when the limit is reached and can continue
    /// that order in a separate transaction.
    /// - callbacks: The callbacks for the market clearinghouse. This is a struct that implements the MarketClearinghouseCallbacks
    /// interface. This is used to validate the order and settle the trade.
    /// Returns the order id, remaining size, cancel reason and number of fills for the order.
    public fun place_order<M: store + copy + drop>(
        self: &mut Market<M>,
        user: &signer,
        price: u64,
        orig_size: u64,
        is_bid: bool,
        time_in_force: u8,
        trigger_condition: Option<TriggerCondition>,
        metadata: M,
        max_fill_limit: u64,
        emit_cancel_on_fill_limit: bool,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): OrderMatchResult {
        let order_id = self.next_order_id();
        self.place_order_with_order_id(
            signer::address_of(user),
            price,
            orig_size,
            orig_size,
            is_bid,
            time_in_force,
            trigger_condition,
            metadata,
            order_id,
            max_fill_limit,
            emit_cancel_on_fill_limit,
            true,
            callbacks
        )
    }

    public fun next_order_id<M: store + copy + drop>(self: &mut Market<M>): u64 {
        self.last_order_id += 1;
        self.last_order_id
    }

    fun next_fill_id<M: store + copy + drop>(self: &mut Market<M>): u64 {
        let next_fill_id = self.next_fill_id;
        self.next_fill_id += 1;
        next_fill_id
    }

    fun emit_event_for_order<M: store + copy + drop>(
        self: &Market<M>,
        order_id: u64,
        user: address,
        orig_size: u64,
        remaining_size: u64,
        size_delta: u64,
        price: u64,
        is_bid: bool,
        is_taker: bool,
        status: u8,
        details: &String
    ) {
        // Final check whether event sending is enabled
        if (self.config.allow_events_emission) {
            event::emit(
                OrderEvent {
                    parent: self.parent,
                    market: self.market,
                    order_id,
                    user,
                    orig_size,
                    remaining_size,
                    size_delta,
                    price,
                    is_buy: is_bid,
                    is_taker,
                    status,
                    details: *details
                }
            );
        };
    }

    /// Similar to `place_order` API but instead of a signer, it takes a user address - can be used in case trading
    /// functionality is delegated to a different address. Please note that it is the responsibility of the caller
    /// to verify that the transaction signer is authorized to place orders on behalf of the user.
    public fun place_order_with_user_addr<M: store + copy + drop>(
        self: &mut Market<M>,
        user_addr: address,
        price: u64,
        orig_size: u64,
        is_bid: bool,
        time_in_force: u8,
        trigger_condition: Option<TriggerCondition>,
        metadata: M,
        max_fill_limit: u64,
        emit_cancel_on_fill_limit: bool,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): OrderMatchResult {
        let order_id = self.next_order_id();
        self.place_order_with_order_id(
            user_addr,
            price,
            orig_size,
            orig_size,
            is_bid,
            time_in_force,
            trigger_condition,
            metadata,
            order_id,
            max_fill_limit,
            emit_cancel_on_fill_limit,
            true,
            callbacks
        )
    }

    fun place_maker_order_internal<M: store + copy + drop>(
        self: &mut Market<M>,
        user_addr: address,
        price: u64,
        orig_size: u64,
        remaining_size: u64,
        fill_sizes: vector<u64>,
        is_bid: bool,
        time_in_force: u8,
        trigger_condition: Option<TriggerCondition>,
        metadata: M,
        order_id: u64,
        emit_order_open: bool,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): OrderMatchResult {
        // Validate that the order is valid from position management perspective
        if (time_in_force == TIME_IN_FORCE_IOC) {
            return self.cancel_order_internal(
                user_addr,
                price,
                order_id,
                orig_size,
                remaining_size,
                fill_sizes,
                is_bid,
                false, // is_taker
                OrderCancellationReason::IOCViolation,
                std::string::utf8(b"IOC Violation"),
                callbacks
            );
        };

        if (emit_order_open) {
            emit_event_for_order(
                self,
                order_id,
                user_addr,
                orig_size,
                remaining_size,
                orig_size,
                price,
                is_bid,
                false, // is_taker
                ORDER_STATUS_OPEN,
                &std::string::utf8(b"")
            );
        };

        callbacks.place_maker_order(
            user_addr, order_id, is_bid, price, remaining_size, metadata
        );
        self.order_book.place_maker_order(
            new_order_request(
                user_addr,
                order_id,
                option::none(),
                price,
                orig_size,
                remaining_size,
                is_bid,
                trigger_condition,
                metadata
            )
        );
        return OrderMatchResult {
            order_id,
            remaining_size,
            cancel_reason: option::none(),
            fill_sizes
        }
    }

    fun cancel_maker_order_internal<M: store + copy + drop>(
        self: &mut Market<M>,
        maker_order: &Order<M>,
        order_id: u64,
        maker_address: address,
        maker_cancellation_reason: String,
        unsettled_size: u64,
        callbacks: &MarketClearinghouseCallbacks<M>
    ) {
        let maker_cancel_size = unsettled_size + maker_order.get_remaining_size();

        emit_event_for_order(
            self,
            order_id,
            maker_address,
            maker_order.get_orig_size(),
            0,
            maker_cancel_size,
            maker_order.get_price(),
            maker_order.is_bid(),
            false,
            ORDER_STATUS_CANCELLED,
            &maker_cancellation_reason
        );
        // If the maker is invalid cancel the maker order and continue to the next maker order
        if (maker_order.get_remaining_size() != 0) {
            self.order_book.cancel_order(maker_address, order_id);
        };
        callbacks.cleanup_order(
            maker_address, order_id, maker_order.is_bid(), maker_cancel_size
        );
    }

    fun cancel_order_internal<M: store + copy + drop>(
        self: &mut Market<M>,
        user_addr: address,
        price: u64,
        order_id: u64,
        orig_size: u64,
        size_delta: u64,
        fill_sizes: vector<u64>,
        is_bid: bool,
        is_taker: bool,
        cancel_reason: OrderCancellationReason,
        cancel_details: String,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): OrderMatchResult {
        emit_event_for_order(
            self,
            order_id,
            user_addr,
            orig_size,
            0, // remaining size
            size_delta,
            price,
            is_bid,
            is_taker,
            ORDER_STATUS_CANCELLED,
            &cancel_details
        );
        callbacks.cleanup_order(
            user_addr, order_id, is_bid, size_delta
        );
        return OrderMatchResult {
            order_id,
            remaining_size: 0,
            cancel_reason: option::some(cancel_reason),
            fill_sizes
        }
    }

    /// Similar to `place_order` API but allows few extra parameters as follows
    /// - order_id: The order id for the order - this is needed because for orders with trigger conditions, the order
    /// id is generated when the order is placed and when they are triggered, the same order id is used to match the order.
    /// - emit_taker_order_open: bool: Whether to emit an order open event for the taker order - this is used when
    /// the caller do not wants to emit an open order event for a taker in case the taker order was intterrupted because
    /// of fill limit violation  in the previous transaction and the order is just a continuation of the previous order.
    public fun place_order_with_order_id<M: store + copy + drop>(
        self: &mut Market<M>,
        user_addr: address,
        price: u64,
        orig_size: u64,
        remaining_size: u64,
        is_bid: bool,
        time_in_force: u8,
        trigger_condition: Option<TriggerCondition>,
        metadata: M,
        order_id: u64,
        max_fill_limit: u64,
        cancel_on_fill_limit: bool,
        emit_taker_order_open: bool,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): OrderMatchResult {
        assert!(
            orig_size > 0 && remaining_size > 0,
            EINVALID_ORDER
        );
        // TODO(skedia) is_taker_order API can actually return false positive as the maker orders might not be valid.
        // Changes are needed to ensure the maker order is valid for this order to be a valid taker order.
        // TODO(skedia) reconsile the semantics around global order id vs account local id.
        if (
            !callbacks.validate_order_placement(
                user_addr,
                order_id,
                true, // is_taker
                is_bid,
                price,
                remaining_size,
                metadata
            )) {
            return self.cancel_order_internal(
                user_addr,
                price,
                order_id,
                orig_size,
                0, // 0 because order was never placed
                vector[],
                is_bid,
                true, // is_taker
                OrderCancellationReason::PositionUpdateViolation,
                std::string::utf8(b"Position Update violation"),
                callbacks
            );
        };

        let is_taker_order =
            self.order_book.is_taker_order(price, is_bid, trigger_condition);
        if (emit_taker_order_open) {
            emit_event_for_order(
                self,
                order_id,
                user_addr,
                orig_size,
                remaining_size,
                orig_size,
                price,
                is_bid,
                is_taker_order,
                ORDER_STATUS_OPEN,
                &std::string::utf8(b"")
            );
        };
        if (!is_taker_order) {
            return self.place_maker_order_internal(
                user_addr,
                price,
                orig_size,
                remaining_size,
                vector[],
                is_bid,
                time_in_force,
                trigger_condition,
                metadata,
                order_id,
                false,
                callbacks
            );
        };

        // NOTE: We should always use is_taker: true for this order past this
        // point so that indexer can consistently track the order's status
        if (time_in_force == TIME_IN_FORCE_POST_ONLY) {
            return self.cancel_order_internal(
                user_addr,
                price,
                order_id,
                orig_size,
                remaining_size,
                vector[],
                is_bid,
                true, // is_taker
                OrderCancellationReason::PostOnlyViolation,
                std::string::utf8(b"Post Only violation"),
                callbacks
            );
        };
        let fill_sizes = vector::empty();
        loop {
            let result =
                self.order_book.get_single_match_for_taker(price, remaining_size, is_bid);
            let (maker_order, maker_matched_size) = result.destroy_single_order_match();
            let (maker_address, maker_order_id) =
                maker_order.get_order_id().destroy_order_id_type();
            if (!self.config.allow_self_trade && maker_address == user_addr) {
                self.cancel_maker_order_internal(
                    &maker_order,
                    maker_order_id,
                    maker_address,
                    std::string::utf8(b"Disallowed self trading"),
                    maker_matched_size,
                    callbacks
                );
                continue;
            };

            let fill_id = self.next_fill_id();

            let settle_result =
                callbacks.settle_trade(
                    user_addr,
                    maker_address,
                    order_id,
                    maker_order_id,
                    fill_id,
                    is_bid,
                    maker_order.get_price(), // Order is always matched at the price of the maker
                    maker_matched_size,
                    metadata,
                    maker_order.get_metadata_from_order()
                );

            let unsettled_maker_size = maker_matched_size;
            let settled_size = settle_result.get_settled_size();
            if (settled_size > 0) {
                remaining_size -= settled_size;
                unsettled_maker_size -= settled_size;
                fill_sizes.push_back(settled_size);
                // Event for taker fill
                emit_event_for_order(
                    self,
                    order_id,
                    user_addr,
                    orig_size,
                    remaining_size,
                    settled_size,
                    maker_order.get_price(),
                    is_bid,
                    true, // is_taker
                    ORDER_STATUS_FILLED,
                    &std::string::utf8(b"")
                );
                // Event for maker fill
                emit_event_for_order(
                    self,
                    maker_order_id,
                    maker_address,
                    maker_order.get_orig_size(),
                    maker_order.get_remaining_size() + unsettled_maker_size,
                    settled_size,
                    maker_order.get_price(),
                    !is_bid,
                    false, // is_taker
                    ORDER_STATUS_FILLED,
                    &std::string::utf8(b"")
                );
            };

            let maker_cancellation_reason = settle_result.get_maker_cancellation_reason();
            if (maker_cancellation_reason.is_some()) {
                self.cancel_maker_order_internal(
                    &maker_order,
                    maker_order_id,
                    maker_address,
                    maker_cancellation_reason.destroy_some(),
                    unsettled_maker_size,
                    callbacks
                );
            };

            let taker_cancellation_reason = settle_result.get_taker_cancellation_reason();
            if (taker_cancellation_reason.is_some()) {
                let result =
                    self.cancel_order_internal(
                        user_addr,
                        price,
                        order_id,
                        orig_size,
                        remaining_size,
                        fill_sizes,
                        is_bid,
                        true, // is_taker
                        OrderCancellationReason::ClearinghouseSettleViolation,
                        taker_cancellation_reason.destroy_some(),
                        callbacks
                    );
                if (maker_cancellation_reason.is_none() && unsettled_maker_size > 0) {
                    // If the taker is cancelled but the maker is not cancelled, then we need to re-insert
                    // the maker order back into the order book
                    self.order_book.reinsert_maker_order(
                        new_order_request(
                            maker_address,
                            maker_order_id,
                            option::some(maker_order.get_unique_priority_idx()),
                            maker_order.get_price(),
                            maker_order.get_orig_size(),
                            unsettled_maker_size,
                            !is_bid,
                            option::none(),
                            maker_order.get_metadata_from_order()
                        )
                    );
                };
                return result;
            };

            if (maker_order.get_remaining_size() == 0) {
                callbacks.cleanup_order(
                    maker_address,
                    maker_order_id,
                    !is_bid, // is_bid is inverted for maker orders
                    0 // 0 because the order is fully filled
                );
            };
            if (remaining_size == 0) {
                callbacks.cleanup_order(
                    user_addr, order_id, is_bid, 0 // 0 because the order is fully filled
                );
                break;
            };

            // Check if the next iteration will still match
            let is_taker_order =
                self.order_book.is_taker_order(price, is_bid, option::none());
            if (!is_taker_order) {
                if (time_in_force == TIME_IN_FORCE_IOC) {
                    return self.cancel_order_internal(
                        user_addr,
                        price,
                        order_id,
                        orig_size,
                        remaining_size,
                        fill_sizes,
                        is_bid,
                        true, // is_taker
                        OrderCancellationReason::IOCViolation,
                        std::string::utf8(b"IOC_VIOLATION"),
                        callbacks
                    );
                } else {
                    // If the order is not a taker order, then we can place it as a maker order
                    return self.place_maker_order_internal(
                        user_addr,
                        price,
                        orig_size,
                        remaining_size,
                        fill_sizes,
                        is_bid,
                        time_in_force,
                        trigger_condition,
                        metadata,
                        order_id,
                        true, // emit_order_open
                        callbacks
                    );
                };
            };

            if (fill_sizes.length() >= max_fill_limit) {
                if (cancel_on_fill_limit) {
                    return self.cancel_order_internal(
                        user_addr,
                        price,
                        order_id,
                        orig_size,
                        remaining_size,
                        fill_sizes,
                        is_bid,
                        true, // is_taker
                        OrderCancellationReason::MaxFillLimitViolation,
                        std::string::utf8(b"Max fill limit reached"),
                        callbacks
                    );
                } else {
                    return OrderMatchResult {
                        order_id,
                        remaining_size,
                        cancel_reason: option::some(
                            OrderCancellationReason::MaxFillLimitViolation
                        ),
                        fill_sizes
                    }
                };
            };
        };
        OrderMatchResult {
            order_id,
            remaining_size,
            cancel_reason: option::none(),
            fill_sizes
        }
    }

    /// Cancels an order - this will cancel the order and emit an event for the order cancellation.
    public fun cancel_order<M: store + copy + drop>(
        self: &mut Market<M>,
        user: &signer,
        order_id: u64,
        callbacks: &MarketClearinghouseCallbacks<M>
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
                is_bid,
                _trigger_condition,
                _metadata
            ) = order.destroy_order();
            callbacks.cleanup_order(
                account, order_id, is_bid, remaining_size
            );
            let (user, order_id) = order_id_type.destroy_order_id_type();
            emit_event_for_order(
                self,
                order_id,
                user,
                orig_size,
                remaining_size,
                remaining_size,
                price,
                is_bid,
                false, // is_taker
                ORDER_STATUS_CANCELLED,
                &std::string::utf8(b"Order cancelled")
            );
        }
    }

    /// Cancels an order - this will cancel the order and emit an event for the order cancellation.
    public fun decrease_order_size<M: store + copy + drop>(
        self: &mut Market<M>,
        user: &signer,
        order_id: u64,
        size_delta: u64,
        callbacks: &MarketClearinghouseCallbacks<M>
    ) {
        let account = signer::address_of(user);
        self.order_book.decrease_order_size(account, order_id, size_delta);
        let maybe_order = self.order_book.get_order(account, order_id);
        assert!(maybe_order.is_some(), EORDER_DOES_NOT_EXIST);
        let (order, _) = maybe_order.destroy_some().destroy_order_from_state();
        let (
            order_id_type,
            _unique_priority_idx,
            price,
            orig_size,
            remaining_size,
            is_bid,
            _trigger_condition,
            _metadata
        ) = order.destroy_order();
        let (user, order_id) = order_id_type.destroy_order_id_type();
        callbacks.decrease_order_size(
            user, order_id, is_bid, price, remaining_size
        );

        emit_event_for_order(
            self,
            order_id,
            user,
            orig_size,
            remaining_size,
            size_delta,
            price,
            is_bid,
            false, // is_taker
            ORDER_SIZE_REDUCED,
            &std::string::utf8(b"Order size reduced")
        );
    }

    /// Remaining size of the order in the order book.
    public fun get_remaining_size<M: store + copy + drop>(
        self: &Market<M>, user: address, order_id: u64
    ): u64 {
        self.order_book.get_remaining_size(user, order_id)
    }

    /// Returns all the pending order ready to be executed based on the oracle price. The caller is responsible to
    /// call the `place_order_with_order_id` API to place the order with the order id returned from this API.
    public fun take_ready_price_based_orders<M: store + copy + drop>(
        self: &mut Market<M>, oracle_price: u64
    ): vector<Order<M>> {
        self.order_book.take_ready_price_based_orders(oracle_price)
    }

    /// Returns all the pending order that are ready to be executed based on current time stamp. The caller is responsible to
    /// call the `place_order_with_order_id` API to place the order with the order id returned from this API.
    public fun take_ready_time_based_orders<M: store + copy + drop>(
        self: &mut Market<M>
    ): vector<Order<M>> {
        self.order_book.take_ready_time_based_orders()
    }

    // ============================= test_only APIs ====================================
    #[test_only]
    public fun destroy_market<M: store + copy + drop>(self: Market<M>) {
        let Market {
            parent: _parent,
            market: _market,
            last_order_id: _last_order_id,
            next_fill_id: _next_fill_id,
            config,
            order_book
        } = self;
        let MarketConfig { allow_self_trade: _, allow_events_emission: _ } = config;
        order_book.destroy_order_book()
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
}

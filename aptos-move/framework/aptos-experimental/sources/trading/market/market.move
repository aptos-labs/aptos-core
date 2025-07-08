/// This module provides a generic trading engine implementation for a market. On a high level, its a data structure,
/// that stores an order book and provides APIs to place orders, cancel orders, and match orders. The market also acts
/// as a wrapper around the order book and pluggable clearinghouse implementation.
/// A clearing house implementation is expected to implement the following APIs
///  - settle_trade(taker, taker_order_id, maker, maker_order_id, fill_id, is_taker_long, price, size): SettleTradeResult ->
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
    use aptos_experimental::order_book_types::{
        new_order_id_type,
        new_ascending_id_generator,
        AscendingIdGenerator,
        TriggerCondition,
        Order,
        OrderIdType
    };
    use aptos_experimental::market_types::{
        Self,
        TimeInForce,
        OrderStatus,
        MarketClearinghouseCallbacks
    };

    // Error codes
    const EINVALID_ORDER: u64 = 1;
    const EORDER_BOOK_FULL: u64 = 2;
    const EMARKET_NOT_FOUND: u64 = 3;
    const ENOT_ADMIN: u64 = 4;
    const EINVALID_FEE_TIER: u64 = 5;
    const EORDER_DOES_NOT_EXIST: u64 = 6;
    const EINVALID_MATCHING_FOR_MAKER_REINSERT: u64 = 9;
    const EINVALID_TAKER_POSITION_UPDATE: u64 = 10;
    const EINVALID_LIQUIDATION: u64 = 11;
    const ENOT_ORDER_CREATOR: u64 = 12;

    enum Market<M: store + copy + drop> has store {
        V1 {
            /// Address of the parent object that created this market
            /// Purely for grouping events based on the source DEX, not used otherwise
            parent: address,
            /// Address of the market object of this market.
            market: address,
            order_id_generator: AscendingIdGenerator,
            // Incremental fill id for matched orders
            next_fill_id: u64,
            config: MarketConfig,
            order_book: OrderBook<M>
        }
    }

    enum MarketConfig has store {
        V1 {
            /// Weather to allow self matching orders
            allow_self_trade: bool,
            /// Whether to allow sending all events for the markett
            allow_events_emission: bool
        }
    }

    #[event]
    struct OrderEvent has drop, copy, store {
        parent: address,
        market: address,
        order_id: u128,
        client_order_id: Option<u64>,
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
        price: Option<u64>,
        is_bid: bool,
        /// Whether the order crosses the orderbook.
        is_taker: bool,
        status: OrderStatus,
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
        order_id: OrderIdType,
        remaining_size: u64,
        cancel_reason: Option<OrderCancellationReason>,
        fill_sizes: vector<u64>
    }

    public fun destroy_order_match_result(
        self: OrderMatchResult
    ): (OrderIdType, u64, Option<OrderCancellationReason>, vector<u64>) {
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

    public fun get_order_id(self: OrderMatchResult): OrderIdType {
        self.order_id
    }

    public fun new_market_config(
        allow_self_matching: bool, allow_events_emission: bool
    ): MarketConfig {
        MarketConfig::V1 {
            allow_self_trade: allow_self_matching,
            allow_events_emission: allow_events_emission
        }
    }

    public fun new_market<M: store + copy + drop>(
        parent: &signer, market: &signer, config: MarketConfig
    ): Market<M> {
        // requiring signers, and not addresses, purely to guarantee different dexes
        // cannot polute events to each other, accidentally or maliciously.
        Market::V1 {
            parent: signer::address_of(parent),
            market: signer::address_of(market),
            order_id_generator: new_ascending_id_generator(),
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
        price: Option<u64>,
        is_bid: bool,
        trigger_condition: Option<TriggerCondition>
    ): bool {
        self.order_book.is_taker_order(price, is_bid, trigger_condition)
    }

    /// Places a limt order - If its a taker order, it will be matched immediately and if its a maker order, it will simply
    /// be placed in the order book. An order id is generated when the order is placed and this id can be used to
    /// uniquely identify the order for this market and can also be used to get the status of the order or cancel the order.
    /// The order is placed with the following parameters:
    /// - user: The user who is placing the order
    /// - price: The price at which the order is placed
    /// - orig_size: The original size of the order
    /// - is_bid: Whether the order is a buy order or a sell order
    /// - time_in_force: The time in force for the order. This can be one of the following:
    ///  - TimeInForce::GTC: Good till cancelled order type
    /// - TimeInForce::POST_ONLY: Post Only order type - ensures that the order is not a taker order
    /// - TimeInForce::IOC: Immediate or Cancel order type - ensures that the order is a taker order. Try to match as much of the
    /// order as possible as taker order and cancel the rest.
    /// - trigger_condition: The trigger condition
    /// - metadata: The metadata for the order. This can be any type that the clearing house implementation supports.
    /// - client_order_id: The client order id for the order. This is an optional field that can be specified by the client
    ///   is solely used for their own tracking of the order. client order id doesn't have semantic meaning and
    ///   is not be inspected by the orderbook internally.
    /// - max_fill_limit: The maximum fill limit for the order. This is the maximum number of fills to trigger for this order.
    /// This knob is present to configure maximum amount of gas any order placement transaction might consume and avoid
    /// hitting the maximum has limit of the blockchain.
    /// - emit_cancel_on_fill_limit: bool,: Whether to emit an order cancellation event when the fill limit is reached.
    /// This is used ful as the caller might not want to cancel the order when the limit is reached and can continue
    /// that order in a separate transaction.
    /// - callbacks: The callbacks for the market clearinghouse. This is a struct that implements the MarketClearinghouseCallbacks
    /// interface. This is used to validate the order and settle the trade.
    /// Returns the order id, remaining size, cancel reason and number of fills for the order.
    public fun place_limit_order<M: store + copy + drop>(
        self: &mut Market<M>,
        user: &signer,
        limit_price: u64,
        orig_size: u64,
        is_bid: bool,
        time_in_force: TimeInForce,
        trigger_condition: Option<TriggerCondition>,
        metadata: M,
        client_order_id: Option<u64>,
        max_fill_limit: u64,
        emit_cancel_on_fill_limit: bool,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): OrderMatchResult {
        self.place_order_with_order_id(
            signer::address_of(user),
            option::some(limit_price),
            orig_size,
            orig_size,
            is_bid,
            time_in_force,
            trigger_condition,
            metadata,
            option::none(), // order_id
            client_order_id,
            max_fill_limit,
            emit_cancel_on_fill_limit,
            true,
            callbacks
        )
    }

    /// Places a market order - The order is guaranteed to be a taker order and will be matched immediately.
    public fun place_market_order<M: store + copy + drop>(
        self: &mut Market<M>,
        user: &signer,
        orig_size: u64,
        is_bid: bool,
        metadata: M,
        client_order_id: Option<u64>,
        max_fill_limit: u64,
        emit_cancel_on_fill_limit: bool,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): OrderMatchResult {
        self.place_order_with_order_id(
            signer::address_of(user),
            option::none(),
            orig_size,
            orig_size,
            is_bid,
            market_types::immediate_or_cancel(), // market orders are always IOC
            option::none(), // trigger_condition
            metadata,
            option::none(), // order_id
            client_order_id,
            max_fill_limit,
            emit_cancel_on_fill_limit,
            true,
            callbacks
        )
    }

    public fun next_order_id<M: store + copy + drop>(self: &mut Market<M>): OrderIdType {
        new_order_id_type(self.order_id_generator.next_ascending_id())
    }

    fun next_fill_id<M: store + copy + drop>(self: &mut Market<M>): u64 {
        let next_fill_id = self.next_fill_id;
        self.next_fill_id += 1;
        next_fill_id
    }

    fun emit_event_for_order<M: store + copy + drop>(
        self: &Market<M>,
        order_id: OrderIdType,
        client_order_id: Option<u64>,
        user: address,
        orig_size: u64,
        remaining_size: u64,
        size_delta: u64,
        price: Option<u64>,
        is_bid: bool,
        is_taker: bool,
        status: OrderStatus,
        details: &String
    ) {
        // Final check whether event sending is enabled
        if (self.config.allow_events_emission) {
            event::emit(
                OrderEvent {
                    parent: self.parent,
                    market: self.market,
                    order_id: order_id.get_order_id_value(),
                    client_order_id,
                    user,
                    orig_size,
                    remaining_size,
                    size_delta,
                    price,
                    is_bid: is_bid,
                    is_taker,
                    status,
                    details: *details
                }
            );
        };
    }

    fun place_maker_order_internal<M: store + copy + drop>(
        self: &mut Market<M>,
        user_addr: address,
        limit_price: Option<u64>,
        orig_size: u64,
        remaining_size: u64,
        fill_sizes: vector<u64>,
        is_bid: bool,
        time_in_force: TimeInForce,
        trigger_condition: Option<TriggerCondition>,
        metadata: M,
        order_id: OrderIdType,
        client_order_id: Option<u64>,
        emit_order_open: bool,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): OrderMatchResult {
        // Validate that the order is valid from position management perspective
        if (time_in_force == market_types::immediate_or_cancel() || limit_price.is_none()) {
            return self.cancel_order_internal(
                user_addr,
                limit_price,
                order_id,
                client_order_id,
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
            self.emit_event_for_order(
                order_id,
                client_order_id,
                user_addr,
                orig_size,
                remaining_size,
                orig_size,
                limit_price,
                is_bid,
                false,
                market_types::order_status_open(),
                &std::string::utf8(b"")
            );
        };

        callbacks.place_maker_order(
            user_addr,
            order_id,
            is_bid,
            limit_price.destroy_some(),
            remaining_size,
            metadata
        );
        self.order_book.place_maker_order(
            new_order_request(
                user_addr,
                order_id,
                client_order_id,
                limit_price.destroy_some(),
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
        client_order_id: Option<u64>,
        maker_address: address,
        order_id: OrderIdType,
        maker_cancellation_reason: String,
        unsettled_size: u64,
        callbacks: &MarketClearinghouseCallbacks<M>
    ) {
        let maker_cancel_size = unsettled_size + maker_order.get_remaining_size();
        self.emit_event_for_order(
            order_id,
            client_order_id,
            maker_address,
            maker_order.get_orig_size(),
            0,
            maker_cancel_size,
            option::some(maker_order.get_price()),
            maker_order.is_bid(),
            false,
            market_types::order_status_cancelled(),
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
        limit_price: Option<u64>,
        order_id: OrderIdType,
        client_order_id: Option<u64>,
        orig_size: u64,
        size_delta: u64,
        fill_sizes: vector<u64>,
        is_bid: bool,
        is_taker: bool,
        cancel_reason: OrderCancellationReason,
        cancel_details: String,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): OrderMatchResult {
        self.emit_event_for_order(
            order_id,
            client_order_id,
            user_addr,
            orig_size,
            0,
            size_delta,
            limit_price,
            is_bid,
            is_taker,
            market_types::order_status_cancelled(),
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

    fun settle_single_trade<M: store + copy + drop>(
        self: &mut Market<M>,
        user_addr: address,
        price: Option<u64>,
        orig_size: u64,
        remaining_size: &mut u64,
        is_bid: bool,
        metadata: M,
        order_id: OrderIdType,
        client_order_id: Option<u64>,
        callbacks: &MarketClearinghouseCallbacks<M>,
        fill_sizes: &mut vector<u64>
    ): Option<OrderCancellationReason> {
        let result = self.order_book
            .get_single_match_for_taker(price, *remaining_size, is_bid);
        let (
            maker_order, maker_matched_size
        ) = result.destroy_single_order_match();
        if (!self.config.allow_self_trade && maker_order.get_account() == user_addr) {
            self.cancel_maker_order_internal(
                &maker_order,
                maker_order.get_client_order_id(),
                maker_order.get_account(),
                maker_order.get_order_id(),
                std::string::utf8(b"Disallowed self trading"),
                maker_matched_size,
                callbacks
            );
            return option::none();
        };
        let fill_id = self.next_fill_id();
        let settle_result = callbacks.settle_trade(
            user_addr,
            order_id,
            maker_order.get_account(),
            maker_order.get_order_id(),
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
            *remaining_size -= settled_size;
            unsettled_maker_size -= settled_size;
            fill_sizes.push_back(settled_size);
                // Event for taker fill
            self.emit_event_for_order(
                order_id,
                client_order_id,
                user_addr,
                orig_size,
                *remaining_size,
                settled_size,
                option::some(maker_order.get_price()),
                is_bid,
                true,
                market_types::order_status_filled(),
                &std::string::utf8(b"")
            );
            // Event for maker fill
            self.emit_event_for_order(
                maker_order.get_order_id(),
                maker_order.get_client_order_id(),
                maker_order.get_account(),
                maker_order.get_orig_size(),
                maker_order.get_remaining_size() + unsettled_maker_size,
                settled_size,
                option::some(maker_order.get_price()),
                !is_bid,
                false,
                market_types::order_status_filled(),
                &std::string::utf8(b"")
            );
        };

        let maker_cancellation_reason = settle_result.get_maker_cancellation_reason();

        let taker_cancellation_reason = settle_result.get_taker_cancellation_reason();
        if (taker_cancellation_reason.is_some()) {
            self.cancel_order_internal(
                user_addr,
                price,
                order_id,
                client_order_id,
                orig_size,
                *remaining_size,
                *fill_sizes,
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
                        maker_order.get_account(),
                        maker_order.get_order_id(),
                        maker_order.get_client_order_id(),
                        maker_order.get_price(),
                        maker_order.get_orig_size(),
                        unsettled_maker_size,
                        !is_bid,
                        option::none(),
                        maker_order.get_metadata_from_order()
                    ),
                    maker_order
                );
            };
            return option::some(OrderCancellationReason::ClearinghouseSettleViolation);
        };
        if (maker_cancellation_reason.is_some()) {
            self.cancel_maker_order_internal(
                &maker_order,
                maker_order.get_client_order_id(),
                maker_order.get_account(),
                maker_order.get_order_id(),
                maker_cancellation_reason.destroy_some(),
                unsettled_maker_size,
                callbacks
            );
        } else if (maker_order.get_remaining_size() == 0) {
            callbacks.cleanup_order(
                maker_order.get_account(),
                maker_order.get_order_id(),
                !is_bid, // is_bid is inverted for maker orders
                0 // 0 because the order is fully filled
            );
        };
        option::none()
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
        limit_price: Option<u64>,
        orig_size: u64,
        remaining_size: u64,
        is_bid: bool,
        time_in_force: TimeInForce,
        trigger_condition: Option<TriggerCondition>,
        metadata: M,
        order_id: Option<OrderIdType>,
        client_order_id: Option<u64>,
        max_fill_limit: u64,
        cancel_on_fill_limit: bool,
        emit_taker_order_open: bool,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): OrderMatchResult {
        assert!(
            orig_size > 0 && remaining_size > 0,
            EINVALID_ORDER
        );
        if (order_id.is_none()) {
            // If order id is not provided, generate a new order id
            order_id = option::some(self.next_order_id());
        };
        let order_id = order_id.destroy_some();
        // TODO(skedia) is_taker_order API can actually return false positive as the maker orders might not be valid.
        // Changes are needed to ensure the maker order is valid for this order to be a valid taker order.
        // TODO(skedia) reconsile the semantics around global order id vs account local id.
        let is_taker_order =
            self.order_book.is_taker_order(limit_price, is_bid, trigger_condition);
        if (
            !callbacks.validate_order_placement(
                user_addr,
                order_id,
                is_taker_order, // is_taker
                is_bid,
                limit_price,
                remaining_size,
                metadata
            )) {
            return self.cancel_order_internal(
                user_addr,
                limit_price,
                order_id,
                client_order_id,
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

        if (emit_taker_order_open) {
            self.emit_event_for_order(
                order_id,
                client_order_id,
                user_addr,
                orig_size,
                remaining_size,
                orig_size,
                limit_price,
                is_bid,
                is_taker_order,
                market_types::order_status_open(),
                &std::string::utf8(b"")
            );
        };
        if (!is_taker_order) {
            return self.place_maker_order_internal(
                user_addr,
                limit_price,
                orig_size,
                remaining_size,
                vector[],
                is_bid,
                time_in_force,
                trigger_condition,
                metadata,
                order_id,
                client_order_id,
                false,
                callbacks
            );
        };

        // NOTE: We should always use is_taker: true for this order past this
        // point so that indexer can consistently track the order's status
        if (time_in_force == market_types::post_only()) {
            return self.cancel_order_internal(
                user_addr,
                limit_price,
                order_id,
                client_order_id,
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
            let taker_cancellation_reason =
                self.settle_single_trade(
                    user_addr,
                    limit_price,
                    orig_size,
                    &mut remaining_size,
                    is_bid,
                    metadata,
                    order_id,
                    client_order_id,
                    callbacks,
                    &mut fill_sizes
                );
            if (taker_cancellation_reason.is_some()) {
                return OrderMatchResult {
                    order_id,
                    remaining_size,
                    cancel_reason: taker_cancellation_reason,
                    fill_sizes
                }
            };
            if (remaining_size == 0) {
                callbacks.cleanup_order(
                    user_addr, order_id, is_bid, 0 // 0 because the order is fully filled
                );
                break;
            };

            // Check if the next iteration will still match
            let is_taker_order =
                self.order_book.is_taker_order(limit_price, is_bid, option::none());
            if (!is_taker_order) {
                if (time_in_force == market_types::immediate_or_cancel()) {
                    return self.cancel_order_internal(
                        user_addr,
                        limit_price,
                        order_id,
                        client_order_id,
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
                        limit_price,
                        orig_size,
                        remaining_size,
                        fill_sizes,
                        is_bid,
                        time_in_force,
                        trigger_condition,
                        metadata,
                        order_id,
                        client_order_id,
                        true, // emit_order_open
                        callbacks
                    );
                };
            };

            if (fill_sizes.length() >= max_fill_limit) {
                if (cancel_on_fill_limit) {
                    return self.cancel_order_internal(
                        user_addr,
                        limit_price,
                        order_id,
                        client_order_id,
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
        order_id: OrderIdType,
        callbacks: &MarketClearinghouseCallbacks<M>
    ) {
        let account = signer::address_of(user);
        let order = self.order_book.cancel_order(account, order_id);
        assert!(account == order.get_account(), ENOT_ORDER_CREATOR);
        let (
            account,
            order_id,
            client_order_id,
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
        self.emit_event_for_order(
            order_id,
            client_order_id,
            account,
            orig_size,
            remaining_size,
            remaining_size,
            option::some(price),
            is_bid,
            false,
            market_types::order_status_cancelled(),
            &std::string::utf8(b"Order cancelled")
        );
    }

    /// Cancels an order - this will cancel the order and emit an event for the order cancellation.
    public fun decrease_order_size<M: store + copy + drop>(
        self: &mut Market<M>,
        user: &signer,
        order_id: OrderIdType,
        size_delta: u64,
        callbacks: &MarketClearinghouseCallbacks<M>
    ) {
        let account = signer::address_of(user);
        self.order_book.decrease_order_size(account, order_id, size_delta);
        let maybe_order = self.order_book.get_order(order_id);
        assert!(maybe_order.is_some(), EORDER_DOES_NOT_EXIST);
        let (order, _) = maybe_order.destroy_some().destroy_order_from_state();
        assert!(order.get_account() == account, ENOT_ORDER_CREATOR);
        let (
            user,
            order_id,
            client_order_id,
            price,
            orig_size,
            remaining_size,
            is_bid,
            _trigger_condition,
            _metadata
        ) = order.destroy_order();
        callbacks.decrease_order_size(
            user, order_id, is_bid, price, remaining_size
        );

        self.emit_event_for_order(
            order_id,
            client_order_id,
            user,
            orig_size,
            remaining_size,
            size_delta,
            option::some(price),
            is_bid,
            false,
            market_types::order_status_size_reduced(),
            &std::string::utf8(b"Order size reduced")
        );
    }

    /// Remaining size of the order in the order book.
    public fun get_remaining_size<M: store + copy + drop>(
        self: &Market<M>, order_id: OrderIdType
    ): u64 {
        self.order_book.get_remaining_size(order_id)
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
        let Market::V1 {
            parent: _parent,
            market: _market,
            order_id_generator: _order_id_generator,
            next_fill_id: _next_fill_id,
            config,
            order_book
        } = self;
        let MarketConfig::V1 { allow_self_trade: _, allow_events_emission: _ } = config;
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
    public fun get_order_id_from_event(self: OrderEvent): OrderIdType {
        new_order_id_type(self.order_id)
    }

    #[test_only]
    public fun verify_order_event(
        self: OrderEvent,
        order_id: OrderIdType,
        client_order_id: Option<u64>,
        market: address,
        user: address,
        orig_size: u64,
        remaining_size: u64,
        size_delta: u64,
        price: Option<u64>,
        is_bid: bool,
        is_taker: bool,
        status: OrderStatus
    ) {
        assert!(self.order_id == order_id.get_order_id_value());
        assert!(self.client_order_id == client_order_id);
        assert!(self.market == market);
        assert!(self.user == user);
        assert!(self.orig_size == orig_size);
        assert!(self.remaining_size == remaining_size);
        assert!(self.size_delta == size_delta);
        assert!(self.price == price);
        assert!(self.is_bid == is_bid);
        assert!(self.is_taker == is_taker);
        assert!(self.status == status);
    }
}

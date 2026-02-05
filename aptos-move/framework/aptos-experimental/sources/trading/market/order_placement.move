/// This module provides a generic trading engine implementation for a market. On a high level, it's a data structure,
/// that stores an order book and provides APIs to place orders, cancel orders, and match orders. The market also acts
/// as a wrapper around the order book and pluggable clearinghouse implementation.
/// A clearing house implementation is expected to implement the following APIs
///  - settle_trade(taker, taker_order_id, maker, maker_order_id, fill_id, is_taker_long, price, size): SettleTradeResult ->
/// Called by the market when there is a match between taker and maker. The clearinghouse is expected to settle the trade
/// and return the result. Please note that the clearing house settlement size might not be the same as the order match size and
/// the settlement might also fail. The fill_id is an incremental counter for matched orders and can be used to track specific fills
///  - validate_order_placement(account, is_taker, is_long, price, size): bool -> Called by the market to validate
///  an order when it's placed. The clearinghouse is expected to validate the order and return true if the order is valid.
///  This API is called for both maker and taker order placements.
///  Check out clearinghouse_test as an example of the simplest form of clearing house implementation that just tracks
///  the position size of the user and does not do any validation.
///
/// - place_maker_order(account, order_id, is_bid, price, size, metadata) -> Called by the market before placing the
/// maker order in the order book. The clearinghouse can use this to track pending orders in the order book and perform
/// any other book keeping operations.
///
/// - cleanup_order(account, order_id, is_bid, remaining_size, order_metadata) -> Called by the market when an order is cancelled or fully filled
/// The clearinhouse can perform any cleanup operations like removing the order from the pending orders list. For every order placement
/// that passes the validate_order_placement check,
/// the market guarantees that the cleanup_order API will be called once and only once with the remaining size of the order.
/// the remaining size of the order being cleaned up - it can be 0, if the order was fully matched
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
/// on the order book until its trigger conditions are met. The following trigger conditions are supported:
/// TakeProfit(price): If it's a buy order it's triggered when the market price is greater than or equal to the price. If
/// it's a sell order it's triggered when the market price is less than or equal to the price.
/// StopLoss(price): If it's a buy order it's triggered when the market price is less than or equal to the price. If it's
/// a sell order it's triggered when the market price is greater than or equal to the price.
/// TimeBased(time): The order is triggered when the current time is greater than or equal to the time.
///
module aptos_experimental::order_placement {
    friend aptos_experimental::order_operations;

    use std::option;
    use std::option::Option;
    use std::signer;
    use std::string::String;
    use std::vector;
    use aptos_trading::order_book_types::{
        OrderId,
        TriggerCondition,
        TimeInForce,
        immediate_or_cancel,
        post_only,
        single_order_type,
        next_order_id,
        OrderType
    };
    use aptos_trading::order_match_types::OrderMatchDetails;
    use aptos_trading::single_order_types::new_single_order_request;
    use aptos_experimental::market_clearinghouse_order_info::new_clearinghouse_order_info;
    use aptos_experimental::pre_cancellation_tracker::{is_pre_cancelled};
    use aptos_experimental::market_types::{
        Self,
        MarketClearinghouseCallbacks,
        Market,
        CallbackResult,
        new_callback_result_not_available,
        OrderCancellationReason,
        order_cancellation_reason_clearinghouse_stopped_matching
    };
    use aptos_framework::transaction_context;
    use aptos_experimental::dead_mans_switch_tracker::is_order_valid;

    // Error codes
    const EINVALID_ORDER: u64 = 1;
    const ECLEARINGHOUSE_SETTLEMENT_VIOLATION: u64 = 2;
    const ECLIENT_ORDER_ID_LENGTH_EXCEEDED: u64 = 3;

    const U64_MAX: u64 = 0xffffffffffffffff;
    const MAX_CLIENT_ORDER_ID_LENGTH: u64 = 32;

    enum OrderMatchResult<R: store + copy + drop> has drop {
        V1 {
            order_id: OrderId,
            remaining_size: u64,
            cancel_reason: Option<OrderCancellationReason>,
            callback_results: vector<R>,
            fill_sizes: vector<u64>,
            match_count: u32 // includes fills and cancels
        }
    }

    public fun destroy_order_match_result<R: store + copy + drop>(
        self: OrderMatchResult<R>
    ): (OrderId, u64, Option<OrderCancellationReason>, vector<R>, vector<u64>, u32) {
        let OrderMatchResult::V1 {
            order_id,
            remaining_size,
            cancel_reason,
            callback_results,
            fill_sizes,
            match_count
        } = self;
        (
            order_id,
            remaining_size,
            cancel_reason,
            callback_results,
            fill_sizes,
            match_count
        )
    }

    public fun number_of_fills<R: store + copy + drop>(
        self: &OrderMatchResult<R>
    ): u64 {
        self.fill_sizes.length()
    }

    /// Includes fills and cancels
    public fun number_of_matches<R: store + copy + drop>(
        self: &OrderMatchResult<R>
    ): u32 {
        self.match_count
    }

    public fun total_fill_size<R: store + copy + drop>(
        self: &OrderMatchResult<R>
    ): u64 {
        self.fill_sizes.fold(0, |acc, fill_size| acc + fill_size)
    }

    public fun get_cancel_reason<R: store + copy + drop>(
        self: &OrderMatchResult<R>
    ): Option<OrderCancellationReason> {
        self.cancel_reason
    }

    public fun get_remaining_size_from_result<R: store + copy + drop>(
        self: &OrderMatchResult<R>
    ): u64 {
        self.remaining_size
    }

    public fun is_ioc_violation(reason: OrderCancellationReason): bool {
        reason == market_types::order_cancellation_reason_ioc_violation()
    }

    public fun is_fill_limit_violation(
        cancel_reason: OrderCancellationReason
    ): bool {
        cancel_reason
            == market_types::order_cancellation_reason_max_fill_limit_violation()
    }

    public fun is_dead_mans_switch_expired(
        cancel_reason: OrderCancellationReason
    ): bool {
        cancel_reason
            == market_types::order_cancellation_reason_dead_mans_switch_expired()
    }

    public fun is_clearinghouse_stopped_matching(
        cancel_reason: OrderCancellationReason
    ): bool {
        cancel_reason
            == market_types::order_cancellation_reason_clearinghouse_stopped_matching()
    }

    public fun get_order_id<R: store + copy + drop>(self: OrderMatchResult<R>): OrderId {
        self.order_id
    }

    /// Places a limit order - If it's a taker order, it will be matched immediately and if it's a maker order, it will simply
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
    /// - max_match_limit: The maximum match limit for the order. This is the maximum number of matches (fills or cancels) to trigger for this order.
    /// This knob is present to configure maximum amount of gas any order placement transaction might consume and avoid
    /// hitting the maximum has limit of the blockchain.
    /// - cancel_on_match_limit: bool: Whether to cancel the given order when the match limit is reached.
    /// This is useful as the caller might not want to cancel the order when the limit is reached and can continue
    /// that order in a separate transaction.
    /// - callbacks: The callbacks for the market clearinghouse. This is a struct that implements the MarketClearinghouseCallbacks
    /// interface. This is used to validate the order and settle the trade.
    /// Returns the order id, remaining size, cancel reason and number of fills for the order.
    public fun place_limit_order<M: store + copy + drop, R: store + copy + drop>(
        market: &mut Market<M>,
        user: &signer,
        limit_price: u64,
        orig_size: u64,
        is_bid: bool,
        time_in_force: TimeInForce,
        trigger_condition: Option<TriggerCondition>,
        metadata: M,
        client_order_id: Option<String>,
        max_match_limit: u32,
        cancel_on_match_limit: bool,
        callbacks: &MarketClearinghouseCallbacks<M, R>
    ): OrderMatchResult<R> {
        place_order_with_order_id(
            market,
            signer::address_of(user),
            limit_price,
            orig_size,
            orig_size,
            is_bid,
            time_in_force,
            trigger_condition,
            metadata,
            option::none(), // order_id
            client_order_id,
            max_match_limit,
            cancel_on_match_limit,
            true,
            callbacks
        )
    }

    /// Places a market order - The order is guaranteed to be a taker order and will be matched immediately.
    public fun place_market_order<M: store + copy + drop, R: store + copy + drop>(
        market: &mut Market<M>,
        user: &signer,
        orig_size: u64,
        is_bid: bool,
        metadata: M,
        client_order_id: Option<String>,
        max_match_limit: u32,
        cancel_on_match_limit: bool,
        callbacks: &MarketClearinghouseCallbacks<M, R>
    ): OrderMatchResult<R> {
        place_order_with_order_id(
            market,
            signer::address_of(user),
            if (is_bid) {
                U64_MAX
            } else { 1 },
            orig_size,
            orig_size,
            is_bid,
            immediate_or_cancel(), // market orders are always IOC
            option::none(), // trigger_condition
            metadata,
            option::none(), // order_id
            client_order_id,
            max_match_limit,
            cancel_on_match_limit,
            true,
            callbacks
        )
    }

    fun place_maker_order_internal<M: store + copy + drop, R: store + copy + drop>(
        market: &mut Market<M>,
        user_addr: address,
        limit_price: u64,
        orig_size: u64,
        remaining_size: u64,
        fill_sizes: vector<u64>,
        match_count: u32,
        is_bid: bool,
        time_in_force: TimeInForce,
        trigger_condition: Option<TriggerCondition>,
        metadata: M,
        order_id: OrderId,
        client_order_id: Option<String>,
        emit_open_for_cancellation: bool,
        callbacks: &MarketClearinghouseCallbacks<M, R>,
        callback_results: vector<R>
    ): OrderMatchResult<R> {
        if (time_in_force == immediate_or_cancel() && trigger_condition.is_none()) {
            return cancel_taker_order_internal(
                market,
                user_addr,
                limit_price,
                order_id,
                client_order_id,
                orig_size,
                remaining_size,
                fill_sizes,
                match_count,
                is_bid,
                market_types::order_cancellation_reason_ioc_violation(),
                std::string::utf8(b"IOC Violation"),
                trigger_condition,
                metadata,
                time_in_force,
                emit_open_for_cancellation,
                callbacks,
                callback_results
            );
        };

        if (trigger_condition.is_some()) {
            // Do not emit an open event for orders with trigger conditions as they are not live in the order book yet
            market.get_order_book_mut().place_maker_order(
                new_single_order_request(
                    user_addr,
                    order_id,
                    client_order_id,
                    limit_price,
                    orig_size,
                    remaining_size,
                    is_bid,
                    trigger_condition,
                    time_in_force,
                    metadata
                )
            );
            return OrderMatchResult::V1 {
                order_id,
                remaining_size,
                cancel_reason: option::none(),
                callback_results,
                fill_sizes,
                match_count
            }
        };

        let result =
            callbacks.place_maker_order(
                new_clearinghouse_order_info(
                    user_addr,
                    order_id,
                    client_order_id,
                    is_bid,
                    limit_price,
                    time_in_force,
                    single_order_type(),
                    option::none(),
                    metadata
                ),
                remaining_size
            );
        if (result.get_place_maker_order_cancellation_reason().is_some()) {
            return cancel_taker_order_internal(
                market,
                user_addr,
                limit_price,
                order_id,
                client_order_id,
                orig_size,
                remaining_size,
                fill_sizes,
                match_count,
                is_bid,
                market_types::order_cancellation_reason_place_maker_order_violation(),
                result.get_place_maker_order_cancellation_reason().destroy_some(),
                option::none(), // trigger_condition
                metadata,
                time_in_force,
                emit_open_for_cancellation,
                callbacks,
                callback_results
            );
        };

        // Emit order open event for the maker order
        market.emit_event_for_order(
            order_id,
            client_order_id,
            user_addr,
            orig_size,
            remaining_size,
            remaining_size,
            limit_price,
            is_bid,
            false,
            market_types::order_status_open(),
            std::string::utf8(b""),
            metadata,
            trigger_condition,
            time_in_force,
            option::none(),
            callbacks
        );

        let actions = result.get_place_maker_order_actions();
        if (actions.is_some()) {
            callback_results.push_back(actions.destroy_some());
        };
        market.get_order_book_mut().place_maker_order(
            new_single_order_request(
                user_addr,
                order_id,
                client_order_id,
                limit_price,
                orig_size,
                remaining_size,
                is_bid,
                trigger_condition,
                time_in_force,
                metadata
            )
        );
        return OrderMatchResult::V1 {
            order_id,
            remaining_size,
            cancel_reason: option::none(),
            callback_results,
            fill_sizes,
            match_count
        }
    }

    fun cancel_bulk_maker_order_internal<M: store + copy + drop, R: store + copy + drop>(
        market: &mut Market<M>,
        maker_order: &OrderMatchDetails<M>,
        maker_address: address,
        order_id: OrderId,
        unsettled_size: u64,
        cancellation_reason: OrderCancellationReason,
        callbacks: &MarketClearinghouseCallbacks<M, R>
    ) {
        let remaining_size = maker_order.get_remaining_size_from_match_details();
        let price = maker_order.get_price_from_match_details();
        let is_bid = maker_order.is_bid_from_match_details();
        let cancelled_size = unsettled_size + remaining_size;

        // Cancel only at the specific price level instead of cancelling the entire bulk order
        let (_actual_cancelled_size, modified_order) =
            if (remaining_size != 0) {
                market.get_order_book_mut().cancel_bulk_order_at_price(
                    maker_address, price, is_bid
                )
            } else {
                // If remaining size is 0, just get the current order state for event emission
                (0, market.get_order_book().get_bulk_order(maker_address))
            };

        callbacks.cleanup_bulk_order_at_price(
            maker_address,
            order_id,
            is_bid,
            price,
            cancelled_size
        );

        // Emit event with the cancelled price level
        let (
            modified_order_request, _order_id, _unique_priority_idx, _creation_time_micros
        ) = modified_order.destroy_bulk_order();
        let (
            _account,
            order_sequence_number,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            _metadata
        ) = modified_order_request.destroy_bulk_order_request();

        // Build cancelled price/size vectors for the specific level that was cancelled
        let (
            cancelled_bid_prices,
            cancelled_bid_sizes,
            cancelled_ask_prices,
            cancelled_ask_sizes
        ) =
            if (is_bid) {
                (vector[price], vector[cancelled_size], vector[], vector[])
            } else {
                (vector[], vector[], vector[price], vector[cancelled_size])
            };

        market.emit_event_for_bulk_order_modified(
            order_id,
            order_sequence_number,
            maker_address,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            cancelled_bid_prices,
            cancelled_bid_sizes,
            cancelled_ask_prices,
            cancelled_ask_sizes,
            option::some(cancellation_reason)
        );
    }

    fun cancel_maker_order_internal<M: store + copy + drop, R: store + copy + drop>(
        market: &mut Market<M>,
        maker_order: &OrderMatchDetails<M>,
        client_order_id: Option<String>,
        maker_address: address,
        order_id: OrderId,
        cancellation_reason: OrderCancellationReason,
        maker_cancellation_reason: String,
        unsettled_size: u64,
        metadata: M,
        time_in_force: TimeInForce,
        callbacks: &MarketClearinghouseCallbacks<M, R>
    ) {
        if (maker_order.is_bulk_order_from_match_details()) {
            return cancel_bulk_maker_order_internal(
                market,
                maker_order,
                maker_address,
                order_id,
                unsettled_size,
                cancellation_reason,
                callbacks
            );
        };
        let maker_cancel_size =
            unsettled_size + maker_order.get_remaining_size_from_match_details();
        market.emit_event_for_order(
            order_id,
            client_order_id,
            maker_address,
            maker_order.get_orig_size_from_match_details(),
            0,
            maker_cancel_size,
            maker_order.get_price_from_match_details(),
            maker_order.is_bid_from_match_details(),
            false,
            market_types::order_status_cancelled(),
            maker_cancellation_reason,
            metadata,
            option::none(), // trigger_condition
            time_in_force,
            option::some(cancellation_reason),
            callbacks
        );
        // If the maker is invalid cancel the maker order and continue to the next maker order
        if (maker_order.get_remaining_size_from_match_details() != 0) {
            market.get_order_book_mut().cancel_single_order(maker_address, order_id);
        };
        cleanup_order_internal(
            maker_address,
            order_id,
            client_order_id,
            maker_order.get_book_type_from_match_details(),
            maker_order.is_bid_from_match_details(),
            time_in_force,
            maker_cancel_size,
            maker_order.get_price_from_match_details(),
            option::none(), // trigger_condition
            metadata,
            callbacks,
            false // is_taker is false as this is a maker order
        );
    }

    #[lint::skip(needless_mutable_reference)]
    fun cancel_taker_order_internal<M: store + copy + drop, R: store + copy + drop>(
        market: &mut Market<M>,
        user_addr: address,
        limit_price: u64,
        order_id: OrderId,
        client_order_id: Option<String>,
        orig_size: u64,
        size_delta: u64,
        fill_sizes: vector<u64>,
        match_count: u32,
        is_bid: bool,
        cancel_reason: OrderCancellationReason,
        cancel_details: String,
        trigger_condition: Option<TriggerCondition>,
        metadata: M,
        time_in_force: TimeInForce,
        emit_order_open: bool,
        callbacks: &MarketClearinghouseCallbacks<M, R>,
        callback_results: vector<R>
    ): OrderMatchResult<R> {
        if (emit_order_open) {
            market.emit_event_for_order(
                order_id,
                client_order_id,
                user_addr,
                orig_size,
                size_delta,
                orig_size,
                limit_price,
                is_bid,
                true, // is_taker - always true for taker orders
                market_types::order_status_open(),
                std::string::utf8(b""),
                metadata,
                option::none(), // trigger_condition
                time_in_force,
                option::none(),
                callbacks
            );
        };
        market.emit_event_for_order(
            order_id,
            client_order_id,
            user_addr,
            orig_size,
            0,
            size_delta,
            limit_price,
            is_bid,
            true, // is_taker - always true for taker orders
            market_types::order_status_cancelled(),
            cancel_details,
            metadata,
            option::none(), // trigger_condition
            time_in_force,
            option::some(cancel_reason),
            callbacks
        );
        callbacks.cleanup_order(
            new_clearinghouse_order_info(
                user_addr,
                order_id,
                client_order_id,
                is_bid,
                limit_price,
                time_in_force,
                single_order_type(),
                trigger_condition,
                metadata
            ),
            size_delta,
            true // is_taker - always true for taker orders
        );
        OrderMatchResult::V1 {
            order_id,
            remaining_size: 0,
            cancel_reason: option::some(cancel_reason),
            fill_sizes,
            callback_results,
            match_count
        }
    }

    public(friend) fun cleanup_order_internal<M: store + copy + drop, R: store + copy + drop>(
        user_addr: address,
        order_id: OrderId,
        client_order_id: Option<String>,
        order_type: OrderType,
        is_bid: bool,
        time_in_force: TimeInForce,
        cleanup_size: u64,
        price: u64,
        trigger_condition: Option<TriggerCondition>,
        metadata: M,
        callbacks: &MarketClearinghouseCallbacks<M, R>,
        is_taker: bool
    ) {
        if (order_type == single_order_type()) {
            callbacks.cleanup_order(
                new_clearinghouse_order_info(
                    user_addr,
                    order_id,
                    client_order_id,
                    is_bid,
                    price,
                    time_in_force,
                    single_order_type(),
                    trigger_condition,
                    metadata
                ),
                cleanup_size,
                is_taker
            );
        } else {
            callbacks.cleanup_bulk_order_at_price(
                user_addr,
                order_id,
                is_bid,
                price,
                cleanup_size
            );
        }
    }

    fun settle_single_trade<M: store + copy + drop, R: store + copy + drop>(
        market: &mut Market<M>,
        user_addr: address,
        price: u64,
        orig_size: u64,
        remaining_size: &mut u64,
        is_bid: bool,
        metadata: M,
        order_id: OrderId,
        client_order_id: Option<String>,
        callbacks: &MarketClearinghouseCallbacks<M, R>,
        time_in_force: TimeInForce,
        fill_sizes: &mut vector<u64>
    ): (Option<OrderCancellationReason>, CallbackResult<R>) {
        let dead_mans_switch_enabled = market.is_dead_mans_switch_enabled();
        if (dead_mans_switch_enabled
            && !is_order_valid(
                market.get_dead_mans_switch_tracker(), user_addr, option::none()
            )) {
            let taker_cancellation_reason =
                std::string::utf8(b"Order invalidated due to dead man's switch expiry");
            cancel_taker_order_internal(
                market,
                user_addr,
                price,
                order_id,
                client_order_id,
                orig_size,
                *remaining_size,
                *fill_sizes,
                0, // match_count - doesn't matter as we don't use the result.
                is_bid,
                market_types::order_cancellation_reason_dead_mans_switch_expired(),
                taker_cancellation_reason,
                option::none(), // trigger_condition
                metadata,
                time_in_force,
                false, // emit_order_open is false as the order was already open
                callbacks,
                vector[]
            );
            return (
                option::some(
                    market_types::order_cancellation_reason_dead_mans_switch_expired()
                ),
                new_callback_result_not_available()
            );
        };
        let result =
            market.get_order_book_mut().get_single_match_for_taker(
                price, *remaining_size, is_bid
            );
        let (maker_order, maker_matched_size) = result.destroy_order_match();
        if (!market.is_allowed_self_trade()
            && maker_order.get_account_from_match_details() == user_addr) {
            cancel_maker_order_internal(
                market,
                &maker_order,
                maker_order.get_client_order_id_from_match_details(),
                maker_order.get_account_from_match_details(),
                maker_order.get_order_id_from_match_details(),
                market_types::order_cancellation_reason_disallowed_self_trading(),
                std::string::utf8(b"Disallowed self trading"),
                maker_matched_size,
                maker_order.get_metadata_from_match_details(),
                maker_order.get_time_in_force_from_match_details(),
                callbacks
            );
            return (option::none(), new_callback_result_not_available());
        };
        if (dead_mans_switch_enabled
            && !is_order_valid(
                market.get_dead_mans_switch_tracker(),
                maker_order.get_account_from_match_details(),
                option::some(
                    maker_order.get_creation_time_micros_from_match_details() / 1000000
                )
            )) {
            cancel_maker_order_internal(
                market,
                &maker_order,
                maker_order.get_client_order_id_from_match_details(),
                maker_order.get_account_from_match_details(),
                maker_order.get_order_id_from_match_details(),
                market_types::order_cancellation_reason_dead_mans_switch_expired(),
                std::string::utf8(b"Order invalidated due to dead man's switch expiry"),
                maker_matched_size,
                maker_order.get_metadata_from_match_details(),
                maker_order.get_time_in_force_from_match_details(),
                callbacks
            );
            return (option::none(), new_callback_result_not_available());
        };
        let fill_id = transaction_context::monotonically_increasing_counter();
        let settle_result =
            callbacks.settle_trade(
                market,
                new_clearinghouse_order_info(
                    user_addr,
                    order_id,
                    client_order_id,
                    is_bid,
                    price,
                    time_in_force,
                    single_order_type(),
                    option::none(), // trigger_condition
                    metadata
                ),
                new_clearinghouse_order_info(
                    maker_order.get_account_from_match_details(),
                    maker_order.get_order_id_from_match_details(),
                    maker_order.get_client_order_id_from_match_details(),
                    maker_order.is_bid_from_match_details(),
                    maker_order.get_price_from_match_details(),
                    maker_order.get_time_in_force_from_match_details(),
                    maker_order.get_book_type_from_match_details(),
                    option::none(), // trigger_condition
                    maker_order.get_metadata_from_match_details()
                ),
                fill_id,
                maker_order.get_price_from_match_details(), // Order is always matched at the price of the maker
                maker_matched_size
            );

        let unsettled_maker_size = maker_matched_size;
        let settled_size = settle_result.get_settled_size();
        if (settled_size > 0) {
            *remaining_size -= settled_size;
            unsettled_maker_size -= settled_size;
            fill_sizes.push_back(settled_size);
            // Event for taker fill
            market.emit_event_for_order(
                order_id,
                client_order_id,
                user_addr,
                orig_size,
                *remaining_size,
                settled_size,
                maker_order.get_price_from_match_details(),
                is_bid,
                true,
                market_types::order_status_filled(),
                std::string::utf8(b""),
                metadata,
                option::none(), // trigger_condition
                time_in_force,
                option::none(),
                callbacks
            );
            // Event for maker fill
            if (maker_order.is_bulk_order_from_match_details()) {
                market.emit_event_for_bulk_order_filled(
                    maker_order.get_order_id_from_match_details(),
                    maker_order.get_sequence_number_from_match_details(),
                    maker_order.get_account_from_match_details(),
                    settled_size,
                    maker_order.get_price_from_match_details(),
                    maker_order.get_price_from_match_details(),
                    !is_bid,
                    fill_id
                );
            } else {
                market.emit_event_for_order(
                    maker_order.get_order_id_from_match_details(),
                    maker_order.get_client_order_id_from_match_details(),
                    maker_order.get_account_from_match_details(),
                    maker_order.get_orig_size_from_match_details(),
                    maker_order.get_remaining_size_from_match_details()
                        + unsettled_maker_size,
                    settled_size,
                    maker_order.get_price_from_match_details(),
                    !is_bid,
                    false,
                    market_types::order_status_filled(),
                    std::string::utf8(b""),
                    maker_order.get_metadata_from_match_details(),
                    option::none(),
                    maker_order.get_time_in_force_from_match_details(),
                    option::none(),
                    callbacks
                );
            };
        };

        let maker_cancellation_reason_str = settle_result.get_maker_cancellation_reason();
        let taker_cancellation_reason_str = settle_result.get_taker_cancellation_reason();
        if (settled_size < maker_matched_size) {
            // If the order is partially settled, the expectation is that the clearinghouse
            // provides cancellation reason for at least one of the orders.
            assert!(
                maker_cancellation_reason_str.is_some()
                    || taker_cancellation_reason_str.is_some(),
                ECLEARINGHOUSE_SETTLEMENT_VIOLATION
            );
        };
        let taker_cancellation_reason =
            if (taker_cancellation_reason_str.is_some()) {
                cancel_taker_order_internal(
                    market,
                    user_addr,
                    price,
                    order_id,
                    client_order_id,
                    orig_size,
                    *remaining_size,
                    *fill_sizes,
                    0, // match_count - doesn't matter as we don't use the result.
                    is_bid,
                    market_types::order_cancellation_reason_clearinghouse_settle_violation(),
                    taker_cancellation_reason_str.destroy_some(),
                    option::none(), // trigger_condition
                    metadata,
                    time_in_force,
                    false, // emit_order_open is false as the order was already open
                    callbacks,
                    vector[]
                );
                option::some(
                    market_types::order_cancellation_reason_clearinghouse_settle_violation()
                )
            } else {
                option::none()
            };
        if (maker_cancellation_reason_str.is_some()) {
            cancel_maker_order_internal(
                market,
                &maker_order,
                maker_order.get_client_order_id_from_match_details(),
                maker_order.get_account_from_match_details(),
                maker_order.get_order_id_from_match_details(),
                market_types::order_cancellation_reason_clearinghouse_settle_violation(),
                maker_cancellation_reason_str.destroy_some(),
                unsettled_maker_size,
                maker_order.get_metadata_from_match_details(),
                maker_order.get_time_in_force_from_match_details(),
                callbacks
            );
        } else {
            if (unsettled_maker_size > 0) {
                //  we need to re-insert the maker order back into the order book
                let reinsertion_request =
                    maker_order.new_order_match_details_with_modified_size(
                        unsettled_maker_size
                    );
                market.get_order_book_mut().reinsert_order(
                    reinsertion_request, &maker_order
                );
            } else if (maker_order.get_remaining_size_from_match_details() == 0) {
                cleanup_order_internal(
                    maker_order.get_account_from_match_details(),
                    maker_order.get_order_id_from_match_details(),
                    maker_order.get_client_order_id_from_match_details(),
                    maker_order.get_book_type_from_match_details(),
                    !is_bid, // is_bid is inverted for maker orders
                    maker_order.get_time_in_force_from_match_details(),
                    0, // 0 because the order is fully filled
                    maker_order.get_price_from_match_details(),
                    option::none(), // trigger_condition
                    maker_order.get_metadata_from_match_details(),
                    callbacks,
                    false // is_taker is false for maker orders
                );
            }
        };
        (taker_cancellation_reason, *settle_result.get_callback_result())
    }

    /// Core function to place an order with a given order id. If the order id is not provided, a new order id is generated.
    /// The function itself doesn't do any validation of the user_address, it's up to the caller to ensure that signer validation
    /// is done before calling this function if needed.
    public fun place_order_with_order_id<M: store + copy + drop, R: store + copy + drop>(
        market: &mut Market<M>,
        user_addr: address,
        limit_price: u64,
        orig_size: u64,
        remaining_size: u64,
        is_bid: bool,
        time_in_force: TimeInForce,
        trigger_condition: Option<TriggerCondition>,
        metadata: M,
        order_id: Option<OrderId>,
        client_order_id: Option<String>,
        max_match_limit: u32,
        cancel_on_match_limit: bool,
        emit_taker_order_open: bool,
        callbacks: &MarketClearinghouseCallbacks<M, R>
    ): OrderMatchResult<R> {
        assert!(
            orig_size > 0
                && remaining_size > 0
                && orig_size >= remaining_size,
            EINVALID_ORDER
        );
        assert!(max_match_limit > 0, EINVALID_ORDER);
        assert!(limit_price > 0, EINVALID_ORDER);
        if (client_order_id.is_some()) {
            assert!(
                client_order_id.borrow().length() <= MAX_CLIENT_ORDER_ID_LENGTH,
                ECLIENT_ORDER_ID_LENGTH_EXCEEDED
            );
        };
        if (order_id.is_none()) {
            // If order id is not provided, generate a new order id
            order_id = option::some(next_order_id());
        };
        let order_id = order_id.destroy_some();
        let callback_results = vector::empty();
        let validation_result =
            callbacks.validate_order_placement(
                new_clearinghouse_order_info(
                    user_addr,
                    order_id,
                    client_order_id,
                    is_bid,
                    limit_price,
                    time_in_force,
                    single_order_type(),
                    trigger_condition,
                    metadata
                ),
                remaining_size
            );
        if (!validation_result.is_validation_result_valid()) {
            return cancel_taker_order_internal(
                market,
                user_addr,
                limit_price,
                order_id,
                client_order_id,
                orig_size,
                remaining_size,
                vector[],
                0, // match_count
                is_bid,
                market_types::order_cancellation_reason_position_update_violation(),
                validation_result.get_validation_failure_reason().destroy_some(),
                trigger_condition,
                metadata,
                time_in_force,
                true, // emit_order_open
                callbacks,
                vector[]
            );
        };

        if (client_order_id.is_some()) {
            if (market.get_order_book().client_order_id_exists(
                user_addr, client_order_id.destroy_some()
            )) {
                // Client provided a client order id that already exists in the order book
                return cancel_taker_order_internal(
                    market,
                    user_addr,
                    limit_price,
                    order_id,
                    client_order_id,
                    orig_size,
                    remaining_size,
                    vector[],
                    0, // match_count
                    is_bid,
                    market_types::order_cancellation_reason_duplicate_client_order_id(),
                    std::string::utf8(b"Duplicate client order id"),
                    trigger_condition,
                    metadata,
                    time_in_force,
                    true, // emit_order_open
                    callbacks,
                    vector[]
                );
            };

            if (is_pre_cancelled(
                market.get_pre_cancellation_tracker_mut(),
                user_addr,
                client_order_id.destroy_some()
            )) {
                return cancel_taker_order_internal(
                    market,
                    user_addr,
                    limit_price,
                    order_id,
                    client_order_id,
                    orig_size,
                    remaining_size,
                    vector[],
                    0, // match_count
                    is_bid,
                    market_types::order_cancellation_reason_order_pre_cancelled(),
                    std::string::utf8(b"Order pre cancelled"),
                    trigger_condition,
                    metadata,
                    time_in_force,
                    true, // emit_order_open
                    callbacks,
                    vector[]
                );
            };
        };
        let is_taker_order =
            market.get_order_book().is_taker_order(
                limit_price, is_bid, trigger_condition
            );

        if (!is_taker_order) {
            return place_maker_order_internal(
                market,
                user_addr,
                limit_price,
                orig_size,
                remaining_size,
                vector[],
                0, // match_count
                is_bid,
                time_in_force,
                trigger_condition,
                metadata,
                order_id,
                client_order_id,
                emit_taker_order_open, // order_open_emitted
                callbacks,
                vector[]
            );
        };

        // NOTE: We should always use is_taker: true for this order past this
        // point so that indexer can consistently track the order's status
        if (time_in_force == post_only()) {
            return cancel_taker_order_internal(
                market,
                user_addr,
                limit_price,
                order_id,
                client_order_id,
                orig_size,
                remaining_size,
                vector[],
                0, // match_count
                is_bid,
                market_types::order_cancellation_reason_post_only_violation(),
                std::string::utf8(b"Post Only violation"),
                option::none(), // trigger_condition
                metadata,
                time_in_force,
                true, // emit_order_open
                callbacks,
                vector[]
            );
        };

        if (emit_taker_order_open) {
            // We don't emit order open events for orders with trigger conditions as they are not
            // actually placed in the order book until they are triggered.
            market.emit_event_for_order(
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
                std::string::utf8(b""),
                metadata,
                trigger_condition,
                time_in_force,
                option::none(),
                callbacks
            );
        };

        let fill_sizes = vector::empty();
        let match_count = 0;
        loop {
            match_count += 1;
            let (taker_cancellation_reason, callback_result) =
                settle_single_trade(
                    market,
                    user_addr,
                    limit_price,
                    orig_size,
                    &mut remaining_size,
                    is_bid,
                    metadata,
                    order_id,
                    client_order_id,
                    callbacks,
                    time_in_force,
                    &mut fill_sizes
                );
            let should_stop = callback_result.should_stop_matching();
            let result = callback_result.extract_results();
            if (result.is_some()) {
                callback_results.push_back(result.destroy_some());
            };
            if (taker_cancellation_reason.is_some()) {
                return OrderMatchResult::V1 {
                    order_id,
                    remaining_size: 0, // 0 because the order is cancelled
                    cancel_reason: taker_cancellation_reason,
                    fill_sizes,
                    callback_results,
                    match_count
                }
            };
            if (remaining_size == 0) {
                cleanup_order_internal(
                    user_addr,
                    order_id,
                    client_order_id,
                    single_order_type(),
                    is_bid,
                    time_in_force,
                    0,
                    limit_price,
                    trigger_condition,
                    metadata,
                    callbacks,
                    true
                );
                break;
            };
            if (should_stop) {
                return OrderMatchResult::V1 {
                    order_id,
                    remaining_size,
                    cancel_reason: option::some(
                        order_cancellation_reason_clearinghouse_stopped_matching()
                    ),
                    fill_sizes,
                    callback_results,
                    match_count
                }
            };
            // Check if the next iteration will still match
            let is_taker_order = market.is_taker_order(
                limit_price, is_bid, option::none()
            );
            if (!is_taker_order) {
                if (time_in_force == immediate_or_cancel()) {
                    return cancel_taker_order_internal(
                        market,
                        user_addr,
                        limit_price,
                        order_id,
                        client_order_id,
                        orig_size,
                        remaining_size,
                        fill_sizes,
                        match_count,
                        is_bid,
                        market_types::order_cancellation_reason_ioc_violation(),
                        std::string::utf8(b"IOC_VIOLATION"),
                        option::none(), // trigger_condition
                        metadata,
                        time_in_force,
                        false, // emit_order_open is false as the order was already open
                        callbacks,
                        callback_results
                    );
                } else {
                    // If the order is not a taker order, then we can place it as a maker order
                    return place_maker_order_internal(
                        market,
                        user_addr,
                        limit_price,
                        orig_size,
                        remaining_size,
                        fill_sizes,
                        match_count,
                        is_bid,
                        time_in_force,
                        option::none(), // trigger_condition
                        metadata,
                        order_id,
                        client_order_id,
                        false,
                        callbacks,
                        callback_results
                    );
                };
            };

            if (match_count >= max_match_limit) {
                if (cancel_on_match_limit) {
                    return cancel_taker_order_internal(
                        market,
                        user_addr,
                        limit_price,
                        order_id,
                        client_order_id,
                        orig_size,
                        remaining_size,
                        fill_sizes,
                        match_count,
                        is_bid,
                        market_types::order_cancellation_reason_max_fill_limit_violation(),
                        std::string::utf8(b"Max fill limit reached"),
                        option::none(), // trigger_condition
                        metadata,
                        time_in_force,
                        false, // emit_order_open is false as the order was already open
                        callbacks,
                        callback_results
                    );
                } else {
                    return OrderMatchResult::V1 {
                        order_id,
                        remaining_size,
                        cancel_reason: option::some(
                            market_types::order_cancellation_reason_max_fill_limit_violation()
                        ),
                        callback_results,
                        fill_sizes,
                        match_count
                    }
                };
            };
        };
        OrderMatchResult::V1 {
            order_id,
            remaining_size,
            cancel_reason: option::none(),
            fill_sizes,
            callback_results,
            match_count
        }
    }

    // ============================= test_only APIs ====================================
    #[test_only]
    public fun is_clearinghouse_settle_violation(
        cancellation_reason: OrderCancellationReason
    ): bool {
        if (cancellation_reason
            == market_types::order_cancellation_reason_clearinghouse_settle_violation()) {
            return true;
        };
        false
    }
}

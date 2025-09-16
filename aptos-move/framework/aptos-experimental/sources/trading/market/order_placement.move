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
/// - cleanup_order(account, order_id, is_bid, remaining_size, order_metadata) -> Called by the market when an order is cancelled or fully filled
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
module aptos_experimental::order_placement {
    use std::option;
    use std::option::Option;
    use std::signer;
    use std::string::String;
    use std::vector;
    use aptos_experimental::order_book::{new_single_order_request};
    use aptos_experimental::pre_cancellation_tracker::{
        is_pre_cancelled
    };
    use aptos_experimental::order_book_types::{
        OrderIdType, OrderMatchDetails, single_order_book_type, OrderBookType
    };
    use aptos_experimental::order_book_types::TriggerCondition;
    use aptos_experimental::order_book_types::{TimeInForce, immediate_or_cancel, post_only};
    use aptos_experimental::market_types::{
        Self,
        MarketClearinghouseCallbacks,
        Market,
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

    const PRE_CANCELLATION_TRACKER_KEY: u8 = 0;
    const U64_MAX: u64 = 0xffffffffffffffff;

    enum OrderCancellationReason has drop, copy {
        PostOnlyViolation,
        IOCViolation,
        PositionUpdateViolation,
        ReduceOnlyViolation,
        ClearinghouseSettleViolation,
        MaxFillLimitViolation,
        DuplicateClientOrderIdViolation,
        OrderPreCancelled,
    }

    struct OrderMatchResult has drop {
        order_id: OrderIdType,
        remaining_size: u64,
        cancel_reason: Option<OrderCancellationReason>,
        fill_sizes: vector<u64>,
        match_count: u32, // includes fills and cancels
    }

    public fun destroy_order_match_result(
        self: OrderMatchResult
    ): (OrderIdType, u64, Option<OrderCancellationReason>, vector<u64>, u32) {
        let OrderMatchResult { order_id, remaining_size, cancel_reason, fill_sizes, match_count } =
            self;
        (order_id, remaining_size, cancel_reason, fill_sizes, match_count)
    }

    public fun number_of_fills(self: &OrderMatchResult): u64 {
        self.fill_sizes.length()
    }

    /// Includes fills and cancels
    public fun number_of_matches(self: &OrderMatchResult): u32 {
        self.match_count
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
    /// - max_match_limit: The maximum match limit for the order. This is the maximum number of matches (fills or cancels) to trigger for this order.
    /// This knob is present to configure maximum amount of gas any order placement transaction might consume and avoid
    /// hitting the maximum has limit of the blockchain.
    /// - cancel_on_match_limit: bool: Whether to cancel the given order when the match limit is reached.
    /// This is used ful as the caller might not want to cancel the order when the limit is reached and can continue
    /// that order in a separate transaction.
    /// - callbacks: The callbacks for the market clearinghouse. This is a struct that implements the MarketClearinghouseCallbacks
    /// interface. This is used to validate the order and settle the trade.
    /// Returns the order id, remaining size, cancel reason and number of fills for the order.
    public fun place_limit_order<M: store + copy + drop>(
        market: &mut Market<M>,
        user: &signer,
        limit_price: u64,
        orig_size: u64,
        is_bid: bool,
        time_in_force: TimeInForce,
        trigger_condition: Option<TriggerCondition>,
        metadata: M,
        client_order_id: Option<u64>,
        max_match_limit: u32,
        cancel_on_match_limit: bool,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): OrderMatchResult {
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
    public fun place_market_order<M: store + copy + drop>(
        market: &mut Market<M>,
        user: &signer,
        orig_size: u64,
        is_bid: bool,
        metadata: M,
        client_order_id: Option<u64>,
        max_match_limit: u32,
        cancel_on_match_limit: bool,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): OrderMatchResult {
        place_order_with_order_id(
            market,
            signer::address_of(user),
            if (is_bid) { U64_MAX } else { 1 },
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

    fun place_maker_order_internal<M: store + copy + drop>(
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
        order_id: OrderIdType,
        client_order_id: Option<u64>,
        emit_order_open: bool,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): OrderMatchResult {
        if (time_in_force == immediate_or_cancel() && trigger_condition.is_none()) {
            return cancel_single_order_internal(
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
                false, // is_taker
                OrderCancellationReason::IOCViolation,
                std::string::utf8(b"IOC Violation"),
                metadata,
                time_in_force,
                callbacks
            );
        };

        if (emit_order_open) {
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
                option::some(metadata),
                trigger_condition,
                time_in_force,
                callbacks
            );
        };

        callbacks.place_maker_order(
            user_addr,
            order_id,
            is_bid,
            limit_price,
            remaining_size,
            metadata
        );
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
        return OrderMatchResult {
            order_id,
            remaining_size,
            cancel_reason: option::none(),
            fill_sizes,
            match_count
        }
    }

    fun cancel_maker_order_internal<M: store + copy + drop>(
        market: &mut Market<M>,
        maker_order: &OrderMatchDetails<M>,
        client_order_id: Option<u64>,
        maker_address: address,
        order_id: OrderIdType,
        maker_cancellation_reason: String,
        unsettled_size: u64,
        metadata: Option<M>,
        time_in_force: TimeInForce,
        callbacks: &MarketClearinghouseCallbacks<M>
    ) {
        let maker_cancel_size = unsettled_size + maker_order.get_remaining_size_from_match_details();
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
            callbacks
        );
        // If the maker is invalid cancel the maker order and continue to the next maker order
        if (maker_order.get_remaining_size_from_match_details() != 0) {
            let order_book_type = maker_order.get_book_type_from_match_details();
            if (order_book_type == single_order_book_type()) {
                market.get_order_book_mut().cancel_order(maker_address, order_id);
            } else {
                market.get_order_book_mut().cancel_bulk_order(maker_address);
            }
        };
        cleanup_order_internal(
            maker_address,
            order_id,
            maker_order.get_book_type_from_match_details(),
            maker_order.is_bid_from_match_details(),
            maker_cancel_size,
            metadata,
            callbacks
        );
    }

    fun cancel_single_order_internal<M: store + copy + drop>(
        market: &mut Market<M>,
        user_addr: address,
        limit_price: u64,
        order_id: OrderIdType,
        client_order_id: Option<u64>,
        orig_size: u64,
        size_delta: u64,
        fill_sizes: vector<u64>,
        match_count: u32,
        is_bid: bool,
        is_taker: bool,
        cancel_reason: OrderCancellationReason,
        cancel_details: String,
        metadata: M,
        time_in_force: TimeInForce,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): OrderMatchResult {
        market.emit_event_for_order(
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
            cancel_details,
            option::some(metadata),
            option::none(), // trigger_condition
            time_in_force,
            callbacks
        );
        callbacks.cleanup_order(
            user_addr, order_id, is_bid, size_delta, metadata
        );
        return OrderMatchResult {
            order_id,
            remaining_size: 0,
            cancel_reason: option::some(cancel_reason),
            fill_sizes,
            match_count
        }
    }

    public fun cleanup_order_internal<M: store + copy + drop>(
        user_addr: address,
        order_id: OrderIdType,
        book_type: OrderBookType,
        is_bid: bool,
        remaining_size: u64,
        metadata: Option<M>,
        callbacks: &MarketClearinghouseCallbacks<M>
    ) {
        if (book_type == single_order_book_type()) {
            callbacks.cleanup_order(
                user_addr, order_id, is_bid, remaining_size, metadata.destroy_some()
            );
        } else {
            callbacks.cleanup_bulk_orders(
                user_addr, is_bid, remaining_size
            );
        }
    }

    fun settle_single_trade<M: store + copy + drop>(
        market: &mut Market<M>,
        user_addr: address,
        price: u64,
        orig_size: u64,
        remaining_size: &mut u64,
        is_bid: bool,
        metadata: M,
        order_id: OrderIdType,
        client_order_id: Option<u64>,
        callbacks: &MarketClearinghouseCallbacks<M>,
        time_in_force: TimeInForce,
        fill_sizes: &mut vector<u64>,
    ): Option<OrderCancellationReason> {
        let result =
            market.get_order_book_mut()
                .get_single_match_for_taker(price, *remaining_size, is_bid);
        let (maker_order, maker_matched_size) = result.destroy_order_match();
        if (!market.is_allowed_self_trade() && maker_order.get_account_from_match_details() == user_addr) {
            cancel_maker_order_internal(
                market,
                &maker_order,
                maker_order.get_client_order_id_from_match_details(),
                maker_order.get_account_from_match_details(),
                maker_order.get_order_id_from_match_details(),
                std::string::utf8(b"Disallowed self trading"),
                maker_matched_size,
                maker_order.get_metadata_from_match_details(),
                maker_order.get_time_in_force_from_match_details(),
                callbacks
            );
            return option::none();
        };
        let fill_id = market.next_fill_id();
        let settle_result = callbacks.settle_trade(
            market,
            user_addr,
            order_id,
            maker_order.get_account_from_match_details(),
            maker_order.get_order_id_from_match_details(),
            fill_id,
            is_bid,
            maker_order.get_price_from_match_details(), // Order is always matched at the price of the maker
            maker_matched_size,
            option::some(metadata),
            // TODO(skedia) fix this to pass option to the callbacks
            maker_order.get_metadata_from_match_details()
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
                option::some(metadata),
                option::none(),
                time_in_force,
                callbacks
            );
            // Event for maker fill
            market.emit_event_for_order(
                maker_order.get_order_id_from_match_details(),
                maker_order.get_client_order_id_from_match_details(),
                maker_order.get_account_from_match_details(),
                maker_order.get_orig_size_from_match_details(),
                maker_order.get_remaining_size_from_match_details() + unsettled_maker_size,
                settled_size,
                maker_order.get_price_from_match_details(),
                !is_bid,
                false,
                market_types::order_status_filled(),
                std::string::utf8(b""),
                maker_order.get_metadata_from_match_details(),
                option::none(),
                maker_order.get_time_in_force_from_match_details(),
                callbacks
            );
        };

        let maker_cancellation_reason = settle_result.get_maker_cancellation_reason();

        let taker_cancellation_reason = settle_result.get_taker_cancellation_reason();
        if (taker_cancellation_reason.is_some()) {
            cancel_single_order_internal(
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
                true, // is_taker
                OrderCancellationReason::ClearinghouseSettleViolation,
                taker_cancellation_reason.destroy_some(),
                metadata,
                time_in_force,
                callbacks
            );
            if (maker_cancellation_reason.is_none() && unsettled_maker_size > 0) {
                // If the taker is cancelled but the maker is not cancelled, then we need to re-insert
                // the maker order back into the order book
                let reinsertion_request = maker_order.new_order_match_details_with_modified_size(unsettled_maker_size);
                market.get_order_book_mut().reinsert_order(
                    reinsertion_request,
                    &maker_order
                );
            };
            return option::some(OrderCancellationReason::ClearinghouseSettleViolation);
        };
        if (maker_cancellation_reason.is_some()) {
            cancel_maker_order_internal(
                market,
                &maker_order,
                maker_order.get_client_order_id_from_match_details(),
                maker_order.get_account_from_match_details(),
                maker_order.get_order_id_from_match_details(),
                maker_cancellation_reason.destroy_some(),
                unsettled_maker_size,
                maker_order.get_metadata_from_match_details(),
                maker_order.get_time_in_force_from_match_details(),
                callbacks
            );
        } else if (maker_order.get_remaining_size_from_match_details() == 0) {
            cleanup_order_internal(
                maker_order.get_account_from_match_details(),
                maker_order.get_order_id_from_match_details(),
                maker_order.get_book_type_from_match_details(),
                !is_bid, // is_bid is inverted for maker orders
                0, // 0 because the order is fully filled
                maker_order.get_metadata_from_match_details(),
                callbacks
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
        market: &mut Market<M>,
        user_addr: address,
        limit_price: u64,
        orig_size: u64,
        remaining_size: u64,
        is_bid: bool,
        time_in_force: TimeInForce,
        trigger_condition: Option<TriggerCondition>,
        metadata: M,
        order_id: Option<OrderIdType>,
        client_order_id: Option<u64>,
        max_match_limit: u32,
        cancel_on_match_limit: bool,
        emit_taker_order_open: bool,
        callbacks: &MarketClearinghouseCallbacks<M>
    ): OrderMatchResult {
        assert!(
            orig_size > 0 && remaining_size > 0,
            EINVALID_ORDER
        );
        if (order_id.is_none()) {
            // If order id is not provided, generate a new order id
            order_id = option::some(market.next_order_id());
        };
        let order_id = order_id.destroy_some();
        // TODO(skedia) is_taker_order API can actually return false positive as the maker orders might not be valid.
        // Changes are needed to ensure the maker order is valid for this order to be a valid taker order.
        // TODO(skedia) reconsile the semantics around global order id vs account local id.
        let is_taker_order =
            market.get_order_book().is_taker_order(limit_price, is_bid, trigger_condition);

        if (emit_taker_order_open && trigger_condition.is_none()) {
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
                option::some(metadata),
                trigger_condition,
                time_in_force,
                callbacks
            );
        };

        if (
            !callbacks.validate_order_placement(
                user_addr,
                order_id,
                is_taker_order, // is_taker
                is_bid,
                limit_price,
                time_in_force,
                remaining_size,
                metadata
            )) {
            return cancel_single_order_internal(
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
                is_taker_order, // is_taker
                OrderCancellationReason::PositionUpdateViolation,
                std::string::utf8(b"Position Update violation"),
                metadata,
                time_in_force,
                callbacks
            );
        };

        if (client_order_id.is_some()) {
            if (market.get_order_book().client_order_id_exists(user_addr, client_order_id.destroy_some())) {
                // Client provided a client order id that already exists in the order book
                return cancel_single_order_internal(
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
                    is_taker_order, // is_taker
                    OrderCancellationReason::DuplicateClientOrderIdViolation,
                    std::string::utf8(b"Duplicate client order id"),
                    metadata,
                    time_in_force,
                    callbacks
                );
            };

            if (is_pre_cancelled(
                market.get_pre_cancellation_tracker_mut(),
                user_addr,
                client_order_id.destroy_some()
            )) {
                return cancel_single_order_internal(
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
                    is_taker_order, // is_taker
                    OrderCancellationReason::OrderPreCancelled,
                    std::string::utf8(b"Order pre cancelled"),
                    metadata,
                    time_in_force,
                    callbacks
                );
            };
        };

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
                false,
                callbacks
            );
        };

        // NOTE: We should always use is_taker: true for this order past this
        // point so that indexer can consistently track the order's status
        if (time_in_force == post_only()) {
            return cancel_single_order_internal(
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
                true, // is_taker
                OrderCancellationReason::PostOnlyViolation,
                std::string::utf8(b"Post Only violation"),
                metadata,
                time_in_force,
                callbacks
            );
        };
        let fill_sizes = vector::empty();
        let match_count = 0;
        loop {
            match_count += 1;
            let taker_cancellation_reason =
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
            if (taker_cancellation_reason.is_some()) {
                return OrderMatchResult {
                    order_id,
                    remaining_size: 0, // 0 because the order is cancelled
                    cancel_reason: taker_cancellation_reason,
                    fill_sizes,
                    match_count
                }
            };
            if (remaining_size == 0) {
                cleanup_order_internal(
                    user_addr, order_id, single_order_book_type(), is_bid, 0, option::some(metadata), callbacks
                );
                break;
            };

            // Check if the next iteration will still match
            let is_taker_order =
                market.is_taker_order(limit_price, is_bid, option::none());
            if (!is_taker_order) {
                if (time_in_force == immediate_or_cancel()) {
                    return cancel_single_order_internal(
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
                        true, // is_taker
                        OrderCancellationReason::IOCViolation,
                        std::string::utf8(b"IOC_VIOLATION"),
                        metadata,
                        time_in_force,
                        callbacks
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
                        trigger_condition,
                        metadata,
                        order_id,
                        client_order_id,
                        true, // emit_order_open
                        callbacks
                    );
                };
            };

            if (match_count >= max_match_limit) {
                if (cancel_on_match_limit) {
                    return cancel_single_order_internal(
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
                        true, // is_taker
                        OrderCancellationReason::MaxFillLimitViolation,
                        std::string::utf8(b"Max fill limit reached"),
                        metadata,
                        time_in_force,
                        callbacks
                    );
                } else {
                    return OrderMatchResult {
                        order_id,
                        remaining_size,
                        cancel_reason: option::some(
                            OrderCancellationReason::MaxFillLimitViolation
                        ),
                        fill_sizes,
                        match_count
                    }
                };
            };
        };
        OrderMatchResult {
            order_id,
            remaining_size,
            cancel_reason: option::none(),
            fill_sizes,
            match_count
        }
    }



    // ============================= test_only APIs ====================================
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
}

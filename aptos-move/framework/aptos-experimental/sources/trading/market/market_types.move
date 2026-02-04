module aptos_experimental::market_types {
    friend aptos_experimental::order_placement;
    friend aptos_experimental::market_bulk_order;
    friend aptos_experimental::order_operations;
    friend aptos_experimental::dead_mans_switch_operations;

    use std::option;
    use std::option::Option;
    use std::signer;
    use std::string::String;
    use aptos_std::table;
    use aptos_std::table::Table;
    use aptos_framework::event;
    use aptos_trading::single_order_types::SingleOrder;
    use aptos_trading::order_book_types::{OrderId, TimeInForce, TriggerCondition};
    use aptos_experimental::dead_mans_switch_tracker;
    use aptos_experimental::dead_mans_switch_tracker::{
        DeadMansSwitchTracker,
        new_dead_mans_switch_tracker
    };
    use aptos_experimental::market_clearinghouse_order_info::MarketClearinghouseOrderInfo;
    use aptos_experimental::order_book::{OrderBook, new_order_book};
    use aptos_experimental::pre_cancellation_tracker::{
        PreCancellationTracker,
        new_pre_cancellation_tracker
    };

    #[test_only]
    use aptos_experimental::pre_cancellation_tracker::destroy_tracker;
    #[test_only]
    use aptos_trading::order_book_types::new_order_id_type;

    const EINVALID_ADDRESS: u64 = 1;
    const EINVALID_SETTLE_RESULT: u64 = 2;
    const EINVALID_TIME_IN_FORCE: u64 = 3;
    const EORDER_DOES_NOT_EXIST: u64 = 6;

    const PRE_CANCELLATION_TRACKER_KEY: u8 = 0;
    const DEAD_MANS_SWITCH_TRACKER_KEY: u8 = 1;

    /// Reasons why an order was cancelled
    enum OrderCancellationReason has drop, copy, store {
        PostOnlyViolation,
        IOCViolation,
        PositionUpdateViolation,
        ReduceOnlyViolation,
        ClearinghouseSettleViolation,
        MaxFillLimitViolation,
        DuplicateClientOrderIdViolation,
        OrderPreCancelled,
        PlaceMakerOrderViolation,
        DeadMansSwitchExpired,
        DisallowedSelfTrading,
        OrderCancelledByUser,
        OrderCancelledBySystem,
        OrderCancelledBySystemDueToError,
        ClearinghouseStoppedMatching
    }

    public fun order_cancellation_reason_post_only_violation(): OrderCancellationReason {
        OrderCancellationReason::PostOnlyViolation
    }

    public fun order_cancellation_reason_ioc_violation(): OrderCancellationReason {
        OrderCancellationReason::IOCViolation
    }

    public fun order_cancellation_reason_position_update_violation()
        : OrderCancellationReason {
        OrderCancellationReason::PositionUpdateViolation
    }

    public fun order_cancellation_reason_clearinghouse_settle_violation()
        : OrderCancellationReason {
        OrderCancellationReason::ClearinghouseSettleViolation
    }

    public fun order_cancellation_reason_max_fill_limit_violation()
        : OrderCancellationReason {
        OrderCancellationReason::MaxFillLimitViolation
    }

    public fun order_cancellation_reason_duplicate_client_order_id()
        : OrderCancellationReason {
        OrderCancellationReason::DuplicateClientOrderIdViolation
    }

    public fun order_cancellation_reason_order_pre_cancelled(): OrderCancellationReason {
        OrderCancellationReason::OrderPreCancelled
    }

    public fun order_cancellation_reason_place_maker_order_violation()
        : OrderCancellationReason {
        OrderCancellationReason::PlaceMakerOrderViolation
    }

    public fun order_cancellation_reason_dead_mans_switch_expired()
        : OrderCancellationReason {
        OrderCancellationReason::DeadMansSwitchExpired
    }

    public fun order_cancellation_reason_disallowed_self_trading(): OrderCancellationReason {
        OrderCancellationReason::DisallowedSelfTrading
    }

    public fun order_cancellation_reason_cancelled_by_user(): OrderCancellationReason {
        OrderCancellationReason::OrderCancelledByUser
    }

    public fun order_cancellation_reason_cancelled_by_system(): OrderCancellationReason {
        OrderCancellationReason::OrderCancelledBySystem
    }

    public fun order_cancellation_reason_cancelled_by_system_due_to_error()
        : OrderCancellationReason {
        OrderCancellationReason::OrderCancelledBySystemDueToError
    }

    public fun order_cancellation_reason_clearinghouse_stopped_matching()
        : OrderCancellationReason {
        OrderCancellationReason::ClearinghouseStoppedMatching
    }

    enum OrderStatus has drop, copy, store {
        /// Order has been accepted by the engine.
        OPEN,
        /// Order has been fully or partially filled.
        FILLED,
        /// Order has been cancelled by the user or engine.
        CANCELLED,
        /// Order has been rejected by the engine. Unlike cancelled orders, rejected
        /// orders are invalid orders. Rejection reasons:
        /// 1. Insufficient margin
        /// 2. Order is reduce_only but does not reduce
        REJECTED,
        SIZE_REDUCED,
        /// Order has been acknowledged by the engine. This is used when the system wants to provide an early acknowledgement
        /// of the order placement along with order id before the order is opened.
        ACKNOWLEDGED
    }

    public fun order_status_open(): OrderStatus {
        OrderStatus::OPEN
    }

    public fun order_status_filled(): OrderStatus {
        OrderStatus::FILLED
    }

    public fun order_status_cancelled(): OrderStatus {
        OrderStatus::CANCELLED
    }

    public fun order_status_rejected(): OrderStatus {
        OrderStatus::REJECTED
    }

    public fun order_status_size_reduced(): OrderStatus {
        OrderStatus::SIZE_REDUCED
    }

    public fun order_status_acknowledged(): OrderStatus {
        OrderStatus::ACKNOWLEDGED
    }

    enum CallbackResult<R: store + copy + drop> has copy, drop {
        NOT_AVAILABLE,
        CONTINUE_MATCHING {
            result: R
        }
        STOP_MATCHING {
            result: R
        }
    }

    enum SettleTradeResult<R: store + copy + drop> has drop {
        V1 {
            settled_size: u64,
            maker_cancellation_reason: Option<String>,
            taker_cancellation_reason: Option<String>,
            callback_result: CallbackResult<R>
        }
    }

    enum ValidationResult has drop, copy {
        V1 {
            // If valid this is none, else contains the reason for invalidity
            failure_reason: Option<String>
        }
    }

    enum PlaceMakerOrderResult<R: store + copy + drop> has drop, copy {
        V1 {
            cancellation_reason: Option<String>,
            action: Option<R>
        }
    }

    enum MarketClearinghouseCallbacks<M: store + copy + drop, R: store + copy + drop> has drop {
        V1 {
            /// settle_trade_f arguments: market, taker, maker, fill_id, settled_price, settled_size,
            settle_trade_f: |
                &mut Market<M>,
                MarketClearinghouseOrderInfo<M>,
                MarketClearinghouseOrderInfo<M>,
                u128,
                u64,
                u64
            | SettleTradeResult<R> has drop + copy,
            /// validate_settlement_update_f arguments: order_info, size
            validate_order_placement_f: |
                MarketClearinghouseOrderInfo<M>,
                u64
            | ValidationResult has drop + copy,
            /// Validate the bulk order placement arguments: account, bids_prices, bids_sizes, asks_prices, asks_sizes
            validate_bulk_order_placement_f: |
                address,
                &vector<u64>,
                &vector<u64>,
                &vector<u64>,
                &vector<u64>,
                &M
            | ValidationResult has drop + copy,
            /// place_maker_order_f arguments: order_info, size
            place_maker_order_f: |
                MarketClearinghouseOrderInfo<M>,
                u64
            | PlaceMakerOrderResult<R> has drop + copy,
            /// cleanup_order_f arguments: order_info, cleanup_size, is_taker
            cleanup_order_f: |
                MarketClearinghouseOrderInfo<M>,
                u64,
                bool
            | has drop + copy,
            /// cleanup_bulk_orders_f arguments: account, is_bid, remaining_sizes
            cleanup_bulk_order_at_price_f: |
                address,
                OrderId,
                bool,
                u64,
                u64
            | has drop + copy,
            /// place_bulk_order_f arguments: account, order_id, bid_prices, bid_sizes, ask_prices, ask_sizes,
            /// cancelled_bid_prices, cancelled_bid_sizes, cancelled_ask_prices, cancelled_ask_sizes, metadata
            place_bulk_order_f: |
                address,
                OrderId,
                &vector<u64>,
                &vector<u64>,
                &vector<u64>,
                &vector<u64>,
                &vector<u64>,
                &vector<u64>,
                &vector<u64>,
                &vector<u64>,
                &M
            | has drop + copy,
            /// decrease_order_size_f arguments: order_info, size
            decrease_order_size_f: |
                MarketClearinghouseOrderInfo<M>,
                u64
            | has drop + copy,
            /// get a string representation of order metadata to be used in events
            get_order_metadata_bytes: |&M| vector<u8> has drop + copy
        }
    }

    public fun new_settle_trade_result<R: store + copy + drop>(
        settled_size: u64,
        maker_cancellation_reason: Option<String>,
        taker_cancellation_reason: Option<String>,
        callback_result: CallbackResult<R>
    ): SettleTradeResult<R> {
        SettleTradeResult::V1 {
            settled_size,
            maker_cancellation_reason,
            taker_cancellation_reason,
            callback_result
        }
    }

    public fun new_validation_result(
        cancellation_reason: Option<String>
    ): ValidationResult {
        ValidationResult::V1 { failure_reason: cancellation_reason }
    }

    public fun new_place_maker_order_result<R: store + copy + drop>(
        cancellation_reason: Option<String>, actions: Option<R>
    ): PlaceMakerOrderResult<R> {
        PlaceMakerOrderResult::V1 { cancellation_reason, action: actions }
    }

    public fun new_market_clearinghouse_callbacks<M: store + copy + drop, R: store + copy + drop>(
        settle_trade_f: |
            &mut Market<M>,
            MarketClearinghouseOrderInfo<M>,
            MarketClearinghouseOrderInfo<M>,
            u128,
            u64,
            u64
        | SettleTradeResult<R> has drop + copy,
        validate_order_placement_f: |
            MarketClearinghouseOrderInfo<M>,
            u64
        | ValidationResult has drop + copy,
        validate_bulk_order_placement_f: |
            address,
            &vector<u64>,
            &vector<u64>,
            &vector<u64>,
            &vector<u64>,
            &M
        | ValidationResult has drop + copy,
        place_maker_order_f: |MarketClearinghouseOrderInfo<M>, u64| PlaceMakerOrderResult<R> has drop
        + copy,
        cleanup_order_f: |
            MarketClearinghouseOrderInfo<M>,
            u64,
            bool
        | has drop + copy,
        cleanup_bulk_order_at_price_f: |
            address,
            OrderId,
            bool,
            u64,
            u64
        | has drop + copy,
        place_bulk_order_f: |
            address,
            OrderId,
            &vector<u64>,
            &vector<u64>,
            &vector<u64>,
            &vector<u64>,
            &vector<u64>,
            &vector<u64>,
            &vector<u64>,
            &vector<u64>,
            &M
        | has drop + copy,
        decrease_order_size_f: |
            MarketClearinghouseOrderInfo<M>,
            u64
        | has drop + copy,
        get_order_metadata_bytes: |&M| vector<u8> has drop + copy
    ): MarketClearinghouseCallbacks<M, R> {
        MarketClearinghouseCallbacks::V1 {
            settle_trade_f,
            validate_order_placement_f,
            validate_bulk_order_placement_f,
            place_maker_order_f,
            cleanup_order_f,
            cleanup_bulk_order_at_price_f,
            place_bulk_order_f,
            decrease_order_size_f,
            get_order_metadata_bytes
        }
    }

    public fun get_settled_size<R: store + copy + drop>(
        self: &SettleTradeResult<R>
    ): u64 {
        self.settled_size
    }

    public fun get_maker_cancellation_reason<R: store + copy + drop>(
        self: &SettleTradeResult<R>
    ): Option<String> {
        self.maker_cancellation_reason
    }

    public fun get_taker_cancellation_reason<R: store + copy + drop>(
        self: &SettleTradeResult<R>
    ): Option<String> {
        self.taker_cancellation_reason
    }

    public fun get_callback_result<R: store + copy + drop>(
        self: &SettleTradeResult<R>
    ): &CallbackResult<R> {
        &self.callback_result
    }

    public fun is_validation_result_valid(self: &ValidationResult): bool {
        self.failure_reason.is_none()
    }

    public fun get_validation_failure_reason(self: &ValidationResult): Option<String> {
        self.failure_reason
    }

    public fun get_place_maker_order_actions<R: store + copy + drop>(
        self: &PlaceMakerOrderResult<R>
    ): Option<R> {
        self.action
    }

    public fun get_place_maker_order_cancellation_reason<R: store + copy + drop>(
        self: &PlaceMakerOrderResult<R>
    ): Option<String> {
        self.cancellation_reason
    }

    public fun extract_results<R: store + copy + drop>(
        self: CallbackResult<R>
    ): Option<R> {
        match(self) {
            CallbackResult::NOT_AVAILABLE => option::none(),
            CallbackResult::CONTINUE_MATCHING { result } => option::some(result),
            CallbackResult::STOP_MATCHING { result } => option::some(result)
        }
    }

    public fun should_stop_matching<R: store + copy + drop>(
        self: &CallbackResult<R>
    ): bool {
        self is CallbackResult::STOP_MATCHING
    }

    public fun new_callback_result_continue_matching<R: store + copy + drop>(
        result: R
    ): CallbackResult<R> {
        CallbackResult::CONTINUE_MATCHING { result }
    }

    public fun new_callback_result_stop_matching<R: store + copy + drop>(
        result: R
    ): CallbackResult<R> {
        CallbackResult::STOP_MATCHING { result }
    }

    public fun new_callback_result_not_available<R: store + copy + drop>()
        : CallbackResult<R> {
        CallbackResult::NOT_AVAILABLE
    }

    #[lint::skip(needless_mutable_reference)]
    public fun settle_trade<M: store + copy + drop, R: store + copy + drop>(
        self: &MarketClearinghouseCallbacks<M, R>,
        market: &mut Market<M>,
        taker: MarketClearinghouseOrderInfo<M>,
        maker: MarketClearinghouseOrderInfo<M>,
        fill_id: u128,
        settled_price: u64,
        settled_size: u64
    ): SettleTradeResult<R> {
        (self.settle_trade_f) (market, taker, maker, fill_id, settled_price, settled_size)
    }

    public fun validate_order_placement<M: store + copy + drop, R: store + copy + drop>(
        self: &MarketClearinghouseCallbacks<M, R>,
        order_info: MarketClearinghouseOrderInfo<M>,
        size: u64
    ): ValidationResult {
        (self.validate_order_placement_f) (order_info, size)
    }

    public fun validate_bulk_order_placement<M: store + copy + drop, R: store + copy + drop>(
        self: &MarketClearinghouseCallbacks<M, R>,
        account: address,
        bids_prices: &vector<u64>,
        bids_sizes: &vector<u64>,
        asks_prices: &vector<u64>,
        asks_sizes: &vector<u64>,
        order_metadata: &M
    ): ValidationResult {
        (self.validate_bulk_order_placement_f) (
            account, bids_prices, bids_sizes, asks_prices, asks_sizes, order_metadata
        )
    }

    public fun place_maker_order<M: store + copy + drop, R: store + copy + drop>(
        self: &MarketClearinghouseCallbacks<M, R>,
        order_info: MarketClearinghouseOrderInfo<M>,
        size: u64
    ): PlaceMakerOrderResult<R> {
        (self.place_maker_order_f) (order_info, size)
    }

    public fun cleanup_order<M: store + copy + drop, R: store + copy + drop>(
        self: &MarketClearinghouseCallbacks<M, R>,
        order_info: MarketClearinghouseOrderInfo<M>,
        cleanup_size: u64,
        is_taker: bool
    ) {
        (self.cleanup_order_f) (order_info, cleanup_size, is_taker)
    }

    public fun cleanup_bulk_order_at_price<M: store + copy + drop, R: store + copy + drop>(
        self: &MarketClearinghouseCallbacks<M, R>,
        account: address,
        order_id: OrderId,
        is_bid: bool,
        price: u64,
        cleanup_size: u64
    ) {
        (self.cleanup_bulk_order_at_price_f) (
            account, order_id, is_bid, price, cleanup_size
        )
    }

    public fun place_bulk_order<M: store + copy + drop, R: store + copy + drop>(
        self: &MarketClearinghouseCallbacks<M, R>,
        account: address,
        order_id: OrderId,
        bid_prices: &vector<u64>,
        bid_sizes: &vector<u64>,
        ask_prices: &vector<u64>,
        ask_sizes: &vector<u64>,
        cancelled_bid_prices: &vector<u64>,
        cancelled_bid_sizes: &vector<u64>,
        cancelled_ask_prices: &vector<u64>,
        cancelled_ask_sizes: &vector<u64>,
        metadata: &M
    ) {
        (self.place_bulk_order_f) (
            account,
            order_id,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            cancelled_bid_prices,
            cancelled_bid_sizes,
            cancelled_ask_prices,
            cancelled_ask_sizes,
            metadata
        )
    }

    public fun decrease_order_size<M: store + copy + drop, R: store + copy + drop>(
        self: &MarketClearinghouseCallbacks<M, R>,
        order_info: MarketClearinghouseOrderInfo<M>,
        new_size: u64
    ) {
        (self.decrease_order_size_f) (order_info, new_size)
    }

    public fun get_order_metadata_bytes<M: store + copy + drop, R: store + copy + drop>(
        self: &MarketClearinghouseCallbacks<M, R>,
        order_metadata: &M
    ): vector<u8> {
        (self.get_order_metadata_bytes) (order_metadata)
    }

    // ============================= Market Types ====================================
    enum Market<M: store + copy + drop> has store {
        V1 {
            /// Address of the parent object that created this market
            /// Purely for grouping events based on the source DEX, not used otherwise
            parent: address,
            /// Address of the market object of this market.
            market: address,
            config: MarketConfig,
            order_book: OrderBook<M>,
            /// Pre cancellation tracker for the market, it is wrapped inside a table
            /// as otherwise any insertion/deletion from the tracker would cause conflict
            /// with the order book.
            pre_cancellation_tracker: Table<u8, PreCancellationTracker>,
            dead_mans_switch_tracker: Table<u8, DeadMansSwitchTracker>
        }
    }

    enum MarketConfig has store, drop {
        V1 {
            /// Weather to allow self matching orders
            allow_self_trade: bool,
            /// Whether to allow sending all events for the markett
            allow_events_emission: bool,
            /// Pre cancellation window in seconds
            pre_cancellation_window_secs: u64,
            /// Enable dead man's switch functionality
            enable_dead_mans_switch: bool,
            min_keep_alive_time_secs: u64
        }
    }

    #[event]
    enum OrderEvent has drop, copy, store {
        V1 {
            parent: address,
            market: address,
            order_id: u128,
            client_order_id: Option<String>,
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
            is_bid: bool,
            /// Whether the order crosses the orderbook.
            is_taker: bool,
            status: OrderStatus,
            details: std::string::String,
            metadata_bytes: vector<u8>,
            time_in_force: TimeInForce,
            trigger_condition: Option<TriggerCondition>, // Only emitted with order placement events
            cancellation_reason: Option<OrderCancellationReason> // Populated when status is CANCELLED
        }
    }

    #[event]
    enum BulkOrderPlacedEvent has drop, copy, store {
        V1 {
            parent: address,
            market: address,
            order_id: u128,
            sequence_number: u64,
            user: address,
            bid_prices: vector<u64>,
            bid_sizes: vector<u64>,
            ask_prices: vector<u64>,
            ask_sizes: vector<u64>,
            cancelled_bid_prices: vector<u64>,
            cancelled_bid_sizes: vector<u64>,
            cancelled_ask_prices: vector<u64>,
            cancelled_ask_sizes: vector<u64>,
            previous_seq_num: u64
        }
    }

    #[event]
    // This event is emitted when a bulk order is modified - especially when some levels of the bulk orders
    // are cancelled.
    enum BulkOrderModifiedEvent has drop, copy, store {
        V1 {
            parent: address,
            market: address,
            order_id: u128,
            sequence_number: u64,
            user: address,
            bid_prices: vector<u64>,
            bid_sizes: vector<u64>,
            ask_prices: vector<u64>,
            ask_sizes: vector<u64>,
            cancelled_bid_prices: vector<u64>,
            cancelled_bid_sizes: vector<u64>,
            cancelled_ask_prices: vector<u64>,
            cancelled_ask_sizes: vector<u64>,
            previous_seq_num: u64,
            cancellation_reason: Option<OrderCancellationReason> // Populated when orders are cancelled
        }
    }

    #[event]
    enum BulkOrderFilledEvent has drop, copy, store {
        V1 {
            parent: address,
            market: address,
            order_id: u128,
            sequence_number: u64,
            user: address,
            filled_size: u64,
            price: u64,
            orig_price: u64,
            is_bid: bool,
            fill_id: u128
        }
    }

    #[event]
    enum BulkOrderRejectionEvent has drop, copy, store {
        V1 {
            parent: address,
            market: address,
            user: address,
            sequence_number: u64,
            existing_sequence_number: u64
        }
    }

    // ============================= Public APIs ====================================
    public fun new_market_config(
        allow_self_matching: bool,
        allow_events_emission: bool,
        pre_cancellation_window_secs: u64,
        enable_dead_mans_switch: bool,
        min_keep_alive_time_secs: u64
    ): MarketConfig {
        MarketConfig::V1 {
            allow_self_trade: allow_self_matching,
            allow_events_emission,
            pre_cancellation_window_secs,
            enable_dead_mans_switch,
            min_keep_alive_time_secs
        }
    }

    public fun new_market<M: store + copy + drop>(
        parent: &signer, market: &signer, config: MarketConfig
    ): Market<M> {
        // requiring signers, and not addresses, purely to guarantee different dexes
        // cannot polute events to each other, accidentally or maliciously.
        let pre_cancellation_window = config.pre_cancellation_window_secs;
        let pre_cancellation_tracker = table::new();
        pre_cancellation_tracker.add(
            PRE_CANCELLATION_TRACKER_KEY,
            new_pre_cancellation_tracker(pre_cancellation_window)
        );
        let dead_mans_switch_tracker = table::new();
        dead_mans_switch_tracker.add(
            DEAD_MANS_SWITCH_TRACKER_KEY,
            new_dead_mans_switch_tracker(config.min_keep_alive_time_secs)
        );
        Market::V1 {
            parent: signer::address_of(parent),
            market: signer::address_of(market),
            config,
            order_book: new_order_book(),
            pre_cancellation_tracker,
            dead_mans_switch_tracker
        }
    }

    public fun set_allow_self_trade<M: store + copy + drop>(
        self: &mut Market<M>, allow_self_trade: bool
    ) {
        self.config.allow_self_trade = allow_self_trade;
    }

    public fun set_allow_events_emission<M: store + copy + drop>(
        self: &mut Market<M>, allow_events_emission: bool
    ) {
        self.config.allow_events_emission = allow_events_emission;
    }

    public fun set_allow_dead_mans_switch<M: store + copy + drop>(
        self: &mut Market<M>, enable_dead_mans_switch: bool
    ) {
        self.config.enable_dead_mans_switch = enable_dead_mans_switch;
    }

    public fun set_dead_mans_switch_min_keep_alive_time_secs<M: store + copy + drop>(
        self: &mut Market<M>, min_keep_alive_time_secs: u64
    ) {
        self.config.min_keep_alive_time_secs = min_keep_alive_time_secs;
        let parent = self.parent;
        let market = self.market;
        dead_mans_switch_tracker::set_min_keep_alive_time_secs(
            self.get_dead_mans_switch_tracker_mut(),
            parent,
            market,
            min_keep_alive_time_secs
        );
    }

    public fun get_order_book<M: store + copy + drop>(self: &Market<M>): &OrderBook<M> {
        &self.order_book
    }

    public fun get_market_address<M: store + copy + drop>(self: &Market<M>): address {
        self.market
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
        is_bid: bool,
        trigger_condition: Option<TriggerCondition>
    ): bool {
        self.order_book.is_taker_order(price, is_bid, trigger_condition)
    }

    public fun is_allowed_self_trade<M: store + copy + drop>(self: &Market<M>): bool {
        self.config.allow_self_trade
    }

    /// Remaining size of the order in the order book.
    public fun get_remaining_size<M: store + copy + drop>(
        self: &Market<M>, order_id: OrderId
    ): u64 {
        self.order_book.get_single_remaining_size(order_id)
    }

    public fun get_bulk_order_remaining_size<M: store + copy + drop>(
        self: &Market<M>, user: address, is_bid: bool
    ): u64 {
        self.order_book.get_bulk_order_remaining_size(user, is_bid)
    }

    public fun get_order_metadata<M: store + copy + drop>(
        self: &Market<M>, order_id: OrderId
    ): Option<M> {
        self.order_book.get_single_order_metadata(order_id)
    }

    /// Returns the order metadata for an order by order id.
    /// It is up to the caller to perform necessary permissions checks
    public fun set_order_metadata<M: store + copy + drop>(
        self: &mut Market<M>, order_id: OrderId, metadata: M
    ) {
        self.order_book.set_single_order_metadata(order_id, metadata);
    }

    public fun get_order_metadata_by_client_id<M: store + copy + drop>(
        self: &Market<M>, user: address, client_order_id: String
    ): Option<M> {
        let order_id = self.order_book.get_order_id_by_client_id(user, client_order_id);
        if (order_id.is_none()) {
            return option::none();
        };
        return self.get_order_metadata(order_id.destroy_some())
    }

    /// Sets the order metadata for an order by client id. It is up to the caller to perform necessary permissions checks
    /// around ownership of the order.
    public fun set_order_metadata_by_client_id<M: store + copy + drop>(
        self: &mut Market<M>,
        user: address,
        client_order_id: String,
        metadata: M
    ) {
        let order_id = self.order_book.get_order_id_by_client_id(user, client_order_id);
        assert!(order_id.is_some(), EORDER_DOES_NOT_EXIST);
        self.set_order_metadata(order_id.destroy_some(), metadata);
    }

    /// Returns all the pending order ready to be executed based on the oracle price. The caller is responsible to
    /// call the `place_order_with_order_id` API to place the order with the order id returned from this API.
    public fun take_ready_price_based_orders<M: store + copy + drop>(
        self: &mut Market<M>, oracle_price: u64, order_limit: u64
    ): vector<SingleOrder<M>> {
        self.order_book.take_ready_price_based_orders(oracle_price, order_limit)
    }

    /// Returns all the pending order that are ready to be executed based on current time stamp. The caller is responsible to
    /// call the `place_order_with_order_id` API to place the order with the order id returned from this API.
    public fun take_ready_time_based_orders<M: store + copy + drop>(
        self: &mut Market<M>, order_limit: u64
    ): vector<SingleOrder<M>> {
        self.order_book.take_ready_time_based_orders(order_limit)
    }

    public fun emit_event_for_order<M: store + copy + drop, R: store + copy + drop>(
        self: &Market<M>,
        order_id: OrderId,
        client_order_id: Option<String>,
        user: address,
        orig_size: u64,
        remaining_size: u64,
        size_delta: u64,
        price: u64,
        is_bid: bool,
        is_taker: bool,
        status: OrderStatus,
        details: String,
        metadata: M,
        trigger_condition: Option<TriggerCondition>,
        time_in_force: TimeInForce,
        cancellation_reason: Option<OrderCancellationReason>,
        callbacks: &MarketClearinghouseCallbacks<M, R>
    ) {
        // Final check whether event sending is enabled
        if (self.config.allow_events_emission) {
            let metadata_bytes = callbacks.get_order_metadata_bytes(&metadata);
            event::emit(
                OrderEvent::V1 {
                    parent: self.parent,
                    market: self.market,
                    order_id: order_id.get_order_id_value(),
                    client_order_id,
                    user,
                    orig_size,
                    remaining_size,
                    size_delta,
                    price,
                    is_bid,
                    is_taker,
                    status,
                    details,
                    metadata_bytes,
                    time_in_force,
                    trigger_condition,
                    cancellation_reason
                }
            );
        };
    }

    public fun emit_event_for_bulk_order_placed<M: store + copy + drop>(
        self: &Market<M>,
        order_id: OrderId,
        sequence_number: u64,
        user: address,
        bid_prices: vector<u64>,
        bid_sizes: vector<u64>,
        ask_prices: vector<u64>,
        ask_sizes: vector<u64>,
        cancelled_bid_prices: vector<u64>,
        cancelled_bid_sizes: vector<u64>,
        cancelled_ask_prices: vector<u64>,
        cancelled_ask_sizes: vector<u64>,
        previous_seq_num: u64
    ) {
        // Final check whether event sending is enabled
        if (self.config.allow_events_emission) {
            event::emit(
                BulkOrderPlacedEvent::V1 {
                    parent: self.parent,
                    market: self.market,
                    order_id: order_id.get_order_id_value(),
                    sequence_number,
                    user,
                    bid_prices,
                    bid_sizes,
                    ask_prices,
                    ask_sizes,
                    cancelled_bid_prices,
                    cancelled_bid_sizes,
                    cancelled_ask_prices,
                    cancelled_ask_sizes,
                    previous_seq_num
                }
            );
        };
    }

    public fun emit_event_for_bulk_order_cancelled<M: store + copy + drop>(
        self: &Market<M>,
        order_id: OrderId,
        sequence_number: u64,
        user: address,
        cancelled_bid_prices: vector<u64>,
        cancelled_bid_sizes: vector<u64>,
        cancelled_ask_prices: vector<u64>,
        cancelled_ask_sizes: vector<u64>,
        cancellation_reason: Option<OrderCancellationReason>
    ) {
        // Final check whether event sending is enabled
        if (self.config.allow_events_emission) {
            event::emit(
                BulkOrderModifiedEvent::V1 {
                    parent: self.parent,
                    market: self.market,
                    order_id: order_id.get_order_id_value(),
                    sequence_number,
                    user,
                    bid_prices: vector[],
                    bid_sizes: vector[],
                    ask_prices: vector[],
                    ask_sizes: vector[],
                    cancelled_bid_prices,
                    cancelled_bid_sizes,
                    cancelled_ask_prices,
                    cancelled_ask_sizes,
                    previous_seq_num: sequence_number,
                    cancellation_reason
                }
            )
        };
    }

    public fun emit_event_for_bulk_order_filled<M: store + copy + drop>(
        self: &Market<M>,
        order_id: OrderId,
        sequence_number: u64,
        user: address,
        filled_size: u64,
        price: u64,
        orig_price: u64,
        is_bid: bool,
        fill_id: u128
    ) {
        // Final check whether event sending is enabled
        if (self.config.allow_events_emission) {
            event::emit(
                BulkOrderFilledEvent::V1 {
                    parent: self.parent,
                    market: self.market,
                    order_id: order_id.get_order_id_value(),
                    sequence_number,
                    user,
                    filled_size,
                    price,
                    orig_price,
                    is_bid,
                    fill_id
                }
            );
        };
    }

    public fun emit_event_for_bulk_order_modified<M: store + copy + drop>(
        self: &Market<M>,
        order_id: OrderId,
        sequence_number: u64,
        user: address,
        bid_prices: vector<u64>,
        bid_sizes: vector<u64>,
        ask_prices: vector<u64>,
        ask_sizes: vector<u64>,
        cancelled_bid_prices: vector<u64>,
        cancelled_bid_sizes: vector<u64>,
        cancelled_ask_prices: vector<u64>,
        cancelled_ask_sizes: vector<u64>,
        cancellation_reason: Option<OrderCancellationReason>
    ) {
        // Final check whether event sending is enabled
        if (self.config.allow_events_emission) {
            event::emit(
                BulkOrderModifiedEvent::V1 {
                    parent: self.parent,
                    market: self.market,
                    order_id: order_id.get_order_id_value(),
                    sequence_number,
                    user,
                    bid_prices,
                    bid_sizes,
                    ask_prices,
                    ask_sizes,
                    cancelled_bid_prices,
                    cancelled_bid_sizes,
                    cancelled_ask_prices,
                    cancelled_ask_sizes,
                    previous_seq_num: sequence_number,
                    cancellation_reason
                }
            );
        };
    }

    public fun emit_event_for_bulk_order_rejection<M: store + copy + drop>(
        self: &Market<M>,
        user: address,
        sequence_number: u64,
        existing_sequence_number: u64
    ) {
        // Final check whether event sending is enabled
        if (self.config.allow_events_emission) {
            event::emit(
                BulkOrderRejectionEvent::V1 {
                    parent: self.parent,
                    market: self.market,
                    user,
                    sequence_number,
                    existing_sequence_number
                }
            );
        };
    }

    // ============================= Public Package APIs ====================================
    public(friend) fun get_order_book_mut<M: store + copy + drop>(
        self: &mut Market<M>
    ): &mut OrderBook<M> {
        &mut self.order_book
    }

    public(friend) fun get_pre_cancellation_tracker_mut<M: store + copy + drop>(
        self: &mut Market<M>
    ): &mut PreCancellationTracker {
        self.pre_cancellation_tracker.borrow_mut(PRE_CANCELLATION_TRACKER_KEY)
    }

    public(friend) fun get_dead_mans_switch_tracker<M: store + copy + drop>(
        self: &Market<M>
    ): &DeadMansSwitchTracker {
        self.dead_mans_switch_tracker.borrow(DEAD_MANS_SWITCH_TRACKER_KEY)
    }

    public(friend) fun get_dead_mans_switch_tracker_mut<M: store + copy + drop>(
        self: &mut Market<M>
    ): &mut DeadMansSwitchTracker {
        self.dead_mans_switch_tracker.borrow_mut(DEAD_MANS_SWITCH_TRACKER_KEY)
    }

    public(friend) fun is_dead_mans_switch_enabled<M: store + copy + drop>(
        self: &Market<M>
    ): bool {
        self.config.enable_dead_mans_switch
    }

    public(friend) fun get_parent<M: store + copy + drop>(self: &Market<M>): address {
        self.parent
    }

    public(friend) fun get_market<M: store + copy + drop>(self: &Market<M>): address {
        self.market
    }

    // ============================= test_only APIs ====================================
    #[test_only]
    public fun destroy_market<M: store + copy + drop>(self: Market<M>) {
        let Market::V1 {
            parent: _parent,
            market: _market,
            config,
            order_book,
            pre_cancellation_tracker,
            dead_mans_switch_tracker
        } = self;
        let MarketConfig::V1 {
            allow_self_trade: _,
            allow_events_emission: _,
            pre_cancellation_window_secs: _,
            enable_dead_mans_switch: _,
            min_keep_alive_time_secs: _
        } = config;
        destroy_tracker(pre_cancellation_tracker.remove(PRE_CANCELLATION_TRACKER_KEY));
        dead_mans_switch_tracker::destroy_tracker(
            dead_mans_switch_tracker.remove(DEAD_MANS_SWITCH_TRACKER_KEY)
        );
        pre_cancellation_tracker.drop_unchecked();
        dead_mans_switch_tracker.drop_unchecked();
        order_book.destroy_order_book()
    }

    #[test_only]
    public fun get_order_id_from_event(self: OrderEvent): OrderId {
        new_order_id_type(self.order_id)
    }

    #[test_only]
    public fun get_client_order_id_from_event(self: OrderEvent): Option<String> {
        self.client_order_id
    }

    #[test_only]
    public fun verify_order_event(
        self: OrderEvent,
        order_id: OrderId,
        client_order_id: Option<String>,
        market: address,
        user: address,
        orig_size: u64,
        remaining_size: u64,
        size_delta: u64,
        price: u64,
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

    #[test_only]
    public fun verify_bulk_order_placed_event(
        self: BulkOrderPlacedEvent,
        order_id: OrderId,
        market: address,
        user: address,
        bid_prices: vector<u64>,
        bid_sizes: vector<u64>,
        ask_prices: vector<u64>,
        ask_sizes: vector<u64>
    ) {
        assert!(self.order_id == order_id.get_order_id_value());
        assert!(self.market == market);
        assert!(self.user == user);
        assert!(self.bid_sizes == bid_sizes);
        assert!(self.bid_prices == bid_prices);
        assert!(self.ask_sizes == ask_sizes);
        assert!(self.ask_prices == ask_prices);
    }

    #[test_only]
    public fun verify_bulk_order_filled_event(
        self: BulkOrderFilledEvent,
        order_id: OrderId,
        sequence_number: u64,
        market: address,
        user: address,
        filled_size: u64,
        price: u64,
        is_bid: bool
    ) {
        assert!(self.order_id == order_id.get_order_id_value());
        assert!(self.sequence_number == sequence_number);
        assert!(self.market == market);
        assert!(self.user == user);
        assert!(self.filled_size == filled_size);
        assert!(self.price == price);
        assert!(self.is_bid == is_bid);
    }

    #[test_only]
    public fun verify_bulk_order_modified_event(
        self: BulkOrderModifiedEvent,
        order_id: OrderId,
        sequence_number: u64,
        market: address,
        user: address,
        bid_prices: vector<u64>,
        bid_sizes: vector<u64>,
        ask_prices: vector<u64>,
        ask_sizes: vector<u64>,
        cancelled_bid_prices: vector<u64>,
        cancelled_bid_sizes: vector<u64>,
        cancelled_ask_prices: vector<u64>,
        cancelled_ask_sizes: vector<u64>,
        previous_seq_num: u64
    ) {
        assert!(self.order_id == order_id.get_order_id_value());
        assert!(self.sequence_number == sequence_number);
        assert!(self.market == market);
        assert!(self.user == user);
        assert!(self.bid_sizes == bid_sizes);
        assert!(self.bid_prices == bid_prices);
        assert!(self.ask_sizes == ask_sizes);
        assert!(self.ask_prices == ask_prices);
        assert!(self.cancelled_bid_sizes == cancelled_bid_sizes);
        assert!(self.cancelled_bid_prices == cancelled_bid_prices);
        assert!(self.cancelled_ask_sizes == cancelled_ask_sizes);
        assert!(self.cancelled_ask_prices == cancelled_ask_prices);
        assert!(self.previous_seq_num == previous_seq_num);
    }
}

module aptos_experimental::market_types {
    use std::option;
    use std::option::Option;
    use std::signer;
    use std::string::String;
    use std::vector;
    use aptos_std::table;
    use aptos_std::table::Table;
    use aptos_framework::event;
    use aptos_experimental::single_order_types::SingleOrder;

    use aptos_experimental::order_book_types::{OrderIdType, new_order_id_type};
    use aptos_experimental::order_book_types::TimeInForce;
    use aptos_experimental::order_book_types::TriggerCondition;
    use aptos_experimental::order_book::{OrderBook, new_order_book};
    use aptos_experimental::pre_cancellation_tracker::{PreCancellationTracker, new_pre_cancellation_tracker};
    use aptos_experimental::order_book_types::AscendingIdGenerator;
    use aptos_experimental::order_book_types::{
        new_ascending_id_generator,
    };
    use aptos_experimental::market_clearinghouse_order_info::MarketClearinghouseOrderInfo;
    #[test_only]
    use aptos_experimental::pre_cancellation_tracker::destroy_tracker;

    friend aptos_experimental::order_placement;
    friend aptos_experimental::market_bulk_order;
    friend aptos_experimental::order_operations;

    const EINVALID_ADDRESS: u64 = 1;
    const EINVALID_SETTLE_RESULT: u64 = 2;
    const EINVALID_TIME_IN_FORCE: u64 = 3;
    const EORDER_DOES_NOT_EXIST: u64 = 6;

    const PRE_CANCELLATION_TRACKER_KEY: u8 = 0;

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
        ACKNOWLEDGED,
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

    enum SettleTradeResult has drop {
        V1 {
            settled_size: u64,
            settled_price: u64,
            maker_cancellation_reason: Option<String>,
            taker_cancellation_reason: Option<String>,
        }
    }

    enum MarketClearinghouseCallbacks<M: store + copy + drop> has drop {
        V1 {
            /// settle_trade_f arguments: taker_order_info, maker_order_info, fill_id.
            /// taker_order_info.size == maker_order_info.size and taker_order_info.is_bid == !maker_order_info.is_bid
            settle_trade_f: |&mut Market<M>, MarketClearinghouseOrderInfo<M>, MarketClearinghouseOrderInfo<M>, u64| SettleTradeResult has drop + copy,
            /// validate_settlement_update_f arguments: order_info, is_taker, time_in_force
            validate_order_placement_f: |MarketClearinghouseOrderInfo<M>, bool, TimeInForce| bool has drop + copy,
            /// Validate the bulk order placement: account, bids_prices, bids_sizes, asks_prices, asks_sizes
            validate_bulk_order_placement_f: |address, vector<u64>, vector<u64>, vector<u64>, vector<u64>| bool has drop + copy,
            /// place_maker_order_f arguments: order_info
            place_maker_order_f: |MarketClearinghouseOrderInfo<M>| has drop + copy,
            /// cleanup_order_f arguments: account, order_id, is_bid, remaining_size, order_metadata
            cleanup_order_f: |address, OrderIdType, bool, u64, M| has drop + copy,
            /// cleanup_bulk_orders_f arguments: account, is_bid, remaining_sizes
            cleanup_bulk_orders_f: |address, bool, u64| has drop + copy,
            /// decrease_order_size_f arguments: order_info
            decrease_order_size_f: |MarketClearinghouseOrderInfo<M>| has drop + copy,
            /// get a string representation of order metadata to be used in events
            get_order_metadata_bytes: |M| vector<u8> has drop + copy
        }
    }

    public fun new_settle_trade_result(
        settled_size: u64,
        settled_price: u64,
        maker_cancellation_reason: Option<String>,
        taker_cancellation_reason: Option<String>
    ): SettleTradeResult {
        SettleTradeResult::V1 {
            settled_size,
            settled_price,
            maker_cancellation_reason,
            taker_cancellation_reason
        }
    }

    /// Arguments to callback functions:
    ///
    /// * settle_trade_f arguments: taker_order_info, maker_order_info, fill_id.
    /// taker_order_info.size == maker_order_info.size and taker_order_info.is_bid == !maker_order_info.is_bid
    /// * validate_settlement_update_f arguments: order_info, is_taker, time_in_force
    /// * validate_bulk_order_placement_f: account, bids_prices, bids_sizes, asks_prices, asks_sizes
    /// * place_maker_order_f arguments: order_info
    /// * cleanup_order_f arguments: account, order_id, is_bid, remaining_size, order_metadata
    /// * cleanup_bulk_orders_f arguments: account, is_bid, remaining_sizes
    /// * decrease_order_size_f arguments: order_info
    /// * get_order_metadata_bytes: metadata. Should return a representation of order metadata to be used in events
    public fun new_market_clearinghouse_callbacks<M: store + copy + drop>(
        settle_trade_f: |&mut Market<M>, MarketClearinghouseOrderInfo<M>, MarketClearinghouseOrderInfo<M>, u64| SettleTradeResult has drop + copy,
        validate_order_placement_f: |MarketClearinghouseOrderInfo<M>, bool, TimeInForce| bool has drop + copy,
        validate_bulk_order_placement_f: |address, vector<u64>, vector<u64>, vector<u64>, vector<u64>| bool has drop + copy,
        place_maker_order_f: |MarketClearinghouseOrderInfo<M>| has drop + copy,
        cleanup_order_f: |address, OrderIdType, bool, u64, M| has drop + copy,
        cleanup_bulk_orders_f: |address, bool, u64| has drop + copy,
        decrease_order_size_f: |MarketClearinghouseOrderInfo<M>| has drop + copy,
        get_order_metadata_bytes: |M| vector<u8> has drop + copy
    ): MarketClearinghouseCallbacks<M> {
        MarketClearinghouseCallbacks::V1 {
            settle_trade_f,
            validate_order_placement_f,
            validate_bulk_order_placement_f,
            place_maker_order_f,
            cleanup_order_f,
            cleanup_bulk_orders_f,
            decrease_order_size_f,
            get_order_metadata_bytes
        }
    }

    public fun get_settled_size(self: &SettleTradeResult): u64 {
        self.settled_size
    }

    public fun get_settled_price(self: &SettleTradeResult): u64 {
        self.settled_price
    }

    public fun get_maker_cancellation_reason(self: &SettleTradeResult): Option<String> {
        self.maker_cancellation_reason
    }

    public fun get_taker_cancellation_reason(self: &SettleTradeResult): Option<String> {
        self.taker_cancellation_reason
    }

    public fun settle_trade<M: store + copy + drop>(
        self: &MarketClearinghouseCallbacks<M>,
        market: &mut Market<M>,
        taker_order: MarketClearinghouseOrderInfo<M>,
        maker_order: MarketClearinghouseOrderInfo<M>,
        fill_id: u64
    ): SettleTradeResult {
        (self.settle_trade_f)(market, taker_order, maker_order, fill_id)
    }

    public fun validate_order_placement<M: store + copy + drop>(
        self: &MarketClearinghouseCallbacks<M>,
        order: MarketClearinghouseOrderInfo<M>,
        is_taker: bool,
        time_in_force: TimeInForce
    ): bool {
        (self.validate_order_placement_f)(order, is_taker, time_in_force)
    }

    public fun validate_bulk_order_placement<M: store + copy + drop>(
        self: &MarketClearinghouseCallbacks<M>,
        account: address,
        bids_prices: vector<u64>,
        bids_sizes: vector<u64>,
        asks_prices: vector<u64>,
        asks_sizes: vector<u64>): bool {
        (self.validate_bulk_order_placement_f)(account, bids_prices, bids_sizes, asks_prices, asks_sizes)
    }

    public fun place_maker_order<M: store + copy + drop>(
        self: &MarketClearinghouseCallbacks<M>,
        order: MarketClearinghouseOrderInfo<M>) {
        (self.place_maker_order_f)(order)
    }

    public fun cleanup_order<M: store + copy + drop>(
        self: &MarketClearinghouseCallbacks<M>,
        account: address,
        order_id: OrderIdType,
        is_bid: bool,
        remaining_size: u64,
        order_metadata: M) {
        (self.cleanup_order_f)(account, order_id, is_bid, remaining_size, order_metadata)
    }

    public fun cleanup_bulk_orders<M: store + copy + drop>(
        self: &MarketClearinghouseCallbacks<M>,
        account: address,
        is_bid: bool,
        remaining_sizes: u64) {
        (self.cleanup_bulk_orders_f)(account, is_bid, remaining_sizes)
    }

    public fun decrease_order_size<M: store + copy + drop>(
        self: &MarketClearinghouseCallbacks<M>,
        order: MarketClearinghouseOrderInfo<M>,
    ) {
        (self.decrease_order_size_f)(order)
    }

    public fun get_order_metadata_bytes<M: store + copy + drop>(
        self: &MarketClearinghouseCallbacks<M>,
        order_metadata: M): vector<u8> {
        (self.get_order_metadata_bytes)(order_metadata)
    }

    // ============================= Market Types ====================================
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
            order_book: OrderBook<M>,
            /// Pre cancellation tracker for the market, it is wrapped inside a table
            /// as otherwise any insertion/deletion from the tracker would cause conflict
            /// with the order book.
            pre_cancellation_tracker: Table<u8, PreCancellationTracker>,
        }
    }

    enum MarketConfig has store {
        V1 {
            /// Weather to allow self matching orders
            allow_self_trade: bool,
            /// Whether to allow sending all events for the markett
            allow_events_emission: bool,
            /// Pre cancellation window in microseconds
            pre_cancellation_window_secs: u64
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
        price: u64,
        is_bid: bool,
        /// Whether the order crosses the orderbook.
        is_taker: bool,
        status: OrderStatus,
        details: std::string::String,
        metadata_bytes: vector<u8>,
        time_in_force: TimeInForce,
        trigger_condition: Option<TriggerCondition>, // Only emitted with order placement events
    }


    public fun new_market_config(
        allow_self_matching: bool, allow_events_emission: bool, pre_cancellation_window_secs: u64
    ): MarketConfig {
        MarketConfig::V1 {
            allow_self_trade: allow_self_matching,
            allow_events_emission,
            pre_cancellation_window_secs,
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
        Market::V1 {
            parent: signer::address_of(parent),
            market: signer::address_of(market),
            order_id_generator: new_ascending_id_generator(),
            next_fill_id: 0,
            config,
            order_book: new_order_book(),
            pre_cancellation_tracker,
        }
    }


    public fun next_order_id<M: store + copy + drop>(self: &mut Market<M>): OrderIdType {
        new_order_id_type(self.order_id_generator.next_ascending_id())
    }

    public fun next_fill_id<M: store + copy + drop>(self: &mut Market<M>): u64 {
        let next_fill_id = self.next_fill_id;
        self.next_fill_id += 1;
        next_fill_id
    }

    public fun get_order_book<M: store + copy + drop>(self: &Market<M>): &OrderBook<M> {
        &self.order_book
    }

    public fun get_order_book_mut<M: store + copy + drop>(
        self: &mut Market<M>
    ): &mut OrderBook<M> {
        &mut self.order_book
    }

    public fun get_market_address<M: store + copy + drop>(self: &Market<M>): address {
        self.market
    }

    public fun get_pre_cancellation_tracker_mut<M: store + copy + drop>(
        self: &mut Market<M>
    ): &mut PreCancellationTracker {
        self.pre_cancellation_tracker.borrow_mut(PRE_CANCELLATION_TRACKER_KEY)
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
        self: &Market<M>, order_id: OrderIdType
    ): u64 {
        self.order_book.get_remaining_size(order_id)
    }

    public fun get_bulk_order_remaining_size<M: store + copy + drop>(
        self: &Market<M>, user: address, is_bid: bool
    ): u64 {
        self.order_book.get_bulk_order_remaining_size(user, is_bid)
    }

    public fun get_order_metadata<M: store + copy + drop>(
        self: &Market<M>, order_id: OrderIdType
    ): Option<M> {
        self.order_book.get_order_metadata(order_id)
    }

    /// Returns the order metadata for an order by order id.
    /// It is up to the caller to perform necessary permissions checks
    public fun set_order_metadata<M: store + copy + drop>(
        self: &mut Market<M>, order_id: OrderIdType, metadata: M
    ) {
        self.order_book.set_order_metadata(order_id, metadata);
    }

    public fun get_order_metadata_by_client_id<M: store + copy + drop>(
        self: &Market<M>, user: address, client_order_id: u64
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
        self: &mut Market<M>, user: address, client_order_id: u64, metadata: M
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

    public fun emit_event_for_order<M: store + copy + drop>(
        self: &Market<M>,
        order_id: OrderIdType,
        client_order_id: Option<u64>,
        user: address,
        orig_size: u64,
        remaining_size: u64,
        size_delta: u64,
        price: u64,
        is_bid: bool,
        is_taker: bool,
        status: OrderStatus,
        details: String,
        metadata: Option<M>,
        trigger_condition: Option<TriggerCondition>,
        time_in_force: TimeInForce,
        callbacks: &MarketClearinghouseCallbacks<M>
    ) {
        // Final check whether event sending is enabled
        if (self.config.allow_events_emission) {
            let metadata_bytes = if (metadata.is_some()) {
                callbacks.get_order_metadata_bytes(metadata.destroy_some())
            } else {
                vector::empty()
            };
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
                    is_bid,
                    is_taker,
                    status,
                    details,
                    metadata_bytes,
                    time_in_force,
                    trigger_condition
                }
            );
        };
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
            order_book,
            pre_cancellation_tracker,
        } = self;
        let MarketConfig::V1 { allow_self_trade: _, allow_events_emission: _, pre_cancellation_window_secs: _ } = config;
        destroy_tracker(pre_cancellation_tracker.remove(PRE_CANCELLATION_TRACKER_KEY));
        pre_cancellation_tracker.drop_unchecked();
        order_book.destroy_order_book()
    }

    #[test_only]
    public fun get_order_id_from_event(self: OrderEvent): OrderIdType {
        new_order_id_type(self.order_id)
    }

    #[test_only]
    public fun get_client_order_id_from_event(self: OrderEvent): Option<u64> {
        self.client_order_id
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

}

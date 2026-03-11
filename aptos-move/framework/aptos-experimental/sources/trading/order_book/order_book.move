module aptos_experimental::order_book {
    friend aptos_experimental::order_placement;
    friend aptos_experimental::order_operations;
    friend aptos_experimental::market_types;
    friend aptos_experimental::market_bulk_order;
    friend aptos_experimental::dead_mans_switch_operations;
    #[test_only]
    friend aptos_experimental::order_book_client_order_id;

    use std::option::Option;
    use std::string::String;
    use aptos_trading::order_book_types::{OrderId, TriggerCondition};
    use aptos_trading::bulk_order_types::{
        BulkOrder,
        BulkOrderRequest,
        BulkOrderPlaceResponse
    };
    use aptos_trading::order_match_types::{OrderMatch, OrderMatchDetails};
    use aptos_trading::single_order_types::{SingleOrder, SingleOrderRequest};
    use aptos_experimental::bulk_order_book::{BulkOrderBook, new_bulk_order_book};
    use aptos_experimental::single_order_book::{SingleOrderBook, new_single_order_book};
    use aptos_experimental::price_time_index::{
        PriceTimeIndex, new_price_time_idx, new_native_price_time_idx,
        native_timing_start, native_timing_end,
    };

    const E_REINSERT_ORDER_MISMATCH: u64 = 8;
    const E_NATIVE_ORDER_BOOK_NOT_ENABLED: u64 = 20;
    const E_ALREADY_NATIVE: u64 = 21;
    const E_ORDER_BOOK_VERSION_EXISTS: u64 = 22;

    /// Version handle for the native PriceTimeIndex overlay.
    /// Written to MVHashMap at flush, creating Block-STM dependency.
    struct OrderBookVersion has key {
        handle: u64
    }

    enum OrderBook<M: store + copy + drop> has store {
        UnifiedV1 {
            single_order_book: SingleOrderBook<M>,
            bulk_order_book: BulkOrderBook<M>,
            price_time_idx: PriceTimeIndex
        }
    }

    public fun new_order_book<M: store + copy + drop>(): OrderBook<M> {
        OrderBook::UnifiedV1 {
            single_order_book: new_single_order_book(),
            bulk_order_book: new_bulk_order_book(),
            price_time_idx: new_price_time_idx()
        }
    }

    /// Creates a new order book with native Rust-backed PriceTimeIndex.
    /// The native index lives in validator memory as BTreeMap overlays.
    /// `market_addr` identifies this market in the native layer.
    /// Requires the NATIVE_ORDER_BOOK feature flag to be enabled.
    ///
    /// Callers must bracket operations with:
    ///   ensure_native_index_ready() ... maybe_flush_handle()
    public fun new_native_order_book<M: store + copy + drop>(
        market_signer: &signer,
        market_addr: address
    ): OrderBook<M> {
        assert!(
            std::features::is_native_order_book_enabled(),
            E_NATIVE_ORDER_BOOK_NOT_ENABLED
        );
        assert!(
            !exists<OrderBookVersion>(market_addr),
            E_ORDER_BOOK_VERSION_EXISTS
        );
        // Create the OrderBookVersion resource for Block-STM handle tracking
        move_to(market_signer, OrderBookVersion { handle: 0 });
        OrderBook::UnifiedV1 {
            single_order_book: new_single_order_book(),
            bulk_order_book: new_bulk_order_book(),
            price_time_idx: new_native_price_time_idx(market_addr)
        }
    }

    /// Migrate an existing V1 order book to NativeV2.
    /// The V1 PriceTimeIndex BigOrderedMaps are destroyed (data is redundant with
    /// SingleOrderBook + BulkOrderBook orders). The native index will be rebuilt
    /// from those orders on first access via cold start.
    /// Requires the NATIVE_ORDER_BOOK feature flag to be enabled.
    public fun migrate_to_native<M: store + copy + drop>(
        self: &mut OrderBook<M>,
        market_signer: &signer,
        market_addr: address
    ) {
        assert!(
            std::features::is_native_order_book_enabled(),
            E_NATIVE_ORDER_BOOK_NOT_ENABLED
        );
        // Ensure not already native
        assert!(
            self.price_time_idx.get_native_market_addr().is_none(),
            E_ALREADY_NATIVE
        );
        assert!(
            !exists<OrderBookVersion>(market_addr),
            E_ORDER_BOOK_VERSION_EXISTS
        );

        // Swap the PriceTimeIndex: destroy V1 BigOrderedMaps, replace with NativeV2
        let old_idx = std::mem::replace(
            &mut self.price_time_idx,
            new_native_price_time_idx(market_addr)
        );
        old_idx.destroy_v1_for_migration();

        // Create the OrderBookVersion resource
        move_to(market_signer, OrderBookVersion { handle: 0 });
    }

    // ============================= Native lifecycle ====================================

    /// Acquire the native overlay for this market. Must be called before any
    /// OrderBook operation when using NativeV2. No-op for V1.
    ///
    /// If cold start (validator restart), triggers a rebuild from on-chain orders.
    public fun ensure_native_index_ready<M: store + copy + drop>(
        self: &OrderBook<M>
    ) acquires OrderBookVersion {
        let addr_opt = self.price_time_idx.get_native_market_addr();
        if (addr_opt.is_some()) {
            let market_addr = addr_opt.destroy_some();
            // Fast path: skip borrow_global if overlay already acquired in this TX.
            if (native_is_acquired(market_addr)) {
                return;
            };
            let handle = borrow_global<OrderBookVersion>(market_addr).handle;
            let needs_rebuild = native_ensure_acquired(market_addr, handle);
            if (needs_rebuild) {
                rebuild_native_index(self, market_addr);
                native_rebuild_complete(market_addr);
            };
        };
    }

    /// Flush the native overlay if modified. Bumps the handle in OrderBookVersion,
    /// creating the MVHashMap WRITE for Block-STM conflict detection.
    /// Called once per market at the end of each entry point.
    public fun maybe_flush_handle<M: store + copy + drop>(
        self: &OrderBook<M>
    ) acquires OrderBookVersion {
        let addr_opt = self.get_native_market_addr();
        if (addr_opt.is_some()) {
            let market_addr = addr_opt.destroy_some();
            let ver = borrow_global_mut<OrderBookVersion>(market_addr);
            let new_handle = ver.handle + 1;
            let modified = native_flush(market_addr, new_handle);
            if (modified) {
                ver.handle = new_handle;
            };
        };
    }

    /// Returns the market address if this is a NativeV2 order book, None otherwise.
    fun get_native_market_addr<M: store + copy + drop>(self: &OrderBook<M>): Option<address> {
        self.price_time_idx.get_native_market_addr()
    }

    /// Rebuild the native index from all active orders (cold start).
    fun rebuild_native_index<M: store + copy + drop>(
        self: &OrderBook<M>, market_addr: address
    ) {
        self.single_order_book.rebuild_native_index(market_addr);
        self.bulk_order_book.rebuild_native_index(market_addr);
    }

    // ============================= APIs relevant to single order only ====================================

    public fun client_order_id_exists<M: store + copy + drop>(
        self: &OrderBook<M>, order_creator: address, client_order_id: String
    ): bool {
        let _ob_t = native_timing_start();
        self.ensure_native_index_ready();
        let _r = self.single_order_book.client_order_id_exists(order_creator, client_order_id);
        native_timing_end(24, &mut _ob_t);
        _r
    }

    public fun get_single_order_metadata<M: store + copy + drop>(
        self: &OrderBook<M>, order_id: OrderId
    ): Option<M> {
        let _ob_t = native_timing_start();
        self.ensure_native_index_ready();
        let _r = self.single_order_book.get_order_metadata(order_id);
        native_timing_end(25, &mut _ob_t);
        _r
    }

    public fun get_order_id_by_client_id<M: store + copy + drop>(
        self: &OrderBook<M>, order_creator: address, client_order_id: String
    ): Option<OrderId> {
        let _ob_t = native_timing_start();
        self.ensure_native_index_ready();
        let _r = self.single_order_book.get_order_id_by_client_id(order_creator, client_order_id);
        native_timing_end(27, &mut _ob_t);
        _r
    }

    public fun get_single_order<M: store + copy + drop>(
        self: &OrderBook<M>, order_id: OrderId
    ): Option<aptos_trading::single_order_types::OrderWithState<M>> {
        let _ob_t = native_timing_start();
        self.ensure_native_index_ready();
        let _r = self.single_order_book.get_order(order_id);
        native_timing_end(23, &mut _ob_t);
        _r
    }

    public fun get_single_remaining_size<M: store + copy + drop>(
        self: &OrderBook<M>, order_id: OrderId
    ): u64 {
        self.ensure_native_index_ready();
        self.single_order_book.get_remaining_size(order_id)
    }

    public fun cancel_single_order<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_creator: address, order_id: OrderId
    ): SingleOrder<M> {
        self.ensure_native_index_ready();
        let _t = native_timing_start();
        let result = self.single_order_book.cancel_order(
            &mut self.price_time_idx, order_creator, order_id
        );
        native_timing_end(11, &mut _t);
        result
    }

    public fun try_cancel_single_order<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_creator: address, order_id: OrderId
    ): Option<SingleOrder<M>> {
        let _ob_t = native_timing_start();
        self.ensure_native_index_ready();
        self.single_order_book.try_cancel_order(
            &mut self.price_time_idx, order_creator, order_id
        )
    }

    public fun try_cancel_single_order_with_client_order_id<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_creator: address, client_order_id: String
    ): Option<SingleOrder<M>> {
        let _ob_t = native_timing_start();
        self.ensure_native_index_ready();
        self.single_order_book.try_cancel_order_with_client_order_id(
            &mut self.price_time_idx, order_creator, client_order_id
        )
    }

    public fun place_maker_order<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_req: SingleOrderRequest<M>
    ) {
        self.ensure_native_index_ready();
        let _t = native_timing_start();
        self.single_order_book.place_maker_or_pending_order(
            &mut self.price_time_idx, order_req
        );
        native_timing_end(10, &mut _t);
    }

    public fun decrease_single_order_size<M: store + copy + drop>(
        self: &mut OrderBook<M>,
        order_creator: address,
        order_id: OrderId,
        size_delta: u64
    ) {
        let _ob_t = native_timing_start();
        self.ensure_native_index_ready();
        self.single_order_book.decrease_order_size(
            &mut self.price_time_idx,
            order_creator,
            order_id,
            size_delta
        )
    }

    public fun set_single_order_metadata<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_id: OrderId, metadata: M
    ) {
        let _ob_t = native_timing_start();
        self.ensure_native_index_ready();
        self.single_order_book.set_order_metadata(order_id, metadata)
    }

    public fun take_ready_price_based_orders<M: store + copy + drop>(
        self: &mut OrderBook<M>, oracle_price: u64, order_limit: u64
    ): vector<SingleOrder<M>> {
        let _ob_t = native_timing_start();
        self.ensure_native_index_ready();
        self.single_order_book.take_ready_price_based_orders(oracle_price, order_limit)
    }

    public fun take_ready_time_based_orders<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_limit: u64
    ): vector<SingleOrder<M>> {
        let _ob_t = native_timing_start();
        self.ensure_native_index_ready();
        self.single_order_book.take_ready_time_based_orders(order_limit)
    }

    // ============================= APIs relevant to both single and bulk order ====================================

    public fun best_bid_price<M: store + copy + drop>(
        self: &OrderBook<M>
    ): Option<u64> {
        let _ob_t = native_timing_start();
        self.ensure_native_index_ready();
        let _r = self.price_time_idx.best_bid_price();
        native_timing_end(14, &mut _ob_t);
        _r
    }

    public fun best_ask_price<M: store + copy + drop>(
        self: &OrderBook<M>
    ): Option<u64> {
        let _ob_t = native_timing_start();
        self.ensure_native_index_ready();
        let _r = self.price_time_idx.best_ask_price();
        native_timing_end(15, &mut _ob_t);
        _r
    }

    public fun get_slippage_price<M: store + copy + drop>(
        self: &OrderBook<M>, is_bid: bool, slippage_bps: u64
    ): Option<u64> {
        self.ensure_native_index_ready();
        self.price_time_idx.get_slippage_price(is_bid, slippage_bps)
    }

    /// Checks if the order is a taker order i.e., matched immediately with the active order book.
    public fun is_taker_order<M: store + copy + drop>(
        self: &OrderBook<M>,
        price: u64,
        is_bid: bool,
        trigger_condition: Option<TriggerCondition>
    ): bool {
        self.ensure_native_index_ready();
        let _t = native_timing_start();
        let result = if (trigger_condition.is_some()) {
            false
        } else {
            self.price_time_idx.is_taker_order(price, is_bid)
        };
        native_timing_end(13, &mut _t);
        result
    }

    public fun get_single_match_for_taker<M: store + copy + drop>(
        self: &mut OrderBook<M>,
        price: u64,
        size: u64,
        is_bid: bool
    ): OrderMatch<M> {
        self.ensure_native_index_ready();
        let _t = native_timing_start();
        let result = self.price_time_idx.get_single_match_result(price, size, is_bid);
        let match_result = if (result.is_active_matched_book_type_single_order()) {
            self.single_order_book.get_single_match_for_taker(result)
        } else {
            self.bulk_order_book.get_single_match_for_taker(
                &mut self.price_time_idx, result, is_bid
            )
        };
        native_timing_end(12, &mut _t);
        match_result
    }

    public fun reinsert_order<M: store + copy + drop>(
        self: &mut OrderBook<M>,
        reinsert_order: OrderMatchDetails<M>,
        original_order: &OrderMatchDetails<M>
    ) {
        let _ob_t = native_timing_start();
        self.ensure_native_index_ready();
        if (reinsert_order.is_single_order_from_match_details()) {
            self.single_order_book.reinsert_order(
                &mut self.price_time_idx, reinsert_order, original_order
            )
        } else {
            self.bulk_order_book.reinsert_order(
                &mut self.price_time_idx, reinsert_order, original_order
            );
        }
    }

    // ============================= APIs relevant to bulk order only ====================================

    public fun get_bulk_order_remaining_size<M: store + copy + drop>(
        self: &OrderBook<M>, order_creator: address, is_bid: bool
    ): u64 {
        self.ensure_native_index_ready();
        self.bulk_order_book.get_remaining_size(order_creator, is_bid)
    }

    public fun place_bulk_order<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_req: BulkOrderRequest<M>
    ): BulkOrderPlaceResponse<M> {
        self.ensure_native_index_ready();
        let _t = native_timing_start();
        let result = self.bulk_order_book.place_bulk_order(&mut self.price_time_idx, order_req);
        native_timing_end(16, &mut _t);
        result
    }

    public fun get_bulk_order<M: store + copy + drop>(
        self: &OrderBook<M>, order_creator: address
    ): BulkOrder<M> {
        let _ob_t = native_timing_start();
        self.ensure_native_index_ready();
        let _r = self.bulk_order_book.get_bulk_order(order_creator);
        native_timing_end(21, &mut _ob_t);
        _r
    }

    public fun cancel_bulk_order<M: store + copy + drop>(
        self: &mut OrderBook<M>, order_creator: address
    ): BulkOrder<M> {
        self.ensure_native_index_ready();
        self.bulk_order_book.cancel_bulk_order(&mut self.price_time_idx, order_creator)
    }

    public fun cancel_bulk_order_at_price<M: store + copy + drop>(
        self: &mut OrderBook<M>,
        order_creator: address,
        price: u64,
        is_bid: bool
    ): (u64, BulkOrder<M>) {
        let _ob_t = native_timing_start();
        self.ensure_native_index_ready();
        self.bulk_order_book.cancel_bulk_order_at_price(
            &mut self.price_time_idx,
            order_creator,
            price,
            is_bid
        )
    }

    // ============================= Native function declarations ====================================

    // Lifecycle natives — called from ensure_native_index_ready / maybe_flush_handle
    native fun native_is_acquired(market_addr: address): bool;
    native fun native_ensure_acquired(market_addr: address, handle: u64): bool;
    native fun native_flush(market_addr: address, new_handle: u64): bool;
    native fun native_rebuild_complete(market_addr: address);

    // ============================= test_only APIs ====================================
    #[test_only]
    public fun destroy_order_book<M: store + copy + drop>(self: OrderBook<M>) {
        let OrderBook::UnifiedV1 {
            single_order_book: retail_order_book,
            bulk_order_book,
            price_time_idx
        } = self;
        bulk_order_book.destroy_bulk_order_book();
        retail_order_book.destroy_single_order_book();
        price_time_idx.destroy_price_time_idx();
    }

    #[test_only]
    public fun set_up_test_with_id(): OrderBook<u64> {
        aptos_framework::timestamp::set_time_has_started_for_testing(
            &aptos_framework::account::create_signer_for_test(@0x1)
        );
        new_order_book()
    }
}

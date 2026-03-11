/// Native order book benchmark module. Identical to order_book_example but uses the native
/// Rust-backed PriceTimeIndex overlay for O(log n) in-memory operations instead of
/// BigOrderedMap B+Tree (which requires storage reads per node).
///
/// Requires the NATIVE_ORDER_BOOK feature flag to be enabled.
module 0xABCD::native_order_book_example {
    use std::signer;
    use std::error;
    use std::option;
    use std::vector;
    use std::table::{Self, Table};
    use aptos_trading::order_book_types::{OrderId, good_till_cancelled};
    use aptos_trading::order_match_types::OrderMatch;
    use aptos_experimental::order_book;
    use aptos_experimental::order_book::OrderBook;

    const ENOT_AUTHORIZED: u64 = 1;
    const EDEX_RESOURCE_NOT_PRESENT: u64 = 2;

    struct Empty has store, copy, drop {}

    struct NativeDex has key {
        order_books: Table<u32, OrderBook<Empty>>,
    }

    /// Create the global NativeDex with native order books.
    /// Each market gets its own resource account for OrderBookVersion.
    /// Skips initialization if NATIVE_ORDER_BOOK feature flag is not enabled.
    fun init_module(publisher: &signer) {
        assert!(
            signer::address_of(publisher) == @publisher_address,
            ENOT_AUTHORIZED,
        );

        // Skip if native order book feature is not enabled
        if (!std::features::is_native_order_book_enabled()) {
            return;
        };

        let order_books = table::new();
        for (i in 0..10) {
            // Each market needs its own address for OrderBookVersion.
            let seed = std::bcs::to_bytes(&i);
            let (market_signer, _cap) = aptos_framework::account::create_resource_account(
                publisher, seed
            );
            let market_addr = signer::address_of(&market_signer);
            order_books.add(i, order_book::new_native_order_book(&market_signer, market_addr));
        };

        move_to(publisher, NativeDex { order_books });
    }

    inline fun borrow_order_book_mut(market_id: u32): &mut OrderBook<Empty> {
        assert!(exists<NativeDex>(@publisher_address), error::invalid_argument(EDEX_RESOURCE_NOT_PRESENT));
        let dex = borrow_global_mut<NativeDex>(@publisher_address);
        // Clamp to pre-created markets (0..10)
        let clamped_id = market_id % 10;
        dex.order_books.borrow_mut(clamped_id)
    }

    inline fun borrow_order_book(market_id: u32): &OrderBook<Empty> {
        assert!(exists<NativeDex>(@publisher_address), error::invalid_argument(EDEX_RESOURCE_NOT_PRESENT));
        let dex = borrow_global<NativeDex>(@publisher_address);
        let clamped_id = market_id % 10;
        dex.order_books.borrow(clamped_id)
    }

    public entry fun place_order(market_id: u32, sender: address, order_id: u64, bid_price: u64, volume: u64, is_bid: bool) acquires NativeDex {
        if (!exists<NativeDex>(@publisher_address)) { return; };
        borrow_order_book(market_id).ensure_native_index_ready();
        let order_book = borrow_order_book_mut(market_id);
        place_order_and_get_matches(
            order_book,
            sender,
            aptos_trading::order_book_types::new_order_id_type(order_id as u128),
            bid_price,
            volume,
            volume,
            is_bid,
        );
        borrow_order_book(market_id).maybe_flush_handle();
    }

    public entry fun cancel_order(market_id: u32, order_id: u64) acquires NativeDex {
        if (!exists<NativeDex>(@publisher_address)) { return; };
        borrow_order_book(market_id).ensure_native_index_ready();
        let order_book = borrow_order_book_mut(market_id);
        order_book.cancel_single_order(
            @publisher_address,
            aptos_trading::order_book_types::new_order_id_type(order_id as u128)
        );
        borrow_order_book(market_id).maybe_flush_handle();
    }

    fun place_order_and_get_matches(
        order_book: &mut OrderBook<Empty>,
        account: address,
        order_id: OrderId,
        price: u64,
        orig_size: u64,
        remaining_size: u64,
        is_bid: bool,
    ): vector<OrderMatch<Empty>> {
        let trigger_condition = option::none();
        let match_results = vector::empty();
        while (remaining_size > 0) {
            if (!order_book.is_taker_order(price, is_bid, trigger_condition)) {
                order_book.place_maker_order(
                    aptos_trading::single_order_types::new_single_order_request(
                        account,
                        order_id,
                        option::none(),
                        price,
                        orig_size,
                        remaining_size,
                        is_bid,
                        trigger_condition,
                        good_till_cancelled(),
                        Empty {},
                    )
                );
                return match_results;
            };
            let match_result =
                order_book.get_single_match_for_taker(price, remaining_size, is_bid);
            let matched_size = match_result.get_matched_size();
            match_results.push_back(match_result);
            remaining_size -= matched_size;
        };
        return match_results
    }
}

module 0xABCD::order_book_example {
    use std::signer;
    use std::error;
    use std::option;

    use aptos_experimental::active_order_book::{Self, ActiveOrderBook};
    use aptos_experimental::order_book::{Self, OrderBook};
    use aptos_experimental::order_book_types;

    const ENOT_AUTHORIZED: u64 = 1;
    // Resource being modified doesn't exist
    const EDEX_RESOURCE_NOT_PRESENT: u64 = 2;

    struct Empty has store, copy, drop {}

    struct ActiveOnly has key {
        active_only: ActiveOrderBook,
    }

    struct Dex has key {
        order_book: OrderBook<Empty>,
    }

    // Create the global `Dex`.
    // Stored under the module publisher address.
    fun init_module(publisher: &signer) {
        assert!(
            signer::address_of(publisher) == @publisher_address,
            ENOT_AUTHORIZED,
        );

        move_to(
            publisher,
            ActiveOnly { active_only: active_order_book::new_active_order_book() }
        );

        move_to(
            publisher,
            Dex { order_book: order_book::new_order_book() }
        );
    }

    public entry fun place_active_post_only_order(sender: address, account_order_id: u64, bid_price: u64, volume: u64, is_buy: bool) acquires ActiveOnly {
        assert!(exists<ActiveOnly>(@publisher_address), error::invalid_argument(EDEX_RESOURCE_NOT_PRESENT));
        let active_only = borrow_global_mut<ActiveOnly>(@publisher_address);

        let order_id = order_book_types::new_order_id_type(sender, account_order_id);
        // TODO change from random to monothonically increasing value
        let unique_priority_idx = order_book_types::generate_unique_idx_fifo_tiebraker();

        active_only.active_only.place_maker_order(
            order_id,
            bid_price,
            unique_priority_idx,
            volume,
            is_buy
        );
    }

    public entry fun place_order(sender: address, account_order_id: u64, bid_price: u64, volume: u64, is_buy: bool) acquires Dex {
        assert!(exists<Dex>(@publisher_address), error::invalid_argument(EDEX_RESOURCE_NOT_PRESENT));
        let dex = borrow_global_mut<Dex>(@publisher_address);
        dex.order_book.place_order_and_get_matches(
            order_book::new_order_request(
                sender, // account
                account_order_id,
                bid_price,
                volume,
                volume,
                is_buy,
                option::none(), // trigger_condition
                Empty {}, //metadata
            )
        );
    }

    public entry fun cancel_order(account_order_id: u64) acquires Dex {
        assert!(exists<Dex>(@publisher_address), error::invalid_argument(EDEX_RESOURCE_NOT_PRESENT));
        let order_book = borrow_global_mut<Dex>(@publisher_address);
        order_book.order_book.cancel_order(@publisher_address, account_order_id);
    }
}

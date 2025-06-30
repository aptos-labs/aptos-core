module 0xABCD::order_book_example {
    use std::signer;
    use std::error;
    use std::option;

    use aptos_experimental::order_book::{Self, OrderBook};
    use aptos_experimental::order_book_types;

    const ENOT_AUTHORIZED: u64 = 1;
    // Resource being modified doesn't exist
    const EDEX_RESOURCE_NOT_PRESENT: u64 = 2;

    struct Empty has store, copy, drop {}

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
            Dex { order_book: order_book::new_order_book() }
        );
    }

    public entry fun place_order(sender: address, order_id: u64, bid_price: u64, volume: u64, is_bid: bool) acquires Dex {
        assert!(exists<Dex>(@publisher_address), error::invalid_argument(EDEX_RESOURCE_NOT_PRESENT));
        let dex = borrow_global_mut<Dex>(@publisher_address);
        dex.order_book.place_order_and_get_matches(
            order_book::new_order_request(
                sender, // account
                order_book_types::new_order_id_type(order_id as u128),
                bid_price,
                volume,
                volume,
                is_bid,
                option::none(), // trigger_condition
                Empty {}, //metadata
            )
        );
    }

    public entry fun cancel_order(order_id: u64) acquires Dex {
        assert!(exists<Dex>(@publisher_address), error::invalid_argument(EDEX_RESOURCE_NOT_PRESENT));
        let order_book = borrow_global_mut<Dex>(@publisher_address);
        order_book.order_book.cancel_order(order_book_types::new_order_id_type(order_id as u128));
    }
}

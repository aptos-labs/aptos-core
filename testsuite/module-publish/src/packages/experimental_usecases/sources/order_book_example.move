module 0xABCD::order_book_example {
    use std::signer;
    use std::error;
    use std::option;
    use std::vector;

    use aptos_experimental::order_book::{Self, OrderBook};
    use aptos_experimental::order_book_types::{Self, OrderIdType};

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
        place_order_and_get_matches(
            &mut dex.order_book,
            sender, // account
            order_book_types::new_order_id_type(order_id as u128),
            bid_price,
            volume,
            volume,
            is_bid,
        );
    }

    public entry fun cancel_order(order_id: u64) acquires Dex {
        assert!(exists<Dex>(@publisher_address), error::invalid_argument(EDEX_RESOURCE_NOT_PRESENT));
        let order_book = borrow_global_mut<Dex>(@publisher_address);
        order_book.order_book.cancel_order(@publisher_address, order_book_types::new_order_id_type(order_id as u128));
    }

    // Copied from order_book, as it's test_only and not part of public API there.
    public fun place_order_and_get_matches(
        order_book: &mut OrderBook<Empty>,
        account: address,
        order_id: OrderIdType,
        price: u64,
        orig_size: u64,
        remaining_size: u64,
        is_bid: bool,
    ): vector<order_book_types::SingleOrderMatch<Empty>> {
        let trigger_condition = option::none();
        let match_results = vector::empty();
        while (remaining_size > 0) {
            if (!order_book.is_taker_order(option::some(price), is_bid, trigger_condition)) {
                order_book.place_maker_order(
                    order_book::new_order_request(
                        account,
                        order_id,
                        option::none(),
                        price,
                        orig_size,
                        remaining_size,
                        is_bid,
                        trigger_condition, // trigger_condition
                        Empty {}, // metadata
                    )
                );
                return match_results;
            };
            let match_result =
                order_book.get_single_match_for_taker(
                    option::some(price), remaining_size, is_bid
                );
            let matched_size = match_result.get_matched_size();
            match_results.push_back(match_result);
            remaining_size -= matched_size;
        };
        return match_results
    }
}

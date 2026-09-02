spec aptos_experimental::pending_order_book_index {

    spec new_pending_order_book_index()
        : 0x7::pending_order_book_index::PendingOrderBookIndex {
        use 0x5::order_book_types;
        use 0x7::order_book_utils;
        pragma opaque = true, aborts_if_is_partial = false;
        ensures [inferred]({
            let a =
                S1..S2 |~ result_of<order_book_utils::new_default_big_ordered_map<PendingDownOrderKey, order_book_types::OrderId
                    >> ();
            let b =
                ..S1 |~ result_of<order_book_utils::new_default_big_ordered_map<PendingUpOrderKey, order_book_types::OrderId
                    >> ();
            let c =
                S2.. |~ result_of<order_book_utils::new_default_big_ordered_map<PendingTimeKey, order_book_types::OrderId
                    >> ();
            result
                == PendingOrderBookIndex::V1 {
                    price_move_down_index: a,
                    price_move_up_index: b,
                    time_based_index: c
                }
        });
        aborts_if false;
    }
}

spec aptos_experimental::bulk_order_book {
    use aptos_framework::big_ordered_map;
    use aptos_trading::bulk_order_types;

    spec get_remaining_size<M: store + copy + drop>(
        self: &BulkOrderBook<M>, account: address, is_bid: bool
    ): u64 {
        pragma opaque;
        let sizes = bulk_order_types::spec_side_sizes(
            big_ordered_map::spec_get(self.orders, account).order_request, is_bid
        );
        aborts_if !big_ordered_map::spec_contains_key(self.orders, account);
        aborts_if bulk_order_types::spec_sum_from(sizes, 0) > MAX_U64;
        ensures result == bulk_order_types::spec_sum_from(sizes, 0);
    }
}

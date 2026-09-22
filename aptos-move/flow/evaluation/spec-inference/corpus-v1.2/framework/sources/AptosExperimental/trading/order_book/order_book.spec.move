spec aptos_experimental::order_book {
    use aptos_framework::big_ordered_map;
    use aptos_trading::order_book_types::AccountClientOrderId;

    spec client_order_id_exists<M: store + copy + drop>(
        self: &OrderBook<M>, order_creator: address, client_order_id: String
    ): bool {
        pragma opaque;
        aborts_if false;
        ensures result == big_ordered_map::spec_contains_key(
            self.single_order_book.client_order_ids,
            AccountClientOrderId { account: order_creator, client_order_id }
        );
    }
}

spec aptos_experimental::pending_order_book_index {
    use aptos_framework::big_ordered_map;

    /// Orders already collected are kept in place; at most `limit` are held
    /// afterwards unless more were held before; the index only shrinks. The
    /// front is borrowed only while the index is non-empty and removed under
    /// its own key, so nothing aborts.
    spec take_ready_price_move_up_orders(
        self: &mut PendingOrderBookIndex,
        current_price: u64,
        orders: &mut vector<OrderId>,
        limit: u64
    ) {
        pragma opaque;
        aborts_if false;
        ensures len(orders) >= old(len(orders));
        ensures len(orders) <= old(len(orders)) || len(orders) <= limit;
        ensures forall i in 0..old(len(orders)): orders[i] == old(orders)[i];
        ensures big_ordered_map::spec_len(self.price_move_up_index)
            <= old(big_ordered_map::spec_len(self.price_move_up_index));
    }
}

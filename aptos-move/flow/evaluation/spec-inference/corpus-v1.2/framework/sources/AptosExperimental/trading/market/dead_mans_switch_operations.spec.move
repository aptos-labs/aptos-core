spec aptos_experimental::dead_mans_switch_operations {
    use aptos_framework::big_ordered_map;

    /// The switch must be enabled and the account must hold a bulk order; the
    /// validity check and the cancellation path add aborts of their own.
    spec cleanup_expired_bulk_order<M: store + copy + drop, R: store + copy + drop>(
        market: &mut Market<M>,
        account: address,
        callbacks: &MarketClearinghouseCallbacks<M, R>
    ) {
        pragma aborts_if_is_partial;
        aborts_if !market.config.enable_dead_mans_switch;
        aborts_if !big_ordered_map::spec_contains_key(
            market.order_book.bulk_order_book.orders, account
        );
    }
}

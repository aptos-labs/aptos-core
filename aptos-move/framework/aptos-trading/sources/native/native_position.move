/// Native perp position storage, keyed by `(exchange, market, account)`.
/// Only the private `native_*` write functions are declared here; the
/// public, capability-gated write API is added separately, so this layer
/// imposes no public surface to stay backward-compatible with. Native
/// state is read only on the validator side (Rust), never from Move.
module aptos_trading::native_position {
    use aptos_trading::native_position_types::Position;

    native fun native_set_position(
        exchange: address,
        market: address,
        account: address,
        position: Position,
    );
    native fun native_delete_position(
        exchange: address,
        market: address,
        account: address,
    );
}

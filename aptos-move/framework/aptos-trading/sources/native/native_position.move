/// Native perp position storage, keyed by `(exchange, market, account)`.
/// Only the private `native_*` write functions are declared here; the
/// public, capability-gated write API is added separately, so this layer
/// imposes no public surface to stay backward-compatible with. Native
/// state is read only on the validator side (Rust), never from Move.
module aptos_trading::native_position {
    use aptos_trading::native_position_types::Position;
    use aptos_trading::trading_native_capability::{Self, TradingNativeCapability};

    /// Set the position at `(exchange, market, account)`. `exchange` is
    /// derived from the cap; every call re-checks the cap via
    /// `assert_valid`.
    public fun set_position(
        cap: &TradingNativeCapability,
        market: address,
        account: address,
        position: Position,
    ) {
        trading_native_capability::assert_valid(cap);
        native_set_position(trading_native_capability::exchange(cap), market, account, position);
    }

    /// Delete the position at `(exchange, market, account)`.
    public fun delete_position(
        cap: &TradingNativeCapability,
        market: address,
        account: address,
    ) {
        trading_native_capability::assert_valid(cap);
        native_delete_position(trading_native_capability::exchange(cap), market, account);
    }

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

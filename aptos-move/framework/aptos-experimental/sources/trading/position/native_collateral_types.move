/// Types crossing the `native_collateral` Move ↔ native boundary. Field
/// order/widths here pin the BCS encoding the Rust `NativeCollateral` must
/// match once the primary balance's aggregator snapshot is materialized.
module aptos_experimental::native_collateral_types {
    use aptos_framework::aggregator_v2::{Self, AggregatorSnapshot};

    /// Offset applied to the signed primary balance so it fits a `u64`.
    const SIGN_OFFSET: u64 = 9223372036854775808;

    /// Balance domain of a collateral entry: the account's cross-margin pool,
    /// or the collateral dedicated to one isolated market.
    enum CollateralScope has copy, drop, store {
        Cross,
        Isolated { market: address },
    }

    /// Balance of one secondary collateral asset, in its own units.
    struct SecondaryBalance has copy, drop, store {
        asset: address,
        amount: u64,
    }

    /// `primary` is the signed primary balance offset by 2^63, as a snapshot
    /// so writing it records no read of the aggregator it came from.
    enum Collateral has drop, store {
        V1 {
            primary: AggregatorSnapshot<u64>,
            secondaries: vector<SecondaryBalance>,
            reserved: u64,
        },
    }

    public fun cross(): CollateralScope {
        CollateralScope::Cross
    }

    public fun isolated(market: address): CollateralScope {
        CollateralScope::Isolated { market }
    }

    public fun new_secondary_balance(asset: address, amount: u64): SecondaryBalance {
        SecondaryBalance { asset, amount }
    }

    /// `primary` must hold the balance offset by 2^63.
    public fun new_collateral_v1(
        primary: AggregatorSnapshot<u64>,
        secondaries: vector<SecondaryBalance>,
        reserved: u64,
    ): Collateral {
        Collateral::V1 { primary, secondaries, reserved }
    }

    public fun new_collateral_v1_from_balance(
        primary: i64,
        secondaries: vector<SecondaryBalance>,
        reserved: u64,
    ): Collateral {
        let offset = ((primary as i128) + (SIGN_OFFSET as i128)) as u64;
        new_collateral_v1(aggregator_v2::create_snapshot(offset), secondaries, reserved)
    }
}

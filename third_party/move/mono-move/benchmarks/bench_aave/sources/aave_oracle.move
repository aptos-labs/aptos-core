// Ported from https://github.com/aave/aptos-aave-v3, module
// `aave_oracle::oracle`, reduced to a mock feed.
// Copyright (c) Aave DAO and contributors.
// SPDX-License-Identifier: Apache-2.0

/// Price feed keyed by underlying asset address. Prices are quoted in the
/// protocol base currency with 8 decimals, as upstream does. There is no
/// Chainlink adapter and no staleness check: the benchmark cares about the
/// per-asset read, not about oracle plumbing.
module bench::aave_oracle {
    use std::signer;
    use aptos_std::table::{Self, Table};

    const ENOT_ADMIN: u64 = 20;
    const EALREADY_INITIALIZED: u64 = 21;
    const ENOT_INITIALIZED: u64 = 22;
    const ENO_PRICE: u64 = 23;

    struct PriceFeed has key {
        prices: Table<address, u256>
    }

    public fun initialize(admin: &signer) {
        assert!(signer::address_of(admin) == @bench, ENOT_ADMIN);
        assert!(!exists<PriceFeed>(@bench), EALREADY_INITIALIZED);
        move_to(admin, PriceFeed { prices: table::new() });
    }

    public fun set_price(admin: &signer, asset: address, price: u256) acquires PriceFeed {
        assert!(signer::address_of(admin) == @bench, ENOT_ADMIN);
        assert!(exists<PriceFeed>(@bench), ENOT_INITIALIZED);
        let feed = borrow_global_mut<PriceFeed>(@bench);
        table::upsert(&mut feed.prices, asset, price);
    }

    public fun get_price(asset: address): u256 acquires PriceFeed {
        assert!(exists<PriceFeed>(@bench), ENOT_INITIALIZED);
        let feed = borrow_global<PriceFeed>(@bench);
        assert!(table::contains(&feed.prices, asset), ENO_PRICE);
        *table::borrow(&feed.prices, asset)
    }

    public fun has_price(asset: address): bool acquires PriceFeed {
        exists<PriceFeed>(@bench)
            && table::contains(&borrow_global<PriceFeed>(@bench).prices, asset)
    }
}

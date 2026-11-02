/// Collateral price feed, shaped after Liquity's single-collateral oracle.
///
/// The price walks a deterministic sawtooth over a fixed band. Vaults sort on
/// the nominal collateral ratio, which does not involve price, so a sweep
/// changes which vaults are liquidatable without ever reordering the list.
/// That is what keeps the liquidation branch supplied with candidates.
///
/// # Reference price
///
/// Every ratio target in the package is measured against `reference_price`,
/// mid-band, rather than against the spot price. A target set at the spot
/// price pins the vault to wherever the sawtooth happened to be, and a vault
/// written near the bottom of the band can then never be taken below the
/// minimum again. Anchoring instead puts every target at a fixed nominal
/// ratio, so a vault is safe over the upper part of the band and a candidate
/// over the lower part however often it is rewritten.
module bench::cdp_oracle {
    use std::signer;

    /// Only the package address configures the feed.
    const E_NOT_BENCH: u64 = 1;

    /// Band the sawtooth sweeps, in stable units per collateral unit. A vault
    /// seeded healthy at the top of the band is liquidatable at the bottom.
    const PRICE_MIN: u64 = 1000;
    const PRICE_SPAN: u64 = 2000;

    struct Price has key {
        price: u64,
        /// Distance into the band the sawtooth has travelled.
        offset: u64,
    }

    /// Admin-only. Creates the feed on the first call, which is how the
    /// publisher's setup sequence initializes it. The price is clamped into
    /// the band, which keeps the ratio products in `cdp_vault` inside u64,
    /// and the offset is derived from it, so the next tick carries on from
    /// where the price was set instead of jumping.
    public entry fun set_price(admin: &signer, price: u64) acquires Price {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        let price = clamp_price(price);
        let offset = price - PRICE_MIN;
        if (!exists<Price>(@bench)) {
            move_to(admin, Price { price, offset });
        } else {
            let feed = borrow_global_mut<Price>(@bench);
            feed.price = price;
            feed.offset = offset;
        }
    }

    /// Advance the sawtooth by `step`. `step` is reduced into the band before
    /// it is added, so no caller-supplied value can overflow the offset.
    public entry fun bench_price_tick(
        _user: &signer, step: u64
    ) acquires Price {
        if (!exists<Price>(@bench)) return;
        let feed = borrow_global_mut<Price>(@bench);
        feed.offset = (feed.offset + (step % PRICE_SPAN)) % PRICE_SPAN;
        feed.price = PRICE_MIN + feed.offset;
    }

    /// Mid-band before the publisher has configured the feed, so ratio math
    /// stays well defined in every order of setup.
    public fun price(): u64 acquires Price {
        if (!exists<Price>(@bench)) {
            reference_price()
        } else {
            borrow_global<Price>(@bench).price
        }
    }

    /// The price every ratio target in the package is measured against. It
    /// sits mid-band, so a vault written at a target is a candidate over the
    /// lower half of the band and safe over the upper half.
    public fun reference_price(): u64 { PRICE_MIN + PRICE_SPAN / 2 }

    public fun price_min(): u64 { PRICE_MIN }

    public fun price_max(): u64 { PRICE_MIN + PRICE_SPAN - 1 }

    public fun price_span(): u64 { PRICE_SPAN }

    fun clamp_price(price: u64): u64 {
        if (price < PRICE_MIN) {
            PRICE_MIN
        } else if (price > price_max()) {
            price_max()
        } else {
            price
        }
    }
}

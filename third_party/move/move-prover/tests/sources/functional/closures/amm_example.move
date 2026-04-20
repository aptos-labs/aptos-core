// Copyright © Aptos Foundation
// AMM (Automated Market Maker) example demonstrating closure invariants
// carried by a wrapper struct `PricingStrategy`. The invariants are
// attached to the wrapper, so pack time is the proof obligation point.
// The Pool simply stores a `PricingStrategy`; creators of a pool only
// need to supply a valid pricing strategy. The compiler automatically
// inserts the wrapper when a bare pricing function is passed to
// `create_pool`, so callers do not need to pack it explicitly.
//
// flag: --split-vcs-by-assert

module 0x42::amm {

    // -------------------------------------------------------
    // Pricing strategy wrapper carrying the pool invariants
    // -------------------------------------------------------

    /// Wraps a pricing function with the invariants every AMM pool
    /// expects. Packing a `PricingStrategy` verifies those invariants
    /// against the concrete pricing function; callers of `create_pool`
    /// then only need to supply a valid `PricingStrategy`.
    struct PricingStrategy(|address, u64, u64, u64|u64) has store, copy, drop;

    spec PricingStrategy {
        modifies_of<self.0> *;

        // No-abort: the pricing function must never abort, for any inputs.
        invariant forall S in *, client: address, r_in: u64, r_out: u64, amt: u64:
            S |~ !aborts_of<self.0>(client, r_in, r_out, amt);

        // Safety: output never exceeds the output reserve.
        invariant forall S in *, client: address, r_in: u64, r_out: u64, amt: u64:
            S.. |~ result_of<self.0>(client, r_in, r_out, amt) <= r_out;

        // Monotonicity: more input yields at least as much output.
        invariant forall S in *, client: address, r_in: u64, r_out: u64,
                        a1: u64, a2: u64:
            S.. |~ a1 <= a2 ==>
                result_of<self.0>(client, r_in, r_out, a1)
                    <= result_of<self.0>(client, r_in, r_out, a2);

        // Constant product preservation: the product of reserves never decreases.
        invariant forall S in *, client: address, r_in: u64, r_out: u64, amt: u64:
            S.. |~ (r_in + amt) * (r_out - result_of<self.0>(client, r_in, r_out, amt)) >= r_in * r_out;

    }

    // -------------------------------------------------------
    // Pool
    // -------------------------------------------------------

    /// A liquidity pool holding reserves of two tokens and a pricing strategy.
    struct Pool has key {
        reserve_x: u64,
        reserve_y: u64,
        pricing: PricingStrategy,
    }

    /// Per-client fee in basis points (e.g. 300 = 3%).
    struct Fee has key {
        bps: u64,
    }

    const E_ZERO_AMOUNT: u64 = 1;
    const E_INSUFFICIENT_LIQUIDITY: u64 = 2;

    // -------------------------------------------------------
    // Concrete pricing curves
    // -------------------------------------------------------

    /// Constant-product pricing (Uniswap v2 style):
    ///   amount_out = reserve_out * amount_in / (reserve_in + amount_in)
    /// Guards the degenerate (0, 0) case so the function never aborts,
    /// satisfying the PricingStrategy no-abort invariant.
    public fun constant_product(
        _client: address, reserve_in: u64, reserve_out: u64, amount_in: u64,
    ): u64 {
        if (reserve_in == 0 && amount_in == 0) {
            0
        } else {
            let num = (reserve_out as u128) * (amount_in as u128);
            let den = (reserve_in as u128) + (amount_in as u128);
            ((num / den) as u64)
        }
    }
    spec constant_product {
        pragma opaque;
        // Never aborts.
        aborts_if false;
        // Degenerate empty-pool, zero-input case produces no swap.
        ensures reserve_in == 0 && amount_in == 0 ==> result == 0;
        // Otherwise result follows the constant-product formula (integer division).
        ensures !(reserve_in == 0 && amount_in == 0)
            ==> result == reserve_out * amount_in / (reserve_in + amount_in);
        // Output never exceeds the output reserve.
        ensures result <= reserve_out;
        // Strict sub-reserve: when both reserves and input are positive,
        // the pool can never be fully drained by a single swap.
        ensures reserve_in > 0 && reserve_out > 0 && amount_in > 0
            ==> result < reserve_out;
    }


    /// Default fee (500 bps = 5%) used when the client has no `Fee` resource
    /// or the stored value is out of range.
    const DEFAULT_FEE_BPS: u64 = 500;

    /// Constant-product pricing with a per-client fee (compliant variant).
    /// Uses `Fee[client].bps` when the client has a valid `Fee` resource,
    /// otherwise falls back to `DEFAULT_FEE_BPS`. Guards the division so
    /// the function never aborts, satisfying the PricingStrategy invariants.
    public fun constant_product_with_fee(
        client: address, reserve_in: u64, reserve_out: u64, amount_in: u64,
    ): u64 {
        let fee_bps = if (exists<Fee>(client) && Fee[client].bps <= 10000) {
            Fee[client].bps
        } else {
            DEFAULT_FEE_BPS
        };
        let effective_in = (amount_in as u128) * (10000 - fee_bps as u128) / 10000;
        if (effective_in == 0) {
            0
        } else {
            let num = (reserve_out as u128) * effective_in;
            let den = (reserve_in as u128) + effective_in;
            ((num / den) as u64)
        }
    }
    spec constant_product_with_fee {
        pragma opaque;
        // Never aborts: the default covers missing/invalid fee configuration,
        // and the effective-input check avoids division by zero.
        aborts_if false;
        let fee_bps = if (exists<Fee>(client) && Fee[client].bps <= 10000) {
            Fee[client].bps
        } else {
            DEFAULT_FEE_BPS
        };
        let effective_in = amount_in * (10000 - fee_bps) / 10000;
        // Zero effective input (small amount fully consumed by the fee)
        // produces no swap.
        ensures effective_in == 0
            ==> result == 0;
        // Otherwise apply the constant-product formula to the fee-adjusted input.
        ensures effective_in > 0
            ==> result == reserve_out * effective_in / (reserve_in + effective_in);
        // Output never exceeds the output reserve.
        ensures result <= reserve_out;
        // Constant-product preservation: the PricingStrategy invariant, stated
        // directly so callers of `create_pool` do not need to re-derive it from
        // the fee-adjusted input formula.
        ensures (reserve_in + amount_in) * (reserve_out - result)
            >= reserve_in * reserve_out;
    } proof {
        // Help the solver discharge the nonlinear ensures above by splitting
        // on whether the fee has consumed all of amount_in. Each variant
        // presents a linear obligation to the solver.
        split effective_in == 0;
    }

    /// Constant-product pricing with a per-client fee (non-compliant variant).
    /// Reads `Fee[client].bps` directly and aborts when the client has no
    /// `Fee` resource — violating the PricingStrategy no-abort invariant.
    public fun constant_product_with_fee_non_compliant(
        client: address, reserve_in: u64, reserve_out: u64, amount_in: u64,
    ): u64 {
        let fee_bps = Fee[client].bps;
        let effective_in = (amount_in as u128) * (10000 - fee_bps as u128) / 10000;
        let num = (reserve_out as u128) * effective_in;
        let den = (reserve_in as u128) + effective_in;
        ((num / den) as u64)
    }
    spec constant_product_with_fee_non_compliant {
        pragma opaque;
        // NON-COMPLIANT: Aborts if client has no fee configuration.
        aborts_if !exists<Fee>(client);
        // NON-COMPLIANT: Aborts if fee exceeds 100% (10000 basis points).
        aborts_if Fee[client].bps > 10000;
        let fee_bps = Fee[client].bps;
        let effective_in = amount_in * (10000 - fee_bps) / 10000;
        // Aborts on division by zero when reserve and effective input are both zero.
        aborts_if reserve_in + effective_in == 0;
        // Result follows the constant-product formula applied to the
        // fee-adjusted input (the fee reduces the effective swap amount).
        ensures result == reserve_out * effective_in / (reserve_in + effective_in);
        // Output never exceeds the output reserve.
        ensures result <= reserve_out;
    }

    // -------------------------------------------------------
    // Pool operations
    // -------------------------------------------------------

    /// Create a pool with a given pricing strategy. Because the
    /// `PricingStrategy` invariants were already discharged at pack time,
    /// `create_pool` itself has no further preconditions on `pricing`.
    public fun create_pool(
        account: &signer,
        reserve_x: u64,
        reserve_y: u64,
        pricing: PricingStrategy,
    ) {
        move_to(account, Pool { reserve_x, reserve_y, pricing });
    }

    /// Swap `amount_in` of token X for token Y.
    /// The output is determined by the pool's stored pricing function.
    public fun swap(pool: &mut Pool, client: address, amount_in: u64): u64 {
        assert!(amount_in > 0, E_ZERO_AMOUNT);
        let amount_out = (pool.pricing)(
            client, pool.reserve_x, pool.reserve_y, amount_in,
        );
        assert!(amount_out <= pool.reserve_y, E_INSUFFICIENT_LIQUIDITY);
        pool.reserve_x = pool.reserve_x + amount_in;
        pool.reserve_y = pool.reserve_y - amount_out;
        amount_out
    }
    spec swap {
        // Aborts if the swap amount is zero.
        aborts_if amount_in == 0;
        // Aborts on reserve overflow. The pricing function itself cannot abort
        // because `amount_in > 0` and the PricingStrategy no-abort invariant.
        aborts_if pool.reserve_x + amount_in > MAX_U64;
        ensures result == result_of<pool.pricing.0>(client, old(pool).reserve_x, old(pool).reserve_y, amount_in);
        // Reserve x increases by the input amount.
        ensures pool.reserve_x == old(pool.reserve_x) + amount_in;
        // Reserve y decreases by the output amount.
        ensures pool.reserve_y == old(pool.reserve_y) - result;
        // Output is bounded by the original y reserve (PricingStrategy safety invariant).
        ensures result <= old(pool.reserve_y);
        // Constant-product preservation: the product of reserves never decreases
        // (PricingStrategy closure invariant applied to the pricing call).
        ensures pool.reserve_x * pool.reserve_y
            >= old(pool.reserve_x) * old(pool.reserve_y);
    }

    // -------------------------------------------------------
    // Usage examples
    // -------------------------------------------------------
    // In each call below the compiler automatically wraps the bare
    // pricing function as `PricingStrategy(..)`, so the pack-time
    // invariant check is what succeeds or fails.

    /// Create a pool using the fee-free constant-product curve.
    /// This satisfies all PricingStrategy invariants: constant_product
    /// never aborts when amount_in > 0 (since reserve_in + amount_in > 0).
    fun create_constant_product_pool(account: &signer) {
        create_pool(account, 1000, 1000, constant_product);
    }

    /// Create a pool using the non-compliant fee-based pricing curve.
    /// This should FAIL verification at pack time: the non-compliant
    /// variant aborts when the client has no Fee resource, violating
    /// the PricingStrategy no-abort invariant.
    fun create_noncompliant_fee_pool(account: &signer) {
        // error: data invariant does not hold
        create_pool(account, 1000, 1000, constant_product_with_fee_non_compliant);
    }

    /// Create a pool using the compliant fee-based pricing curve.
    /// The compliant variant defaults to `DEFAULT_FEE_BPS` when the
    /// client has no `Fee` resource, so the PricingStrategy no-abort
    /// invariant is satisfied.
    fun create_compliant_fee_pool(account: &signer) {
        create_pool(account, 1000, 1000, constant_product_with_fee);
    }
}

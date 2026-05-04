// Copyright © Aptos Foundation
// AMM (Automated Market Maker) example where the pricing function is
// stored directly in the `Pool` struct. The pool-level invariants
// constrain any closure assigned to `pool.pricing`, so pack time
// (construction of `Pool`) is the proof-obligation point.
//
// flag: --split-vcs-by-assert

module 0x42::amm {
    use std::signer;

    // -------------------------------------------------------
    // Pool
    // -------------------------------------------------------

    /// A liquidity pool holding reserves of two tokens and a pricing
    /// function. The pricing closure is checked against the pool
    /// invariants below at construction time.
    struct Pool has key {
        reserve_x: u64,
        reserve_y: u64,
        // let amt_out = pricing(reserve_in, reserve_out, amt_in)
        pricing: |u64, u64, u64| u64 has copy + store + drop,
    }

    spec Pool {
        modifies_of<self.pricing> *;

        // No-abort: the pricing function must never abort, for any inputs.
        invariant forall S in *, r_in: u64, r_out: u64, amt: u64:
            S |~ !aborts_of<self.pricing>(r_in, r_out, amt);

        // Safety: output never exceeds the output reserve.
        invariant forall S in *, r_in: u64, r_out: u64, amt: u64:
            S.. |~ result_of<self.pricing>(r_in, r_out, amt) <= r_out;

        // Monotonicity: more input yields at least as much output.
        invariant forall S in *, r_in: u64, r_out: u64,
                        a1: u64, a2: u64:
            S.. |~ a1 <= a2 ==>
                result_of<self.pricing>(r_in, r_out, a1)
                    <= result_of<self.pricing>(r_in, r_out, a2);

        // Constant-product preservation: the product of reserves never decreases.
        invariant forall S in *, r_in: u64, r_out: u64, amt: u64:
            S.. |~ (r_in + amt) * (r_out - result_of<self.pricing>(r_in, r_out, amt)) >= r_in * r_out;
    }

    /// Per-owner fee in basis points (e.g. 300 = 3%).
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
    /// satisfying the Pool no-abort invariant.
    public fun constant_product(
        reserve_in: u64, reserve_out: u64, amount_in: u64,
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


    /// Default fee (500 bps = 5%) used when the owner has no `Fee` resource
    /// or the stored value is out of range.
    const DEFAULT_FEE_BPS: u64 = 500;

    /// Constant-product pricing with a per-owner fee (compliant variant).
    /// Uses `Fee[owner].bps` when the owner has a valid `Fee` resource,
    /// otherwise falls back to `DEFAULT_FEE_BPS`. Guards the division so
    /// the function never aborts, satisfying the Pool invariants.
    public fun constant_product_with_fee(
        owner: address, reserve_in: u64, reserve_out: u64, amount_in: u64,
    ): u64 {
        let fee_bps = if (exists<Fee>(owner) && Fee[owner].bps <= 10000) {
            Fee[owner].bps
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
        let fee_bps = if (exists<Fee>(owner) && Fee[owner].bps <= 10000) {
            Fee[owner].bps
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
        // Constant-product preservation: the Pool invariant, stated directly
        // so callers constructing a `Pool` do not need to re-derive it from
        // the fee-adjusted input formula.
        ensures (reserve_in + amount_in) * (reserve_out - result)
            >= reserve_in * reserve_out;
    } proof {
        // Stepping-stone at the return point. CP-preservation follows
        // from the closed-form `result` together with `eff <= amt`,
        // but Z3 won't derive that bound across the nonlinear fee formula
        // `eff == amt * (10000 - fee) / 10000`. The hint supplies it.
        post assert effective_in <= amount_in;
    }

    /// Constant-product pricing with a per-owner fee (non-compliant variant).
    /// Reads `Fee[owner].bps` directly and aborts when the owner has no
    /// `Fee` resource — violating the Pool no-abort invariant.
    public fun constant_product_with_fee_non_compliant(
        owner: address, reserve_in: u64, reserve_out: u64, amount_in: u64,
    ): u64 {
        let fee_bps = Fee[owner].bps;
        let effective_in = (amount_in as u128) * (10000 - fee_bps as u128) / 10000;
        let num = (reserve_out as u128) * effective_in;
        let den = (reserve_in as u128) + effective_in;
        ((num / den) as u64)
    }
    spec constant_product_with_fee_non_compliant {
        pragma opaque;
        // NON-COMPLIANT: Aborts if owner has no fee configuration.
        aborts_if !exists<Fee>(owner);
        // NON-COMPLIANT: Aborts if fee exceeds 100% (10000 basis points).
        aborts_if Fee[owner].bps > 10000;
        let fee_bps = Fee[owner].bps;
        let effective_in = amount_in * (10000 - fee_bps) / 10000;
        // Aborts on division by zero when reserve and effective input are both zero.
        aborts_if reserve_in + effective_in == 0;
        // Result follows the constant-product formula applied to the
        // fee-adjusted input (the fee reduces the effective swap amount).
        ensures result == reserve_out * effective_in / (reserve_in + effective_in);
        // Output never exceeds the output reserve.
        ensures result <= reserve_out;
    } proof {
        // Isolate the nonlinear ensures from the zero-swap degenerate case.
        split effective_in == 0;
        post assert effective_in <= amount_in;
    }

    // -------------------------------------------------------
    // Pool operations
    // -------------------------------------------------------

    /// Swap `amount_in` of token X for token Y.
    /// The output is determined by the pool's stored pricing function.
    public fun swap(pool: &mut Pool, amount_in: u64): u64 {
        let amount_out = (pool.pricing)(
            pool.reserve_x, pool.reserve_y, amount_in,
        );
        pool.reserve_x = pool.reserve_x + amount_in;
        pool.reserve_y = pool.reserve_y - amount_out;
        amount_out
    }
    spec swap {
        // Aborts on reserve overflow. The pricing function itself cannot abort
        // because of the Pool no-abort invariant; `reserve_y - amount_out`
        // cannot underflow because of the safety invariant.
        aborts_if pool.reserve_x + amount_in > MAX_U64;
        // Output is determined by the pricing function at the pre-state reserves.
        ensures result == old(result_of<pool.pricing>(pool.reserve_x, pool.reserve_y, amount_in));
        // Reserve x increases by the input amount.
        ensures pool.reserve_x == old(pool.reserve_x) + amount_in;
        // Reserve y decreases by the output amount.
        ensures pool.reserve_y == old(pool.reserve_y) - result;
        // Output is bounded by the original y reserve (Pool safety invariant).
        ensures result <= old(pool.reserve_y);
        // Constant-product preservation: the Pool invariant applied to the pricing call.
        ensures pool.reserve_x * pool.reserve_y
            >= old(pool.reserve_x) * old(pool.reserve_y);
    }

    // -------------------------------------------------------
    // Usage examples
    // -------------------------------------------------------
    // Each constructor publishes a `Pool` at `account`'s address; the
    // pack-time invariant check against `pool.pricing` is what succeeds or
    // fails.

    /// Create a pool using the fee-free constant-product curve.
    /// This satisfies all Pool invariants: constant_product never aborts,
    /// and its closed-form CP formula satisfies safety, monotonicity, and
    /// constant-product preservation.
    fun create_constant_product_pool(account: &signer) {
        move_to(account, Pool {
            reserve_x: 1000,
            reserve_y: 1000,
            pricing: constant_product,
        });
    }

    /// Create a pool using the non-compliant fee-based pricing curve.
    /// The owner address is captured in a closure so the stored function
    /// has the 3-argument pricing signature. Pack time should FAIL:
    /// the non-compliant variant aborts when the captured owner has no
    /// `Fee` resource, violating the Pool no-abort invariant.
    fun create_noncompliant_fee_pool(account: &signer) {
        let owner = signer::address_of(account);
        // error: data invariant does not hold
        move_to(account, Pool {
            reserve_x: 1000,
            reserve_y: 1000,
            pricing: |r_in, r_out, amt|
                constant_product_with_fee_non_compliant(owner, r_in, r_out, amt),
        });
    }

    /// Create a pool using the compliant fee-based pricing curve.
    /// The owner address is captured in a closure; the compliant variant
    /// defaults to `DEFAULT_FEE_BPS` when the owner has no `Fee` resource,
    /// so the Pool no-abort invariant is satisfied.
    fun create_compliant_fee_pool(account: &signer) {
        let owner = signer::address_of(account);
        move_to(account, Pool {
            reserve_x: 1000,
            reserve_y: 1000,
            pricing: |r_in, r_out, amt|
                constant_product_with_fee(owner, r_in, r_out, amt),
        });
    }
}

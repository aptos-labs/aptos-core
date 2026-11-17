// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A pricing strategy carries an automated market maker's rules as
// behavioural invariants of a function-typed field, quantified over every
// state. Packing a strategy is where the rules are proved, so a pool's
// creator supplies a strategy and a quote states no precondition on it. The
// unit is mixed: `Fee` carries an ordinary data invariant beside the
// strategy's behavioural ones, and the states the behavioural binder ranges
// over are those whose stored values satisfy the ordinary one.
//
// Modelled on move-prover/tests/sources/functional/closures/amm_example.move,
// cut to what runs in seconds.

module 0x42::mini_amm {
    struct Pricing(|address, u64, u64|u64) has copy, drop, store;

    spec Pricing {
        modifies_of<self.0> *;

        /// The pricing function never aborts, in any state.
        invariant forall S in *, client: address, reserve: u64, amount: u64:
            S |~ !aborts_of<self.0>(client, reserve, amount);

        /// Its output never exceeds the reserve, in any state.
        invariant forall S in *, client: address, reserve: u64, amount: u64, out: u64:
            S.. |~ ensures_of<self.0>(client, reserve, amount, out) ==> out <= reserve;
    }

    struct Pool has key {
        reserve: u64,
        pricing: Pricing,
    }

    /// A client's fee in basis points. Its invariant is an ordinary data
    /// invariant, not a behavioural one.
    struct Fee has key {
        bps: u64,
    }
    spec Fee {
        invariant bps <= 10000;
    }

    // ---------------------------------------------------------------- pricing

    /// Quotes the whole amount while the reserve covers it.
    public fun capped(_client: address, reserve: u64, amount: u64): u64 {
        if (amount < reserve) { amount } else { reserve }
    }
    spec capped {
        pragma opaque;
        aborts_if false;
        ensures result <= reserve;
    }

    /// Packing a compliant pricing function: both rules are proved here.
    fun make_capped(): Pricing { capped }
    spec make_capped {
        pragma opaque;
        aborts_if false;
    }

    // ------------------------------------------------------------- using both

    /// A quote off a strategy, the swap of this AMM. It does not abort
    /// because of the no-abort rule and its output is bounded because of the
    /// result rule; the contract states no user-written precondition on
    /// `pricing` (the generated parameter and behavioural assumptions are
    /// there as for any parameter).
    fun quote(pricing: Pricing, client: address, reserve: u64, amount: u64): u64 {
        pricing(client, reserve, amount)
    }
    spec quote {
        aborts_if false;
        ensures result <= reserve;
    }

    // ---------------------------------------------------------------- storage
    //
    // Three distinct observations, kept apart on purpose so that closing one
    // cannot hide the next.

    fun create(account: &signer, reserve: u64, pricing: Pricing) {
        move_to(account, Pool { reserve, pricing });
    }

    /// (1) OPEN: the specification-global-read boundary of a resource that
    /// holds a function value. The postcondition reads `global<Pool>` in a
    /// specification, which goes through `nativeSlot`: that eagerly builds a
    /// fallback for the WHOLE resource carrier and requires it inhabitable,
    /// which a function type is not. The unit therefore reports that `Pool`
    /// has no values and this target is never scheduled. The pricing function
    /// may write memory (`modifies_of<self.0> *`), so the bound is read in the
    /// entry state.
    fun quote_stored(owner: address, client: address, amount: u64): u64 {
        let pool = &Pool[owner];
        (pool.pricing)(client, pool.reserve, amount)
    }
    spec quote_stored {
        aborts_if !exists<Pool>(owner);
        // open: the specification global read needs the resource carrier's
        // fallback, which a function-valued field has none of
        ensures result <= old(Pool[owner]).reserve;
    }

    /// (2) OPEN, and a different obligation: the same body without the
    /// specification global read. The unit's complaint disappears, the walk
    /// reaches storage and the invocation and obtains the strategy's no-abort
    /// and result facts, and what is left is the preservation of the unit's
    /// stored invariants after a call under `modifies_of<self.0> *`.
    fun quote_stored_unbounded(owner: address, client: address, amount: u64): u64 {
        let pool = &Pool[owner];
        (pool.pricing)(client, pool.reserve, amount)
    }
    spec quote_stored_unbounded {
        // open: the residual obligation is `stored_invariant`, the
        // preservation of the unit's stored invariants after the call
        aborts_if !exists<Pool>(owner);
    }

    /// (3) The pool as a parameter, which is the shape the Move Prover's own
    /// AMM example uses: this verifies, and so does the `&mut` form below.
    /// The core use of a stored strategy is therefore supported.
    fun quote_pool(pool: &Pool, client: address, amount: u64): u64 {
        (pool.pricing)(client, pool.reserve, amount)
    }
    spec quote_pool {
        aborts_if false;
        ensures result <= old(pool.reserve);
    }

    fun quote_pool_mut(pool: &mut Pool, client: address, amount: u64): u64 {
        (pool.pricing)(client, pool.reserve, amount)
    }
    spec quote_pool_mut {
        aborts_if false;
        ensures result <= old(pool.reserve);
    }

    // -------------------------------------------------------------- negatives

    /// Pricing that can abort.
    public fun aborting(_client: address, reserve: u64, amount: u64): u64 {
        assert!(amount > 0, 1);
        if (amount < reserve) { amount } else { reserve }
    }
    spec aborting {
        pragma opaque;
        aborts_if amount == 0;
        ensures result <= reserve;
    }

    /// Packing it must fail on the no-abort rule alone: the result rule's
    /// `ensures_of` observes successful runs only.
    fun make_aborting(): Pricing {
        // error: data invariant does not hold
        aborting
    }

    /// Pricing that never aborts and quotes more than the reserve.
    public fun whole(_client: address, _reserve: u64, amount: u64): u64 {
        amount
    }
    spec whole {
        pragma opaque;
        aborts_if false;
        ensures result == amount;
    }

    /// Packing it must fail on the result rule alone.
    fun make_whole(): Pricing {
        // error: data invariant does not hold
        whole
    }

    /// Claiming more than the rules give.
    fun quote_strict(pricing: Pricing, client: address, reserve: u64, amount: u64): u64 {
        pricing(client, reserve, amount)
    }
    spec quote_strict {
        aborts_if false;
        // error: the result rule bounds the output by the reserve, not below it
        ensures result < reserve;
    }

    // -------------------------------------------- the behavioural binder's states

    /// Pricing whose only abort needs `Fee[client].bps > 10000`, which the
    /// `Fee` data invariant forbids. Packing it succeeds. With its negative
    /// below, this pair pins two named cases of the behavioural binder's
    /// state domain: that a state with `bps > 10000` is excluded, and that a
    /// function requiring a `Fee` to exist is rejected because the empty
    /// state is admitted. The wider reading — that the domain is the states
    /// satisfying the unit's installed ORDINARY data invariants and not its
    /// behavioural ones — rests on an inspection of the generator; this pair
    /// is its regression test, not its proof.
    public fun fee_guarded(client: address, reserve: u64, amount: u64): u64 {
        let bps = if (exists<Fee>(client)) { Fee[client].bps } else { 0 };
        // Underflows exactly when the client's `Fee` breaks `bps <= 10000`.
        let keep = 10000 - bps;
        if (keep == 0) { 0 } else if (amount < reserve) { amount } else { reserve }
    }
    spec fee_guarded {
        pragma opaque;
        aborts_if exists<Fee>(client) && Fee[client].bps > 10000;
        ensures result <= reserve;
    }

    fun make_fee_guarded(): Pricing { fee_guarded }
    spec make_fee_guarded {
        pragma opaque;
        aborts_if false;
    }

    /// Pricing that aborts where a client has no `Fee` at all — a state the
    /// ordinary invariants admit.
    public fun fee_unguarded(client: address, reserve: u64, amount: u64): u64 {
        let bps = Fee[client].bps;
        if (bps == 0) { 0 } else if (amount < reserve) { amount } else { reserve }
    }
    spec fee_unguarded {
        pragma opaque;
        aborts_if !exists<Fee>(client);
        ensures result <= reserve;
    }

    /// Packing it must fail: the abort is possible in a state the ordinary
    /// invariants admit.
    fun make_fee_unguarded(): Pricing {
        // error: data invariant does not hold
        fee_unguarded
    }

    // ------------------------------------------- the result rule by `result_of`

    /// The same result rule written with `result_of` instead of
    /// `ensures_of`. Packing a compliant function proves it, but a caller
    /// cannot use it: `quote_bounded` below does not reach its bound. The
    /// cell is open, not intended, and is pinned so that a tree which gains
    /// the capability fails this fixture.
    struct Bounded(|u64|u64) has copy, drop;

    spec Bounded {
        invariant forall S in *, amount: u64:
            S.. |~ result_of<self.0>(amount) <= amount;
    }

    public fun identity(amount: u64): u64 { amount }
    spec identity {
        pragma opaque;
        aborts_if false;
        ensures result == amount;
    }

    fun make_bounded(): Bounded { identity }
    spec make_bounded {
        pragma opaque;
        aborts_if false;
    }

    fun quote_bounded(bounded: Bounded, amount: u64): u64 {
        bounded(amount)
    }
    spec quote_bounded {
        pragma aborts_if_is_partial;
        // open: the result rule by `result_of` is not read at a call
        ensures result <= amount;
    }
}

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.Rendered.Support

/-!
# `mini_amm` as a frontend renders it

The module below is the rendering of
`VerexE2ETests/SourceVerify/mini_amm.move` that the source-verification driver
writes to `.verex/mini_amm.move.lean`, its `verex sources at` command dropped;
the diagnostics are those of `mini_amm.exp`. Neither is edited here:
`scripts/prover-smoke-renderings.sh --write` writes this file again, and
without `--write` tells whether it is what the frontend renders.
-/

/--
error: the native type 0x42.mini_amm.unit.nominal.namespace.0.declaration✝¹⁻⁰ has no values
---
error: verification failed with a residual obligation:
case returns.left
executable✝ : VerexIR.Validation.ExecutableUnit unit
nativesTyped : VerexIR.NativesTyped executable✝
nativesShift : VerexIR.NativesShift executable✝
runsAgree : VerexIR.Proofs.RunsAgree executable✝
ownersRepresentable : VerexIR.Proofs.OwnersRepresentable unit
verexAgree : VerexIR.Proofs.TypesAgree unit
verexResources : VerexIR.Proofs.ResourcesTyped unit
owner client : VerexIR.Proofs.Denote.NTy.address.carrier
amount : (VerexIR.Proofs.Denote.NTy.int 64 false).carrier
initialState✝ : VerexIR.Proofs.Denote.Memory unit
permitted : VerexIR.Proofs.Denote.DataInvariants (storedInvariant unit executable✝) initialState✝
fst✝¹ : (VerexIR.Proofs.Denote.NTy.int 64 false).carrier
fst✝ :
  (VerexIR.Proofs.Denote.NTy.function
      (VerexIR.Proofs.Denote.NRow.cons VerexIR.Proofs.Denote.NTy.address
        (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false)
          (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false) VerexIR.Proofs.Denote.NRow.nil)))
      { parameters := [false, false, false], results := [false] }
      (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false) VerexIR.Proofs.Denote.NRow.nil)).carrier
heq :
  VerexIR.Proofs.Denote.Skolems.plainAt 0x42.mini_amm.unit.nominal.namespace.0.declaration✝¹⁻⁰
      VerexIR.Proofs.Denote.NRow.nil (VerexIR.StorageKey.address owner) initialState✝ =
    some (fst✝¹, (fst✝, ()), ())
stored_1 :
  ∀ (S : VerexIR.Proofs.Denote.Memory unit) (client : String) (reserve : Int)
    (fits : 0 ≤ reserve ∧ reserve ≤ 18446744073709551615) (amount : Int)
    (fits_1 : 0 ≤ amount ∧ amount ≤ 18446744073709551615),
    ¬VerexIR.Proofs.AbortsOfEntries (VerexIR.Proofs.Denote.closureMeaning executable✝)
        (VerexIR.Proofs.Denote.NRow.cons VerexIR.Proofs.Denote.NTy.address
          (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false)
            (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false) VerexIR.Proofs.Denote.NRow.nil)))
        { parameters := [false, false, false], results := [false] }
        (VerexIR.Proofs.Denote.ResultShape.one (VerexIR.Proofs.Denote.NTy.int 64 false)) fst✝
        (client, { val := reserve, fits := ⋯ }, { val := amount, fits := ⋯ }, ()) S
stored_2 :
  ∀ (memory S : VerexIR.Proofs.Denote.Memory unit) (client : String) (reserve : Int)
    (fits : 0 ≤ reserve ∧ reserve ≤ 18446744073709551615) (amount : Int)
    (fits_1 : 0 ≤ amount ∧ amount ≤ 18446744073709551615) (out : Int) (fits_2 : 0 ≤ out ∧ out ≤ 18446744073709551615),
    VerexIR.Proofs.EnsuresOfEntries (VerexIR.Proofs.Denote.closureMeaning executable✝)
        (VerexIR.Proofs.Denote.NRow.cons VerexIR.Proofs.Denote.NTy.address
          (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false)
            (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false) VerexIR.Proofs.Denote.NRow.nil)))
        { parameters := [false, false, false], results := [false] }
        (VerexIR.Proofs.Denote.ResultShape.one (VerexIR.Proofs.Denote.NTy.int 64 false)) fst✝
        (client, { val := reserve, fits := ⋯ }, { val := amount, fits := ⋯ }, ()) () { val := out, fits := ⋯ } S
        memory →
      out ≤ reserve
verexResult : (VerexIR.Proofs.Denote.ResultShape.one (VerexIR.Proofs.Denote.NTy.int 64 false)).carrier
verexFinal : VerexIR.Proofs.Denote.Memory unit
verexEnsures :
  VerexIR.Proofs.EnsuresOf (VerexIR.Proofs.Denote.closureMeaning executable✝)
    (VerexIR.Proofs.Denote.NRow.cons VerexIR.Proofs.Denote.NTy.address
      (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false)
        (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false) VerexIR.Proofs.Denote.NRow.nil)))
    { parameters := [false, false, false], results := [false] }
    (VerexIR.Proofs.Denote.ResultShape.one (VerexIR.Proofs.Denote.NTy.int 64 false)) fst✝ (client, fst✝¹, amount, ())
    verexResult initialState✝ verexFinal
verexUnaborted :
  ¬VerexIR.Proofs.AbortsOfEntries (VerexIR.Proofs.Denote.closureMeaning executable✝)
      (VerexIR.Proofs.Denote.NRow.cons VerexIR.Proofs.Denote.NTy.address
        (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false)
          (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false) VerexIR.Proofs.Denote.NRow.nil)))
      { parameters := [false, false, false], results := [false] }
      (VerexIR.Proofs.Denote.ResultShape.one (VerexIR.Proofs.Denote.NTy.int 64 false)) fst✝
      (VerexIR.Proofs.Denote.HList.settle
        (VerexIR.Proofs.Denote.NRow.cons VerexIR.Proofs.Denote.NTy.address
          (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false)
            (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false) VerexIR.Proofs.Denote.NRow.nil)))
        (client, fst✝¹, amount, ()))
      initialState✝
verexResultOf :
  ∀ (default : (VerexIR.Proofs.Denote.ResultShape.one (VerexIR.Proofs.Denote.NTy.int 64 false)).carrier),
    VerexIR.Proofs.ResultOf (VerexIR.Proofs.Denote.closureMeaning executable✝)
        (VerexIR.Proofs.Denote.NRow.cons VerexIR.Proofs.Denote.NTy.address
          (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false)
            (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false) VerexIR.Proofs.Denote.NRow.nil)))
        { parameters := [false, false, false], results := [false] }
        (VerexIR.Proofs.Denote.ResultShape.one (VerexIR.Proofs.Denote.NTy.int 64 false)) fst✝
        (client, fst✝¹, amount, ()) initialState✝ default =
      verexResult
verexStateOf :
  VerexIR.Proofs.StateOf (VerexIR.Proofs.Denote.closureMeaning executable✝)
      (VerexIR.Proofs.Denote.NRow.cons VerexIR.Proofs.Denote.NTy.address
        (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false)
          (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false) VerexIR.Proofs.Denote.NRow.nil)))
      { parameters := [false, false, false], results := [false] }
      (VerexIR.Proofs.Denote.ResultShape.one (VerexIR.Proofs.Denote.NTy.int 64 false)) fst✝
      (VerexIR.Proofs.Denote.HList.settle
        (VerexIR.Proofs.Denote.NRow.cons VerexIR.Proofs.Denote.NTy.address
          (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false)
            (VerexIR.Proofs.Denote.NRow.cons (VerexIR.Proofs.Denote.NTy.int 64 false) VerexIR.Proofs.Denote.NRow.nil)))
        (client, fst✝¹, amount, ()))
      initialState✝ =
    verexFinal
verexResultHolds : True
a✝ :
  (VerexIR.Proofs.Denote.Skolems.plainAt 0x42.mini_amm.unit.nominal.namespace.0.declaration✝¹⁻⁰
        VerexIR.Proofs.Denote.NRow.nil (VerexIR.StorageKey.address owner) initialState✝).isSome =
    true
⊢ VerexIR.Proofs.Denote.DataInvariants (storedInvariant unit executable✝) verexFinal
---
error: verex verification failed: the automatic verification of `quote_stored_unbounded` failed; provide a proof in `VerexE2ETests/SourceVerify/mini_amm.proof.lean`:
  verify quote_stored_unbounded by lean =>
    verex_obligations
    verex_on
    | stored_invariant => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
---
error: the specification clause `ensures result < reserve` is not established
---
error: verex verification failed: the automatic verification of `quote_strict` failed; provide a proof in `VerexE2ETests/SourceVerify/mini_amm.proof.lean`:
  verify quote_strict by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
---
error: the specification clause `ensures result <= amount` is not established
---
error: verex verification failed: the automatic verification of `quote_bounded` failed; provide a proof in `VerexE2ETests/SourceVerify/mini_amm.proof.lean`:
  verify quote_bounded by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
---
error: the data invariant `invariant ∀ (S : StateDomain;
      client : Address;
      reserve : u64;
      amount : u64),
        !(S.. |~ aborts_of<this.0>(client, reserve, amount))` does not hold where a value is constructed
---
error: verex verification failed: the automatic verification of `make_aborting` failed; provide a proof in `VerexE2ETests/SourceVerify/mini_amm.proof.lean`:
  verify make_aborting by lean =>
    verex_obligations
    verex_on
    | data_invariant_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
---
error: the data invariant `invariant ∀ (S : StateDomain;
      client : Address;
      reserve : u64;
      amount : u64;
      out : u64),
        (S.. |~ ensures_of<this.0>(client, reserve, amount, out))
          ==> out <= reserve` does not hold where a value is constructed
---
error: verex verification failed: the automatic verification of `make_whole` failed; provide a proof in `VerexE2ETests/SourceVerify/mini_amm.proof.lean`:
  verify make_whole by lean =>
    verex_obligations
    verex_on
    | data_invariant_2 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
---
error: the data invariant `invariant ∀ (S : StateDomain;
      client : Address;
      reserve : u64;
      amount : u64),
        !(S.. |~ aborts_of<this.0>(client, reserve, amount))` does not hold where a value is constructed
---
error: verex verification failed: the automatic verification of `make_fee_unguarded` failed; provide a proof in `VerexE2ETests/SourceVerify/mini_amm.proof.lean`:
  verify make_fee_unguarded by lean =>
    verex_obligations
    verex_on
    | data_invariant_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_rendering in
verex module 0x42::mini_amm where
  pragma proof_file = "VerexE2ETests/SourceVerify/mini_amm.proof.lean"

  -- Copyright © Aptos Foundation
  -- SPDX-License-Identifier: Apache-2.0
  -- A pricing strategy carries an automated market maker's rules as
  -- behavioural invariants of a function-typed field, quantified over every
  -- state. Packing a strategy is where the rules are proved, so a pool's
  -- creator supplies a strategy and a quote states no precondition on it. The
  -- unit is mixed: `Fee` carries an ordinary data invariant beside the
  -- strategy's behavioural ones, and the states the behavioural binder ranges
  -- over are those whose stored values satisfy the ordinary one.
  --
  -- Modelled on move-prover/tests/sources/functional/closures/amm_example.move,
  -- cut to what runs in seconds.
  struct Pricing has Copy, Drop, Store where
    0 : Fn(Address, u64, u64) -> u64 has Copy, Drop, Store

  spec Pricing where
    invariant ∀ (S : StateDomain;
      client : Address;
      reserve : u64;
      amount : u64),
        !(S.. |~ aborts_of<this.0>(client, reserve, amount))
    invariant ∀ (S : StateDomain;
      client : Address;
      reserve : u64;
      amount : u64;
      out : u64),
        (S.. |~ ensures_of<this.0>(client, reserve, amount, out))
          ==> out <= reserve
    modifies_of<«0»> *

  struct Pool has Key where
    reserve : u64
    pricing : Pricing

  /--
  A client's fee in basis points. Its invariant is an ordinary data
  invariant, not a behavioural one.
  -/
  struct Fee has Key where
    bps : u64

  spec Fee where
    invariant bps <= 10000

  -- ---------------------------------------------------------------- pricing
  /--
  Quotes the whole amount while the reserve covers it.
  -/
  public fun capped(_client : Address, reserve : u64, amount : u64) -> u64 :=
    if amount < reserve then amount else reserve

  spec capped where
    pragma opaque
    aborts_if false
    ensures result <= reserve

  /--
  Packing a compliant pricing function: both rules are proved here.
  -/
  fun make_capped() -> Pricing :=
    new Pricing {
      0 := function[Fn(Address, u64, u64) -> u64 has Copy, Drop, Store](capped)
    }

  spec make_capped where
    pragma opaque
    aborts_if false

  -- ------------------------------------------------------------- using both
  /--
  A quote off a strategy, the swap of this AMM. It does not abort
  because of the no-abort rule and its output is bounded because of the
  result rule; the contract states no user-written precondition on
  `pricing` (the generated parameter and behavioural assumptions are
  there as for any parameter).
  -/
  fun quote(
    pricing : Pricing, client : Address, reserve : u64, amount : u64
  ) -> u64 := invoke(pricing.0, client, reserve, amount)

  spec quote where
    aborts_if false
    ensures result <= reserve
    modifies *

  -- ---------------------------------------------------------------- storage
  --
  -- Three distinct observations, kept apart on purpose so that closing one
  -- cannot hide the next.
  fun create(account : &Signer, reserve : u64, pricing : Pricing) -> Unit := do
    move_to<Pool>(account, new Pool { reserve, pricing })

  spec create where
    modifies *

  /--
  (1) OPEN: the specification-global-read boundary of a resource that
  holds a function value. The postcondition reads `global<Pool>` in a
  specification, which goes through `nativeSlot`: that eagerly builds a
  fallback for the WHOLE resource carrier and requires it inhabitable,
  which a function type is not. The unit therefore reports that `Pool`
  has no values and this target is never scheduled. The pricing function
  may write memory (`modifies_of<self.0> *`), so the bound is read in the
  entry state.
  -/
  fun quote_stored(owner : Address, client : Address, amount : u64) -> u64 := do
    let pool := &Pool[owner]
    invoke((pool.pricing).0, client, pool.reserve, amount)

  spec quote_stored where
    aborts_if !exists<Pool>(owner)
    ensures result <= old(global<Pool>(owner)).reserve
    modifies *

  -- open: the specification global read needs the resource carrier's
  -- fallback, which a function-valued field has none of
  /--
  (2) OPEN, and a different obligation: the same body without the
  specification global read. The unit's complaint disappears, the walk
  reaches storage and the invocation and obtains the strategy's no-abort
  and result facts, and what is left is the preservation of the unit's
  stored invariants after a call under `modifies_of<self.0> *`.
  -/
  fun quote_stored_unbounded(
    owner : Address, client : Address, amount : u64
  ) -> u64 := do
    let pool := &Pool[owner]
    invoke((pool.pricing).0, client, pool.reserve, amount)

  spec quote_stored_unbounded where
    aborts_if !exists<Pool>(owner)
    modifies *

  -- open: the residual obligation is `stored_invariant`, the
  -- preservation of the unit's stored invariants after the call
  /--
  (3) The pool as a parameter, which is the shape the Move Prover's own
  AMM example uses: this verifies, and so does the `&mut` form below.
  The core use of a stored strategy is therefore supported.
  -/
  fun quote_pool(pool : &Pool, client : Address, amount : u64) -> u64 :=
    invoke((pool.pricing).0, client, pool.reserve, amount)

  spec quote_pool where
    aborts_if false
    ensures result <= old(pool.reserve)
    modifies *

  fun quote_pool_mut(pool : &mut Pool, client : Address, amount : u64) -> u64 :=
    invoke((pool.pricing).0, client, pool.reserve, amount)

  spec quote_pool_mut where
    aborts_if false
    ensures result <= old(pool.reserve)
    modifies *

  -- -------------------------------------------------------------- negatives
  /--
  Pricing that can abort.
  -/
  public fun aborting(
    _client : Address, reserve : u64, amount : u64
  ) -> u64 := do
    assert!(amount > 0, 1)
    if amount < reserve then amount else reserve

  spec aborting where
    pragma opaque
    aborts_if amount == 0
    ensures result <= reserve

  /--
  Packing it must fail on the no-abort rule alone: the result rule's
  `ensures_of` observes successful runs only.
  -/
  fun make_aborting() -> Pricing :=
    new Pricing {
      0 := function[Fn(Address, u64, u64) -> u64 has Copy, Drop, Store](aborting)
    }

  -- error: data invariant does not hold
  /--
  Pricing that never aborts and quotes more than the reserve.
  -/
  public fun whole(_client : Address, _reserve : u64, amount : u64) -> u64 :=
    amount

  spec whole where
    pragma opaque
    aborts_if false
    ensures result == amount

  /--
  Packing it must fail on the result rule alone.
  -/
  fun make_whole() -> Pricing :=
    new Pricing {
      0 := function[Fn(Address, u64, u64) -> u64 has Copy, Drop, Store](whole)
    }

  -- error: data invariant does not hold
  /--
  Claiming more than the rules give.
  -/
  fun quote_strict(
    pricing : Pricing, client : Address, reserve : u64, amount : u64
  ) -> u64 := invoke(pricing.0, client, reserve, amount)

  spec quote_strict where
    aborts_if false
    ensures result < reserve
    modifies *

  -- error: the result rule bounds the output by the reserve, not below it
  -- -------------------------------------------- the behavioural binder's states
  /--
  Pricing whose only abort needs `Fee[client].bps > 10000`, which the
  `Fee` data invariant forbids. Packing it succeeds. With its negative
  below, this pair pins two named cases of the behavioural binder's
  state domain: that a state with `bps > 10000` is excluded, and that a
  function requiring a `Fee` to exist is rejected because the empty
  state is admitted. The wider reading — that the domain is the states
  satisfying the unit's installed ORDINARY data invariants and not its
  behavioural ones — rests on an inspection of the generator; this pair
  is its regression test, not its proof.
  -/
  public fun fee_guarded(
    client : Address, reserve : u64, amount : u64
  ) -> u64 := do
    let bps := if exists<Fee>(client) then Fee[client].bps else 0
    let keep := 10000 - bps
    if keep == 0 then 0
    else
      if amount < reserve then amount else reserve

  spec fee_guarded where
    pragma opaque
    aborts_if exists<Fee>(client) && global<Fee>(client).bps > 10000
    ensures result <= reserve

  -- Underflows exactly when the client's `Fee` breaks `bps <= 10000`.
  fun make_fee_guarded() -> Pricing :=
    new Pricing {
      0 := function[Fn(Address, u64, u64) -> u64 has Copy, Drop, Store](fee_guarded)
    }

  spec make_fee_guarded where
    pragma opaque
    aborts_if false

  /--
  Pricing that aborts where a client has no `Fee` at all — a state the
  ordinary invariants admit.
  -/
  public fun fee_unguarded(
    client : Address, reserve : u64, amount : u64
  ) -> u64 := do
    let bps := Fee[client].bps
    if bps == 0 then 0
    else
      if amount < reserve then amount else reserve

  spec fee_unguarded where
    pragma opaque
    aborts_if !exists<Fee>(client)
    ensures result <= reserve

  /--
  Packing it must fail: the abort is possible in a state the ordinary
  invariants admit.
  -/
  fun make_fee_unguarded() -> Pricing :=
    new Pricing {
      0 := function[Fn(Address, u64, u64) -> u64 has Copy, Drop, Store](fee_unguarded)
    }

  -- error: data invariant does not hold
  -- ------------------------------------------- the result rule by `result_of`
  /--
  The same result rule written with `result_of` instead of
  `ensures_of`. Packing a compliant function proves it, but a caller
  cannot use it: `quote_bounded` below does not reach its bound. The
  cell is open, not intended, and is pinned so that a tree which gains
  the capability fails this fixture.
  -/
  struct Bounded has Copy, Drop where
    0 : Fn(u64) -> u64 has Copy, Drop

  spec Bounded where
    invariant ∀ (S : StateDomain; amount : u64),
        (S.. |~ result_of<this.0>(amount)) <= amount

  public fun identity(amount : u64) -> u64 := amount

  spec identity where
    pragma opaque
    aborts_if false
    ensures result == amount

  fun make_bounded() -> Bounded :=
    new Bounded { 0 := function[Fn(u64) -> u64 has Copy, Drop](identity) }

  spec make_bounded where
    pragma opaque
    aborts_if false

  fun quote_bounded(bounded : Bounded, amount : u64) -> u64 :=
    invoke(bounded.0, amount)

  spec quote_bounded where
    pragma aborts_if_is_partial
    ensures result <= amount
    modifies *

  -- open: the result rule by `result_of` is not read at a call

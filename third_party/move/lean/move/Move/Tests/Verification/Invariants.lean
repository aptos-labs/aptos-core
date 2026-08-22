-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: specification and verification.

import Move
import MoveModel.Tests.Common

/-! Data invariants: a value of a type which declares one carries its proof,
so the invariant is available wherever the value is and is owed only where a
value is created. -/

namespace Tests.MovePrograms

open Move
open scoped Move Move.Compiler Move.Spec

module Invariants where

  /-! ## Types -/

  struct Percent where
    value : U64

  -- `.field` reads the value's field; `this` is only needed where the value
  -- as a whole is.  Clauses
  -- read like the other spec blocks and may be repeated.
  spec Percent where
    invariant .value.toNat ≤ 100

  struct Range where
    low : U64
    high : U64

  -- Repeated clauses are conjoined, as in a function contract.
  spec Range where
    invariant .low.toNat ≤ .high.toNat;
    invariant .high.toNat ≤ 1000

  -- An enum invariant matches on `this`; each constructor carries the proof
  -- of its own variant, so patterns bind it with a trailing `_`.
  enum Payment where
    | None
    | Direct (amount : U64)
    | Split (left right : U64)

  spec Payment where
    invariant match this with
      | .None => True
      | .Direct amount => 0 < amount.toNat
      | .Split left right => 0 < left.toNat ∧ 0 < right.toNat

  -- A resource with an invariant: reads are free, and the value is re-created
  -- — and its invariant re-established — where a mutable borrow of it dies.
  struct Gauge has Key where
    level : U64

  spec Gauge where
    invariant .level.toNat ≤ 100

  /-! ## Functions -/

  -- Creating a value discharges the invariant during elaboration: the source
  -- carries no proof text, and a violation is reported at the literal.
  fun empty : Percent :=
    { value := 0 }

  fun half : Percent :=
    { value := 50 }

  fun unit : Range :=
    { low := 0, high := 0 }

  fun span (range : Range) : U64 :=
    range.high - range.low

  -- Both clauses are available from the value: the first rules the
  -- subtraction's abort out, the second bounds the result.
  spec span (range : Range) where
    ensures (result : U64).toNat ≤ range.high.toNat ∧ result.toNat ≤ 1000;
    aborts_if False

  -- Creating a value under a condition: the dependent `if` puts the branch
  -- condition in scope, and the creation discharges its invariant from it.
  fun direct (amount : U64) : Payment :=
    if h : 0 < amount then .Direct amount else .None

  fun first_part (payment : Payment) : U64 :=
    match payment with
    | .None _ => 0
    | .Direct amount _ => amount
    | .Split left _ _ => left

  -- The invariant of the matched value is in scope in every arm.
  spec first_part (payment : Payment) where
    ensures match payment with
      | .None _ => (result : U64) = 0
      | _ => 0 < (result : U64).toNat

  entry fun set_level (addr : Address) (amount : U64) : Action Unit := do
    let level ← &mut Gauge[addr].level
    if amount < 101 then
      level := amount
    else
      abort 1

  spec set_level (addr : Address) (amount : U64) where
    requires existsAt<Gauge>(addr);
    modifies Gauge[addr];
    ensures Gauge[addr].level = amount;
    aborts_if ¬amount.toNat < 101 with 1

  fun reading (percent : Percent) : U64 :=
    percent.value

  -- The invariant is available from the value itself, with no precondition.
  spec reading (percent : Percent) where
    ensures (result : U64).toNat ≤ 100

  /-! ## Proofs -/

  verify span by
    -- `data_invariants` supplies the value's own invariant, so the two
    -- clauses need no prologue naming `Range.Invariant` here.
    contract_intro
    wp_norm
    data_invariants
    spec_norm at *
    refine ⟨fun _ => ⟨?_, trivial⟩, fun below => ?_⟩ <;> omega

  verify first_part by
    simp only [first_part.contract]
    intro payment
    cases payment with
    | None holds => simp [first_part]
    | Direct amount holds =>
        simp only [first_part]
        exact holds
    | Split left right holds =>
        simp only [first_part]
        exact holds.1

  verify set_level

  verify reading by
    simp only [reading.contract, reading]
    exact fun percent => percent.invariant

  /-! ## Tests -/

  def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.Invariants

  -- The certifying field is a proof, so Move never sees it.
  #guard (compiled.structMeta? "Percent").map
    (·.fieldNames) == some ["value"]
  #guard (compiled.structMeta? "Gauge").map
    (·.fieldNames) == some ["level"]
  -- Enum variants carry only their data fields.
  #guard (compiled.structMeta? "Payment").map
    (·.variantNames) ==
    some (some [("None", []), ("Direct", ["amount"]), ("Split", ["left", "right"])])

  private def run := Tests.run compiled

  #test run "reading" [] [.struct [.u64 50]] = Tests.okRet [] [.u64 50]
  #test run "first_part" [] [.variant 1 [.u64 7]] = Tests.okRet [] [.u64 7]
  #test run "direct" [] [.u64 0] = Tests.okRet [] [.variant 0 []]
  private def gaugeId := compiled.resourceId "Gauge"
  private def gauge (addr level : Nat) : MoveModel.IR.IMem :=
    [(gaugeId, addr, .struct [.u64 level])]
  #test run "set_level" (gauge 4 10) [.address 4, .u64 42] =
    Tests.okRet (gauge 4 42) []
  #test run "set_level" (gauge 4 10) [.address 4, .u64 500] =
    Tests.abortedIn (gauge 4 10) 1

end Tests.MovePrograms

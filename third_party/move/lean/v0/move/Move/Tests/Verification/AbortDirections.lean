-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: source verification.

import Move

/-! The two directions of `aborts_if`.  A declared condition is *sufficient*
(the function must abort where it holds) and, without `pragma
aborts_if_is_partial`, the declared conditions are also *necessary* (every
abort matches a clause, with its code).  `pragma aborts_if_is_strict` makes an
empty clause list mean "never aborts"; a pragma-led `ensures` on a pure
function selects the relational reading with uninterpreted abort behavior. -/

namespace Tests.MovePrograms.AbortDirections

open Move
open scoped Move Move.Spec

module AbortDirections where

  def E_ZERO : U64 := 1
  def E_LARGE : U64 := 2

  struct Counter has Key where
    value : U64

  -- Both directions: the clause is exactly where the function aborts.
  fun halve (value : U64) : Action U64 := do
    if value < 1 then
      abort E_ZERO
    pure (value / 2)

  spec halve (value : U64) where
    ensures result.toNat = value.toNat / 2;
    aborts_if value < 1 with E_ZERO

  verify halve

  -- Partial: the clause names one sufficient condition; the other abort (the
  -- overflow) is permitted without being declared.
  fun bump_checked (value : U64) (limit : U64) : Action U64 := do
    if limit ≤ value then
      abort E_LARGE
    pure (value + 1)

  spec bump_checked (value : U64) (limit : U64) where
    pragma aborts_if_is_partial;
    ensures result = value + 1;
    aborts_if limit ≤ value with E_LARGE

  verify bump_checked

  -- Strict without clauses: never aborts.
  fun identity (value : U64) : Action U64 := do
    pure value

  spec identity (value : U64) where
    pragma aborts_if_is_strict;
    ensures result = value

  verify identity

  -- A pure function whose abort behavior is left uninterpreted: the pragma
  -- selects the relational contract, so the postcondition is owed on every
  -- successful execution and nothing is claimed about the overflow.
  fun successor (value : U64) : U64 :=
    value + 1

  spec successor (value : U64) where
    pragma aborts_if_is_partial;
    ensures result = value + 1

  verify successor

  -- Clauses without a code pin no code; several clauses are disjoined.
  entry fun withdraw (addr : Address) (amount : U64) : Action Unit := do
    let value ← &mut Counter[addr].value
    let current ← *value
    assert!(current >= amount, E_LARGE)
    value := *value - amount

  spec withdraw (addr : Address) (amount : U64) where
    modifies *;
    ensures Counter[addr].value = old(Counter[addr].value) - amount;
    aborts_if ¬existsAt<Counter>(addr);
    aborts_if old(Counter[addr].value) < amount with E_LARGE

  verify withdraw

end Tests.MovePrograms.AbortDirections

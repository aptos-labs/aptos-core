-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: source verification.

import Move

/-! Contract summaries.  A native has no body: its `spec` is the whole
knowledge callers get, and its source semantics is the contract's summary
(`Contract.summary`).  A function specified `pragma opaque` is summarized for
its callers the same way; its own body is still verified against the
contract when its source semantics can be generated, and only assumed (with a
warning) when it cannot. -/

namespace Tests.MovePrograms.Summaries

open Move
open scoped Move Move.Spec

module Summaries where

  def E_LIMIT : U64 := 7

  -- A native: modeled by its contract.
  native fun host_double (value : U64) : U64

  spec host_double (value : U64) where
    pragma aborts_if_is_partial;
    ensures result.toNat = 2 * value.toNat

  fun quadruple (value : U64) : U64 :=
    host_double (host_double value)

  spec quadruple (value : U64) where
    pragma aborts_if_is_partial;
    ensures result.toNat = 4 * value.toNat

  verify quadruple

  -- An opaque function: callers see the contract; the body is verified.
  fun checked_succ (value : U64) : Action U64 := do
    if value ≥ E_LIMIT then
      abort E_LIMIT
    pure (value + 1)

  spec checked_succ (value : U64) where
    pragma opaque;
    ensures result = value + 1;
    aborts_if value ≥ E_LIMIT with E_LIMIT

  verify checked_succ

  fun succ_twice (value : U64) : Action U64 := do
    let once ← checked_succ value
    checked_succ once

  spec succ_twice (value : U64) where
    pragma aborts_if_is_partial;
    ensures result.toNat = value.toNat + 2;
    aborts_if value ≥ E_LIMIT with E_LIMIT

  verify succ_twice

  -- An opaque function the translator cannot follow (a local borrowed
  -- while the mutable parameter is live): its contract is assumed.
  native fun swap {T} (left : &mut T) (right : &mut T) : Action Unit

  spec swap {T} (left : &mut T) (right : &mut T) where
    ensures right = old(left) ∧ left = old(right);
    aborts_if False

  fun replace {T} (ref : &mut T) (new : T) : Action T := do
    let mut new' := new
    let ref0 ← &mut new'
    swap ref ref0
    new' ← *ref0
    pure new'

  spec replace {T} (ref : &mut T) (new : T) where
    pragma opaque;
    ensures result = old(ref) ∧ ref = new;
    aborts_if False

end Tests.MovePrograms.Summaries

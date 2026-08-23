-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: specification and verification.

import Move
import MoveModel.Tests.Common

/-! Immutable borrow, returned values, and conditional aborts from Move. -/

namespace Tests.MovePrograms

open Move
open scoped Move Move.Compiler Move.Spec

module Read where

  struct Reading has Key where
    value : U64

  def E_TOO_SMALL : U64 := 7

  /-! ## Functions -/

  fun read (addr : Address) : Action U64 := do
    let value ← &Reading[addr].value
    (*value)

  spec read (addr : Address) where
    requires existsAt<Reading>(addr);
    ensures
      result = old(Reading[addr].value);
    aborts_if False

  fun read_at_least (addr : Address) (minimum : U64) : Action U64 := do
    let value ← &Reading[addr].value
    let current ← *value
    if current < minimum then
      abort E_TOO_SMALL
    pure current

  spec read_at_least (addr : Address) (minimum : U64) where
    requires existsAt<Reading>(addr);
    ensures
      result = old(Reading[addr].value) ∧
      minimum.toNat ≤ result.toNat;
    aborts_if
      old(Reading[addr].value).toNat < minimum.toNat
      with E_TOO_SMALL

  /-! ## Proofs -/

  verify read by
    contract_intro
    rcases Option.isSome_iff_exists.mp permitted with ⟨reading, lookup⟩
    rw [Move.Verify.wp_bind, Move.Verify.wp_borrowSpec]
    simp [Move.Semantics.ResourceStore.get, lookup]

  verify read_at_least by
    contract_intro
    obtain ⟨addr, minimum⟩ := args
    rcases Option.isSome_iff_exists.mp permitted with ⟨reading, lookup⟩
    rw [Move.Verify.wp_bind, Move.Verify.wp_borrowSpec]
    refine ⟨fun owner ownerLookup => ?_, fun missing => by simp_all⟩
    rw [Move.Semantics.ResourceStore.descriptor_lookup, lookup] at ownerLookup
    injection ownerLookup with ownerEq
    subst ownerEq
    move_cases tooSmall : Move.Verify.Source.logicalLT reading.value minimum
    · simp [wp_norm, Move.Semantics.ResourceStore.get, lookup, tooSmall]
    · simp [wp_norm, Move.Semantics.ResourceStore.get, lookup]

  /-! ## Tests -/

  def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.Read

  private def readingId := compiled.resourceId "Reading"
  private def memory (addr value : Nat) : MoveModel.IR.IMem :=
    [(readingId, addr, .struct [.u64 value])]
  private def run := Tests.run compiled

  #test run "read" (memory 4 99) [.address 4] =
    Tests.okRet (memory 4 99) [.u64 99]
  #test run "read" [] [.address 4] = Tests.aborted 0
  #test run "read_at_least" (memory 4 10) [.address 4, .u64 10] =
    Tests.okRet (memory 4 10) [.u64 10]
  #test run "read_at_least" (memory 4 9) [.address 4, .u64 10] =
    Tests.abortedIn (memory 4 9) 7

end Tests.MovePrograms

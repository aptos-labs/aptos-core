-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move
import Tests.Common

/-! Immutable borrow, returned values, and conditional aborts from Move. -/

namespace Tests.MovePrograms

open Move
open MoveModel.Frontend.XIR
open scoped Move Move.Compiler Move.Spec

move_module Read where

  @[move_struct]
  structure Reading where
    value : U64
    deriving Key

  def E_TOO_SMALL : U64 := 7

  /-! ## Functions -/

  fun read (addr : Address) : Action U64 := do
    let value ← &Reading[addr].value
    (*value)

  spec read (addr : Address) where
    requires exists<Reading>(addr);
    ensures
      result = old(Reading[addr].value) ∧ final = initial;
    aborts_if False

  fun readAtLeast (addr : Address) (minimum : U64) : Action U64 := do
    let value ← &Reading[addr].value
    let current ← *value
    if current < minimum then
      abort E_TOO_SMALL
    pure current

  spec readAtLeast (addr : Address) (minimum : U64) where
    requires exists<Reading>(addr);
    ensures
      result = old(Reading[addr].value) ∧
      minimum.toNat ≤ result.toNat ∧
      final = initial;
    aborts_if
      old(Reading[addr].value).toNat < minimum.toNat
      with E_TOO_SMALL

  /-! ## Proofs -/

  verify read by
    contract_intro
    rcases Option.isSome_iff_exists.mp permitted with ⟨reading, lookup⟩
    rw [Move.Verify.wp_bind, Move.Verify.wp_borrowSpec]
    simp [Move.Semantics.ResourceStore.get, lookup]

  verify readAtLeast by
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
      omega

  /-! ## Tests -/

  def compiled : MModule := move_module% "ReadTest"

  private def readingId := compiled.resourceId "Reading"
  private def memory (addr value : Nat) : MoveModel.IR.IMem :=
    [(readingId, addr, .struct [.u64 value])]
  private def run := Tests.run compiled

  #test run "read" (memory 4 99) [.address 4] =
    Tests.okRet (memory 4 99) [.u64 99]
  #test run "read" [] [.address 4] = Tests.aborted 0
  #test run "readAtLeast" (memory 4 10) [.address 4, .u64 10] =
    Tests.okRet (memory 4 10) [.u64 10]
  #test run "readAtLeast" (memory 4 9) [.address 4, .u64 10] =
    Tests.abortedIn (memory 4 9) 7

end Tests.MovePrograms

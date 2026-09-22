-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: specification and verification.

import Move
import MoveModel.Tests.Common

/-! Calls between selected Leaner functions, including pure, effectful, and recursive helpers. -/

namespace Tests.MovePrograms

open Move
open scoped Move Move.Compiler Move.Spec

module Calls where

  /-! ## Functions -/

  struct Counter has Key where
    value : U64

  fun twice (value : U64) : U64 := value + value

  spec twice (value : U64) where
    ensures result = value + value;
    aborts_if ¬value.toNat + value.toNat < U64.size
      with Semantics.Checked.arithmeticAbortCode

  fun increment (value : U64) : Action U64 := do
    pure (value + 1)

  spec increment (value : U64) where
    ensures result = value + 1;
    aborts_if ¬value.toNat + 1 < U64.size
      with Semantics.Checked.arithmeticAbortCode

  fun increment_unspecified (value : U64) : Action U64 := do
    pure (value + 1)

  -- Declaring no abort condition leaves abort behavior uninterpreted: any
  -- abort code is permitted, but the postcondition still has to hold for
  -- every successful execution.
  spec increment_unspecified (value : U64) where
    requires value.toNat + 1 < U64.size;
    ensures result = value + 1

  fun pure_caller (value : U64) : Action U64 := do
    pure (twice value)

  fun effect_caller (value : U64) : Action U64 := do
    increment value

  spec effect_caller (value : U64) where
    ensures result = value + 1;
    aborts_if ¬value.toNat + 1 < U64.size
      with Semantics.Checked.arithmeticAbortCode

  fun composed (value : U64) : Action U64 := do
    let doubled := twice value
    increment doubled

  fun bound_caller (value : U64) : Action U64 := do
    let incremented ← increment value
    pure (twice incremented)

  /-- Minimal source-verification examples for compositional and recursive
  Move calls.  Keeping these independent of arithmetic makes failures point
  at call semantics rather than checked-integer side conditions. -/
  fun choose (flag : Bool) : Action U64 := do
    if flag then pure 7 else pure 8

  spec choose (flag : Bool) where
    ensures result = if flag then 7 else 8;
    aborts_if False

  fun call_choose (flag : Bool) : Action U64 := do
    choose flag

  spec call_choose (flag : Bool) where
    ensures result = if flag then 7 else 8;
    aborts_if False

  partial fun recursive_choose (done : Bool) : Action U64 := do
    if done then pure 7 else continue recursive_choose true

  spec recursive_choose (done : Bool) where
    ensures result = 7;
    aborts_if False

  partial fun ordinary_recursive_choose (done : Bool) : Action U64 := do
    if done then pure 7 else ordinary_recursive_choose true

  spec ordinary_recursive_choose (done : Bool) where
    ensures result = 7;
    aborts_if False

  partial fun sum_down (value : U64) : U64 :=
    if value < 1 then 0 else value + sum_down (value - 1)

  partial fun countdown (value accumulator : U64) : U64 :=
    if value < 1 then accumulator else continue countdown (value - 1) (accumulator + 1)

  partial fun alternate (remaining left right : U64) : U64 :=
    if remaining < 1 then left else continue alternate (remaining - 1) right left

  partial fun effect_countdown (value accumulator : U64) : Action U64 := do
    if value < 1 then
      pure accumulator
    else
      continue effect_countdown (value - 1) (accumulator + 1)

  partial fun unmarked_countdown (value accumulator : U64) : U64 :=
    if value < 1 then accumulator else unmarked_countdown (value - 1) (accumulator + 1)

  partial fun mixed_countdown (value accumulator : U64) : U64 :=
    if value < 1 then
      accumulator
    else if value < 2 then
      mixed_countdown (value - 1) (accumulator + 1)
    else
      continue mixed_countdown (value - 1) (accumulator + 1)

  mutual
    partial fun even_flag (value : U64) : U64 :=
      if value < 1 then 1 else odd_flag (value - 1)

    partial fun odd_flag (value : U64) : U64 :=
      if value < 1 then 0 else even_flag (value - 1)
  end

  friend fun add_to (addr : Address) (amount : U64) : Action Unit := do
    let value ← &mut Counter[addr].value
    value := *value + amount

  -- A family-level `modifies` leaves the family unconstrained while every
  -- other resource stays framed.
  spec add_to (addr : Address) (amount : U64) where
    requires existsAt<Counter>(addr);
    modifies Counter;
    ensures Counter[addr].value = old(Counter[addr].value) + amount;
    aborts_if ¬old(Counter[addr].value).toNat + amount.toNat < U64.size
      with Semantics.Checked.arithmeticAbortCode

  entry fun add_twice (addr : Address) (amount : U64) : Action Unit := do
    add_to addr (twice amount)

  entry fun add_twice_then_one (addr : Address) (amount : U64) : Action Unit := do
    add_twice addr amount
    add_to addr 1

  fun read_counter (addr : Address) : Action U64 := do
    let value ← &Counter[addr].value
    (*value)

  spec read_counter (addr : Address) where
    requires existsAt<Counter>(addr);
    ensures
      result = old(Counter[addr].value);
    aborts_if False

  fun forwarded_read (addr : Address) : Action U64 := do
    read_counter addr

  /-! ## Proofs -/

  verify twice

  verify increment

  verify increment_unspecified

  verify add_to

  verify effect_caller

  verify choose

  verify call_choose

  verify recursive_choose by
    contract_intro
    cases args with
    | false =>
        simpa [wp_norm] using Move.Verify.wp_of_satisfies
          (args := true) (initial := initial) recursiveVerified trivial
    | true => simp [wp_norm]

  verify ordinary_recursive_choose by
    contract_intro
    cases args with
    | false =>
        simpa [wp_norm] using Move.Verify.wp_of_satisfies
          (args := true) (initial := initial) recursiveVerified trivial
    | true => simp [wp_norm]

  verify read_counter by
    contract_intro
    rcases Option.isSome_iff_exists.mp permitted with ⟨counter, lookup⟩
    rw [Move.Verify.wp_bind, Move.Verify.wp_borrowSpec]
    refine ⟨fun owner ownerLookup => ?_, fun missing => by simp_all⟩
    rw [Move.Semantics.ResourceStore.descriptor_lookup, lookup] at ownerLookup
    injection ownerLookup with ownerEq
    subst ownerEq
    simp [wp_norm, Move.Semantics.ResourceStore.get, lookup]

  /-! ## Tests -/

  def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.Calls

  private def counterId := compiled.resourceId "Counter"
  private def memory (addr value : Nat) : MoveModel.IR.IMem :=
    [(counterId, addr, .struct [.u64 value])]
  private def run := Tests.run compiled

  private def invokes (callee : MoveModel.IR.FunId) : MoveModel.IR.Instr → Bool
    | .call _ (.function target) _ => target == callee
    | _ => false

  private def hasSelfCall (name : String) : Bool :=
    let id := compiled.funId name
    match compiled.program.funs id with
    | some decl => decl.blocksList.any fun block => block.instrs.any (invokes id)
    | none => false

  private def hasEntryBackEdge (name : String) : Bool :=
    let id := compiled.funId name
    match compiled.program.funs id with
    | some decl => decl.blocksList.any fun block => block.term == .jump decl.body.entry
    | none => false

  #test run "pure_caller" [] [.u64 7] = Tests.okU64 14
  #test run "effect_caller" [] [.u64 7] = Tests.okU64 8
  #test run "composed" [] [.u64 7] = Tests.okU64 15
  #test run "composed" [] [.u64 18446744073709551615] = Tests.aborted 0
  #test run "bound_caller" [] [.u64 7] = Tests.okU64 16
  #test run "call_choose" [] [.bool true] = Tests.okU64 7
  #test run "call_choose" [] [.bool false] = Tests.okU64 8
  #test run "recursive_choose" [] [.bool false] = Tests.okU64 7
  #test run "ordinary_recursive_choose" [] [.bool false] = Tests.okU64 7
  #test run "effect_caller" [] [.u64 18446744073709551615] = Tests.aborted 0
  #test run "sum_down" [] [.u64 5] = Tests.okU64 15
  #test run "countdown" [] [.u64 100, .u64 40] = Tests.okU64 140
  #test run "alternate" [] [.u64 3, .u64 10, .u64 20] = Tests.okU64 20
  #test run "alternate" [] [.u64 4, .u64 10, .u64 20] = Tests.okU64 10
  #test run "effect_countdown" [] [.u64 100, .u64 40] = Tests.okU64 140
  #test hasSelfCall "countdown" = false
  #test hasEntryBackEdge "countdown" = true
  #test hasSelfCall "alternate" = false
  #test hasEntryBackEdge "alternate" = true
  #test hasSelfCall "effect_countdown" = false
  #test hasEntryBackEdge "effect_countdown" = true
  #test hasSelfCall "unmarked_countdown" = true
  #test hasEntryBackEdge "unmarked_countdown" = false
  #test hasSelfCall "mixed_countdown" = true
  #test hasEntryBackEdge "mixed_countdown" = true
  #test hasSelfCall "sum_down" = true
  #test run "even_flag" [] [.u64 6] = Tests.okU64 1
  #test run "odd_flag" [] [.u64 6] = Tests.okU64 0
  #test run "add_twice" (memory 3 10) [.address 3, .u64 6]
    = Tests.okRet (memory 3 22) []
  #test run "add_twice" [] [.address 3, .u64 6] = Tests.aborted 0
  #test run "add_twice_then_one" (memory 3 10) [.address 3, .u64 6]
    = Tests.okRet (memory 3 23) []
  #test run "forwarded_read" (memory 3 41) [.address 3]
    = Tests.okRet (memory 3 41) [.u64 41]
  #test run "forwarded_read" [] [.address 3] = Tests.aborted 0

  -- The visibility keywords map onto the exported function metadata.
  #guard match compiled.funMeta? "add_to" with
    | some info => info.visibility == MoveModel.IR.Visibility.friend &&
        !info.isEntry
    | none => false
  #guard match compiled.funMeta? "add_twice" with
    | some info => info.visibility == MoveModel.IR.Visibility.public_ &&
        info.isEntry
    | none => false
  #guard match compiled.funMeta? "twice" with
    | some info => info.visibility == MoveModel.IR.Visibility.private_
    | none => false

end Tests.MovePrograms

-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move
import Tests.Common

/-! Calls between selected Leaner functions, including pure, effectful, and recursive helpers. -/

namespace Tests.MovePrograms

open Move
open MoveModel.Frontend.XIR
open scoped Move Move.Compiler Move.Spec

move_module Calls where

  /-! ## Functions -/

  struct Counter where
    value : U64
    deriving Key

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

  fun incrementUnspecified (value : U64) : Action U64 := do
    pure (value + 1)

  -- Declaring no abort condition leaves abort behavior uninterpreted: any
  -- abort code is permitted, but the postcondition still has to hold for
  -- every successful execution.
  spec incrementUnspecified (value : U64) where
    requires value.toNat + 1 < U64.size;
    ensures result = value + 1

  fun pureCaller (value : U64) : Action U64 := do
    pure (twice value)

  fun effectCaller (value : U64) : Action U64 := do
    increment value

  spec effectCaller (value : U64) where
    ensures result = value + 1;
    aborts_if ¬value.toNat + 1 < U64.size
      with Semantics.Checked.arithmeticAbortCode

  fun composed (value : U64) : Action U64 := do
    let doubled := twice value
    increment doubled

  fun boundCaller (value : U64) : Action U64 := do
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

  fun callChoose (flag : Bool) : Action U64 := do
    choose flag

  spec callChoose (flag : Bool) where
    ensures result = if flag then 7 else 8;
    aborts_if False

  partial fun recursiveChoose (done : Bool) : Action U64 := do
    if done then pure 7 else continue recursiveChoose true

  spec recursiveChoose (done : Bool) where
    ensures result = 7;
    aborts_if False

  partial fun ordinaryRecursiveChoose (done : Bool) : Action U64 := do
    if done then pure 7 else ordinaryRecursiveChoose true

  spec ordinaryRecursiveChoose (done : Bool) where
    ensures result = 7;
    aborts_if False

  partial fun sumDown (value : U64) : U64 :=
    if value < 1 then 0 else value + sumDown (value - 1)

  partial fun countdown (value accumulator : U64) : U64 :=
    if value < 1 then accumulator else continue countdown (value - 1) (accumulator + 1)

  partial fun alternate (remaining left right : U64) : U64 :=
    if remaining < 1 then left else continue alternate (remaining - 1) right left

  partial fun effectCountdown (value accumulator : U64) : Action U64 := do
    if value < 1 then
      pure accumulator
    else
      continue effectCountdown (value - 1) (accumulator + 1)

  partial fun unmarkedCountdown (value accumulator : U64) : U64 :=
    if value < 1 then accumulator else unmarkedCountdown (value - 1) (accumulator + 1)

  partial fun mixedCountdown (value accumulator : U64) : U64 :=
    if value < 1 then
      accumulator
    else if value < 2 then
      mixedCountdown (value - 1) (accumulator + 1)
    else
      continue mixedCountdown (value - 1) (accumulator + 1)

  mutual
    partial fun evenFlag (value : U64) : U64 :=
      if value < 1 then 1 else oddFlag (value - 1)

    partial fun oddFlag (value : U64) : U64 :=
      if value < 1 then 0 else evenFlag (value - 1)
  end

  friend fun addTo (addr : Address) (amount : U64) : Action Unit := do
    let value ← &mut Counter[addr].value
    value := *value + amount

  -- A family-level `modifies` leaves the family unconstrained while every
  -- other resource stays framed.
  spec addTo (addr : Address) (amount : U64) where
    requires exists<Counter>(addr);
    modifies Counter;
    ensures Counter[addr].value = old(Counter[addr].value) + amount;
    aborts_if ¬old(Counter[addr].value).toNat + amount.toNat < U64.size
      with Semantics.Checked.arithmeticAbortCode

  entry fun addTwice (addr : Address) (amount : U64) : Action Unit := do
    addTo addr (twice amount)

  entry fun addTwiceThenOne (addr : Address) (amount : U64) : Action Unit := do
    addTwice addr amount
    addTo addr 1

  fun readCounter (addr : Address) : Action U64 := do
    let value ← &Counter[addr].value
    (*value)

  spec readCounter (addr : Address) where
    requires exists<Counter>(addr);
    ensures
      result = old(Counter[addr].value);
    aborts_if False

  fun forwardedRead (addr : Address) : Action U64 := do
    readCounter addr

  /-! ## Proofs -/

  verify twice

  verify increment

  verify incrementUnspecified

  verify addTo

  verify effectCaller

  verify choose

  verify callChoose

  verify recursiveChoose by
    contract_intro
    cases args with
    | false =>
        simpa [wp_norm] using Move.Verify.wp_of_satisfies
          (args := true) (initial := initial) recursiveVerified trivial
    | true => simp [wp_norm]

  verify ordinaryRecursiveChoose by
    contract_intro
    cases args with
    | false =>
        simpa [wp_norm] using Move.Verify.wp_of_satisfies
          (args := true) (initial := initial) recursiveVerified trivial
    | true => simp [wp_norm]

  verify readCounter by
    contract_intro
    rcases Option.isSome_iff_exists.mp permitted with ⟨counter, lookup⟩
    rw [Move.Verify.wp_bind, Move.Verify.wp_borrowSpec]
    refine ⟨fun owner ownerLookup => ?_, fun missing => by simp_all⟩
    rw [Move.Semantics.ResourceStore.descriptor_lookup, lookup] at ownerLookup
    injection ownerLookup with ownerEq
    subst ownerEq
    simp [wp_norm, Move.Semantics.ResourceStore.get, lookup]

  /-! ## Tests -/

  def compiled : MModule := move_module% "CallsTest"

  private def counterId := compiled.resourceId "Counter"
  private def memory (addr value : Nat) : MoveModel.IR.IMem :=
    [(counterId, addr, .struct [.u64 value])]
  private def run := Tests.run compiled

  private def invokes (callee : MoveModel.IR.FunId) : MoveModel.IR.Instr → Bool
    | .call _ (.function target) _ => target == callee
    | _ => false

  private def hasSelfCall (name : String) : Bool :=
    let id := compiled.funId name
    match compiled.funs[id]? with
    | some decl => decl.blocks.any fun block => block.instrs.any (invokes id)
    | none => false

  private def hasEntryBackEdge (name : String) : Bool :=
    let id := compiled.funId name
    match compiled.funs[id]? with
    | some decl => decl.blocks.any fun block => block.term == .jump decl.entry
    | none => false

  #test run "pureCaller" [] [.u64 7] = Tests.okU64 14
  #test run "effectCaller" [] [.u64 7] = Tests.okU64 8
  #test run "composed" [] [.u64 7] = Tests.okU64 15
  #test run "composed" [] [.u64 18446744073709551615] = Tests.aborted 0
  #test run "boundCaller" [] [.u64 7] = Tests.okU64 16
  #test run "callChoose" [] [.bool true] = Tests.okU64 7
  #test run "callChoose" [] [.bool false] = Tests.okU64 8
  #test run "recursiveChoose" [] [.bool false] = Tests.okU64 7
  #test run "ordinaryRecursiveChoose" [] [.bool false] = Tests.okU64 7
  #test run "effectCaller" [] [.u64 18446744073709551615] = Tests.aborted 0
  #test run "sumDown" [] [.u64 5] = Tests.okU64 15
  #test run "countdown" [] [.u64 100, .u64 40] = Tests.okU64 140
  #test run "alternate" [] [.u64 3, .u64 10, .u64 20] = Tests.okU64 20
  #test run "alternate" [] [.u64 4, .u64 10, .u64 20] = Tests.okU64 10
  #test run "effectCountdown" [] [.u64 100, .u64 40] = Tests.okU64 140
  #test hasSelfCall "countdown" = false
  #test hasEntryBackEdge "countdown" = true
  #test hasSelfCall "alternate" = false
  #test hasEntryBackEdge "alternate" = true
  #test hasSelfCall "effectCountdown" = false
  #test hasEntryBackEdge "effectCountdown" = true
  #test hasSelfCall "unmarkedCountdown" = true
  #test hasEntryBackEdge "unmarkedCountdown" = false
  #test hasSelfCall "mixedCountdown" = true
  #test hasEntryBackEdge "mixedCountdown" = true
  #test hasSelfCall "sumDown" = true
  #test run "evenFlag" [] [.u64 6] = Tests.okU64 1
  #test run "oddFlag" [] [.u64 6] = Tests.okU64 0
  #test run "addTwice" (memory 3 10) [.address 3, .u64 6]
    = Tests.okRet (memory 3 22) []
  #test run "addTwice" [] [.address 3, .u64 6] = Tests.aborted 0
  #test run "addTwiceThenOne" (memory 3 10) [.address 3, .u64 6]
    = Tests.okRet (memory 3 23) []
  #test run "forwardedRead" (memory 3 41) [.address 3]
    = Tests.okRet (memory 3 41) [.u64 41]
  #test run "forwardedRead" [] [.address 3] = Tests.aborted 0

  -- The visibility keywords map onto the exported function metadata.
  #guard match compiled.funMeta.find? (·.name == "addTo") with
    | some info => info.visibility == MoveModel.IR.Visibility.friend &&
        !info.isEntry
    | none => false
  #guard match compiled.funMeta.find? (·.name == "addTwice") with
    | some info => info.visibility == MoveModel.IR.Visibility.public_ &&
        info.isEntry
    | none => false
  #guard match compiled.funMeta.find? (·.name == "twice") with
    | some info => info.visibility == MoveModel.IR.Visibility.private_
    | none => false

  #emit_leaner_xir compiled

end Tests.MovePrograms

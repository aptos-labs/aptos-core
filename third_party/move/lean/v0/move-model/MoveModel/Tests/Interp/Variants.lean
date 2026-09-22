-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Tests.Common
import MoveModel.IR.Interp.Exec

/-!
# Interpreter Tests: Enum Payload References

`borrow_variant_field` (the field of a listed variant through a reference,
aborting for any other variant) and `test_variant_ref` (the variant test of a
referent), with reads and writes through the borrowed payload.
-/

namespace Tests.Interp.Variants

open MoveModel.IR

/-- A one-block function over a program with one enum declaration:
resource 0 is `Option(U64)` — variant 0 `None`, variant 1 `Some(value)`. -/
private def fn (numParams : Nat) (locals : List Ty) (returns : List Ty)
    (instrs : List Instr) (rets : List LocalIndex) : FunDecl where
  numParams := numParams
  numLocals := locals.length
  locals := fun t => locals[t]?
  returns := returns
  body :=
    { blocks := fun b => if b = 0 then some ⟨instrs, .ret rets⟩ else none
      entry := 0
      size := 1 }
  loopSpecs := fun _ => none
  contract :=
    { requires := .value (.bool true)
      aborts := none
      ensures := .value (.bool true)
      modifies := [] }

private def optionDecl : StructDecl :=
  { fields := [], variants := some [[], [.u64]] }

/-- Run a single hand-written function on empty memory. -/
private def run1 (d : FunDecl) (args : List Value) : Tests.Outcome :=
  interpFun ⟨fun f => if f = 0 then some d else none,
    fun r => if r = 0 then some optionDecl else none⟩ 1000 0 [] args

private def someVal (n : Nat) : Value := .variant 1 [.u64 n]
private def noneVal : Value := .variant 0 []

/-- Read the payload of `Some` through the variant field borrow. -/
private def readPayload : FunDecl :=
  fn 1 [.enum 0, .mutRef (.enum 0), .mutRef .u64, .u64] [.u64]
    [.call [1] .borrowLoc [0],
     .call [2] (.borrowVariantField [1] 0) [1],
     .call [3] .readRef [2]] [3]

#test run1 readPayload [someVal 7] = okU64 7
#test run1 readPayload [noneVal] = aborted runtimeAbortCode

/-- Write the payload of `Some` through the variant field borrow. -/
private def writePayload : FunDecl :=
  fn 2 [.enum 0, .u64, .mutRef (.enum 0), .mutRef .u64] [.enum 0]
    [.call [2] .borrowLoc [0],
     .call [3] (.borrowVariantField [1] 0) [2],
     .call [] .writeRef [3, 1]] [0]

#test run1 writePayload [someVal 7, .u64 9] = okVals [someVal 9]
#test run1 writePayload [noneVal, .u64 9] = aborted runtimeAbortCode

/-- The variant test through a reference. -/
private def isSome : FunDecl :=
  fn 1 [.enum 0, .ref (.enum 0), .bool] [.bool]
    [.call [1] .borrowLoc [0],
     .call [2] (.testVariantRef 1) [1]] [2]

#test run1 isSome [someVal 3] = okVals [.bool true]
#test run1 isSome [noneVal] = okVals [.bool false]

end Tests.Interp.Variants

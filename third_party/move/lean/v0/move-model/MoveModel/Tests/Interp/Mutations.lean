-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Tests.Common
import MoveModel.IR.Interp.Exec

/-!
# Interpreter Tests: The Mutation Algebra

The mutation operations (`mk_mut_loc` … `mut_addr`) — the `$Mutation`
algebra of the Boogie prelude that the full reference elimination emits.
Hand-written IR (frontends never produce these operations): checkout,
sub-mutation derivation by field and by dynamic index, read/replace,
`is_parent` dispatch tests, dynamic-index recovery, root identification,
and a complete manual write-back cycle of the shape the elimination
generates.
-/

namespace Tests.Interp.Mutations

open MoveModel.IR

/-- A one-block function (parameters are the prefix of `locals`). -/
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

private def run1 (d : FunDecl) (mem : IMem) (args : List Value) :
    Tests.Outcome :=
  interpFun ⟨fun f => if f = 0 then some d else none, fun _ => none⟩
    1000 0 mem args

/-! ## Checkout and read/replace -/

private def checkoutRead : FunDecl :=
  fn 1 [.u64, .mutRef .u64, .u64] [.u64]
    [.call [1] (.mkMutLoc 0) [0],
     .call [2] .getMut [1]] [2]

#test run1 checkoutRead [] [.u64 7] = okU64 7

private def setThenGet : FunDecl :=
  fn 2 [.u64, .u64, .mutRef .u64, .u64] [.u64]
    [.call [2] (.mkMutLoc 0) [0],
     .call [2] .setMut [2, 1],
     .call [3] .getMut [2]] [3]

#test run1 setThenGet [] [.u64 1, .u64 9] = okU64 9

/-! ## Sub-mutations: static field and dynamic index -/

private def fieldChild : FunDecl :=
  fn 1 [.struct 0, .mutRef (.struct 0), .mutRef .u64, .u64] [.u64]
    [.call [1] (.mkMutLoc 0) [0],
     .call [2] (.childMutField 1) [1],
     .call [3] .getMut [2]] [3]

#test run1 fieldChild [] [.struct [.u64 1, .u64 42]] = okU64 42

private def indexChild : FunDecl :=
  fn 2 [.vector .u64, .u64, .mutRef (.vector .u64), .mutRef .u64, .u64]
    [.u64]
    [.call [2] (.mkMutLoc 0) [0],
     .call [3] .childMutIndex [2, 1],
     .call [4] .getMut [3]] [4]

#test run1 indexChild [] [.vector [.u64 10, .u64 20], .u64 1] = okU64 20
#test run1 indexChild [] [.vector [.u64 10], .u64 3] = aborted 0

/-! ## Global checkout (aborts like `borrow_global` when absent) -/

private def globalCheckout : FunDecl :=
  fn 1 [.address, .mutRef (.struct 0), .struct 0] [.struct 0]
    [.call [1] (.mkMutGlobal 0) [0],
     .call [2] .getMut [1]] [2]

#test run1 globalCheckout [(0, 5, .struct [.u64 3])] [.address 5]
  = okRet [(0, 5, .struct [.u64 3])] [.struct [.u64 3]]
#test run1 globalCheckout [] [.address 5] = aborted 0

/-! ## `is_parent` dispatch, dynamic-index recovery, root tests -/

private def parentTests : FunDecl :=
  fn 1 [.struct 0, .mutRef (.struct 0), .mutRef .u64,
        .bool, .bool, .bool, .bool, .bool] [.bool, .bool, .bool, .bool, .bool]
    [.call [1] (.mkMutLoc 0) [0],
     .call [2] (.childMutField 1) [1],
     .call [3] (.isParent [some 1]) [1, 2],
     .call [4] (.isParent [some 0]) [1, 2],
     .call [5] (.isParent []) [1, 1],
     .call [6] (.isMutLoc 0) [1],
     .call [7] (.isMutLoc 3) [1]] [3, 4, 5, 6, 7]

#test run1 parentTests [] [.struct [.u64 1, .u64 2]]
  = okVals [.bool true, .bool false, .bool true, .bool true, .bool false]

private def indexRecovery : FunDecl :=
  fn 2 [.vector .u64, .u64, .mutRef (.vector .u64), .mutRef .u64,
        .bool, .u64] [.bool, .u64]
    [.call [2] (.mkMutLoc 0) [0],
     .call [3] .childMutIndex [2, 1],
     .call [4] (.isParent [none]) [2, 3],
     .call [5] (.mutPathIndex 0) [2, 3]] [4, 5]

#test run1 indexRecovery [] [.vector [.u64 10, .u64 20, .u64 30], .u64 2]
  = okVals [.bool true, .u64 2]

private def globalRoot : FunDecl :=
  fn 1 [.address, .mutRef (.struct 0), .bool, .bool, .address]
    [.bool, .bool, .address]
    [.call [1] (.mkMutGlobal 0) [0],
     .call [2] (.isMutGlobal 0) [1],
     .call [3] (.isMutGlobal 1) [1],
     .call [4] .mutAddr [1]] [2, 3, 4]

#test run1 globalRoot [(0, 5, .struct [.u64 1])] [.address 5]
  = okRet [(0, 5, .struct [.u64 1])] [.bool true, .bool false, .address 5]

/-! ## A complete manual write-back cycle (the elimination's shape) -/

private def writeBackCycle : FunDecl :=
  fn 2 [.struct 0, .u64,
        .mutRef (.struct 0), .mutRef .u64,
        .struct 0, .u64, .struct 0, .struct 0] [.struct 0]
    [.call [2] (.mkMutLoc 0) [0],
     .call [3] (.childMutField 0) [2],
     .call [3] .setMut [3, 1],
     -- write-back child → parent (field edge 0)
     .call [4] .getMut [2],
     .call [5] .getMut [3],
     .call [6] (.updateField 0) [4, 5],
     .call [2] .setMut [2, 6],
     -- write-back parent → local root
     .call [7] .getMut [2],
     .assign 0 7] [0]

#test run1 writeBackCycle [] [.struct [.u64 1, .u64 2], .u64 99]
  = okVals [.struct [.u64 99, .u64 2]]

/-! ## The container discipline: mutations are not storable values -/

private def mutIntoVec : FunDecl :=
  fn 1 [.u64, .mutRef .u64, .vector .u64] [.vector .u64]
    [.call [1] (.mkMutLoc 0) [0],
     .call [2] .vecPack [1]] [2]

#test run1 mutIntoVec [] [.u64 1] matches .error (.stuck _)

end Tests.Interp.Mutations

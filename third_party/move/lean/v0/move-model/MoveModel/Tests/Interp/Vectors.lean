-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Tests.Common
import MoveModel.IR.Interp.Exec

/-!
# Interpreter Tests: Vectors

The vector value operations (`vec_pack`/`vec_len`/`vec_get`/`vec_set`/
`vec_push`/`vec_pop`/`vec_insert`/`vec_remove`) and element borrows
(`borrow_vec_elem` + read/write through the reference), on normal and abort
paths. These programs exercise the hand-written IR layer directly; source
coverage lives in `Tests.Move.VectorOperations`.
-/

namespace Tests.Interp.Vectors

open MoveModel.IR

/-- A one-block function: parameters are the prefix of `locals`, the block
runs `instrs` and returns the locals `rets`. -/
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

/-- Run a single hand-written function on empty memory. -/
private def run1 (d : FunDecl) (args : List Value) : Tests.Outcome :=
  interpFun ⟨fun f => if f = 0 then some d else none, fun _ => none⟩
    1000 0 [] args

/-- A `vector<u64>` value. -/
private def vecU64 (ns : List Nat) : Value := .vector (ns.map .u64)

/-! ## `vec_pack` and `vec_len` -/

private def pack2 : FunDecl :=
  fn 2 [.u64, .u64, .vector .u64] [.vector .u64]
    [.call [2] .vecPack [0, 1]] [2]

#test run1 pack2 [.u64 5, .u64 7] = okVals [vecU64 [5, 7]]

private def packEmpty : FunDecl :=
  fn 0 [.vector .u64] [.vector .u64]
    [.call [0] .vecPack []] [0]

#test run1 packEmpty [] = okVals [.vector []]

private def len : FunDecl :=
  fn 1 [.vector .u64, .u64] [.u64]
    [.call [1] .vecLen [0]] [1]

#test run1 len [vecU64 [10, 20, 30]] = okU64 3
#test run1 len [.vector []] = okU64 0

/-- Vector observations work directly through a reference; no owned vector is
materialized merely to ask for its length. -/
private def borrowedLen : FunDecl :=
  fn 1 [.vector .u64, .mutRef (.vector .u64), .ref (.vector .u64), .u64]
    [.u64]
    [.call [1] .borrowLoc [0],
     .call [2] .freezeRef [1],
     .call [3] .vecLen [2]] [3]

#test run1 borrowedLen [vecU64 [10, 20, 30]] = okU64 3

/-! ## `vec_get` and `vec_set` (abort out of range) -/

private def get : FunDecl :=
  fn 2 [.vector .u64, .u64, .u64] [.u64]
    [.call [2] .vecGet [0, 1]] [2]

#test run1 get [vecU64 [10, 20, 30], .u64 1] = okU64 20
#test run1 get [vecU64 [10], .u64 1] = aborted 0x20000

private def set : FunDecl :=
  fn 3 [.vector .u64, .u64, .u64, .vector .u64] [.vector .u64]
    [.call [3] .vecSet [0, 1, 2]] [3]

#test run1 set [vecU64 [1, 2], .u64 0, .u64 9] = okVals [vecU64 [9, 2]]
#test run1 set [vecU64 [1, 2], .u64 2, .u64 9] = aborted 0x20000

/-! ## `vec_push` and `vec_pop` (abort on the empty vector) -/

private def push : FunDecl :=
  fn 2 [.vector .u64, .u64, .vector .u64] [.vector .u64]
    [.call [2] .vecPush [0, 1]] [2]

#test run1 push [vecU64 [1, 2], .u64 3] = okVals [vecU64 [1, 2, 3]]
#test run1 push [.vector [], .u64 1] = okVals [vecU64 [1]]

private def pop : FunDecl :=
  fn 1 [.vector .u64, .vector .u64, .u64] [.vector .u64, .u64]
    [.call [1, 2] .vecPop [0]] [1, 2]

#test run1 pop [vecU64 [1, 2, 3]] = okVals [vecU64 [1, 2], .u64 3]
#test run1 pop [.vector []] = aborted 0x20000

/-! ## Indexed insertion and stable removal -/

private def insert : FunDecl :=
  fn 3 [.vector .u64, .u64, .u64, .vector .u64] [.vector .u64]
    [.call [3] .vecInsert [0, 1, 2]] [3]

#test run1 insert [vecU64 [10, 30], .u64 1, .u64 20]
  = okVals [vecU64 [10, 20, 30]]
#test run1 insert [vecU64 [20], .u64 0, .u64 10]
  = okVals [vecU64 [10, 20]]
#test run1 insert [vecU64 [20], .u64 1, .u64 30]
  = okVals [vecU64 [20, 30]]
#test run1 insert [vecU64 [20], .u64 2, .u64 30] = aborted 0x20000

private def remove : FunDecl :=
  fn 2 [.vector .u64, .u64, .vector .u64, .u64]
    [.vector .u64, .u64]
    [.call [2, 3] .vecRemove [0, 1]] [2, 3]

#test run1 remove [vecU64 [10, 20, 30], .u64 1]
  = okVals [vecU64 [10, 30], .u64 20]
#test run1 remove [vecU64 [10], .u64 0]
  = okVals [vecU64 [], .u64 10]
#test run1 remove [vecU64 [10], .u64 1] = aborted 0x20000

/-! ## Structural equality (vectors are compared elementwise) -/

private def eqv : FunDecl :=
  fn 2 [.vector .u64, .vector .u64, .bool] [.bool]
    [.call [2] .eq [0, 1]] [2]

#test run1 eqv [vecU64 [1, 2], vecU64 [1, 2]] = okBool true
#test run1 eqv [vecU64 [1, 2], vecU64 [1, 3]] = okBool false
#test run1 eqv [vecU64 [1], vecU64 [1, 1]] = okBool false

/- Move compares references by comparing their referents. This applies to
both equality and the language's generic structural ordering. -/
private def compareRefs : FunDecl :=
  fn 2 [.u64, .u64, .mutRef .u64, .mutRef .u64, .ref .u64, .ref .u64,
        .bool, .bool]
    [.bool, .bool]
    [.call [2] .borrowLoc [0],
     .call [3] .borrowLoc [1],
     .call [4] .freezeRef [2],
     .call [5] .freezeRef [3],
     .call [6] .eq [4, 5],
     .call [7] .lt [4, 5]] [6, 7]

#test run1 compareRefs [.u64 3, .u64 3] = okVals [.bool true, .bool false]
#test run1 compareRefs [.u64 2, .u64 3] = okVals [.bool false, .bool true]

/-! ## Element borrows: read and write through `borrow_vec_elem` -/

private def readElem : FunDecl :=
  fn 2 [.vector .u64, .u64, .mutRef (.vector .u64), .mutRef .u64, .u64]
    [.u64]
    [.call [2] .borrowLoc [0],
     .call [3] .borrowVecElem [2, 1],
     .call [4] .readRef [3]] [4]

#test run1 readElem [vecU64 [10, 20, 30], .u64 2] = okU64 30
#test run1 readElem [vecU64 [10], .u64 5] = aborted 0x20000

private def writeElem : FunDecl :=
  fn 3 [.vector .u64, .u64, .u64, .mutRef (.vector .u64), .mutRef .u64]
    [.vector .u64]
    [.call [3] .borrowLoc [0],
     .call [4] .borrowVecElem [3, 1],
     .call [] .writeRef [4, 2]] [0]

#test run1 writeElem [vecU64 [1, 2, 3], .u64 1, .u64 9]
  = okVals [vecU64 [1, 9, 3]]

/-- A borrow chain through a vector *of structs*: bump `v[0]`'s field. -/
private def bumpField : FunDecl :=
  fn 1 [.vector (.struct 0), .u64, .mutRef (.vector (.struct 0)),
        .mutRef (.struct 0), .mutRef .u64, .u64, .u64, .u64]
    [.vector (.struct 0)]
    [.load 1 (.u64 0),
     .call [2] .borrowLoc [0],
     .call [3] .borrowVecElem [2, 1],
     .call [4] (.borrowField 0) [3],
     .call [5] .readRef [4],
     .load 6 (.u64 1),
     .call [7] (.add .u64) [5, 6],
     .call [] .writeRef [4, 7]] [0]

#test run1 bumpField [.vector [.struct [.u64 41]]]
  = okVals [.vector [.struct [.u64 42]]]

/-! ## The container discipline: references do not enter vectors -/

private def packRef : FunDecl :=
  fn 1 [.u64, .mutRef .u64, .vector .u64] [.vector .u64]
    [.call [1] .borrowLoc [0],
     .call [2] .vecPack [1]] [2]

#test run1 packRef [.u64 1] matches .error (.stuck _)

end Tests.Interp.Vectors

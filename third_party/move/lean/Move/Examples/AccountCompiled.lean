-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Examples.Account
import Move.Compiler.Elab

/-! End-to-end golden test: Lean source → named LIR → modeled Move IR. -/

namespace Move.Examples.Account

open scoped Move Move.Compiler
open MoveModel.IR MoveModel.Frontend.XIR

/-- Pin reference notation in type position independently of place borrows. -/
private def immutableRefType (ref : &U64) : &U64 := ref
private def mutableRefType (ref : &mut U64) : &mut U64 := ref

def compiled : MModule :=
  move_module% "Account" structs [BalanceValue, Balance] functions [deposit, withdraw]

def program : Program := compiled.toProgram

/-- The reassignment elaborator falls back to Lean for ordinary mutable
locals; only `MutRef` locals are interpreted as writes. -/
private def ordinaryLocalAssignment : Nat := Id.run do
  let mut value := 1
  value := 2
  return value

#guard ordinaryLocalAssignment == 2
#guard compiled.structs.length == 2
#guard compiled.funs.length == 2
#guard match compiled.funs with
  | depositM :: _ => depositM.params == 2 && depositM.blocks == [{
      instrs := [
        .call [2] (.borrowGlobal 1) [0],
        .call [3] (.borrowField 0) [2],
        .call [4] (.borrowField 0) [3],
        .call [5] .readRef [4],
        .call [6] (.add .u64) [5, 1],
        .call [] .writeRef [4, 6]
      ]
      term := .ret []
    }]
  | _ => false
#guard match compiled.funs with
  | _ :: withdrawM :: _ =>
      withdrawM.blocks.length == 4 &&
      match withdrawM.blocks with
      | entry :: _ :: _ :: abortBlock :: _ =>
          entry.term == .branch 8 3 1 && abortBlock.term == .abort 9
      | _ => false
  | _ => false

end Move.Examples.Account

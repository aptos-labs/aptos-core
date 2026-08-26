-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Frontend.Elab
import MoveModel.Tests.XIRCommon
import MoveModel.IR.Interp.Exec

/-!
# Interpreter Tests: Structs

Pack, unpack, field access (compiler v2 lowers value field reads to borrow
chains, so these exercise the reference semantics implicitly), nested
structs, tuple returns, and structural equality.
-/

namespace Tests.Interp.Structs

open MoveModel.IR
open MoveModel.Frontend.XIR

def structsM : MProgram := moveM% "
module 0x42::structs {
    struct Inner has copy, drop { v: u64 }
    struct Pair has copy, drop { a: u64, b: Inner }

    fun mk(x: u64, y: u64): Pair { Pair { a: x, b: Inner { v: y } } }
    fun get_a(p: Pair): u64 { p.a }
    fun get_inner_v(p: Pair): u64 { p.b.v }
    fun take_apart(p: Pair): (u64, u64) {
        let Pair { a, b } = p;
        let Inner { v } = b;
        (a, v)
    }
    fun eq_pair(p: Pair, q: Pair): bool { p == q }
}
"

def structs : Program := structsM.toProgram

def run (f : String) (args : List Value) : Outcome :=
  Tests.runXIR structsM f [] args

/-- A `Pair { a: x, b: Inner { v: y } }` value (fields in offset order). -/
def pair (x y : Nat) : Value := .struct [.u64 x, .struct [.u64 y]]

/-! ## Pack -/

#test run "mk" [.u64 1, .u64 2]
  = okVals [.struct [.u64 1, .struct [.u64 2]]]

/-! ## Field access (borrow chains on values) -/

#test run "get_a" [pair 7 8] = okU64 7
#test run "get_inner_v" [pair 7 8] = okU64 8

/-! ## Unpack and tuple return -/

#test run "take_apart" [pair 3 4] = okVals [.u64 3, .u64 4]

/-! ## Structural equality -/

#test run "eq_pair" [pair 1 2, pair 1 2] = okBool true
#test run "eq_pair" [pair 1 2, pair 1 3] = okBool false

end Tests.Interp.Structs

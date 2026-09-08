-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Frontend.Elab
import MoveModel.Tests.XIRCommon
import MoveModel.IR.Interp.Exec

/-!
# Interpreter Tests: Arithmetic, Comparisons, Boolean Operations

`u64` arithmetic with Move's aborting semantics (overflow, underflow,
division by zero), the comparison operators — including `>`/`>=`, which the
exchange producer normalizes to `<`/`<=` with swapped operands, and `!=`,
which it lowers to `==` plus `!` through a synthesized local — and the
boolean connectives.
-/

namespace Tests.Interp.Arith

open MoveModel.IR
open MoveModel.Frontend.XIR

def arithM : MProgram := moveM% "
module 0x42::arith {
    fun add(x: u64, y: u64): u64 { x + y }
    fun sub(x: u64, y: u64): u64 { x - y }
    fun mul(x: u64, y: u64): u64 { x * y }
    fun div(x: u64, y: u64): u64 { x / y }
    fun rem(x: u64, y: u64): u64 { x % y }
    fun lt(x: u64, y: u64): bool { x < y }
    fun le(x: u64, y: u64): bool { x <= y }
    fun gt(x: u64, y: u64): bool { x > y }
    fun ge(x: u64, y: u64): bool { x >= y }
    fun eq(x: u64, y: u64): bool { x == y }
    fun neq(x: u64, y: u64): bool { x != y }
    fun band(x: bool, y: bool): bool { x && y }
    fun bor(x: bool, y: bool): bool { x || y }
    fun bnot(x: bool): bool { !x }
}
"

def arith : Program := arithM.toProgram

def run (f : String) (args : List Value) : Outcome :=
  Tests.runXIR arithM f [] args

/-! ## Addition: result, and abort (code 0) on overflow -/

#test run "add" [.u64 2, .u64 3] = okU64 5
#test run "add" [.u64 18446744073709551615, .u64 0]
  = okU64 18446744073709551615
#test run "add" [.u64 18446744073709551615, .u64 1] = aborted 0

/-! ## Subtraction: result, and abort on underflow -/

#test run "sub" [.u64 7, .u64 3] = okU64 4
#test run "sub" [.u64 3, .u64 3] = okU64 0
#test run "sub" [.u64 2, .u64 3] = aborted 0

/-! ## Multiplication: result, and abort on overflow -/

#test run "mul" [.u64 6, .u64 7] = okU64 42
#test run "mul" [.u64 18446744073709551615, .u64 2] = aborted 0

/-! ## Division and remainder: results, and abort on zero divisor -/

#test run "div" [.u64 17, .u64 5] = okU64 3
#test run "div" [.u64 17, .u64 0] = aborted 0
#test run "rem" [.u64 17, .u64 5] = okU64 2
#test run "rem" [.u64 17, .u64 0] = aborted 0

/-! ## Comparisons (`>`/`>=` are operand-swapped `<`/`<=`) -/

#test run "lt" [.u64 1, .u64 2] = okBool true
#test run "lt" [.u64 2, .u64 2] = okBool false
#test run "le" [.u64 2, .u64 2] = okBool true
#test run "le" [.u64 3, .u64 2] = okBool false
#test run "gt" [.u64 3, .u64 2] = okBool true
#test run "gt" [.u64 2, .u64 2] = okBool false
#test run "ge" [.u64 2, .u64 2] = okBool true
#test run "ge" [.u64 1, .u64 2] = okBool false

/-! ## Equality and the `!=` lowering -/

#test run "eq" [.u64 5, .u64 5] = okBool true
#test run "eq" [.u64 5, .u64 6] = okBool false
#test run "neq" [.u64 5, .u64 6] = okBool true
#test run "neq" [.u64 5, .u64 5] = okBool false

/-! ## Boolean connectives -/

#test run "band" [.bool true, .bool true] = okBool true
#test run "band" [.bool true, .bool false] = okBool false
#test run "bor" [.bool false, .bool true] = okBool true
#test run "bor" [.bool false, .bool false] = okBool false
#test run "bnot" [.bool false] = okBool true
#test run "bnot" [.bool true] = okBool false

/-! ## Ill-typed configurations are stuck, not aborted -/

#test run "add" [.bool true, .u64 1] matches .error (.stuck _)
#test run "eq" [.u64 1, .bool true] matches .error (.stuck _)

end Tests.Interp.Arith

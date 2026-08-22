-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Frontend.Elab
import MoveModel.Tests.XIRCommon
import MoveModel.IR.Interp.Exec

/-!
# Interpreter Tests: Control Flow, Calls, Aborts, Fuel

Branches, `while` loops, recursion (intra-module calls), user abort codes
(`abort`/`assert!` — the code is observable in the outcome, unlike runtime
aborts which carry code 0), and fuel exhaustion on divergence.
-/

namespace Tests.Interp.ControlFlow

open MoveModel.IR
open MoveModel.Frontend.XIR

def controlM : MProgram := moveM% "
module 0x42::control {
    fun sum_to(n: u64): u64 {
        let s = 0;
        let i = 0;
        while (i < n) {
            i = i + 1;
            s = s + i;
        };
        s
    }
    fun max(x: u64, y: u64): u64 { if (x > y) x else y }
    fun fact(n: u64): u64 { if (n == 0) 1 else n * fact(n - 1) }
    fun checked(x: u64): u64 { assert!(x < 10, 42); x }
    fun always_abort(): u64 { abort 7 }
    fun spin() { while (true) {} }
}
"

def control : Program := controlM.toProgram

def run (f : String) (args : List Value) : Outcome :=
  Tests.runXIR controlM f [] args

/-! ## `while` loop -/

#test run "sum_to" [.u64 0] = okU64 0
#test run "sum_to" [.u64 5] = okU64 15
#test run "sum_to" [.u64 10] = okU64 55

/-! ## Branching -/

#test run "max" [.u64 3, .u64 9] = okU64 9
#test run "max" [.u64 9, .u64 3] = okU64 9
#test run "max" [.u64 4, .u64 4] = okU64 4

/-! ## Recursion (each call consumes fuel) -/

#test run "fact" [.u64 0] = okU64 1
#test run "fact" [.u64 5] = okU64 120
#test run "fact" [.u64 10] = okU64 3628800

/-! ## User abort codes survive to the outcome -/

#test run "checked" [.u64 3] = okU64 3
#test run "checked" [.u64 10] = aborted 42
#test run "always_abort" [] = aborted 7

/-! ## Divergence exhausts the fuel -/

#test run "spin" [] = .error .outOfFuel
#test interpFun control 10 (controlM.funId "fact") [] [.u64 100]
  = .error .outOfFuel

/-! ## Undeclared functions and wrong arity are stuck -/

#test run "no_such_fun" [] matches .error (.stuck _)
#test run "max" [.u64 1] matches .error (.stuck _)
#test run "max" [.u64 1, .u64 2, .u64 3] matches .error (.stuck _)

end Tests.Interp.ControlFlow

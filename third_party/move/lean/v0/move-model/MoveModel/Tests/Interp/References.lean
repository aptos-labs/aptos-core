-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Frontend.Elab
import MoveModel.Tests.XIRCommon
import MoveModel.IR.Interp.Exec

/-!
# Interpreter Tests: References (within one frame)

The reference semantics on locals: `&mut` borrows of locals and fields,
reads and writes through references (including nested field paths), and
freezing.  Global borrows are covered in `Globals.lean`; references
crossing call boundaries (the checkout call semantics) in
`CrossCallRefs.lean`.
-/

namespace Tests.Interp.References

open MoveModel.IR
open MoveModel.Frontend.XIR

def refsM : MProgram := moveM% "
module 0x42::refs {
    struct Inner has copy, drop { v: u64 }
    struct Box has copy, drop { f: u64, inner: Inner }

    fun bump(x: u64): u64 {
        let r = &mut x;
        *r = *r + 1;
        x
    }
    fun set_field(b: Box, v: u64): Box {
        let r = &mut b.f;
        *r = v;
        b
    }
    fun set_nested(b: Box, v: u64): Box {
        b.inner.v = v;
        b
    }
    fun read_frozen(x: u64): u64 {
        let r = &mut x;
        *r = 5;
        let f: &u64 = r;
        *f
    }
    fun swap(x: u64, y: u64): (u64, u64) {
        let t = x;
        let rx = &mut x;
        *rx = y;
        let ry = &mut y;
        *ry = t;
        (x, y)
    }
}
"

def refs : Program := refsM.toProgram

def run (f : String) (args : List Value) : Outcome :=
  Tests.runXIR refsM f [] args

/-- A `Box { f: x, inner: Inner { v: y } }` value. -/
def box (x y : Nat) : Value := .struct [.u64 x, .struct [.u64 y]]

/-! ## Write through a `&mut` local -/

#test run "bump" [.u64 41] = okU64 42
#test run "bump" [.u64 18446744073709551615] = aborted 0

/-! ## Write through a field borrow -/

#test run "set_field" [box 1 2, .u64 9]
  = okVals [.struct [.u64 9, .struct [.u64 2]]]

/-! ## Write through a nested field path -/

#test run "set_nested" [box 1 2, .u64 9]
  = okVals [.struct [.u64 1, .struct [.u64 9]]]

/-! ## Freeze: reads through the frozen reference see prior writes -/

#test run "read_frozen" [.u64 0] = okU64 5

/-! ## Two borrows of different locals in one frame -/

#test run "swap" [.u64 1, .u64 2] = okVals [.u64 2, .u64 1]

/-! ## `==` on references compares the referred-to values -/

def refEqM : MProgram := moveM% "
module 0x42::refeq {
    fun ref_eq(x: u64, y: u64): bool {
        &x == &y
    }
}
"

#test Tests.runXIR refEqM "ref_eq" [] [.u64 5, .u64 5] = okBool true
#test Tests.runXIR refEqM "ref_eq" [] [.u64 5, .u64 6] = okBool false

end Tests.Interp.References

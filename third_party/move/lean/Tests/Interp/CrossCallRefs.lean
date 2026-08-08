-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Frontend.Elab
import Tests.Common
import Move.IR.Interp

/-!
# Interpreter Tests: References Across Calls

The checkout call semantics on real compiler output: loc-rooted reference
arguments are checked out into shadow slots of the callee and written back
on return; references returned by the callee are re-rooted onto the
argument targets (continued borrows); global-rooted references pass
verbatim.  Covers the TACAS'22 `increment_field`/`caller` shapes: borrows
into caller locals, returned sub-borrows, a dynamic origin choice inside
the callee, nested checkout through two frames, abort propagation (the
write-back is discarded), and a cross-call borrow into global memory.
-/

namespace Tests.Interp.CrossCallRefs

open Move.IR
open Move.Frontend

def refsM : MProgram := moveM% "
module 0x42::cross_call_refs {
    struct S has copy, drop { f: u64, g: u64 }
    struct G has key { v: u64 }

    fun increment(x: &mut u64) { *x = *x + 1 }

    fun bump_local(v: u64): u64 { increment(&mut v); v }

    fun set_f(s: &mut S, x: u64) { s.f = x }
    fun bump_struct(): u64 { let s = S { f: 1, g: 0 }; set_f(&mut s, 9); s.f }

    fun get_f(s: &mut S): &mut u64 { &mut s.f }
    fun via_ret_ref(): u64 {
        let s = S { f: 7, g: 0 };
        let r = get_f(&mut s);
        *r = *r + 3;
        s.f
    }

    fun choose(p: bool, s: &mut S): &mut u64 {
        if (p) &mut s.f else &mut s.g
    }
    fun via_choice(p: bool): u64 {
        let s = S { f: 10, g: 20 };
        let r = choose(p, &mut s);
        *r = *r + 1;
        s.f * 100 + s.g
    }

    fun inc_f(s: &mut S) { increment(get_f(s)) }
    fun nested(): u64 { let s = S { f: 100, g: 0 }; inc_f(&mut s); s.f }

    fun read_it(x: &u64): u64 { *x }
    fun read_local(v: u64): u64 { read_it(&v) }

    fun write_then_abort(x: &mut u64) { *x = 99; abort 7 }
    fun caller_aborts(): u64 { let v = 1; write_then_abort(&mut v); v }

    fun bump_global(a: address) {
        increment(&mut G[a].v)
    }
}
"

def run (f : String) (mem : IMem) (args : List Value) : Tests.Outcome :=
  Tests.run refsM f mem args

/-! ## Borrows into caller locals: checkout + write-back -/

#test run "bump_local" [] [.u64 5] = okU64 6
#test run "bump_struct" [] [] = okU64 9

/-! ## Returned references: continued borrows, re-rooted in the caller -/

#test run "via_ret_ref" [] [] = okU64 10

/-! ## Dynamic origin choice inside the callee -/

#test run "via_choice" [] [.bool true] = okU64 1120
#test run "via_choice" [] [.bool false] = okU64 1021

/-! ## Nested checkout: a shadow-rooted reference checked out again -/

#test run "nested" [] [] = okU64 101

/-! ## Immutable references cross calls the same way -/

#test run "read_local" [] [.u64 3] = okU64 3

/-! ## Abort propagation: the write-back is discarded -/

#test run "caller_aborts" [] [] = aborted 7

/-! ## Global-rooted references pass verbatim (memory is shared) -/

def gid : ResourceId := refsM.resourceId "G"

#test run "bump_global" [(gid, 5, .struct [.u64 41])] [.address 5]
  = okRet [(gid, 5, .struct [.u64 42])] []
#test run "bump_global" [] [.address 5] = aborted 0

end Tests.Interp.CrossCallRefs

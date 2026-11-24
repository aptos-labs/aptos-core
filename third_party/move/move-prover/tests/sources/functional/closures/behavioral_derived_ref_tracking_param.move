// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A lambda that writes through an untracked reference under the name of its
// `&mut` parameter (assigned, or bound again) has no derived contract.
module 0x42::lam_param {
    struct T has copy, drop, store { a: u64, b: u64 }
    inline fun app_neg(s: &mut T, f: |&mut T|) { let pre = *s; f(s); spec { assert ensures_of<f>(pre, s) ==> s == pre; }; }
    inline fun app2_neg(t: &mut u64, s: &mut T, f: |&mut u64, &mut T|) { let pt = *t; let pre = *s; f(t, s); spec { assert ensures_of<f>(pt, pre, t, s) ==> s == pre; }; }

    fun c_move_reassigned_incorrect(y: &mut u64, x: &mut T) { app2_neg(y, x, |t2, s2| { t2 = &mut s2.a; let q = move t2; *q = 1 }) }
    spec c_move_reassigned_incorrect { pragma aborts_if_is_partial; }

    fun c_reassigned_incorrect(y: &mut u64, x: &mut T) { app2_neg(y, x, |t2, s2| { t2 = &mut s2.a; *t2 = 1 }) }
    spec c_reassigned_incorrect { pragma aborts_if_is_partial; }

    fun c_reassign_cond_incorrect(y: &mut u64, x: &mut T) { app2_neg(y, x, |t2, s2| { t2 = if (s2.b > 0) { &mut s2.a } else { &mut s2.b }; *t2 = 1 }) }
    spec c_reassign_cond_incorrect { pragma aborts_if_is_partial; }

    fun c_shadow_cond_incorrect(x: &mut T) { app_neg(x, |t| { let c = t.b > 0; let t = if (c) { &mut t.a } else { &mut t.b }; *t = 1 }) }
    spec c_shadow_cond_incorrect { pragma aborts_if_is_partial; }

    fun c_shadow_tracked_incorrect(x: &mut T) { app_neg(x, |t| { let t = &mut t.a; *t = 1 }) }
    spec c_shadow_tracked_incorrect { pragma aborts_if_is_partial; }
}

// exclude_for: lean
// The expected output is the Boogie backend's error for a lambda without a derived contract.

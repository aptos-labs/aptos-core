// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// Receiver-style calls in every expression position of a proof block. Each `_incorrect`
// function is a false spec rejected through its proof step; for steps that only add facts
// (`assume`, `forall ... apply`) the true twin is provable only through the step; for `split`
// see below.
module 0x42::proof_receiver_call {
    struct S has copy, drop { f: u8, h: u8 }
    struct T has copy, drop { s: S }

    fun get_f(self: &S): u8 { self.f }
    fun get_h(self: &S): u8 { self.h }

    spec fun le_self(x: u8): bool { x <= x }

    spec fun is_zero(x: u8): bool { x == 0 }

    spec fun sum(n: num): num {
        if (n == 0) { 0 } else { n + sum(n - 1) }
    }

    spec module {
        lemma monotonicity(x: num, y: num) {
            requires 0 <= x;
            requires x <= y;
            ensures sum(x) <= sum(y);
        } proof {
            if (x < y) {
                assert sum(y - 1) <= sum(y);
                apply monotonicity(x, y - 1);
            }
        }

        lemma is_three(x: u8) {
            requires x == 3;
            ensures x == x;
        }

        lemma needs_three(x: u8, b: bool) {
            requires b && x == 3;
            ensures x == x;
        }
    }

    fun f_let(s: &S): u8 { s.f }
    spec f_let {
        requires s.f == 3;
        ensures result == 3;
    } proof {
        let v = s.get_f();
        apply is_three(v);
    }

    fun f_let_incorrect(s: &S): u8 { s.f }
    spec f_let_incorrect {
        requires s.f == 3;
        ensures result == 3;
    } proof {
        let v = s.get_h();
        apply is_three(v);
    }

    // The lemma requirement holds only under the condition.
    fun f_if(s: &S): u8 { s.f }
    spec f_if {
        ensures result == s.f;
    } proof {
        if (s.get_f() == 3) {
            apply is_three(s.f);
        } else {
            assert s.f != 3;
        }
    }

    fun f_if_incorrect(s: &S): u8 { s.f }
    spec f_if_incorrect {
        ensures result == s.f;
    } proof {
        if (s.get_f() > 0) {
            apply is_three(s.f);
        }
    }

    fun f_assert(s: &S): u8 { s.f }
    spec f_assert {
        ensures result == s.f;
    } proof {
        assert s.get_f() == s.f;
    }

    fun f_assert_incorrect(s: &S): u8 { s.f }
    spec f_assert_incorrect {
        ensures result == s.f;
    } proof {
        assert s.get_f() == s.f + 1;
    }

    fun f_assume(s: &S): u8 { s.f }
    spec f_assume {
        ensures result == 5;
    } proof {
        assume [trusted] s.get_f() == 5;
    }

    fun f_assume_incorrect(s: &S): u8 { s.f }
    spec f_assume_incorrect {
        ensures result == 5;
    } proof {
        assume [trusted] s.get_f() == 4;
    }

    fun f_apply_lemma(s: &S): u8 { s.f }
    spec f_apply_lemma {
        requires s.f == 3;
        ensures result == 3;
    } proof {
        apply is_three(s.get_f());
    }

    fun f_apply_lemma_incorrect(s: &S): u8 { s.f }
    spec f_apply_lemma_incorrect {
        requires s.f == 3;
        ensures result == 3;
    } proof {
        apply is_three(s.get_h());
    }

    fun f_apply_spec_fun(s: &S): u8 { s.f }
    spec f_apply_spec_fun {
        ensures result == s.f;
    } proof {
        apply le_self(s.get_f());
    }

    fun f_apply_spec_fun_incorrect(s: &S): u8 { s.f }
    spec f_apply_spec_fun_incorrect {
        ensures result == s.f;
    } proof {
        apply is_zero(s.get_f());
    }

    // Holds only through the lemma instances.
    fun f_forall_apply(s: &S): u8 { s.f }
    spec f_forall_apply {
        requires s.f <= s.h;
        ensures sum(s.f) <= sum(s.h);
    } proof {
        forall y: num {sum(y)} apply monotonicity(s.get_f(), y);
    }

    fun f_forall_apply_incorrect(s: &S): u8 { s.f }
    spec f_forall_apply_incorrect {
        requires s.f <= s.h;
        ensures sum(s.h) < sum(s.f);
    } proof {
        forall y: num {sum(y)} apply monotonicity(s.get_f(), y);
    }

    // The deferred field selection `w.f` comes before the argument that relates `w` to `t`.
    fun f_apply_arg_order_incorrect(t: T): u8 { t.s.f }
    spec f_apply_arg_order_incorrect {
        ensures result == t.s.f;
    } proof {
        let w = t.s;
        apply needs_three(w.f, w == t.s);
    }

    fun f_calc(s: &S): u8 { s.f }
    spec f_calc {
        ensures result == s.f;
    } proof {
        calc(
            s.get_f()
            == s.f
        );
    }

    fun f_calc_incorrect(s: &S): u8 { s.f }
    spec f_calc_incorrect {
        ensures result == s.f;
    } proof {
        calc(
            s.get_f()
            == s.f + 1
        );
    }

    // A split only adds a case analysis, which none of these proofs needs: the true spec holds
    // without it and the false one fails with it. A split on a non-boolean receiver result is
    // rejected in `receiver_call_type_err.move`.
    fun f_split(s: &S): u8 { if (s.f >= 5) s.f else 5 }
    spec f_split {
        ensures result >= 5;
    } proof {
        split s.get_f() >= 5;
    }

    fun f_split_incorrect(s: &S): u8 { if (s.f >= 5) s.f else 5 }
    spec f_split_incorrect {
        ensures result >= 10;
    } proof {
        split s.get_f() >= 5;
    }
}

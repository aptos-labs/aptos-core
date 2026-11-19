// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// The result of a receiver-style call is checked against the epected type exactly as the
// result of the function call form is. Each case is given in both forms, which are rejected
// with the same error.
module 0x42::proof_receiver_call_instance_err {
    struct W<T> has copy, drop { x: T }

    struct S has copy, drop { w: W<u64>, v: vector<u64>, vv: vector<vector<u64>>, vw: vector<W<u64>> }

    fun get_w(self: &S): W<u64> { self.w }

    fun values(self: &S): vector<u64> { self.v }

    fun nested(self: &S): vector<vector<u64>> { self.vv }

    fun structs(self: &S): vector<W<u64>> { self.vw }

    fun same<T: copy + drop>(self: &W<T>): W<T> { *self }

    fun wrap<T: copy + drop>(self: &W<T>): vector<T> { vector[self.x] }

    spec schema Check {
        v: vector<u8>;
        ensures len(v) == len(v);
    }

    spec module {
        lemma v8(v: vector<u8>) {
            ensures forall i in 0..len(v): v[i] <= 255;
        }

        lemma w8(w: W<u8>) {
            ensures true;
        }

        lemma vv8(v: vector<vector<u8>>) {
            ensures true;
        }

        lemma vw8(v: vector<W<u8>>) {
            ensures true;
        }
    }

    // A struct of another instance.
    fun receiver_arg(s: S): u64 { s.w.x }
    spec receiver_arg {
        ensures result == s.w.x;
    } proof {
        apply w8(s.get_w());
    }

    fun call_arg(s: S): u64 { s.w.x }
    spec call_arg {
        ensures result == s.w.x;
    } proof {
        apply w8(get_w(s));
    }

    fun receiver_in_if(b: bool, s: S, w: W<u8>): u64 { s.w.x }
    spec receiver_in_if {
        ensures result == s.w.x;
    } proof {
        assert (if (b) s.get_w() else w) == w;
    }

    fun call_in_if(b: bool, s: S, w: W<u8>): u64 { s.w.x }
    spec call_in_if {
        ensures result == s.w.x;
    } proof {
        assert (if (b) get_w(s) else w) == w;
    }

    // A vector of another element type, as a schema argument.
    fun receiver_vector(s: &S) {}
    spec receiver_vector {
        include Check { v: s.values() };
    }

    fun call_vector(s: &S) {}
    spec call_vector {
        include Check { v: values(s) };
    }

    // The same as a lemma argument: assuming the conclusion would prove `false`.
    fun receiver_vector_arg(s: S): u64 { s.w.x }
    spec receiver_vector_arg {
        requires len(s.v) > 0 && s.v[0] == 1000;
        ensures false;
    } proof {
        apply v8(s.values());
    }

    fun call_vector_arg(s: S): u64 { s.w.x }
    spec call_vector_arg {
        requires len(s.v) > 0 && s.v[0] == 1000;
        ensures false;
    } proof {
        apply v8(values(s));
    }

    // A nested vector.
    fun receiver_nested(s: S): u64 { s.w.x }
    spec receiver_nested {
        ensures result == s.w.x;
    } proof {
        apply vv8(s.nested());
    }

    fun call_nested(s: S): u64 { s.w.x }
    spec call_nested {
        ensures result == s.w.x;
    } proof {
        apply vv8(nested(s));
    }

    // A vector of a struct of another instance.
    fun receiver_structs(s: S): u64 { s.w.x }
    spec receiver_structs {
        ensures result == s.w.x;
    } proof {
        apply vw8(s.structs());
    }

    fun call_structs(s: S): u64 { s.w.x }
    spec call_structs {
        ensures result == s.w.x;
    } proof {
        apply vw8(structs(s));
    }

    // Generic functions.
    fun generic_receiver_arg(w: W<u64>): u64 { w.x }
    spec generic_receiver_arg {
        ensures result == w.x;
    } proof {
        apply w8(w.same());
    }

    fun generic_call_arg(w: W<u64>): u64 { w.x }
    spec generic_call_arg {
        ensures result == w.x;
    } proof {
        apply w8(same(w));
    }

    fun generic_receiver_vector(w: &W<u64>) {}
    spec generic_receiver_vector {
        include Check { v: w.wrap() };
    }

    fun generic_call_vector(w: &W<u64>) {}
    spec generic_call_vector {
        include Check { v: wrap(w) };
    }

    spec fun arb<T>(): T;

    // In a proof block, a receiver call whose receiver type is inferred only after the call
    // is translated has its result checked once it is resolved. (The function call form is
    // checked when it is translated, before the equality relates the two vectors, and is
    // accepted.)
    fun receiver_inferred_late(s: S, v: vector<u8>): u64 { s.w.x }
    spec receiver_inferred_late {
        ensures result == s.w.x;
    } proof {
        assert { let x = arb(); x.values() == v && x == s } ==> true;
    }

    // A field selected from a value whose type is inferred only after the selection is
    // translated is checked when it is resolved, as the typed form is checked at once.
    // Assuming the conclusion of the lemma for it would prove `false`.
    fun field_inferred_late(w: vector<u8>) {}
    spec field_inferred_late {
        requires len(arb<S>().v) > 0 && arb<S>().v[0] == 1000;
        ensures false;
    } proof {
        apply v8({ let x = arb(); let r = x.v; if (r == w && x == arb<S>()) r else r });
    }

    fun field_typed(w: vector<u8>) {}
    spec field_typed {
        requires len(arb<S>().v) > 0 && arb<S>().v[0] == 1000;
        ensures false;
    } proof {
        apply v8({ let x: S = arb(); let r = x.v; if (r == w && x == arb<S>()) r else r });
    }

    // The result of a spec function.
    spec fun receiver_result(s: S): bool { s.values() }

    spec fun call_result(s: S): bool { values(s) }

    // A proof local bound to a receiver call has the type of the call, as for the call form.
    fun receiver_let(s: S): u64 { s.w.x }
    spec receiver_let {
        ensures result == s.w.x;
    } proof {
        let w = s.get_w();
        apply w8(w);
    }

    fun call_let(s: S): u64 { s.w.x }
    spec call_let {
        ensures result == s.w.x;
    } proof {
        let w = get_w(s);
        apply w8(w);
    }
}

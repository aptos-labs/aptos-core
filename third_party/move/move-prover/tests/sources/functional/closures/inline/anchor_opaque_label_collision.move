// Memory-label uniqueness between inline-expansion anchor labels and the
// freshened labels of opaque callee specs: all labels come from the
// environment's single global id allocator. (With an independent
// per-function freshening counter starting at zero, an opaque call's
// old-state save could reuse the anchor label and overwrite the anchor
// snapshot — taken BEFORE f ran — with a later state, which let the false
// claim below verify and made the true claim fail. The several opaque
// calls make the freshened labels sweep a range of small ids, so a
// reintroduced local counter would collide with the anchor label again.)
module 0x42::anchor_opaque_label_collision {

    struct R has key { v: u64 }

    fun bump_opaque(a: address) {
        let r = &mut R[a];
        r.v = r.v + 1;
    }
    spec bump_opaque {
        pragma opaque;
        modifies global<R>(a);
        aborts_if !exists<R>(a) || R[a].v == MAX_U64;
        ensures R[a].v == old(R[a].v) + 1;
    }

    inline fun apply_then_bump(f: |address|, a: address) {
        f(a);
        bump_opaque(a);
        bump_opaque(a);
        bump_opaque(a);
        bump_opaque(a);
        spec {
            // ensures_of relates f's application pre-state (the anchor
            // snapshot) to the assertion state: the value moved by +5.
            assert ensures_of<f>(a); // error: for the `false_claim` caller, the delta from f's pre-state is +5, not +1
        };
    }

    // FALSE claim: the lambda spec says +1, the pre-to-assert delta is +5.
    fun false_claim(a: address) {
        apply_then_bump(
            |x| { let r = &mut R[x]; r.v = r.v + 1; } spec { ensures R[x].v == old(R[x].v) + 1; },
            a,
        );
    }
    spec false_claim {
        requires exists<R>(a) && R[a].v < MAX_U64 - 5;
    }

    // TRUE claim: +5 from f's pre-state to the assertion state.
    fun true_claim(a: address) {
        apply_then_bump(
            |x| { let r = &mut R[x]; r.v = r.v + 1; } spec { ensures R[x].v == old(R[x].v) + 5; },
            a,
        );
    }
    spec true_claim {
        requires exists<R>(a) && R[a].v < MAX_U64 - 5;
    }
}

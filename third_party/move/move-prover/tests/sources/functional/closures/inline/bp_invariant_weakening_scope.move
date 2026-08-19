module 0x42::bp_invariant_weakening_scope {
    use std::type_name;

    inline fun apply_both(f: |u64|, g: |u64|, x: u64) {
        let i = 0;
        while (i < 1) {
            f(x);
            g(x);
            i = i + 1;
        } spec {
            invariant i <= 1;
            invariant forall _j in 0..i: ensures_of<f>(x);
            invariant false && (forall _j in 0..i: ensures_of<f>(x));
            invariant forall _j in 0..i: ensures_of<g>(x);
        };
    }

    inline fun apply(f: |u64|, x: u64) {
        let i = 0;
        while (i < 1) {
            f(x);
            i = i + 1;
        } spec {
            invariant i <= 1;
            invariant forall _j in 0..i: ensures_of<f>(x);
        };
    }

    fun resolved_invariant_is_not_weakened(x: u64) {
        apply_both(
            |a| {
                let i = 0;
                while (i < 1) {
                    i = i + 1;
                };
                let _ = a;
            },
            |a| { let _ = a; } spec { ensures false; },
            x,
        ); // error: the invariant for `g` remains false
    }

    inline fun forward_both(f: |u64|, g: |u64|, x: u64) {
        apply_both(|a| f(a), |a| g(a), x);
    }

    fun nested_resolved_invariant_is_not_weakened(x: u64) {
        forward_both(
            |a| {
                let i = 0;
                while (i < 1) {
                    i = i + 1;
                };
                let _ = a;
            },
            |a| { let _ = a; } spec { ensures false; },
            x,
        ); // error: nested forwarding weakens only the invariant for `f`
    }

    fun uses_generic_type_reflection<T>(_x: u64) {
        let _ = type_name::get<T>();
    }

    fun reflection_in_spec<T>(_x: u64) {}
    spec reflection_in_spec {
        ensures type_name::get<T>() == type_name::get<T>();
        ensures false;
    }

    fun generic_reflection_invariant_is_weakened<T>(x: u64) {
        apply(|a| uses_generic_type_reflection<T>(a), x);
    }

    fun monomorphic_reflection_invariant_is_not_weakened(x: u64) {
        apply(|a| reflection_in_spec<u64>(a), x);
        // error: the fully instantiated reflection does not weaken the false invariant
    }
}

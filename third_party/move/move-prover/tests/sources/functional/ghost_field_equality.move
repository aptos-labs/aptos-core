// Equality over ghost-bearing types: Move `==` must coincide with runtime
// equality, ignoring ghost state, on every equality path — struct field-wise
// equality (chosen whenever a ghost is transitively present, including
// through generic instantiations), tuple equality, and enum variant
// equality. See `ghost_field.move` for the basic mechanics.
module 0x42::ghost_field_equality {
    struct Inner has copy, drop { v: u64 }
    spec Inner {
        ghost gi: u64;
    }

    // A benign generic instantiation (`WrapG<u64>`) must not shield a ghost
    // one (`WrapG<Inner>`) in the transitive-ghost check for equality: the
    // visited set keys on the full instantiation. Equality must be fieldwise
    // over runtime state, ignoring the distinct ghosts set below.
    struct WrapG<T> has copy, drop { v: T }
    struct OuterGen has copy, drop { a: WrapG<u64>, b: WrapG<Inner> }

    fun nested_generic_ghost_eq(): bool {
        let g1 = Inner { v: 2 };
        let g2 = Inner { v: 2 };
        spec { update g1.gi = 1; };
        spec { update g2.gi = 2; };
        let l = OuterGen { a: WrapG { v: 1 }, b: WrapG { v: g1 } };
        let r = OuterGen { a: WrapG { v: 1 }, b: WrapG { v: g2 } };
        l == r
    }
    spec nested_generic_ghost_eq {
        ensures result;
    }

    // Tuple equality is componentwise runtime equality: ghosts are ignored
    // even though they are constructor arguments of the Boogie datatype.
    fun tuple_ghost_eq(): (Inner, Inner) {
        let a = Inner { v: 1 };
        let b = Inner { v: 1 };
        spec { update a.gi = 1; };
        spec { update b.gi = 2; };
        (a, b)
    }
    spec tuple_ghost_eq {
        ensures (result_1, result_2) == (result_1, result_1);
    }

    // Enum equality with a ghost-bearing variant field: the per-field
    // comparison delegates to the field type's $IsEqual, which excludes
    // ghosts, so runtime-equal variants with distinct nested ghosts are
    // equal.
    enum EN has copy, drop { A { i: Inner }, B }

    fun enum_nested_ghost_eq(): bool {
        let g1 = Inner { v: 1 };
        let g2 = Inner { v: 1 };
        spec { update g1.gi = 1; };
        spec { update g2.gi = 2; };
        let l = EN::A { i: g1 };
        let r = EN::A { i: g2 };
        l == r
    }
    spec enum_nested_ghost_eq { ensures result; }

    // Vector equality with ghost-bearing elements: element-wise comparison
    // delegates to the element's $IsEqual, so runtime-equal vectors whose
    // elements differ only in ghosts are equal.
    fun vec_ghost_eq(): bool {
        let g1 = Inner { v: 1 };
        let g2 = Inner { v: 1 };
        spec { update g1.gi = 1; };
        spec { update g2.gi = 2; };
        let v1 = vector[g1];
        let v2 = vector[g2];
        v1 == v2
    }
    spec vec_ghost_eq { ensures result; }

    // Generic equality instantiated at a ghost-bearing type: the callee is
    // verified with an uninterpreted type parameter; the caller's use of its
    // spec instantiates the equality at `Inner`, which must be the
    // ghost-excluding relation.
    fun generic_eq<T: copy + drop>(a: T, b: T): bool { a == b }
    spec generic_eq { ensures result == (a == b); }

    fun generic_eq_ghost(): bool {
        let g1 = Inner { v: 1 };
        let g2 = Inner { v: 1 };
        spec { update g1.gi = 1; };
        spec { update g2.gi = 2; };
        generic_eq(g1, g2)
    }
    spec generic_eq_ghost { ensures result; }
}

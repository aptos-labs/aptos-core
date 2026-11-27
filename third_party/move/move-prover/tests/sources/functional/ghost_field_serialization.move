// Serialization of ghost-bearing values: BCS serializes the runtime value,
// which holds no ghost contents, so equal serializations of two parameters
// equate them as runtime values (ghost slots ignored), not as model values.
// For a type without ghosts the premise is plain equality, as before.
module 0x42::ghost_field_serialization {
    use std::bcs;

    struct G has copy, drop { v: u64 }
    spec G {
        ghost g: u64;
    }

    struct P has copy, drop { v: u64 }

    // Equal bytes say nothing about the ghosts: `{v: 1, g: 1}` and
    // `{v: 1, g: 2}` serialize alike.
    fun g_ghost(a: G, b: G): bool { bcs::to_bytes(&a) == bcs::to_bytes(&b) }
    spec g_ghost {
        ensures result ==> a.g == b.g; // error: the bytes do not hold the ghost
    }

    fun g_ghost_spec(a: G, b: G): bool { bcs::to_bytes(&a) == bcs::to_bytes(&b) }
    spec g_ghost_spec {
        ensures bcs::serialize(a) == bcs::serialize(b) ==> a.g == b.g; // error: the bytes do not hold the ghost
    }

    // The premise is generated for a serialization in the specification alone.
    fun g_ghost_spec_only(a: G, b: G): bool { a.v == b.v }
    spec g_ghost_spec_only {
        ensures bcs::serialize(a) == bcs::serialize(b) ==> a.g == b.g; // error: the bytes do not hold the ghost
    }

    fun g_field(a: G, b: G): bool { bcs::to_bytes(&a) == bcs::to_bytes(&b) }
    spec g_field {
        ensures result ==> a.v == b.v;
    }

    fun g_equal(a: G, b: G): bool { bcs::to_bytes(&a) == bcs::to_bytes(&b) }
    spec g_equal {
        ensures result ==> a == b;
    }

    fun g_equal_ref(a: &G, b: &G): bool { bcs::to_bytes(a) == bcs::to_bytes(b) }
    spec g_equal_ref {
        ensures result ==> a == b;
    }

    // At a type parameter equal serializations give nothing: an instance's
    // logical value may hold more than its physical layout serializes.
    fun generic_equal<T>(a: &T, b: &T): bool { bcs::to_bytes(a) == bcs::to_bytes(b) }
    spec generic_equal {
        ensures result ==> a == b; // error: no observation is known at a type parameter
    }

    // Without a ghost, equal bytes give equal values.
    fun p_field(a: P, b: P): bool { bcs::to_bytes(&a) == bcs::to_bytes(&b) }
    spec p_field {
        ensures result ==> a.v == b.v;
    }

    fun p_equal(a: P, b: P): bool { bcs::to_bytes(&a) == bcs::to_bytes(&b) }
    spec p_equal {
        ensures result ==> a == b;
    }

    fun p_equal_spec_only(a: P, b: P): bool { a.v == b.v }
    spec p_equal_spec_only {
        ensures bcs::serialize(a) == bcs::serialize(b) ==> a == b;
    }

    fun p_wrong(a: P, b: P): bool { bcs::to_bytes(&a) == bcs::to_bytes(&b) }
    spec p_wrong {
        ensures result ==> a.v != b.v; // error: equal bytes give equal fields
    }
}

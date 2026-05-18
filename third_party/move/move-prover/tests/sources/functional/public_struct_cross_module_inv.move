// Cross-module data invariant test: module A defines a public struct S with an
// invariant; module B holds a mutable reference and writes a violating value.
// The prover should report exactly one error — in B's violate()

module 0x42::public_struct_cross_module_inv_a {

    public struct S has drop {
        x: u64,
    }

    spec S {
        invariant self.x > 0;
    }

    // Correct: precondition guarantees the invariant at construction.
    public fun new(x: u64): S {
        S { x }
    }
    spec new {
        requires x > 0;
        ensures result.x == x;
    }
}

module 0x42::public_struct_cross_module_inv_b {
    use 0x42::public_struct_cross_module_inv_a::S;

    // Violates the invariant: writes 0 to the field through a mutable borrow.
    // The prover must catch this when the &mut S param goes out of scope and
    // PackRefDeep asserts the invariant.
    public fun violate(s: &mut S) {
        s.x = 0;
    }
}

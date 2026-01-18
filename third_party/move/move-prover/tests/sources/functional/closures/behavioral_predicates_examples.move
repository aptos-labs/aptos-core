// Examples demonstrating behavioral predicates with higher-order functions.

module 0x42::behavioral_predicates_examples {

    // =========================================================================
    // Apply_twice (opaque): applying a function twice with opaque pragma
    // =========================================================================

    /// Applies function f twice: f(f(x))
    /// This version is opaque - verification relies on behavioral predicates.
    fun apply_twice_opaque(f: |u64| u64 has copy, x: u64): u64 {
        f(f(x))
    }
    spec apply_twice_opaque {
        // Make this opaque so we get semantics from spec at caller side.
        pragma opaque = true;
        // Without knowing `f`, we can still specify how its
        // behavior effects the calling function.
        let y = choose y: u64 where ensures_of<f>(x, y);
        requires requires_of<f>(x) && requires_of<f>(y);
        aborts_if aborts_of<f>(x) || aborts_of<f>(y);
        ensures ensures_of<f>(y, result);
    }

    fun add_opaque(): u64 {
        apply_twice_opaque(
            |x| x + 1 spec { aborts_if x == MAX_U64; ensures result == x + 1;},
        1)
    }
    spec add_opaque {
        ensures result == 3;
    }

    // =========================================================================
    // Apply_twice (transparent): applying a function twice without opaque
    // =========================================================================

    /// Applies function f twice: f(f(x))
    /// This version is transparent - verification inlines the implementation.
    fun apply_twice_transparent(f: |u64| u64 has copy, x: u64): u64 {
        f(f(x))
    }
    spec apply_twice_transparent {
        // NOT opaque - the implementation is inlined at call sites.
        // Behavioral predicates still define the abstract contract.
        let y = choose y: u64 where ensures_of<f>(x, y);
        requires requires_of<f>(x) && requires_of<f>(y);
        aborts_if aborts_of<f>(x) || aborts_of<f>(y);
        ensures ensures_of<f>(y, result);
    }

    fun add_transparent(): u64 {
        apply_twice_transparent(
            |x| x + 1 spec { aborts_if x == MAX_U64; ensures result == x + 1;},
        1)
    }
    spec add_transparent {
        ensures result == 3;
    }
}

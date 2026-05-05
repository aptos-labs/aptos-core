// Copyright © Aptos Foundation
// Sequential composition of two higher-order |&mut T| closures expressed
// via state labels.  The `followed_by` function applies `f` then `g` to `x`
// and witnesses the intermediate value with `exists S in *`.
//
// Relationship to amm_example.move:
//   The AMM Pool struct carries complex invariants (no-abort, monotonicity,
//   constant-product preservation). This module gives the same pattern in
//   isolation so the state-label encoding is clear without the invariant noise.

module 0x42::followed_by {

    // Simple numeric accumulator — no abilities needed beyond copy/drop.
    struct Acc has copy, drop { value: u64 }

    // -------------------------------------------------------------------------
    // Abstract sequential composition
    // -------------------------------------------------------------------------

    /// Apply `f` then `g` to `x`.
    /// The spec witnesses the intermediate value of `x` (after `f`, before `g`)
    /// as an existentially quantified state label S:
    ///   - `..S |~ ensures_of<f>(old(x), x)` — S is the state f produces
    ///   - `S.. |~ ensures_of<g>(old(x), x)` — g starts from S
    fun followed_by(
        f: |&mut Acc| has drop,
        g: |&mut Acc| has drop,
        x: &mut Acc,
    ) {
        f(x);
        g(x)
    }
    spec followed_by {
        // f can abort on the input.
        aborts_if aborts_of<f>(x);
        // g can abort on the intermediate value produced by f.
        aborts_if aborts_of<g>(result_of<f>(x));
        ensures exists S in *:
            (..S |~ ensures_of<f>(old(x), x)) &&
            (S.. |~ ensures_of<g>(old(x), x));
    }

    // -------------------------------------------------------------------------
    // Concrete transformations (opaque so callers use only their specs)
    // -------------------------------------------------------------------------

    /// Add 1 to the accumulator.
    fun add_one(a: &mut Acc) {
        a.value = a.value + 1;
    }
    spec add_one {
        pragma opaque;
        aborts_if a.value + 1 > MAX_U64;
        ensures a.value == old(a.value) + 1;
    }

    /// Double the accumulator.
    fun double_val(a: &mut Acc) {
        a.value = a.value * 2;
    }
    spec double_val {
        pragma opaque;
        aborts_if a.value * 2 > MAX_U64;
        ensures a.value == old(a.value) * 2;
    }

    // -------------------------------------------------------------------------
    // Concrete use — verifies from followed_by + opaque specs
    // -------------------------------------------------------------------------

    /// Add 1, then double: result = (v + 1) * 2.
    fun add_then_double(x: &mut Acc) {
        followed_by(add_one, double_val, x)
    }
    spec add_then_double {
        // add_one aborts on the initial value.
        aborts_if x.value + 1 > MAX_U64;
        // double_val aborts on the intermediate value (x.value + 1).
        aborts_if (x.value + 1) * 2 > MAX_U64;
        ensures x.value == (old(x.value) + 1) * 2;
    }
}

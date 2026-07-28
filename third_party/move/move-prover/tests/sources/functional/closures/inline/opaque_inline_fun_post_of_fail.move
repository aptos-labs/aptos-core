// A `fun_post_of` chain claims an exact application sequence: a callee whose
// body performs fewer applications than the chain describes cannot verify it
// (for carrying instantiations the live fun value advances once per
// application, and the chain names a different successor). See
// opaque_inline_fun_post_of.move for the conforming shapes.
module 0x42::opaque_inline_fun_post_of_fail {

    inline fun claim_two_apply_one(f: |u64| has copy + drop) {
        f(1);
    }
    spec claim_two_apply_one {
        pragma opaque;
        ensures f == fun_post_of<fun_post_of<old(f)>(1)>(2); // error: post-condition does not hold
    }

    /// Carrying caller: the chain claim is refutable only for carrying
    /// instantiations (for pure values `fun_post_of` is the identity and the
    /// claim holds trivially).
    fun make_carrying(): u64 {
        let x = 0;
        claim_two_apply_one(|i| x = x + i spec { ensures x == old(x) + i; });
        x
    }
}

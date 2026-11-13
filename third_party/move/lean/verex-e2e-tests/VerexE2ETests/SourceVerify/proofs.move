/// A specification whose proof is authored beside the source, in
/// `proofs.proof.lean`: the automatic verification decides linear
/// arithmetic, not the expansion of a product.
module 0x42::proofs {
    fun square_of_sum(a: u64, b: u64): u64 {
        (a + b) * (a + b)
    }
    spec square_of_sum {
        pragma verify = manual;
        aborts_if a + b > MAX_U64;
        aborts_if (a + b) * (a + b) > MAX_U64;
        ensures result == a * a + 2 * a * b + b * b;
    }

    /// Verified automatically: no proof file item names it.
    fun twice(a: u64): u64 {
        a + a
    }
    spec twice {
        aborts_if a + a > MAX_U64;
        ensures result == 2 * a;
    }
}

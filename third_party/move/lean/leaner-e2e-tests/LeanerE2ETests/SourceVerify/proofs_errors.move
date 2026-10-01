/// Specifications the automatic verification cannot establish: one asks
/// for a proof in `proofs_errors.proof.lean`, one has a proof there that
/// does not close its obligation. One is manual and has no proof there.
module 0x42::proofs_errors {
    fun square_of_sum(a: u64, b: u64): u64 {
        (a + b) * (a + b)
    }
    spec square_of_sum {
        aborts_if a + b > MAX_U64;
        aborts_if (a + b) * (a + b) > MAX_U64;
        ensures result == a * a + 2 * a * b + b * b;
    }

    fun product(a: u64, b: u64): u64 {
        a * b
    }
    spec product {
        aborts_if a * b > MAX_U64;
        ensures result == b * a;
    }

    /// Not attempted automatically, although it would verify.
    fun twice(a: u64): u64 {
        a + a
    }
    spec twice {
        pragma verify = manual;
        aborts_if a + a > MAX_U64;
        ensures result == 2 * a;
    }
}

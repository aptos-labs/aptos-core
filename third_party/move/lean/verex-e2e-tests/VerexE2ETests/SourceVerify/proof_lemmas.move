/// Proofs over the built-in arithmetic and vector lemmas and over lemma
/// items of the proof file, `proof_lemmas.proof.lean`.
module 0x42::proof_lemmas {
    use std::vector;

    /// The square of the smaller of two numbers is at most the larger's.
    fun square(a: u64, b: u64): u64 {
        a * a
    }
    spec square {
        pragma verify = manual;
        requires a <= b;
        requires b * b <= MAX_U64;
        aborts_if false;
        ensures result <= b * b;
    }

    /// Removing an element keeps the elements before it and shifts those
    /// after it down by one.
    fun drop_at(v: vector<u64>, i: u64): vector<u64> {
        vector::remove(&mut v, i);
        v
    }
    spec drop_at {
        pragma verify = manual;
        requires i < len(v);
        aborts_if false;
        ensures len(result) == len(v) - 1;
        ensures forall k in 0..len(result): k < i ==> result[k] == v[k];
        ensures forall k in 0..len(result): k >= i ==> result[k] == v[k + 1];
    }

    /// The position a search finds holds the needle.
    fun find(v: vector<u64>, e: u64): u64 {
        let (_, i) = vector::index_of(&v, &e);
        i
    }
    spec find {
        pragma verify = manual;
        aborts_if false;
        ensures contains(v, e) ==> result < len(v);
        ensures contains(v, e) ==> v[result] == e;
    }

    /// Removing the first element shifts every element down by one.
    fun drop_first(v: vector<u64>): vector<u64> {
        vector::remove(&mut v, 0);
        v
    }
    spec drop_first {
        pragma verify = manual;
        requires len(v) > 0;
        aborts_if false;
        ensures forall k in 0..len(v) - 1: result[k] == v[k + 1];
    }

    fun same(a: u64): u64 {
        a
    }
    spec same {
        pragma opaque;
        aborts_if false;
        ensures result == a;
    }

    /// A local the body binds is named at a point past its binding.
    fun shift(x: u64): u64 {
        let y = x + 1;
        let z = same(y);
        z + y
    }
    spec shift {
        pragma verify = manual;
        aborts_if 2 * x + 2 > MAX_U64;
        ensures result == 2 * x + 2;
    }
}

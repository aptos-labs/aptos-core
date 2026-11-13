/// Lemma items whose proofs fail, in `proof_lemmas_errors.proof.lean`.
module 0x42::proof_lemmas_errors {
    fun id(x: u64): u64 {
        x
    }
    spec id {
        ensures result == x;
    }
}

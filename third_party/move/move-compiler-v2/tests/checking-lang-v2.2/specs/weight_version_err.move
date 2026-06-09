// The `[weight = N]` quantifier-instantiation annotation is only available
// starting at language version 2.4. Under earlier versions, all three forms of
// the annotation — on `spec fun` signatures, on `forall ... apply` proof blocks,
// and on general `forall` / `exists` quantifier expressions — must produce a
// clear "not enabled before version 2.4" diagnostic from the parser.

module 0x42::M {

    // Site 1: `[weight = N]` on a recursive `spec fun` signature.
    spec fun id_num(n: num): num [weight = 20] {
        if (n == 0) { 0 } else { id_num(n - 1) + 1 }
    }

    // Site 2: `[weight = N]` on a `forall ... apply` proof block.
    spec module {
        lemma trivial(n: num) {
            ensures n == n;
        }
    }

    fun id_zero(): u64 {
        0
    }
    spec id_zero {
        // Site 3: `[weight = N]` on a general `forall` in `ensures`.
        ensures forall y: u64 [weight = 7]: y == y;
        ensures result == id_num(0);
    } proof {
        forall x: num {id_num(x)} [weight = 5]
            apply trivial(x);
    }
}

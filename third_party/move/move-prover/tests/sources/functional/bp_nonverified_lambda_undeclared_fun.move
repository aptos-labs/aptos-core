// Minimal repro: a lambda living in a NON-VERIFIED function is referenced by a
// behavioral predicate.
//
// `spec_inference` deliberately skips inference for lambdas that are not verified
// ("must not contribute a trusted-but-unproven spec"), so at monomorphisation time
// the lambda's spec is empty and nothing registers the functions it calls. The
// Boogie backend nevertheless emits `$bp_aborts_of` for that lambda, with a body
// calling the PURE spec-level form of `get` -- `$42_producer_$get`, distinct from
// the procedure `$42_producer_get` -- which was never declared.
//
// Expected today: Boogie rejects the program with
//   "use of undeclared function: $42_producer_$get"
// rather than either verifying, or reporting the backend's own diagnostic
// ("this function has no specification but is referenced by a behavioral predicate").
//
// REQUIRED (each checked by deleting it and watching the error vanish):
//   1. `apply`'s spec has `aborts_if aborts_of<f>(s)` -- a behavioral predicate over
//      its function PARAMETER. Delete the spec entirely and this verifies;
//   2. `producer` is unverified, so inference skips its lambda. `verify = false` is
//      the file-local equivalent of `--verify-exclude` or a narrow `-f` scope;
//   3. `consumer` exists and is verified -- `apply` is generic over `f` and never
//      names the lambda, so without a verified caller reaching through `producer`
//      the lambda never enters a verified cone and no predicate is emitted;
//   4. the lambda has an abort condition mentioning the callee. With a plain
//      `let _y = get(x);` the derived contract never mentions `get` and this
//      verifies. The callee reaches the contract ONLY via `aborts_of`.
//
// NOT required (each checked; all reproduce identically):
//   - `pragma opaque` on `apply`: removing it still fails. What matters is that a
//     behavioral predicate is applied to `f`, not how the body is treated;
//   - a struct: `u64` suffices;
//   - a capture in the lambda: a constant works;
//   - splitting the accessor/HOF into their own module. Doing so is closer to the
//     real-world shape (a verified framework HOF called from an out-of-scope
//     module) and reproduces the same way; it is merged here purely for size.
//     `consumer` cannot be merged in as well -- that is a module dependency cycle.

module 0x42::producer {
    spec module {
        pragma verify = false;
    }

    public fun get(s: &u64): u64 {
        *s
    }

    spec get {
        aborts_if false;
    }

    public fun apply(s: &u64, f: |&u64| has drop) {
        f(s)
    }

    spec apply {
        pragma opaque;
        aborts_if aborts_of<f>(s);
    }

    public fun run(s: &u64) {
        apply(s, |x| {
            assert!(get(x) == 7, 1);
        })
    }
}

module 0x42::consumer {
    use 0x42::producer;

    public fun go(s: &u64) {
        producer::run(s)
    }
}

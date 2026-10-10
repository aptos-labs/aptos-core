// Regression test for two-state spec-fun calls inside behavioral evaluator
// bodies: `bump`'s spec references a `&mut`-parameter spec fun (whose Boogie
// signature takes a doubled `(old_p, p)` pair). Instrumentation doubles the
// arguments for procedure contexts; the evaluator emission translates the
// raw spec directly and must double them itself, else the emitted call is
// one argument short.
module 0x42::lambda_spec_two_state_specfun {
    spec fun unchanged_or_bumped(x: &mut u64): bool {
        x == old(x) || x == old(x) + 1
    }

    fun bump(x: &mut u64) {
        *x += 1;
    }
    spec bump {
        aborts_if x == 18446744073709551615;
        ensures unchanged_or_bumped(x);
        ensures x == old(x) + 1;
    }

    public fun apply(f: |&mut u64| has drop, x: &mut u64) {
        f(x)
    }
    spec apply {
        pragma opaque;
        pragma verify = false;
        requires requires_of<f>(x);
        aborts_if aborts_of<f>(x);
        ensures ensures_of<f>(old(x), x);
    }

    public fun drive(x: &mut u64) {
        apply(|v| bump(v), x)
    }
}

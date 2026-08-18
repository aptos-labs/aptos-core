// Regression test for lifted-lambda behavioral evaluators over global
// memory (#20326): the lambda's contract is inferred AFTER the compiler
// computed spec memory summaries, so the `$bp_*` evaluator functions were
// emitted without memory parameters while their bodies referenced memory
// globals — a Boogie name-resolution error. The inference processor now
// refreshes the summaries. `settle`'s contract is provable only if the
// evaluators carry the correct two-state memory semantics through the
// opaque function-value boundary.
module 0x42::lambda_spec_global_memory {
    struct Counter has key { total: u64 }

    fun debit(amount: u64) acquires Counter {
        let c = &mut Counter[@0x42];
        c.total -= amount;
    }
    spec debit {
        modifies global<Counter>(@0x42);
        aborts_if !exists<Counter>(@0x42);
        aborts_if global<Counter>(@0x42).total < amount;
        ensures global<Counter>(@0x42).total == old(global<Counter>(@0x42).total) - amount;
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

    public fun settle(x: &mut u64) acquires Counter {
        apply(|v| {
            debit(*v);
            *v = 0;
        }, x)
    }
    spec settle {
        aborts_if !exists<Counter>(@0x42);
        aborts_if global<Counter>(@0x42).total < x;
        ensures x == 0;
        ensures global<Counter>(@0x42).total == old(global<Counter>(@0x42).total) - old(x);
    }
}

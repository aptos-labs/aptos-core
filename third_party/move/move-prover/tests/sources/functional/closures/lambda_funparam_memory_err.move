// A lambda capturing a function-typed PARAMETER whose possible targets
// access global memory: the lambda's evaluator would dispatch through the
// per-type evaluator over memory it neither receives nor forwards. Until
// the encoding threads the per-type memory union, this configuration is
// rejected with a diagnostic (previously: Boogie name-resolution errors).
module 0x42::lambda_funparam_memory_err {
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

    public fun apply(f: |u64| has drop, x: u64) {
        f(x)
    }
    spec apply {
        pragma opaque;
        pragma verify = false;
        requires requires_of<f>(x);
        aborts_if aborts_of<f>(x);
        ensures ensures_of<f>(x);
    }

    public fun wrap(g: |u64| has copy + drop, x: u64) {
        apply(|v| g(v + 1), x)
    }

    public fun drive(x: u64) acquires Counter {
        let _ = Counter[@0x42].total;
        wrap(|v| debit(v), x)
    }
}

// Nested lambdas through a higher-order function: the HOF's own evaluator
// applies behavioral predicates to its function parameter, whose possible
// targets (the inner lambda) access global memory — the same conservative
// rejection as lambda_funparam_memory_err (previously: Boogie
// name-resolution errors).
module 0x42::lambda_nested_hof_memory_err {
    struct Bank has key { total: u64 }

    fun credit(amount: u64) acquires Bank {
        let b = &mut Bank[@0x42];
        b.total += amount;
    }
    spec credit {
        modifies global<Bank>(@0x42);
        aborts_if !exists<Bank>(@0x42);
        aborts_if global<Bank>(@0x42).total + amount > 18446744073709551615;
        ensures global<Bank>(@0x42).total == old(global<Bank>(@0x42).total) + amount;
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

    public fun drive(x: u64) {
        apply(|outer| {
            apply(|inner| credit(inner + 1), outer + 1)
        }, x)
    }
}

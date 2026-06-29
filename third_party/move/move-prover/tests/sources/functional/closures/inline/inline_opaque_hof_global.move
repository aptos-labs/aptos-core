// End-to-end test: opaque inline HOF where lambdas modify global state at
// each iteration. Demonstrates that `modifies global<R>(addr)` and global
// reads/writes propagate through behavioral predicates of an opaque inline
// HOF taking a `|address|` lambda.
module 0x42::inline_opaque_hof_global {
    use std::vector;

    struct Counter has key {
        n: u64,
    }

    /// Inline-opaque iterator over a vector of addresses, calling a void
    /// lambda on each. The closure is `copy + drop`; per-element global
    /// effects are described by the lambda's spec.
    ///
    /// The body would have to carry a frame condition stating that the
    /// lambda's effects do not change its own abort condition on the
    /// remaining elements — a property that depends on aliasing and is not
    /// expressible generically. The spec is trusted via `pragma verify = false`,
    /// the same idiom as `sum_trusted` in `opaque_inline_loop_sum.move`.
    inline fun for_each_addr(addrs: &vector<address>, f: |address| has copy + drop) {
        let i = 0;
        let n = vector::length(addrs);
        while (i < n) {
            f(addrs[i]);
            i = i + 1;
        };
    }
    spec for_each_addr {
        pragma opaque;
        pragma verify = false;
        modifies_of<f>(a: address) Counter[a];
        requires forall i in 0..len(addrs): !aborts_of<f>(addrs[i]);
        aborts_if false;
        ensures forall i in 0..len(addrs): ensures_of<f>(addrs[i]);
    }

    /// Caller: bump each address's counter by one. The lambda's spec
    /// describes the global effect at one address; the HOF's spec lifts it
    /// to the whole address list. Requires distinct addresses so that the
    /// per-element effects don't interfere through aliasing.
    fun bump_all(addrs: &vector<address>) acquires Counter {
        for_each_addr(addrs, |a| {
            let c = &mut Counter[a];
            c.n = c.n + 1;
        } spec {
            aborts_if !exists<Counter>(a);
            aborts_if global<Counter>(a).n == MAX_U64;
            modifies global<Counter>(a);
            ensures global<Counter>(a).n == old(global<Counter>(a)).n + 1;
        });
    }
    spec bump_all {
        requires forall i in 0..len(addrs): exists<Counter>(addrs[i]);
        requires forall i in 0..len(addrs): global<Counter>(addrs[i]).n < MAX_U64;
        // Pairwise distinct, so the post-condition at index i talks about a
        // counter that no other iteration touched.
        requires forall i in 0..len(addrs), j in 0..len(addrs):
            i != j ==> addrs[i] != addrs[j];
        aborts_if false;
        ensures forall i in 0..len(addrs):
            global<Counter>(addrs[i]).n == old(global<Counter>(addrs[i])).n + 1;
    }

    /// Caller: reset every counter to zero. Uses an `ensures` that does not
    /// reference `old`; the HOF still threads it through.
    fun reset_all(addrs: &vector<address>) acquires Counter {
        for_each_addr(addrs, |a| {
            Counter[a].n = 0;
        } spec {
            aborts_if !exists<Counter>(a);
            modifies global<Counter>(a);
            ensures global<Counter>(a).n == 0;
        });
    }
    spec reset_all {
        requires forall i in 0..len(addrs): exists<Counter>(addrs[i]);
        requires forall i in 0..len(addrs), j in 0..len(addrs):
            i != j ==> addrs[i] != addrs[j];
        aborts_if false;
        ensures forall i in 0..len(addrs): global<Counter>(addrs[i]).n == 0;
    }
}

// A temporary fed by two different closures has no single footprint, so the call site cannot
// say what the argument writes and deferral does not apply. The rejection comes from the
// bytecode pipeline rather than from spec rewriting: the argument is a local, not a parameter
// the enclosing function declared a frame for, so there is nothing to check it against
// earlier. The check does not care *why* a footprint is unavailable, only that it is.
module 0x42::lambda_joined_closure_not_deferred {
    struct R has key { n: u64 }

    fun bump(a: address) acquires R {
        R[a].n = R[a].n + 1;
    }
    spec bump {
        pragma opaque;
        aborts_if !exists<R>(a);
        aborts_if global<R>(a).n + 1 > MAX_U64;
        modifies global<R>(a);
        ensures global<R>(a).n == old(global<R>(a).n) + 1;
    }

    public fun apply(fv: |address| has drop, a: address) {
        fv(a)
    }
    spec apply {
        pragma opaque;
        pragma verify = false;
    }

    /// Must be rejected: only one branch writes `R`, and nothing here can havoc it.
    public fun joined(a: address, c: bool) acquires R {
        let f: |address| has drop = if (c) { |x| bump(x) } else { |_x| { } };
        apply(f, a)
    }
    spec joined {
        pragma aborts_if_is_partial = true;
        ensures global<R>(a).n == old(global<R>(a).n);
    }
}

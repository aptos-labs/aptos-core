// A closure passed to a generic opaque callee must still cause the caller to havoc the
// memory the closure writes. If a caller could claim the resource is unchanged, the
// relaxed frame check would have silenced the rejection without accounting for the
// effect, which is unsound.
module 0x42::lambda_declared_frame_havoc {
    struct Counter has key { n: u64 }
    struct Box has key { v: u64 }

    /// Generic over its callers: declares no frame for `f` and constrains no global
    /// memory of its own.
    public fun apply(b: &mut Box, f: |&mut u64|) {
        f(&mut b.v)
    }
    spec apply {
        pragma opaque;
        pragma verify = false;
    }

    fun bump(addr: address) acquires Counter {
        Counter[addr].n = Counter[addr].n + 1;
    }
    spec bump {
        pragma opaque;
        aborts_if !exists<Counter>(addr);
        aborts_if global<Counter>(addr).n + 1 > MAX_U64;
        modifies global<Counter>(addr);
        ensures global<Counter>(addr).n == old(global<Counter>(addr).n) + 1;
    }

    /// Must fail: the callback writes `Counter` through the opaque call, so the caller
    /// cannot claim it is unchanged.
    public fun claims_unchanged(b: &mut Box, addr: address) acquires Counter {
        apply(b, |v| {
            *v = *v + 1;
            bump(addr);
        })
    }
    spec claims_unchanged {
        pragma aborts_if_is_partial = true;
        modifies global<Counter>(addr);
        ensures global<Counter>(addr).n == old(global<Counter>(addr).n);
    }

    /// Must fail, and this one isolates where the rejection comes from: the caller
    /// declares no `modifies` of its own, so nothing but the callback's effect being
    /// accounted for at the call site can refute the claim.
    public fun claims_unchanged_no_caller_modifies(
        b: &mut Box, addr: address
    ): bool acquires Counter {
        let before = Counter[addr].n;
        apply(b, |v| {
            *v = *v + 1;
            bump(addr);
        });
        Counter[addr].n == before
    }
    spec claims_unchanged_no_caller_modifies {
        pragma aborts_if_is_partial = true;
        ensures result;
    }
}

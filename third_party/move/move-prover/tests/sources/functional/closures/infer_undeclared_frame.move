// An opaque callee that takes a callback but declares no frame for it, and
// constrains no global memory of its own. It is generic over its callers and
// cannot name the resources their callbacks touch, so the callback is accepted
// on the strength of its own footprint. No specification on the callback is
// needed for this.
module 0x42::infer_undeclared_frame {
    struct Counter has key { n: u64 }
    struct Box has key { v: u64 }

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

    public fun caller(b: &mut Box, addr: address) acquires Counter {
        apply(b, |v| {
            *v = *v + 1;
            bump(addr);
        })
    }
}

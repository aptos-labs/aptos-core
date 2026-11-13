// flag: --verify-only=caller
// Deferring the frame is a property of what the callee's specification says, not of what a
// given run happens to verify. Scoping a run to one function must not change which calls are
// accepted: this is the `infer_undeclared_frame` program under `--verify-only`, and it has to
// verify exactly as it does at package scope.
module 0x42::lambda_frame_deferral_scoped {
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

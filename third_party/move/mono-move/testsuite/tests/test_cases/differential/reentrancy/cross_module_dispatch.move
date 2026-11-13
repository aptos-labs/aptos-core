// Returning from cross-module and closure calls restores module activity and
// resource access. Same-module recursion inherits restrictions when its module
// has been re-entered.
// RUN: publish
module 0x42::c {
    struct RC has key, drop {
        v: u64
    }

    public fun init(s: &signer) {
        move_to(s, RC { v: 3 })
    }

    public fun read(): u64 {
        borrow_global<RC>(@0x42).v
    }

    public fun noop() {}

    // Dispatches with c's frame on the stack, then reads c's own resource.
    public fun dispatch_then_read(action: ||u64): u64 {
        action() + read()
    }
}

module 0x42::b {
    use 0x42::c;
    struct RB has key, drop {
        v: u64
    }

    public fun init(s: &signer) {
        move_to(s, RB { v: 2 })
    }

    public fun read(): u64 {
        borrow_global<RB>(@0x42).v
    }

    public fun chain(): u64 {
        read() + c::read()
    }

    // c is entered before the dispatch and returns after it; the second call
    // must not see c as still active.
    public fun dispatch_in_c_then_reenter_c(action: ||u64): u64 {
        let first = c::dispatch_then_read(action);
        first + c::read()
    }

    // c returns before the closure call, so later calls can access its resources.
    public fun call_c_dispatch_recall_c(action: ||u64): u64 {
        let first = c::read();
        first + action() + c::read()
    }
}

module 0x42::a {
    use 0x42::b;
    use 0x42::c;
    struct RA has key, drop {
        v: u64
    }

    fun init(s: &signer) {
        move_to(s, RA { v: 1 });
        b::init(s);
        c::init(s);
    }

    fun one(): u64 {
        1
    }

    // Same-module recursion; the innermost frame makes a cross-module call.
    fun recurse(n: u64): u64 {
        if (n == 0) {
            c::noop();
            0
        } else {
            1 + recurse(n - 1)
        }
    }

    // The innermost recursive call reads a's resource.
    fun recurse_then_read(n: u64): u64 {
        if (n == 0) {
            borrow_global<RA>(@0x42).v
        } else {
            1 + recurse_then_read(n - 1)
        }
    }

    // Regular calls only; every module reads its own resource.
    public fun regular_chain(s: signer): u64 {
        init(&s);
        borrow_global<RA>(@0x42).v + b::chain()
    }

    // After the a -> b -> c -> closure chain returns, further cross-module
    // calls and a's resource read remain allowed.
    public fun dispatch_deep_and_unwind(s: signer): u64 {
        init(&s);
        let total = b::dispatch_in_c_then_reenter_c(|| one());
        total + b::chain() + borrow_global<RA>(@0x42).v
    }

    public fun recall_after_dispatch(s: signer): u64 {
        init(&s);
        b::call_c_dispatch_recall_c(|| one()) + borrow_global<RA>(@0x42).v
    }

    // a is re-entered through the closure; ten same-module frames later a
    // cross-module call runs and returns.
    public fun recursion_in_reentered_module(s: signer): u64 {
        init(&s);
        c::dispatch_then_read(|| recurse(10))
    }

    // The resource access at the bottom of the recursion is reentrant.
    public fun recursion_then_read_fails(s: signer): u64 {
        init(&s);
        c::dispatch_then_read(|| recurse_then_read(10))
    }
}

// RUN: execute 0x42::a::regular_chain --args 0x42
// CHECK: results: 6

// RUN: execute 0x42::a::dispatch_deep_and_unwind --args 0x42
// CHECK: results: 13

// RUN: execute 0x42::a::recall_after_dispatch --args 0x42
// CHECK: results: 8

// RUN: execute 0x42::a::recursion_in_reentered_module --args 0x42
// CHECK: results: 13

// RUN: execute 0x42::a::recursion_then_read_fails --args 0x42
// CHECK-V1-SUBSTR: RUNTIME_DISPATCH_ERROR
// CHECK-V2-SUBSTR: resource `0x42::a::RA` is locked while its module is re-entered
// CHECK-ERROR-PARITY

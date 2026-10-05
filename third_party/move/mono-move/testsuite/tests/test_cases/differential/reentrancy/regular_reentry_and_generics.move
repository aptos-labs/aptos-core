// Regular calls, generic locking functions, and nested or recursive closures
// enforce reentry restrictions. Returning through the full call chain restores
// resource access and allows later closure calls.
// RUN: publish
module 0x42::b {
    public fun run(action: ||u64): u64 {
        action()
    }

    public fun make(): ||u64 {
        || 7
    }
}

module 0x42::c {
    use 0x42::b;
    struct RC has key, drop {
        v: u64
    }

    public fun init(s: &signer) {
        move_to(s, RC { v: 3 })
    }

    public fun read(): u64 {
        borrow_global<RC>(@0x42).v
    }

    public fun exists_rc(): u64 {
        if (exists<RC>(@0x42)) 1 else 0
    }

    public fun noop() {}

    #[module_lock]
    public fun locked_generic<T: drop>(x: T, action: |T|) {
        action(x)
    }

    // `c` is on the stack below the dispatch, which happens in `b`.
    public fun via_b(action: ||u64): u64 {
        b::run(action)
    }
}

module 0x42::d {
    use 0x42::c;

    public fun to_c_read(): u64 {
        c::read()
    }

    public fun consume(_x: u64) {}
}

module 0x42::a {
    use 0x42::b;
    use 0x42::c;
    use 0x42::d;
    struct RA has key, drop {
        v: u64
    }

    fun init(s: &signer) {
        move_to(s, RA { v: 1 });
        c::init(s);
    }

    fun bump(_x: u64) {}

    // With c active beneath b, a closure call back into c rejects its resource read.
    public fun c_active_below_reenter(s: signer): u64 {
        init(&s);
        c::via_b(|| c::read())
    }

    // Reentry rejects `exists` even when c's resource is present.
    public fun c_active_below_exists(s: signer): u64 {
        init(&s);
        c::via_b(|| c::exists_rc())
    }

    // Generic `#[module_lock]` callee across modules; closure back into a.
    public fun generic_locked(s: signer): u64 {
        init(&s);
        c::locked_generic(5u64, |x| bump(x));
        1
    }

    // Generic `#[module_lock]` callee; closure into an inactive module.
    public fun generic_locked_ok(s: signer): u64 {
        init(&s);
        c::locked_generic(5u64, |x| d::consume(x));
        1
    }

    // A closure capturing a closure; both same-module.
    public fun nested_capture(s: signer): u64 {
        init(&s);
        let inner = || borrow_global<RA>(@0x42).v;
        let outer = || inner();
        outer()
    }

    // A closure created in b, called in a.
    public fun returned_closure(): u64 {
        let f = b::make();
        f()
    }

    fun rec2(n: u64): u64 {
        if (n == 0) {
            borrow_global<RA>(@0x42).v
        } else {
            let f = |k| rec2(k);
            f(n - 1)
        }
    }

    // Recursion through a same-module closure, resource op at the bottom.
    public fun closure_recursion(s: signer): u64 {
        init(&s);
        rec2(3)
    }

    public fun closure_recursion_zero(s: signer): u64 {
        init(&s);
        rec2(0)
    }

    #[module_lock]
    fun locked_then_closure(): u64 {
        let f = || borrow_global<RA>(@0x42).v;
        f()
    }

    // A same-module closure call is allowed under the module lock, but its
    // resource access is rejected because it re-enters the module.
    public fun lock_then_local_closure(s: signer): u64 {
        init(&s);
        locked_then_closure()
    }

    // Locked root; c active below; the closure targets c directly.
    #[module_lock]
    public fun locked_root_reenter_c(s: signer): u64 {
        init(&s);
        c::via_b(|| { c::noop(); 0 })
    }

    // Locked root; c active below; the closure enters inactive d, whose
    // regular call re-enters c.
    #[module_lock]
    public fun locked_root_reenter_c_via_d(s: signer): u64 {
        init(&s);
        c::via_b(|| d::to_c_read())
    }

    // Locked root; dispatch from b into inactive d, which reads c (inactive).
    #[module_lock]
    public fun locked_root_ok(s: signer): u64 {
        init(&s);
        b::run(|| d::to_c_read())
    }

    // Without a module lock, d's regular call can re-enter active c, but c's
    // resource read is rejected.
    public fun c_reentered_by_regular_call(s: signer): u64 {
        init(&s);
        c::via_b(|| d::to_c_read())
    }

    // After the call chain returns successfully, the root can access its
    // resource and make another closure call.
    public fun unwind_then_reuse(s: signer): u64 {
        init(&s);
        let x = c::via_b(|| 2);
        let y = borrow_global<RA>(@0x42).v;
        let z = b::run(|| c::read());
        x + y + z
    }
}

// RUN: execute 0x42::a::c_active_below_reenter --args 0x42
// CHECK-V1-SUBSTR: RUNTIME_DISPATCH_ERROR
// CHECK-V2-SUBSTR: resource `0x42::c::RC` is locked while its module is re-entered
// CHECK-ERROR-PARITY

// RUN: execute 0x42::a::c_active_below_exists --args 0x42
// CHECK-V1-SUBSTR: RUNTIME_DISPATCH_ERROR
// CHECK-V2-SUBSTR: resource `0x42::c::RC` is locked while its module is re-entered
// CHECK-ERROR-PARITY

// RUN: execute 0x42::a::generic_locked --args 0x42
// CHECK-V1-SUBSTR: RUNTIME_DISPATCH_ERROR
// CHECK-V2-SUBSTR: call: `0x42::a::bump` re-enters its module while a module lock is active
// CHECK-ERROR-PARITY

// RUN: execute 0x42::a::generic_locked_ok --args 0x42
// CHECK: results: 1

// RUN: execute 0x42::a::nested_capture --args 0x42
// CHECK-V1-SUBSTR: RUNTIME_DISPATCH_ERROR
// CHECK-V2-SUBSTR: resource `0x42::a::RA` is locked while its module is re-entered
// CHECK-ERROR-PARITY

// RUN: execute 0x42::a::returned_closure
// CHECK: results: 7

// RUN: execute 0x42::a::closure_recursion --args 0x42
// CHECK-V1-SUBSTR: RUNTIME_DISPATCH_ERROR
// CHECK-V2-SUBSTR: resource `0x42::a::RA` is locked while its module is re-entered
// CHECK-ERROR-PARITY

// RUN: execute 0x42::a::closure_recursion_zero --args 0x42
// CHECK: results: 1

// RUN: execute 0x42::a::lock_then_local_closure --args 0x42
// CHECK-V1-SUBSTR: RUNTIME_DISPATCH_ERROR
// CHECK-V2-SUBSTR: resource `0x42::a::RA` is locked while its module is re-entered
// CHECK-ERROR-PARITY

// RUN: execute 0x42::a::locked_root_reenter_c --args 0x42
// CHECK-V1-SUBSTR: RUNTIME_DISPATCH_ERROR
// CHECK-V2-SUBSTR: re-enters its module while a module lock is active
// CHECK-ERROR-PARITY

// RUN: execute 0x42::a::locked_root_reenter_c_via_d --args 0x42
// CHECK-V1-SUBSTR: RUNTIME_DISPATCH_ERROR
// CHECK-V2-SUBSTR: call: `0x42::c::read` re-enters its module while a module lock is active
// CHECK-ERROR-PARITY

// RUN: execute 0x42::a::locked_root_ok --args 0x42
// CHECK: results: 3

// RUN: execute 0x42::a::c_reentered_by_regular_call --args 0x42
// CHECK-V1-SUBSTR: RUNTIME_DISPATCH_ERROR
// CHECK-V2-SUBSTR: resource `0x42::c::RC` is locked while its module is re-entered
// CHECK-ERROR-PARITY

// RUN: execute 0x42::a::unwind_then_reuse --args 0x42
// CHECK: results: 6

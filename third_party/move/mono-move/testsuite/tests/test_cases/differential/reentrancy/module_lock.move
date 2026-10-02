// While a `#[module_lock]` function is on the stack, cross-module reentry is
// rejected even without resource access. Locks held by the root, regular
// callees, and closure targets are enforced and released on return.
// RUN: publish --print(stackless)
module 0x42::callee {
    public fun run(action: ||) {
        action()
    }

    #[module_lock]
    public fun locked_run(action: ||) {
        action()
    }

    #[module_lock]
    public fun locked_noop() {}

    public fun noop() {}

    // A dispatch that stays inside `callee`.
    public fun run_self() {
        run(|| noop())
    }
}

module 0x42::caller {
    use 0x42::callee;

    // The locked root allows calls into inactive modules and same-module
    // closure calls within `callee`.
    #[module_lock]
    public fun locked_root_no_reentry(): u64 {
        callee::noop();
        callee::run_self();
        7
    }

    // The lock is held by a same-module callee entered before the dispatch.
    public fun lock_via_same_module_callee(): bool {
        locked_helper()
    }

    #[module_lock]
    fun locked_helper(): bool {
        callee::run(|| pure());
        true
    }

    // The lock is held by a cross-module callee entered before the dispatch.
    public fun lock_via_cross_module_callee(): bool {
        callee::locked_run(|| pure());
        true
    }

    // The locking callee returned before the dispatch, so no lock is held.
    public fun lock_released_before_dispatch(): u64 {
        locked_noop();
        callee::run(|| pure());
        3
    }

    #[module_lock]
    fun locked_noop() {}

    // Returning from the locking closure target releases the lock, allowing
    // the second closure call to re-enter `caller`.
    public fun locked_closure_target_releases(): u64 {
        callee::run(|| callee::locked_noop());
        callee::run(|| pure());
        5
    }

    // A cross-module call to a locking function is rejected when its module
    // is already active, even when no other lock is held.
    public fun locked_closure_target_reenters(): bool {
        callee::run(|| locked_pure());
        true
    }

    #[module_lock]
    fun locked_pure() {}

    fun pure() {}
}

// The locked root allows the closure call into inactive `mid`, then rejects
// `mid`'s regular call back into active `low`.
module 0x42::low {
    public fun run(action: ||) {
        action()
    }

    public fun noop() {}
}

module 0x42::mid {
    use 0x42::low;

    public fun back() {
        low::noop()
    }
}

module 0x42::top {
    use 0x42::low;
    use 0x42::mid;

    #[module_lock]
    public fun entry(): bool {
        low::run(|| mid::back());
        true
    }
}

// RUN: execute 0x42::caller::locked_root_no_reentry
// CHECK: results: 7

// RUN: execute 0x42::caller::lock_via_same_module_callee
// CHECK-V1-SUBSTR: RUNTIME_DISPATCH_ERROR
// CHECK-V2-SUBSTR: re-enters its module while a module lock is active
// CHECK-ERROR-PARITY

// RUN: execute 0x42::caller::lock_via_cross_module_callee
// CHECK-V1-SUBSTR: RUNTIME_DISPATCH_ERROR
// CHECK-V2-SUBSTR: re-enters its module while a module lock is active
// CHECK-ERROR-PARITY

// RUN: execute 0x42::caller::lock_released_before_dispatch
// CHECK: results: 3

// RUN: execute 0x42::caller::locked_closure_target_releases
// CHECK: results: 5

// RUN: execute 0x42::caller::locked_closure_target_reenters
// CHECK-V1-SUBSTR: RUNTIME_DISPATCH_ERROR
// CHECK-V2-SUBSTR: call: `0x42::caller::locked_pure` re-enters its module while a module lock is active
// CHECK-ERROR-PARITY

// RUN: execute 0x42::top::entry
// CHECK-V1-SUBSTR: RUNTIME_DISPATCH_ERROR
// CHECK-V2-SUBSTR: call: `0x42::low::noop` re-enters its module while a module lock is active
// CHECK-ERROR-PARITY

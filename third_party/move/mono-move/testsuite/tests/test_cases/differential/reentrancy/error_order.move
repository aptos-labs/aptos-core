// Borrows reject reentry before reporting a missing resource, and `exists`
// rejects it before returning an existence result. `move_from` and `move_to`
// report storage-operation failures before checking reentry.
// RUN: publish
module 0x42::callee {
    public fun run(action: ||) {
        action()
    }
}

module 0x42::caller {
    use 0x42::callee;
    struct R has key, drop {
        x: u64
    }

    fun borrow_missing() {
        let _ = borrow_global<R>(@0x77).x;
    }

    fun borrow_mut_missing() {
        borrow_global_mut<R>(@0x77).x = 1;
    }

    fun exists_check() {
        let _ = exists<R>(@0x77);
    }

    fun take(addr: address) {
        let R { x: _ } = move_from<R>(addr);
    }

    fun publish(s: signer) {
        move_to(&s, R { x: 2 })
    }

    public fun borrow_missing_reentered() {
        callee::run(|| borrow_missing())
    }

    public fun borrow_mut_missing_reentered() {
        callee::run(|| borrow_mut_missing())
    }

    public fun exists_reentered() {
        callee::run(|| exists_check())
    }

    public fun move_from_missing_reentered() {
        callee::run(|| take(@0x77))
    }

    public fun move_from_existing_reentered(s: signer) {
        move_to(&s, R { x: 1 });
        callee::run(|| take(@0x42))
    }

    public fun move_to_existing_reentered(s: signer) {
        move_to(&s, R { x: 1 });
        callee::run(|| publish(s))
    }

    public fun move_to_new_reentered(s: signer) {
        callee::run(|| publish(s))
    }
}

// RUN: execute 0x42::caller::borrow_missing_reentered
// CHECK-V1-SUBSTR: RUNTIME_DISPATCH_ERROR
// CHECK-V2-SUBSTR: resource `0x42::caller::R` is locked while its module is re-entered
// CHECK-ERROR-PARITY

// RUN: execute 0x42::caller::borrow_mut_missing_reentered
// CHECK-V1-SUBSTR: RUNTIME_DISPATCH_ERROR
// CHECK-V2-SUBSTR: resource `0x42::caller::R` is locked while its module is re-entered
// CHECK-ERROR-PARITY

// RUN: execute 0x42::caller::exists_reentered
// CHECK-V1-SUBSTR: RUNTIME_DISPATCH_ERROR
// CHECK-V2-SUBSTR: resource `0x42::caller::R` is locked while its module is re-entered
// CHECK-ERROR-PARITY

// RUN: execute 0x42::caller::move_from_missing_reentered
// CHECK-V1-SUBSTR: MISSING_DATA
// CHECK-V2-SUBSTR: MoveFrom: resource does not exist
// CHECK-ERROR-PARITY

// RUN: execute 0x42::caller::move_from_existing_reentered --args 0x42
// CHECK-V1-SUBSTR: RUNTIME_DISPATCH_ERROR
// CHECK-V2-SUBSTR: resource `0x42::caller::R` is locked while its module is re-entered
// CHECK-ERROR-PARITY

// RUN: execute 0x42::caller::move_to_existing_reentered --args 0x42
// CHECK-V1-SUBSTR: RESOURCE_ALREADY_EXISTS
// CHECK-V2-SUBSTR: MoveTo: resource already exists
// CHECK-ERROR-PARITY

// RUN: execute 0x42::caller::move_to_new_reentered --args 0x42
// CHECK-V1-SUBSTR: RUNTIME_DISPATCH_ERROR
// CHECK-V2-SUBSTR: resource `0x42::caller::R` is locked while its module is re-entered
// CHECK-ERROR-PARITY

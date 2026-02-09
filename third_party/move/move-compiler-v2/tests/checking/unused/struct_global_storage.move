module 0x42::m {
    // Used via move_to
    struct UsedInMoveTo has key {
        x: u64
    }

    // Used via borrow_global
    struct UsedInBorrowGlobal has key {
        y: u64
    }

    // Used via borrow_global_mut
    struct UsedInBorrowGlobalMut has key {
        z: u64
    }

    // Used via move_from
    struct UsedInMoveFrom has key {
        w: u64
    }

    // Used via exists
    struct UsedInExists has key {
        a: u64
    }

    // Used in acquires clause AND actually accessed
    struct UsedInAcquires has key {
        b: u64
    }

    // Really unused
    struct TrulyUnused has key {
        c: u64
    }

    public fun test_move_to(s: &signer) {
        move_to(s, UsedInMoveTo { x: 1 });
    }

    public fun test_borrow_global(addr: address): u64 acquires UsedInBorrowGlobal {
        borrow_global<UsedInBorrowGlobal>(addr).y
    }

    public fun test_borrow_global_mut(addr: address) acquires UsedInBorrowGlobalMut {
        borrow_global_mut<UsedInBorrowGlobalMut>(addr).z = 2;
    }

    public fun test_move_from(addr: address): UsedInMoveFrom acquires UsedInMoveFrom {
        move_from<UsedInMoveFrom>(addr)
    }

    public fun test_exists(addr: address): bool {
        exists<UsedInExists>(addr)
    }

    public fun test_acquires(addr: address): u64 acquires UsedInAcquires {
        // Acquires and actually accesses the resource
        borrow_global<UsedInAcquires>(addr).b
    }
}

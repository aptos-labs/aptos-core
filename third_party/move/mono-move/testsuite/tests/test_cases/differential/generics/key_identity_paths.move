// One resource key reached through a concrete type and through a generic
// type parameter, from the defining module and from another module, within a
// transaction and across committed steps.

// RUN: publish
module 0x42::store {
    struct S<T> has key, drop { v: T }

    public fun put<T: store + drop>(account: &signer, value: T) { move_to(account, S<T> { v: value }) }
    public fun is_present<T: store + drop>(addr: address): bool { exists<S<T>>(addr) }
    public fun take<T: store + drop>(addr: address): T { let S { v: value } = move_from<S<T>>(addr); value }
    public fun bump<T: store + drop + copy>(addr: address, value: T) { borrow_global_mut<S<T>>(addr).v = value }

    public fun put_u64(account: &signer, value: u64) { move_to(account, S<u64> { v: value }) }
    public fun has_u64(addr: address): bool { exists<S<u64>>(addr) }
    public fun get_u64(addr: address): u64 { borrow_global<S<u64>>(addr).v }
    public fun has_vec(addr: address): bool { exists<S<vector<u8>>>(addr) }

    // Concrete store, generic presence check and removal.
    public fun concrete_then_generic(account: signer, value: u64): u64 {
        let addr = std::signer::address_of(&account);
        put_u64(&account, value);
        let bits = if (is_present<u64>(addr)) 1 else 0;
        if (!is_present<u8>(addr)) bits = bits | 2;
        let got = take<u64>(addr);
        if (!has_u64(addr)) bits = bits | 4;
        got * 100 + bits
    }

    // Generic store, concrete presence check and read.
    public fun generic_then_concrete(account: signer, value: u64): u64 {
        let addr = std::signer::address_of(&account);
        put<u64>(&account, value);
        put<vector<u8>>(&account, b"x");
        let bits = if (has_u64(addr)) 1 else 0;
        if (has_vec(addr)) bits = bits | 2;
        bump<u64>(addr, value + 1);
        get_u64(addr) * 100 + bits
    }
}

module 0x42::user {
    use 0x42::store;

    struct X has store, drop { n: u8 }

    public fun cross_module(account: signer, value: u64): u64 {
        let addr = std::signer::address_of(&account);
        store::put<u64>(&account, value);
        store::put<X>(&account, X { n: 3 });
        let bits = if (store::has_u64(addr)) 1 else 0;
        if (store::is_present<X>(addr)) bits = bits | 2;
        let X { n: num } = store::take<X>(addr);
        if (!store::is_present<X>(addr)) bits = bits | 4;
        (store::get_u64(addr) + (num as u64)) * 100 + bits
    }

    public fun put_generic(account: signer, value: u64) {
        store::put<u64>(&account, value);
        store::put<X>(&account, X { n: 9 });
    }

    public fun read_both(addr: address): u64 {
        let bits = if (store::has_u64(addr)) 1 else 0;
        if (store::is_present<u64>(addr)) bits = bits | 2;
        if (store::is_present<X>(addr)) bits = bits | 4;
        store::get_u64(addr) * 100 + bits
    }

    public fun take_both(addr: address): u64 {
        let X { n: num } = store::take<X>(addr);
        store::take<u64>(addr) + (num as u64)
    }
}

// RUN: execute 0x42::store::concrete_then_generic --args 0x42, 5
// CHECK: results: 507

// RUN: execute 0x42::store::generic_then_concrete --args 0x42, 5
// CHECK: results: 603

// RUN: execute 0x42::user::cross_module --args 0x42, 5
// CHECK: results: 807

// RUN: execute 0x42::user::put_generic --args 0x43, 7 --commit
// CHECK: writes: create 0x42::store::S<u64> at 0x43 0x0700000000000000; create 0x42::store::S<0x42::user::X> at 0x43 0x09

// RUN: execute 0x42::user::read_both --args 0x43
// CHECK: results: 707

// RUN: execute 0x42::user::take_both --args 0x43 --commit
// CHECK: results: 16 | writes: delete 0x42::store::S<u64> at 0x43; delete 0x42::store::S<0x42::user::X> at 0x43

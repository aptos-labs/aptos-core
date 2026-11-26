// Calls to `public(friend)` / `friend` and `public(package)` / `package` functions of other
// modules at the same address: plain, generic, returning a struct of the callee's module, and
// writing resources of the callee's modules that a later step reads back through such calls.

// RUN: publish
module 0x42::by_friend {
    friend 0x42::caller;

    struct Pair<T> has copy, drop { left: T, right: T }

    struct Counter has key { hits: u64 }

    public(friend) fun scale(value: u64, by: u64): u64 { value * by + 1 }

    friend fun offset(value: u64): u64 { value + 1000 }

    public(friend) fun pair_of<T: copy + drop>(left: T, right: T): Pair<T> { Pair { left, right } }

    public(friend) fun fields_of<T: copy + drop>(pair: &Pair<T>): (T, T) { (pair.left, pair.right) }

    friend fun bump(owner: &signer, by: u64): u64 acquires Counter {
        let addr = std::signer::address_of(owner);
        if (!exists<Counter>(addr)) {
            move_to(owner, Counter { hits: 0 });
        };
        let counter = borrow_global_mut<Counter>(addr);
        counter.hits = counter.hits + by;
        counter.hits
    }

    public(friend) fun hits(addr: address): u64 acquires Counter {
        borrow_global<Counter>(addr).hits
    }
}

module 0x42::by_package {
    struct Tally has key { total: u128 }

    public(package) fun mix(high: u8, low: u16): u32 { ((high as u32) << 16) | (low as u32) }

    package fun add(owner: &signer, by: u128): u128 acquires Tally {
        let addr = std::signer::address_of(owner);
        if (!exists<Tally>(addr)) {
            move_to(owner, Tally { total: 0 });
        };
        let tally = borrow_global_mut<Tally>(addr);
        tally.total = tally.total + by;
        tally.total
    }

    public(package) fun total(addr: address): u128 acquires Tally {
        borrow_global<Tally>(addr).total
    }
}

module 0x42::caller {
    use 0x42::by_friend;
    use 0x42::by_package;

    public fun friend_calls(value: u64): u64 {
        by_friend::offset(by_friend::scale(value, 3))
    }

    public fun friend_generic(): (u128, u128, u8, u8) {
        let (wide_left, wide_right) = by_friend::fields_of(&by_friend::pair_of<u128>(7, 1 << 100));
        let (narrow_left, narrow_right) = by_friend::fields_of(&by_friend::pair_of(1u8, 9u8));
        (wide_left, wide_right, narrow_left, narrow_right)
    }

    public fun package_call(high: u8, low: u16): u32 {
        by_package::mix(high, low)
    }

    public fun write_both(owner: signer): u128 {
        let first = by_friend::bump(&owner, 5);
        let second = by_friend::bump(&owner, 40);
        let total = by_package::add(&owner, 1 << 64);
        ((first * 1000 + second) as u128) + total
    }

    public fun read_both(addr: address): u128 {
        (by_friend::hits(addr) as u128) + by_package::total(addr)
    }
}

// RUN: execute 0x42::caller::friend_calls --args 14
// CHECK: results: 1043

// RUN: execute 0x42::caller::friend_generic
// CHECK: results: 7, 1267650600228229401496703205376, 1, 9

// RUN: execute 0x42::caller::package_call --args 171, 4660
// CHECK: results: 11211316

// RUN: execute 0x42::caller::write_both --args 0x42 --commit
// CHECK: results: 18446744073709556661 | writes: create 0x42::by_friend::Counter at 0x42 0x2d00000000000000; create 0x42::by_package::Tally at 0x42 0x00000000000000000100000000000000

// RUN: execute 0x42::caller::read_both --args 0x42
// CHECK: results: 18446744073709551661

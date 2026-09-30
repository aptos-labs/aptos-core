// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Global storage: every resource type is a memory of its own, every
/// instantiation of a generic resource too, and a caller frames a callee
/// by the callee's modifies clauses.
module bench::storage {
    use std::signer;

    struct Counter has key {
        value: u64,
    }

    struct Bounded has key {
        value: u64,
    }
    spec Bounded {
        invariant value <= 100;
    }

    struct Coin<phantom C> has key {
        value: u64,
    }

    struct A {}
    struct B {}

    const E_EXISTS: u64 = 1;
    const E_BOUND: u64 = 2;

    public fun publish(account: &signer, value: u64) {
        let addr = signer::address_of(account);
        assert!(!exists<Counter>(addr), E_EXISTS);
        move_to(account, Counter { value });
    }
    spec publish {
        let addr = signer::address_of(account);
        aborts_if exists<Counter>(addr) with E_EXISTS;
        ensures exists<Counter>(addr);
        ensures global<Counter>(addr).value == value;
        modifies global<Counter>(addr);
    }

    public fun value(addr: address): u64 {
        Counter[addr].value
    }
    spec value {
        aborts_if !exists<Counter>(addr);
        ensures result == global<Counter>(addr).value;
    }

    public fun increment(addr: address) {
        let counter = &mut Counter[addr];
        counter.value = counter.value + 1;
    }
    spec increment {
        aborts_if !exists<Counter>(addr);
        aborts_if global<Counter>(addr).value + 1 > MAX_U64;
        ensures global<Counter>(addr).value == old(global<Counter>(addr).value) + 1;
        modifies global<Counter>(addr);
    }

    /// Two calls in a row: the second callee's precondition holds in the
    /// memory the first one leaves.
    public fun increment_twice(addr: address) {
        increment(addr);
        increment(addr);
    }
    spec increment_twice {
        aborts_if !exists<Counter>(addr);
        aborts_if global<Counter>(addr).value + 2 > MAX_U64;
        ensures global<Counter>(addr).value == old(global<Counter>(addr).value) + 2;
        modifies global<Counter>(addr);
    }

    /// The callee's frame keeps every other address.
    public fun increment_beside(addr: address, _other: address) {
        increment(addr);
    }
    spec increment_beside {
        requires addr != _other;
        aborts_if !exists<Counter>(addr);
        aborts_if global<Counter>(addr).value + 1 > MAX_U64;
        ensures exists<Counter>(_other) == old(exists<Counter>(_other));
        ensures global<Counter>(_other) == old(global<Counter>(_other));
    }

    public fun take(addr: address): u64 {
        let Counter { value } = move_from<Counter>(addr);
        value
    }
    spec take {
        aborts_if !exists<Counter>(addr);
        ensures result == old(global<Counter>(addr).value);
        ensures !exists<Counter>(addr);
        modifies global<Counter>(addr);
    }

    /// Read, take, and publish again: the memory after each step.
    public fun reset(account: &signer) {
        let addr = signer::address_of(account);
        take(addr);
        publish(account, 0);
    }
    spec reset {
        let addr = signer::address_of(account);
        aborts_if !exists<Counter>(addr);
        ensures global<Counter>(addr).value == 0;
        modifies global<Counter>(addr);
    }

    public fun swap(a: address, b: address) {
        let first = Counter[a].value;
        let second = Counter[b].value;
        let counter = &mut Counter[a];
        counter.value = second;
        let counter = &mut Counter[b];
        counter.value = first;
    }
    spec swap {
        aborts_if !exists<Counter>(a) || !exists<Counter>(b);
        ensures global<Counter>(a).value == old(global<Counter>(b).value);
        ensures global<Counter>(b).value == old(global<Counter>(a).value);
        modifies global<Counter>(a);
        modifies global<Counter>(b);
    }

    /// A stored resource keeps its data invariant.
    public fun set_bounded(addr: address, value: u64) {
        assert!(value <= 100, E_BOUND);
        let bounded = &mut Bounded[addr];
        bounded.value = value;
    }
    spec set_bounded {
        aborts_if value > 100 with E_BOUND;
        aborts_if !exists<Bounded>(addr);
        ensures global<Bounded>(addr).value == value;
        modifies global<Bounded>(addr);
    }

    public fun bounded(addr: address): u64 {
        Bounded[addr].value
    }
    spec bounded {
        aborts_if !exists<Bounded>(addr);
        ensures result <= 100;
    }

    public fun deposit<C>(addr: address, amount: u64) {
        let coin = &mut Coin<C>[addr];
        coin.value = coin.value + amount;
    }
    spec deposit {
        aborts_if !exists<Coin<C>>(addr);
        aborts_if global<Coin<C>>(addr).value + amount > MAX_U64;
        ensures global<Coin<C>>(addr).value == old(global<Coin<C>>(addr).value) + amount;
        modifies global<Coin<C>>(addr);
    }

    /// `Coin<A>` and `Coin<B>` are separate memories, and both are separate
    /// from `Counter`.
    public fun deposit_a(addr: address, amount: u64) {
        deposit<A>(addr, amount);
    }
    spec deposit_a {
        aborts_if !exists<Coin<A>>(addr);
        aborts_if global<Coin<A>>(addr).value + amount > MAX_U64;
        ensures global<Coin<A>>(addr).value == old(global<Coin<A>>(addr).value) + amount;
        ensures exists<Coin<B>>(addr) == old(exists<Coin<B>>(addr));
        ensures global<Coin<B>>(addr) == old(global<Coin<B>>(addr));
        ensures global<Counter>(addr) == old(global<Counter>(addr));
    }

    public fun deposit_both(addr: address, amount: u64) {
        deposit<A>(addr, amount);
        deposit<B>(addr, amount);
    }
    spec deposit_both {
        aborts_if !exists<Coin<A>>(addr) || !exists<Coin<B>>(addr);
        aborts_if global<Coin<A>>(addr).value + amount > MAX_U64;
        aborts_if global<Coin<B>>(addr).value + amount > MAX_U64;
        ensures global<Coin<A>>(addr).value == old(global<Coin<A>>(addr).value) + amount;
        ensures global<Coin<B>>(addr).value == old(global<Coin<B>>(addr).value) + amount;
        modifies global<Coin<A>>(addr);
        modifies global<Coin<B>>(addr);
    }

    /// A generic caller: the callee's frame at the caller's own parameter.
    public fun deposit_twice<C>(addr: address, amount: u64) {
        deposit<C>(addr, amount);
        deposit<C>(addr, amount);
    }
    spec deposit_twice {
        aborts_if !exists<Coin<C>>(addr);
        aborts_if global<Coin<C>>(addr).value + 2 * amount > MAX_U64;
        ensures global<Coin<C>>(addr).value == old(global<Coin<C>>(addr).value) + 2 * amount;
        modifies global<Coin<C>>(addr);
    }
}

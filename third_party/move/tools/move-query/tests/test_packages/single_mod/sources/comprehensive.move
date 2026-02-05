// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A comprehensive module demonstrating various Move features:
/// structs, abilities, generics, entry functions, and visibility.
module test_addr::comprehensive {
    // ========================================
    // Structs with various abilities
    // ========================================

    /// A simple counter with copy and drop.
    struct Counter has copy, drop {
        value: u64,
    }

    /// A resource stored under an account.
    struct Balance has key {
        amount: u64,
    }

    /// A generic container with store ability.
    struct Box<T: store> has key, store {
        value: T,
    }

    /// A wrapper with phantom type parameter.
    struct Wrapper<phantom T> has copy, drop {
        id: u64,
    }

    /// A generic pair of two types.
    struct Pair<T1: copy + drop, T2: copy + drop> has copy, drop {
        first: T1,
        second: T2,
    }

    /// An enum representing different states.
    enum Status has copy, drop {
        Pending,
        Active { since: u64 },
        Completed { result: u64, timestamp: u64 },
    }

    // ========================================
    // Constants
    // ========================================

    /// Error code for insufficient balance.
    const E_INSUFFICIENT_BALANCE: u64 = 1;

    /// Error code for zero value.
    const E_ZERO: u64 = 2;

    /// Maximum allowed value.
    const MAX_VALUE: u64 = 1000000;

    // ========================================
    // Entry functions
    // ========================================

    /// Initialize balance for an account.
    public entry fun initialize(account: &signer, initial: u64) {
        move_to(account, Balance { amount: initial });
    }

    /// Deposit to balance.
    public entry fun deposit(account: &signer, amount: u64) acquires Balance {
        let addr = address_of(account);
        let balance = borrow_global_mut<Balance>(addr);
        balance.amount = balance.amount + amount;
    }

    /// Transfer between accounts.
    public entry fun transfer(from: &signer, to: address, amount: u64) acquires Balance {
        let from_addr = address_of(from);
        let from_balance = borrow_global_mut<Balance>(from_addr);
        assert!(from_balance.amount >= amount, E_INSUFFICIENT_BALANCE);
        from_balance.amount = from_balance.amount - amount;

        let to_balance = borrow_global_mut<Balance>(to);
        to_balance.amount = to_balance.amount + amount;
    }

    // ========================================
    // Public functions
    // ========================================

    /// Create a new counter.
    public fun new_counter(initial: u64): Counter {
        Counter { value: initial }
    }

    /// Increment counter.
    public fun increment(counter: &mut Counter) {
        counter.value = counter.value + 1;
    }

    /// Get counter value.
    public fun get_counter_value(counter: &Counter): u64 {
        counter.value
    }

    #[view]
    /// Get balance of an address.
    public fun balance_of(addr: address): u64 acquires Balance {
        borrow_global<Balance>(addr).amount
    }

    /// Check if account has balance.
    public fun has_balance(addr: address): bool {
        exists<Balance>(addr)
    }

    /// Inline helper for doubling.
    public inline fun double(x: u64): u64 {
        x * 2
    }

    /// Native function to get address from signer.
    native fun address_of(s: &signer): address;

    // ========================================
    // Generic functions
    // ========================================

    /// Create a new box.
    public fun new_box<T: store>(value: T): Box<T> {
        Box { value }
    }

    /// Unpack a box.
    public fun unbox<T: store>(box: Box<T>): T {
        let Box { value } = box;
        value
    }

    /// Create a new wrapper.
    public fun new_wrapper<T>(id: u64): Wrapper<T> {
        Wrapper { id }
    }

    /// Create a new pair.
    public fun new_pair<T1: copy + drop, T2: copy + drop>(first: T1, second: T2): Pair<T1, T2> {
        Pair { first, second }
    }

    /// Swap elements of a pair.
    public fun swap<T: copy + drop>(pair: Pair<T, T>): Pair<T, T> {
        Pair { first: pair.second, second: pair.first }
    }

    /// Get first element.
    public fun first<T1: copy + drop, T2: copy + drop>(pair: &Pair<T1, T2>): T1 {
        pair.first
    }

    // ========================================
    // Private functions
    // ========================================

    /// Internal validation.
    fun validate_amount(amount: u64): bool {
        amount > 0 && amount <= MAX_VALUE
    }

    #[deprecated]
    /// Reset counter to zero.
    fun reset(counter: &mut Counter) {
        counter.value = 0;
    }

    #[test_only]
    /// Test helper function.
    fun test_helper(): u64 {
        42
    }

    #[test]
    #[expected_failure(abort_code = E_ZERO)]
    fun test_abort() {
        abort E_ZERO
    }

    #[test]
    #[expected_failure(abort_code = 42)]
    fun test_abort_literal() {
        abort 42
    }
}

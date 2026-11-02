// Function values: captures in leading and trailing parameter positions, a
// generic target, a closure returned from a function and stored in a struct
// field, a closure passed to a higher-order function, an abort raised inside
// the closure's target, a `&mut` argument through an invocation, and closure
// equality. Closures without `copy` are moved into their invocation.

// RUN: publish
module 0x4c::closures {
    struct Op has copy, drop {
        f: |u64| u64 has copy + drop,
    }

    fun add(x: u64, y: u64): u64 {
        x + y
    }

    fun sub(x: u64, y: u64): u64 {
        x - y
    }

    fun pick<T: copy + drop>(first: bool, a: T, b: T): T {
        if (first) a else b
    }

    fun check(limit: u64, x: u64): u64 {
        assert!(x <= limit, 7);
        x
    }

    fun bump(counter: &mut u64, by: u64) {
        *counter = *counter + by
    }

    fun adder(x: u64): |u64| u64 has copy + drop {
        |y| add(x, y)
    }

    fun apply(f: |u64| u64 has copy + drop, x: u64): u64 {
        f(x)
    }

    public fun leading(x: u64, y: u64): u64 {
        let f = |z| add(x, z);
        f(y)
    }

    public fun trailing(x: u64, y: u64): u64 {
        let f = |z| sub(z, y);
        f(x)
    }

    public fun generic(first: bool, a: u64, b: u64): u64 {
        let f = |p, q| pick<u64>(first, p, q);
        f(a, b)
    }

    public fun returned(x: u64, y: u64): u64 {
        adder(x)(y)
    }

    public fun field(x: u64, y: u64): u64 {
        let op = Op { f: adder(x) };
        (op.f)(y)
    }

    public fun higher_order(x: u64, y: u64): u64 {
        apply(|z| sub(z, x), y)
    }

    public fun checked(limit: u64, x: u64): u64 {
        let f = |z| check(limit, z);
        apply(f, x)
    }

    public fun mutated(x: u64, by: u64): u64 {
        let f: |&mut u64| has copy + drop = |counter| bump(counter, by);
        let value = x;
        f(&mut value);
        f(&mut value);
        value
    }

    public fun same(x: u64, y: u64): bool {
        adder(x) == adder(y)
    }
}

// RUN: execute 0x4c::closures::leading --args 3, 4
// RUN: execute 0x4c::closures::trailing --args 10, 4
// RUN: execute 0x4c::closures::generic --args true, 1, 2
// RUN: execute 0x4c::closures::generic --args false, 1, 2
// RUN: execute 0x4c::closures::returned --args 5, 6
// RUN: execute 0x4c::closures::field --args 7, 8
// RUN: execute 0x4c::closures::higher_order --args 2, 9
// RUN: execute 0x4c::closures::checked --args 10, 3
// RUN: execute 0x4c::closures::checked --args 10, 30
// RUN: execute 0x4c::closures::mutated --args 1, 5
// RUN: execute 0x4c::closures::same --args 4, 4
// RUN: execute 0x4c::closures::same --args 4, 5

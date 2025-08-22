module 0xABCD::function_calls {
    // 1) Identity (monomorphic) and generic identity, with hot loops

    public fun id_u64(x: u64): u64 { x }

    public fun id<T>(x: T): T { x }

    public entry fun loop_id_u64(n: u64) {
        let i = 0u64;
        let acc = 0u64;
        while (i < n) {
            acc = id_u64(acc);
            i = i + 1;
        };
    }

    public entry fun loop_id_generic_u64(n: u64) {
        let i = 0u64;
        let acc = 0u64;
        while (i < n) {
            acc = id<u64>(acc);
            i = i + 1;
        };
    }

    // 2) Nested calls (recursive Fibonacci)
    public fun fib_impl(n: u64): u64 {
        if (n < 2) n else fib_impl(n - 1) + fib_impl(n - 2)
    }

    public entry fun fib(n: u64) {
        fib_impl(n);
    }

    // 2b) Distinct nested calls f0..f20 (no recursion), each forwarding to next
    public fun f0(x: u64): u64 { f1(x) }
    public fun f1(x: u64): u64 { f2(x) }
    public fun f2(x: u64): u64 { f3(x) }
    public fun f3(x: u64): u64 { f4(x) }
    public fun f4(x: u64): u64 { f5(x) }
    public fun f5(x: u64): u64 { f6(x) }
    public fun f6(x: u64): u64 { f7(x) }
    public fun f7(x: u64): u64 { f8(x) }
    public fun f8(x: u64): u64 { f9(x) }
    public fun f9(x: u64): u64 { f10(x) }
    public fun f10(x: u64): u64 { f11(x) }
    public fun f11(x: u64): u64 { f12(x) }
    public fun f12(x: u64): u64 { f13(x) }
    public fun f13(x: u64): u64 { f14(x) }
    public fun f14(x: u64): u64 { f15(x) }
    public fun f15(x: u64): u64 { f16(x) }
    public fun f16(x: u64): u64 { f17(x) }
    public fun f17(x: u64): u64 { f18(x) }
    public fun f18(x: u64): u64 { f19(x) }
    public fun f19(x: u64): u64 { f20(x) }
    public fun f20(x: u64): u64 { x }
    public entry fun chain_call_once() { f0(100); }

    // 3) Generic borrow-heavy call in a loop (exercise &mut passing)
    public fun global_borrow<T: store>() acquires A {
        let _ = borrow_global<A<T>>(@0xABCD).y;
    }

    struct A<T: store> has store,key {
        x: T,
        y: u8,
        z: B,
    }

    struct B has store, key {
        a: vector<u64>,
        b: u8,
    }

    fun init_module(acc: &signer) {
        let x = B {
            a: 0x1::vector::empty(),
            b: 0,
        };
        let z = B {
            a: 0x1::vector::empty(),
            b: 1,
        };
        let a = A<B> {
            x,
            y: 0,
            z,
        };
        move_to(acc, a);
    }

    public entry fun loop_borrow_heavy_generic(n: u64) acquires A {
        let i = 0u64;
        while (i < n) {
            global_borrow<B>();
            i = i + 1;
        };
    }
}

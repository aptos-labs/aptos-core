module 0x123::any_with_function_values {
    use 0x1::any::pack;

    // Should fail: we cannot gain abilities.
    public entry fun roundtrip_fails_1() {
        let f: || has drop + store = || dummy();
        let _g = pack(f).unpack<|| has drop + store + copy>();
        // g();
    }

    // Should fail: we cannot drop abilities.
    public entry fun roundtrip_fails_2() {
        let f: || has drop + store + copy = || dummy();
        let g = pack(f).unpack<|| has drop + store>();
        g();
    }

    public fun dummy() {}

    public fun dummy_with_args(x: u64): u64 {
        x
    }

    public fun returns_dummy(): || {
        || dummy()
    }

    // Should fail: cannot confuse between abilities.
    public entry fun roundtrip_fails_3() {
        let f: || (||) has drop + store = returns_dummy;
        let g = pack(f).unpack<|| (|| has drop + store)>();
        g();
    }

    struct X { x: u64 }
    struct Xu64 { x: u64 }
    struct S<T, phantom U, phantom V> has key { x: T }

    public fun create(): S<X, u64, Xu64> {
        S { x: X { x: 100 } }
    }

    // Should fail: cannot confuse between different generic parameters - they are comma-separated.
    public entry fun roundtrip_fails_4() {
        let f: || S<X, u64, Xu64> has drop + store = || create();
        let g = pack(f).unpack<||S<Xu64, X, u64> has drop + store>();

        let S { x } = g();
        let Xu64 { x } = x;
        assert!(x == 100);
    }

    public entry fun roundtrip_success_1() {
        let x: u64 = 1;
        let f: ||u64 has drop + store = || dummy_with_args(x);
        let g = pack(f).unpack<||u64 has drop + store>();
        assert!(g() == 1, 404);
    }

    public entry fun roundtrip_success_2() {
        let f: || has drop + store = || dummy();
        let g = pack(f).unpack<|| has drop + store>();
        g();
    }
}

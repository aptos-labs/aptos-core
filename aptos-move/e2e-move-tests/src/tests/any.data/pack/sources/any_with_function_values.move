module 0x123::any_with_function_values {
    public fun dummy() {}

    public fun returns_dummy(): || {
        || dummy()
    }

    public entry fun roundtrip_1() {
        let f: || (||) has drop + store = returns_dummy;
        let g = 0x1::any::unpack<|| (|| has drop + store)>(0x1::any::pack(f));
        g();
    }

    struct X { x: u64 }
    struct Xu64 { x: u64 }
    struct S<T, phantom U, phantom V> has key { x: T }

    public fun create(): S<X, u64, Xu64> {
        S { x: X { x: 100 } }
    }

    public entry fun roundtrip_2() {
        let f: || S<X, u64, Xu64> has drop + store = || create();
        let g = 0x1::any::unpack<||S<Xu64, X, u64> has drop + store>(0x1::any::pack(f));

        let S { x } = g();
        let Xu64 { x } = x;
        assert!(x == 100);
    }
}

module 0x42::m {
    // Struct with type parameter
    struct Container<T> has drop {
        value: T
    }

    // Struct with phantom type parameter
    struct Marker<phantom T> has drop {}

    // Function with type parameter
    fun identity<T>(x: T): T {
        x
    }

    // Function using phantom type
    fun create_marker<T>(): Marker<T> {
        Marker {}
    }

    public fun test(): u64 {
        let c = Container { value: 42u64 };
        let _m: Marker<u8> = create_marker();
        identity(c.value)
    }
}

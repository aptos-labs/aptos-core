module 0x42::m {
    // Constant used in another constant
    const BASE: u64 = 10;
    const DERIVED: u64 = BASE * 2;

    // Constant used in function body
    const THRESHOLD: u64 = 100;

    // Constant used in struct field default (via function)
    const DEFAULT_VALUE: u64 = 42;

    // Constant used in assert
    const ERROR_CODE: u64 = 1;

    struct Data has drop {
        value: u64
    }

    fun create_default(): Data {
        Data { value: DEFAULT_VALUE }
    }

    public fun test(x: u64): u64 {
        assert!(x < THRESHOLD, ERROR_CODE);
        let d = create_default();
        DERIVED + d.value + x
    }
}

// Test that structs used in match patterns are tracked.
module 0x42::m {
    enum MyEnum has drop {
        Variant1 { value: u64 },
        Variant2 { data: u64 },
    }

    public fun test(e: MyEnum): u64 {
        match (e) {
            MyEnum::Variant1 { value } => value,
            MyEnum::Variant2 { data } => data,
        }
    }
}

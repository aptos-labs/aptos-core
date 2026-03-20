// Test that structs used as type arguments in generic calls are tracked.
module 0x42::m {
    use std::vector;

    struct Item has drop, copy { value: u64 }

    public fun test(): u64 {
        let v = vector::empty<Item>();
        vector::push_back(&mut v, Item { value: 42 });
        vector::pop_back(&mut v).value
    }
}

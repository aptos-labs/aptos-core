// use-aptos-stdlib
// A memory-free summary must not hide a nested lambda's global effects.
module 0x42::stateful_nested_map_ref {
    struct R has key { value: u64 }

    fun nested(v: &vector<vector<u64>>): vector<vector<u64>> acquires R {
        v.map_ref(|inner| inner.map_ref(|x| {
            R[@0x42].value = R[@0x42].value + 1;
            *x + 1
        }))
    }
}

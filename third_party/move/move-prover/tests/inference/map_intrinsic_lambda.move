// A lambda over an intrinsic map operation has an exact value model, so the
// result of the inline higher-order function applying it is derived.
module 0x42::map_intrinsic_lambda {
    use std::option::Option;
    use aptos_framework::ordered_map::OrderedMap;

    fun lookups(map: &OrderedMap<u64, u64>, keys: &vector<u64>): vector<Option<u64>> {
        keys.map_ref(|key| map.get(key))
    }
}

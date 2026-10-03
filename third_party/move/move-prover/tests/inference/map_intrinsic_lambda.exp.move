// A lambda over an intrinsic map operation has an exact value model, so the
// result of the inline higher-order function applying it is derived.
module 0x42::map_intrinsic_lambda {
    use std::option::Option;
    use aptos_framework::ordered_map::OrderedMap;

    fun lookups(map: &OrderedMap<u64, u64>, keys: &vector<u64>): vector<Option<u64>> {
        keys.map_ref(|key| map.get(key))
    }
    spec lookups(map: &0x1::ordered_map::OrderedMap<u64, u64>, keys: &vector<u64>): vector<0x1::option::Option<u64>> {
        use 0x1::vector;
        use 0x1::option;
        use 0x1::ordered_map;
        pragma opaque = true;
        ensures [inferred] vector::length<option::Option<u64>>(vec<option::Option<u64>>()) == 0 && ((forall x in 0..0: vec<option::Option<u64>>()[x] == (if (ordered_map::spec_contains_key<u64, u64>(map, keys[x])) option::Option::Some<u64>{e: ordered_map::spec_get<u64, u64>(map, keys[x])} else option::Option::None<u64>{})) && (vector::length<option::Option<u64>>(vector::spec_map_ref<u64, option::Option<u64>>(|key| ordered_map::get<u64, u64>(map, key), keys, len(keys))) == len(keys) && vector::spec_map_ref<u64, option::Option<u64>>(|key| ordered_map::get<u64, u64>(map, key), keys, len(keys)) == vector::spec_map_ref<u64, option::Option<u64>>(|key| ordered_map::get<u64, u64>(map, key), keys, vector::length<option::Option<u64>>(vector::spec_map_ref<u64, option::Option<u64>>(|key| ordered_map::get<u64, u64>(map, key), keys, len(keys)))) && !vector::spec_map_ref_aborts<u64, option::Option<u64>>(|key| ordered_map::get<u64, u64>(map, key), keys, vector::length<option::Option<u64>>(vector::spec_map_ref<u64, option::Option<u64>>(|key| ordered_map::get<u64, u64>(map, key), keys, len(keys)))) && (forall x in 0..vector::length<option::Option<u64>>(vector::spec_map_ref<u64, option::Option<u64>>(|key| ordered_map::get<u64, u64>(map, key), keys, len(keys))): vector::spec_map_ref<u64, option::Option<u64>>(|key| ordered_map::get<u64, u64>(map, key), keys, len(keys))[x] == (if (ordered_map::spec_contains_key<u64, u64>(map, keys[x])) option::Option::Some<u64>{e: ordered_map::spec_get<u64, u64>(map, keys[x])} else option::Option::None<u64>{})))) ==> result == vector::spec_map_ref<u64, option::Option<u64>>(|key| ordered_map::get<u64, u64>(map, key), keys, len(keys));
        aborts_if [inferred = sathard] vector::length<option::Option<u64>>(vec<option::Option<u64>>()) == 0 && (forall x in 0..0: vec<option::Option<u64>>()[x] == (if (ordered_map::spec_contains_key<u64, u64>(map, keys[x])) option::Option::Some<u64>{e: ordered_map::spec_get<u64, u64>(map, keys[x])} else option::Option::None<u64>{})) && (exists y: vector<option::Option<u64>>: y == vector::spec_map_ref<u64, option::Option<u64>>(|key| ordered_map::get<u64, u64>(map, key), keys, vector::length<option::Option<u64>>(y)) && !vector::spec_map_ref_aborts<u64, option::Option<u64>>(|key| ordered_map::get<u64, u64>(map, key), keys, vector::length<option::Option<u64>>(y)) && (forall x in 0..vector::length<option::Option<u64>>(y): y[x] == (if (ordered_map::spec_contains_key<u64, u64>(map, keys[x])) option::Option::Some<u64>{e: ordered_map::spec_get<u64, u64>(map, keys[x])} else option::Option::None<u64>{})) && vector::length<option::Option<u64>>(y) < len(keys) && vector::length<option::Option<u64>>(y) == MAX_U64);
    }

}
/*
Verification: Succeeded.
*/

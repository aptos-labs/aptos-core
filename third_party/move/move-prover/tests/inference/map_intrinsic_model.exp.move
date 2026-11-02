// The prover's map model defines the aborts and the values of intrinsic map
// operations; inference uses it instead of the executable implementation.
module 0x42::map_intrinsic_model {
    use std::option::{Self, Option};
    use aptos_framework::big_ordered_map::{Self, BigOrderedMap};
    use aptos_framework::ordered_map::{Self, OrderedMap};

    fun size(map: &BigOrderedMap<u64, u64>): u64 {
        big_ordered_map::compute_length(map)
    }
    spec size(map: &0x1::big_ordered_map::BigOrderedMap<u64, u64>): u64 {
        use 0x1::big_ordered_map;
        pragma opaque = true;
        ensures [inferred] result == big_ordered_map::spec_len<u64, u64>(map);
        aborts_if [inferred] false;
    }


    fun take(map: &mut BigOrderedMap<u64, u64>, key: u64): Option<u64> {
        big_ordered_map::remove_or_none(map, &key)
    }
    spec take(map: &mut 0x1::big_ordered_map::BigOrderedMap<u64, u64>, key: u64): 0x1::option::Option<u64> {
        use 0x1::option;
        use 0x1::big_ordered_map;
        pragma opaque = true;
        ensures [inferred] result == (if (big_ordered_map::spec_contains_key<u64, u64>(old(map), key)) option::Option::Some<u64>{e: big_ordered_map::spec_get<u64, u64>(old(map), key)} else option::Option::None<u64>{});
        ensures [inferred] map == (if (big_ordered_map::spec_contains_key<u64, u64>(old(map), key)) big_ordered_map::spec_remove<u64, u64>(old(map), key) else old(map));
        aborts_if [inferred] false;
    }


    fun take_or_zero(map: &mut BigOrderedMap<u64, u64>, key: u64): u64 {
        let removed = big_ordered_map::remove_or_none(map, &key);
        if (option::is_some(&removed)) option::destroy_some(removed) else 0
    }
    spec take_or_zero(map: &mut 0x1::big_ordered_map::BigOrderedMap<u64, u64>, key: u64): u64 {
        use 0x1::option;
        use 0x1::big_ordered_map;
        pragma opaque = true;
        ensures [inferred] map == (if (big_ordered_map::spec_contains_key<u64, u64>(old(map), key)) big_ordered_map::spec_remove<u64, u64>(old(map), key) else old(map));
        ensures [inferred] result == (if (option::is_some<u64>(if (big_ordered_map::spec_contains_key<u64, u64>(old(map), key)) option::Option::Some<u64>{e: big_ordered_map::spec_get<u64, u64>(old(map), key)} else option::Option::None<u64>{})) option::destroy_some<u64>(if (big_ordered_map::spec_contains_key<u64, u64>(old(map), key)) option::Option::Some<u64>{e: big_ordered_map::spec_get<u64, u64>(old(map), key)} else option::Option::None<u64>{}) else 0);
        aborts_if [inferred] option::is_some<u64>(if (big_ordered_map::spec_contains_key<u64, u64>(map, key)) option::Option::Some<u64>{e: big_ordered_map::spec_get<u64, u64>(map, key)} else option::Option::None<u64>{}) && aborts_of<option::destroy_some<u64>>(if (big_ordered_map::spec_contains_key<u64, u64>(map, key)) option::Option::Some<u64>{e: big_ordered_map::spec_get<u64, u64>(map, key)} else option::Option::None<u64>{});
    }


    fun put(map: &mut BigOrderedMap<u64, u64>, key: u64, value: u64): Option<u64> {
        big_ordered_map::upsert(map, key, value)
    }
    spec put(map: &mut 0x1::big_ordered_map::BigOrderedMap<u64, u64>, key: u64, value: u64): 0x1::option::Option<u64> {
        use 0x1::option;
        use 0x1::big_ordered_map;
        pragma opaque = true;
        ensures [inferred] result == (if (big_ordered_map::spec_contains_key<u64, u64>(old(map), key)) option::Option::Some<u64>{e: big_ordered_map::spec_get<u64, u64>(old(map), key)} else option::Option::None<u64>{});
        ensures [inferred] map == big_ordered_map::spec_set<u64, u64>(old(map), key, value);
        aborts_if [inferred] false;
    }


    fun lookup(map: &BigOrderedMap<u64, u64>, key: u64): Option<u64> {
        big_ordered_map::get(map, &key)
    }
    spec lookup(map: &0x1::big_ordered_map::BigOrderedMap<u64, u64>, key: u64): 0x1::option::Option<u64> {
        use 0x1::option;
        use 0x1::big_ordered_map;
        pragma opaque = true;
        ensures [inferred] result == (if (big_ordered_map::spec_contains_key<u64, u64>(map, key)) option::Option::Some<u64>{e: big_ordered_map::spec_get<u64, u64>(map, key)} else option::Option::None<u64>{});
        aborts_if [inferred] false;
    }


    fun take_ordered(map: &mut OrderedMap<u64, u64>, key: u64): Option<u64> {
        ordered_map::remove_or_none(map, &key)
    }
    spec take_ordered(map: &mut 0x1::ordered_map::OrderedMap<u64, u64>, key: u64): 0x1::option::Option<u64> {
        use 0x1::option;
        use 0x1::ordered_map;
        pragma opaque = true;
        ensures [inferred] result == (if (ordered_map::spec_contains_key<u64, u64>(old(map), key)) option::Option::Some<u64>{e: ordered_map::spec_get<u64, u64>(old(map), key)} else option::Option::None<u64>{});
        ensures [inferred] map == (if (ordered_map::spec_contains_key<u64, u64>(old(map), key)) ordered_map::spec_remove<u64, u64>(old(map), key) else old(map));
        aborts_if [inferred] false;
    }


    fun put_ordered(map: &mut OrderedMap<u64, u64>, key: u64, value: u64): Option<u64> {
        ordered_map::upsert(map, key, value)
    }
    spec put_ordered(map: &mut 0x1::ordered_map::OrderedMap<u64, u64>, key: u64, value: u64): 0x1::option::Option<u64> {
        use 0x1::option;
        use 0x1::ordered_map;
        pragma opaque = true;
        ensures [inferred] result == (if (ordered_map::spec_contains_key<u64, u64>(old(map), key)) option::Option::Some<u64>{e: ordered_map::spec_get<u64, u64>(old(map), key)} else option::Option::None<u64>{});
        ensures [inferred] map == ordered_map::spec_set<u64, u64>(old(map), key, value);
        aborts_if [inferred] false;
    }

}
/*
Verification: Succeeded.
*/

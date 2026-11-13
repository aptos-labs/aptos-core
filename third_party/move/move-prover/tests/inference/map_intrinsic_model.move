// The prover's map model defines the aborts and the values of intrinsic map
// operations; inference uses it instead of the executable implementation.
module 0x42::map_intrinsic_model {
    use std::option::{Self, Option};
    use aptos_framework::big_ordered_map::{Self, BigOrderedMap};
    use aptos_framework::ordered_map::{Self, OrderedMap};

    fun size(map: &BigOrderedMap<u64, u64>): u64 {
        big_ordered_map::compute_length(map)
    }

    fun take(map: &mut BigOrderedMap<u64, u64>, key: u64): Option<u64> {
        big_ordered_map::remove_or_none(map, &key)
    }

    fun take_or_zero(map: &mut BigOrderedMap<u64, u64>, key: u64): u64 {
        let removed = big_ordered_map::remove_or_none(map, &key);
        if (option::is_some(&removed)) option::destroy_some(removed) else 0
    }

    fun put(map: &mut BigOrderedMap<u64, u64>, key: u64, value: u64): Option<u64> {
        big_ordered_map::upsert(map, key, value)
    }

    fun lookup(map: &BigOrderedMap<u64, u64>, key: u64): Option<u64> {
        big_ordered_map::get(map, &key)
    }

    fun take_ordered(map: &mut OrderedMap<u64, u64>, key: u64): Option<u64> {
        ordered_map::remove_or_none(map, &key)
    }

    fun put_ordered(map: &mut OrderedMap<u64, u64>, key: u64, value: u64): Option<u64> {
        ordered_map::upsert(map, key, value)
    }
}

spec velor_std::big_ordered_map {

    spec BigOrderedMap {
        pragma intrinsic = map,
            map_new = new,
            map_destroy_empty = destroy_empty,
            map_has_key = contains,
            map_add_no_override = add,
            map_borrow = borrow,
            map_borrow_mut = borrow_mut,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_remove,
            map_spec_len = spec_len,
            map_spec_has_key = spec_contains_key,
            map_is_empty = is_empty;
    }

    spec native fun spec_len<K, V>(t: BigOrderedMap<K, V>): num;
    spec native fun spec_contains_key<K, V>(t: BigOrderedMap<K, V>, k: K): bool;
    spec native fun spec_set<K, V>(t: BigOrderedMap<K, V>, k: K, v: V): BigOrderedMap<K, V>;
    spec native fun spec_remove<K, V>(t: BigOrderedMap<K, V>, k: K): BigOrderedMap<K, V>;
    spec native fun spec_get<K, V>(t: BigOrderedMap<K, V>, k: K): V;


    spec new_with_config {
        pragma verify = false;
        pragma opaque;
    }

    spec new {
        pragma intrinsic;
    }

    spec new_with_reusable {
        pragma verify = false;
        pragma opaque;
    }

    spec new_with_type_size_hints {
        pragma verify = false;
        pragma opaque;
    }

    spec borrow {
        pragma intrinsic;
    }

    spec borrow_mut {
        pragma intrinsic;
    }

    spec contains {
        pragma intrinsic;
    }

    spec destroy_empty {
        pragma intrinsic;
    }

    spec add {
        pragma intrinsic;
    }

    spec remove {
        pragma opaque;
        pragma verify = false;
        aborts_if [abstract] !spec_contains_key(self, key);
        ensures [abstract] !spec_contains_key(self, key);
        ensures [abstract] spec_get(old(self), key) == result;
        ensures [abstract] spec_len(old(self)) == spec_len(self) + 1;
        ensures [abstract] forall k: K where k != key: spec_contains_key(self, k) ==> spec_get(self, k) == spec_get(old(self), k);
        ensures [abstract] forall k: K where k != key: spec_contains_key(old(self), k) == spec_contains_key(self, k);
    }

    spec is_empty {
        pragma intrinsic;
    }

    spec iter_is_end {
        pragma opaque;
        pragma verify = false;
    }

    spec iter_borrow {
        pragma opaque;
        pragma verify = false;
    }

    spec iter_borrow_mut {
        pragma opaque;
        pragma verify = false;
    }

    spec iter_is_begin {
        pragma opaque;
        pragma verify = false;
    }

    spec lower_bound {
        pragma opaque;
        pragma verify = false;
    }

    spec iter_borrow_key {
        pragma opaque;
        pragma verify = false;
    }

    spec allocate_spare_slots {
        pragma verify = false;
        pragma opaque;
    }

    spec validate_size_and_init_max_degrees {
        pragma verify = false;
        pragma opaque;
    }

    spec validate_dynamic_size_and_init_max_degrees {
        pragma verify = false;
        pragma opaque;
    }

    spec validate_static_size_and_init_max_degrees {
        pragma verify = false;
        pragma opaque;
    }

    spec keys {
        pragma verify = false;
        pragma opaque;
        ensures [abstract] forall k: K: vector::spec_contains(result, k) <==> spec_contains_key(self, k);
    }

    spec new_from<K: drop + copy + store, V: store>(keys: vector<K>, values: vector<V>): BigOrderedMap<K, V> {
        pragma opaque;
        pragma verify = false;
        aborts_if [abstract] exists i in 0..len(keys), j in 0..len(keys) where i != j : keys[i] == keys[j];
        aborts_if [abstract] len(keys) != len(values);
        ensures [abstract] forall k: K {spec_contains_key(result, k)} : vector::spec_contains(keys,k) <==> spec_contains_key(result, k);
        ensures [abstract] forall i in 0..len(keys) : spec_get(result, keys[i]) == values[i];
        ensures [abstract] spec_len(result) == len(keys);
    }

    spec upsert {
        pragma opaque;
        pragma verify = false;
        ensures [abstract] !spec_contains_key(old(self), key) ==> option::is_none(result);
        ensures [abstract] spec_contains_key(self, key);
        ensures [abstract] spec_get(self, key) == value;
        ensures [abstract] spec_contains_key(old(self), key) ==> ((option::is_some(result)) && (option::spec_borrow(result) == spec_get(old(
            self), key)));
        ensures [abstract] !spec_contains_key(old(self), key) ==> spec_len(old(self)) + 1 == spec_len(self);
        ensures [abstract] spec_contains_key(old(self), key) ==> spec_len(old(self)) == spec_len(self);
        ensures [abstract] forall k: K: spec_contains_key(old(self), k) && k != key ==> spec_get(old(self), k) == spec_get(self, k);
        ensures [abstract] forall k: K: spec_contains_key(old(self), k) ==> spec_contains_key(self, k);
    }

    spec add_all {
        pragma opaque;
        pragma verify = false;
    }

    spec borrow_front<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>): (K, &V) {
        pragma opaque;
        pragma verify = false;
        ensures [abstract] spec_contains_key(self, result_1);
        ensures [abstract] spec_get(self, result_1) == result_2;
        ensures [abstract] forall k: K where k != result_1: spec_contains_key(self, k) ==>
        std::cmp::compare(result_1, k) == std::cmp::Ordering::Less;
    }

    spec borrow_back {
        pragma opaque;
        pragma verify = false;
        ensures [abstract] spec_contains_key(self, result_1);
        ensures [abstract] spec_get(self, result_1) == result_2;
        ensures [abstract] forall k: K where k != result_1: spec_contains_key(self, k) ==>
        std::cmp::compare(result_1, k) == std::cmp::Ordering::Greater;
    }

    spec pop_front<K: drop + copy + store, V: store>(self: &mut BigOrderedMap<K, V>): (K, V) {
        pragma opaque;
        pragma verify = false;
    }

    spec pop_back {
        pragma opaque;
        pragma verify = false;
    }

    spec prev_key<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, key: &K): Option<K> {
        pragma opaque;
        pragma verify = false;
        ensures [abstract] result == std::option::spec_none() <==>
        (forall k: K {spec_contains_key(self, k)} where spec_contains_key(self, k)
        && k != key: std::cmp::compare(key, k) == std::cmp::Ordering::Less);
        ensures [abstract] result.is_some() <==>
            spec_contains_key(self, option::spec_borrow(result)) &&
            (std::cmp::compare(option::spec_borrow(result), key) == std::cmp::Ordering::Less)
            && (forall k: K {spec_contains_key(self, k), std::cmp::compare(option::spec_borrow(result), k), std::cmp::compare(key, k)} where k != option::spec_borrow(result): ((spec_contains_key(self, k) &&
            std::cmp::compare(k, key) == std::cmp::Ordering::Less)) ==>
            std::cmp::compare(option::spec_borrow(result), k) == std::cmp::Ordering::Greater);
    }


    spec next_key<K: drop + copy + store, V: store>(self: &BigOrderedMap<K, V>, key: &K): Option<K>  {
        pragma opaque;
        pragma verify = false;
        ensures [abstract] result == std::option::spec_none() <==>
        (forall k: K {spec_contains_key(self, k)} where spec_contains_key(self, k) && k != key:
        std::cmp::compare(key, k) == std::cmp::Ordering::Greater);
        ensures [abstract] result.is_some() <==>
            spec_contains_key(self, option::spec_borrow(result)) &&
            (std::cmp::compare(option::spec_borrow(result), key) == std::cmp::Ordering::Greater)
            && (forall k: K {spec_contains_key(self, k)} where k != option::spec_borrow(result): ((spec_contains_key(self, k) &&
            std::cmp::compare(k, key) == std::cmp::Ordering::Greater)) ==>
            std::cmp::compare(option::spec_borrow(result), k) == std::cmp::Ordering::Less);
    }


    spec find {
        pragma opaque;
        pragma verify = false;
    }

    spec new_begin_iter {
        pragma opaque;
        pragma verify = false;
    }

    spec new_end_iter {
        pragma opaque;
        pragma verify = false;
    }

    spec iter_next {
        pragma opaque;
        pragma verify = false;
    }

    spec iter_prev {
        pragma opaque;
        pragma verify = false;
    }

    spec compute_length {
        pragma verify = false;
        pragma opaque;
        ensures [abstract] result == spec_len(self);
    }


}

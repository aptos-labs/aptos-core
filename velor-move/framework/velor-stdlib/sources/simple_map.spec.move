/// Specifications of the `simple_map` module.
spec velor_std::simple_map {

    // Make most of the public API intrinsic. Those functions have custom specifications in the prover.

    spec SimpleMap {
        pragma intrinsic = map,
            map_new = create,
            map_len = length,
            map_destroy_empty = destroy_empty,
            map_has_key = contains_key,
            map_add_no_override = add,
            map_del_return_key = remove,
            map_borrow = borrow,
            map_borrow_mut = borrow_mut,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_remove,
            map_spec_len = spec_len,
            map_spec_has_key = spec_contains_key;
    }

    spec length {
        pragma intrinsic;
    }

    spec create {
        pragma intrinsic;
    }

    spec borrow {
        pragma intrinsic;
    }

    spec borrow_mut {
        pragma intrinsic;
    }

    spec contains_key {
        pragma intrinsic;
    }

    spec destroy_empty {
        pragma intrinsic;
    }

    spec add {
        pragma intrinsic;
    }

    spec add_all {
        pragma intrinsic;
    }

    spec remove {
        pragma intrinsic;
    }

    spec find {
        pragma verify=false;
    }

    spec keys {
        pragma verify=false;
    }

    spec values {
        pragma verify=false;
    }

    spec new<Key: store, Value: store>(): SimpleMap<Key, Value> {
        pragma intrinsic;
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] spec_len(result) == 0;
        ensures [abstract] forall k: Key: !spec_contains_key(result, k);
    }

    spec new_from<Key: store, Value: store>(
    keys: vector<Key>,
    values: vector<Value>,
    ): SimpleMap<Key, Value> {
        pragma intrinsic;
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] spec_len(result) == len(keys);
        ensures [abstract] forall k: Key: spec_contains_key(result, k) <==> vector::spec_contains(keys, k);
        ensures [abstract] forall i in 0..len(keys):
            spec_get(result, keys.borrow(i)) == values.borrow(i);
    }

    spec to_vec_pair<Key: store, Value: store>(self: SimpleMap<Key, Value>): (vector<Key>, vector<Value>) {
        pragma intrinsic;
        pragma opaque;
        ensures [abstract]
            forall k: Key: vector::spec_contains(result_1, k) <==>
                spec_contains_key(self, k);
        ensures [abstract] forall i in 0..len(result_1):
            spec_get(self, result_1.borrow(i)) == result_2.borrow(i);
    }

    spec upsert<Key: store, Value: store>(
        self: &mut SimpleMap<Key, Value>,
        key: Key,
        value: Value
        ): (std::option::Option<Key>, std::option::Option<Value>) {
        pragma intrinsic;
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] !spec_contains_key(old(self), key) ==> option::is_none(result_1);
        ensures [abstract] !spec_contains_key(old(self), key) ==> option::is_none(result_2);
        ensures [abstract] spec_contains_key(self, key);
        ensures [abstract] spec_get(self, key) == value;
        ensures [abstract] spec_contains_key(old(self), key) ==> ((option::is_some(result_1)) && (option::spec_borrow(result_1) == key));
        ensures [abstract] spec_contains_key(old(self), key) ==> ((option::is_some(result_2)) && (option::spec_borrow(result_2) == spec_get(old(
            self
        ), key)));
    }

    // Specification functions for tables
    spec native fun spec_len<K, V>(t: SimpleMap<K, V>): num;
    spec native fun spec_contains_key<K, V>(t: SimpleMap<K, V>, k: K): bool;
    spec native fun spec_set<K, V>(t: SimpleMap<K, V>, k: K, v: V): SimpleMap<K, V>;
    spec native fun spec_remove<K, V>(t: SimpleMap<K, V>, k: K): SimpleMap<K, V>;
    spec native fun spec_get<K, V>(t: SimpleMap<K, V>, k: K): V;
}

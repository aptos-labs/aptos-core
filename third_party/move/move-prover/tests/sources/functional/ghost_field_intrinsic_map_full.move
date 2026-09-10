// The template-surface sweep as a test: a ghost-bearing intrinsic map
// binding every Move-function role plus every native abort-predicate role.
// Boogie type-checks all emitted code whether or not it is called, so a
// single instance below forces every template block through the type
// checker with the ghost carrier substituted for the raw table — any block
// missing a carrier wrap/unwrap fails this test. `to_ordered_map` targets a
// ghost-LESS destination (a ghost-bearing destination is rejected, see
// intrinsic_map_conv_ghost_err).
//
// Option-returning roles (`map_upsert`, `map_remove_or_none`, `map_get`,
// `map_prev_key`, `map_next_key`) are deliberately NOT bound: their template
// blocks construct `Option` with the framework stdlib's enum constructors,
// while this test suite's stdlib has the struct representation — a
// pre-existing template dependency independent of ghost carriers; those
// blocks are covered by the framework prover battery.
module 0x42::ghost_field_intrinsic_map_full {
    enum Iter<K: copy + drop> has copy, drop {
        Some { key: K },
        End,
    }

    struct Dest<phantom K: copy + drop, phantom V> has store, drop {}
    spec Dest {
        pragma intrinsic = map,
            map_new = dest_new;
    }
    native fun dest_new<K: copy + drop, V: store>(): Dest<K, V>;

    struct MapG<phantom K: copy + drop, phantom V> has store, drop {}
    spec MapG {
        pragma intrinsic = map,
            map_new = new,
            map_new_with_config = new_with_config,
            map_len = length,
            map_is_empty = is_empty,
            map_destroy_empty = destroy_empty,
            map_has_key = contains,
            map_add_no_override = add,
            map_add_override_if_exists = set,
            map_del_must_exist = remove,
            map_del_return_key = remove_return_key,
            map_borrow = borrow,
            map_borrow_mut = borrow_mut,
            map_borrow_mut_with_default = borrow_mut_with_default,
            map_borrow_with_default = borrow_with_default,
            map_borrow_front = borrow_front,
            map_borrow_back = borrow_back,
            map_front_key = front_key,
            map_back_key = back_key,
            map_pop_front = pop_front,
            map_pop_back = pop_back,
            map_keys = keys,
            map_values = values,
            map_to_vec_pair = to_vec_pair,
            map_to_ordered_map = to_dest,
            map_new_from = new_from,
            map_add_all = add_all,
            map_upsert_all = upsert_all,
            map_append = append,
            map_append_disjoint = append_disjoint,
            map_trim = trim,
            map_replace_key_inplace = replace_key_inplace,
            map_iter_borrow_mut = iter_borrow_mut,
            map_spec_aborts_iter_borrow_mut = spec_aborts_ibm,
            map_spec_new = spec_new,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_remove,
            map_spec_len = spec_len,
            map_spec_has_key = spec_contains,
            map_spec_aborts_destroy_empty = spec_aborts_destroy_empty,
            map_spec_aborts_add = spec_aborts_add,
            map_spec_aborts_del = spec_aborts_del,
            map_spec_aborts_borrow = spec_aborts_borrow,
            map_spec_iter_valid = spec_iter_valid,
            map_spec_iter_preserved = spec_preserved;
    }

    native fun new<K: copy + drop, V: store>(): MapG<K, V>;
    native fun new_with_config<K: copy + drop, V: store>(inner_max_degree: u16, leaf_max_degree: u16, reuse_slots: bool): MapG<K, V>;
    native fun length<K: copy + drop, V>(m: &MapG<K, V>): u64;
    native fun is_empty<K: copy + drop, V>(m: &MapG<K, V>): bool;
    native fun destroy_empty<K: copy + drop, V>(m: MapG<K, V>);
    native fun contains<K: copy + drop, V>(m: &MapG<K, V>, k: K): bool;
    native fun add<K: copy + drop, V>(m: &mut MapG<K, V>, k: K, v: V);
    native fun set<K: copy + drop, V>(m: &mut MapG<K, V>, k: K, v: V);
    native fun remove<K: copy + drop, V>(m: &mut MapG<K, V>, k: K): V;
    native fun remove_return_key<K: copy + drop, V>(m: &mut MapG<K, V>, k: K): (K, V);
    native fun borrow<K: copy + drop, V>(m: &MapG<K, V>, k: K): &V;
    native fun borrow_mut<K: copy + drop, V>(m: &mut MapG<K, V>, k: K): &mut V;
    native fun borrow_mut_with_default<K: copy + drop, V: drop>(m: &mut MapG<K, V>, k: K, d: V): &mut V;
    native fun borrow_with_default<K: copy + drop, V>(m: &MapG<K, V>, k: K, d: &V): &V;
    native fun borrow_front<K: copy + drop, V>(m: &MapG<K, V>): (K, &V);
    native fun borrow_back<K: copy + drop, V>(m: &MapG<K, V>): (K, &V);
    native fun front_key<K: copy + drop, V>(m: &MapG<K, V>): K;
    native fun back_key<K: copy + drop, V>(m: &MapG<K, V>): K;
    native fun pop_front<K: copy + drop, V>(m: &mut MapG<K, V>): (K, V);
    native fun pop_back<K: copy + drop, V>(m: &mut MapG<K, V>): (K, V);
    native fun keys<K: copy + drop, V>(m: &MapG<K, V>): vector<K>;
    native fun values<K: copy + drop, V: copy>(m: &MapG<K, V>): vector<V>;
    native fun to_vec_pair<K: copy + drop, V>(m: MapG<K, V>): (vector<K>, vector<V>);
    native fun to_dest<K: copy + drop, V>(m: MapG<K, V>): Dest<K, V>;
    native fun new_from<K: copy + drop, V: store>(keys: vector<K>, values: vector<V>): MapG<K, V>;
    native fun add_all<K: copy + drop, V>(m: &mut MapG<K, V>, keys: vector<K>, values: vector<V>);
    native fun upsert_all<K: copy + drop, V>(m: &mut MapG<K, V>, keys: vector<K>, values: vector<V>);
    native fun append<K: copy + drop, V>(m: &mut MapG<K, V>, other: MapG<K, V>);
    native fun append_disjoint<K: copy + drop, V>(m: &mut MapG<K, V>, other: MapG<K, V>);
    native fun trim<K: copy + drop, V>(m: &mut MapG<K, V>, at: u64): MapG<K, V>;
    native fun replace_key_inplace<K: copy + drop, V>(m: &mut MapG<K, V>, old_k: K, new_k: K);
    native fun iter_borrow_mut<K: copy + drop, V>(it: Iter<K>, m: &mut MapG<K, V>): &mut V;

    spec native fun spec_new<K: copy + drop, V>(): MapG<K, V>;
    spec native fun spec_get<K: copy + drop, V>(m: MapG<K, V>, k: K): V;
    spec native fun spec_set<K: copy + drop, V>(m: MapG<K, V>, k: K, v: V): MapG<K, V>;
    spec native fun spec_remove<K: copy + drop, V>(m: MapG<K, V>, k: K): MapG<K, V>;
    spec native fun spec_len<K: copy + drop, V>(m: MapG<K, V>): num;
    spec native fun spec_contains<K: copy + drop, V>(m: MapG<K, V>, k: K): bool;
    spec native fun spec_aborts_ibm<K: copy + drop, V>(it: Iter<K>, m: MapG<K, V>): bool;
    spec native fun spec_aborts_destroy_empty<K: copy + drop, V>(m: MapG<K, V>): bool;
    spec native fun spec_aborts_add<K: copy + drop, V>(m: MapG<K, V>, k: K, v: V): bool;
    spec native fun spec_aborts_del<K: copy + drop, V>(m: MapG<K, V>, k: K): bool;
    spec native fun spec_aborts_borrow<K: copy + drop, V>(m: MapG<K, V>, k: K): bool;
    spec native fun spec_iter_valid<K: copy + drop, V>(it: Iter<K>, m: MapG<K, V>): bool;
    spec native fun spec_preserved<K: copy + drop, V>(m1: MapG<K, V>, m2: MapG<K, V>): bool;

    // One instance registers every bound role's template block for
    // MapG<u64, u64>; Boogie then type-checks them all.
    fun touch(): u64 {
        let m = new<u64, u64>();
        add(&mut m, 1, 2);
        length(&m)
    }
    spec touch {
        ensures result == 1;
    }
}

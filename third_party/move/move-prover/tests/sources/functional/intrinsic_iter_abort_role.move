// A custom intrinsic map binding `map_spec_aborts_iter_borrow_mut` to a
// NATIVE spec fun: the template must emit the predicate's definition
// (spec-function translation skips natives), or `aborts_if` over it would
// reference an undefined Boogie symbol. The template body mirrors the
// `iter_borrow_mut` procedure's abort behavior, so the aborts_if below is
// exact and the function verifies.
module 0x42::intrinsic_iter_abort_role {
    enum Iter<K: copy + drop> has copy, drop {
        Some { key: K },
        End,
    }

    struct Map<phantom K: copy + drop, phantom V> has store, drop {}
    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_spec_get = spec_get,
            map_iter_borrow_mut = ibm,
            map_spec_aborts_iter_borrow_mut = spec_aborts_ibm;
    }
    native fun new<K: copy + drop, V: store>(): Map<K, V>;
    native fun ibm<K: copy + drop, V>(it: Iter<K>, m: &mut Map<K, V>): &mut V;
    spec native fun spec_aborts_ibm<K: copy + drop, V>(it: Iter<K>, m: Map<K, V>): bool;
    spec native fun spec_get<K: copy + drop, V>(m: Map<K, V>, k: K): V;

    fun poke(m: &mut Map<u64, u64>, it: Iter<u64>): u64 {
        *ibm(it, m)
    }
    spec poke {
        aborts_if spec_aborts_ibm(it, m);
    }
}

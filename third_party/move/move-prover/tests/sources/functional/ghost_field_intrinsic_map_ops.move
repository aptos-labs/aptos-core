// A bound intrinsic map with a ghost `brand`: content operations must behave
// exactly as without the carrier, spec code can read the brand, structural
// mutations havoc it (nothing about it is provable afterwards), creation
// leaves it fresh-unconstrained, and writes through borrowed values PRESERVE
// it (a value write is not a structural mutation).
module 0x42::ghost_field_intrinsic_map_ops {
    struct Map<phantom K: copy + drop, phantom V> has store, drop {}

    spec Map {
        pragma intrinsic = map,
            map_new = new,
            map_has_key = contains,
            map_add_no_override = add,
            map_del_must_exist = remove,
            map_borrow = borrow,
            map_borrow_mut = borrow_mut,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_remove,
            map_spec_len = spec_len,
            map_spec_has_key = spec_contains;
        ghost brand: num;
    }

    public native fun new<K: copy + drop, V: store>(): Map<K, V>;
    public native fun contains<K: copy + drop, V>(m: &Map<K, V>, key: K): bool;
    public native fun add<K: copy + drop, V>(m: &mut Map<K, V>, key: K, val: V);
    public native fun remove<K: copy + drop, V>(m: &mut Map<K, V>, key: K): V;
    public native fun borrow<K: copy + drop, V>(m: &Map<K, V>, key: K): &V;
    public native fun borrow_mut<K: copy + drop, V>(m: &mut Map<K, V>, key: K): &mut V;

    spec native fun spec_len<K, V>(m: Map<K, V>): num;
    spec native fun spec_contains<K, V>(m: Map<K, V>, k: K): bool;
    spec native fun spec_set<K, V>(m: Map<K, V>, k: K, v: V): Map<K, V>;
    spec native fun spec_remove<K, V>(m: Map<K, V>, k: K): Map<K, V>;
    spec native fun spec_get<K, V>(m: Map<K, V>, k: K): V;

    // Content behavior through the carrier.
    fun add_get(m: &mut Map<u64, u64>) {
        add(m, 1, 7);
    }
    spec add_get {
        aborts_if spec_contains(m, 1);
        ensures spec_get(m, 1) == 7;
        ensures spec_contains(m, 1);
        ensures spec_len(m) == spec_len(old(m)) + 1;
    }

    fun rem(m: &mut Map<u64, u64>): u64 {
        remove(m, 1)
    }
    spec rem {
        aborts_if !spec_contains(m, 1);
        ensures result == spec_get(old(m), 1);
        ensures !spec_contains(m, 1);
        ensures spec_len(m) == spec_len(old(m)) - 1;
    }

    // Writes through a borrowed value go through the carrier write-back and
    // must PRESERVE the brand: a value write is not a structural mutation.
    fun borrow_write(m: &mut Map<u64, u64>) {
        let r = borrow_mut(m, 1);
        *r = 9;
    }
    spec borrow_write {
        aborts_if !spec_contains(m, 1);
        ensures spec_get(m, 1) == 9;
        ensures spec_len(m) == spec_len(old(m));
        ensures m.brand == old(m).brand;
    }

    // Creation leaves the brand fresh-unconstrained: no value is provable.
    fun mk(): Map<u64, u64> {
        new()
    }
    spec mk {
        ensures spec_len(result) == 0;
    }

    fun mk_brand_bad(): Map<u64, u64> {
        new()
    }
    spec mk_brand_bad {
        ensures result.brand == 0; // FAILS: fresh brand is unconstrained
    }

    // Structural mutation havocs the brand: preservation is not provable.
    fun add_brand_bad(m: &mut Map<u64, u64>) {
        add(m, 2, 2);
    }
    spec add_brand_bad {
        aborts_if spec_contains(m, 2);
        ensures m.brand == old(m).brand; // FAILS: mutation havocs the brand
    }
}

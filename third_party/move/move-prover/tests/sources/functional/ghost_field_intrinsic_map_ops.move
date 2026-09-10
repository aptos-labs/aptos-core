// A bound intrinsic map with a hidden validity slot (via the
// `map_spec_iter_preserved` binding): content operations must behave exactly
// as without the carrier, structural mutations havoc the slot (preservation
// is not provable afterwards), and writes through borrowed values PRESERVE
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
            map_spec_has_key = spec_contains,
            map_new_from = new_from,
            map_add_all = add_all,
            map_upsert_all = upsert_all,
            map_append = append,
            map_append_disjoint = append_disjoint,
            map_trim = trim,
            map_replace_key_inplace = replace_key_inplace,
            map_spec_iter_preserved = spec_preserved;
    }

    public native fun new<K: copy + drop, V: store>(): Map<K, V>;
    public native fun contains<K: copy + drop, V>(m: &Map<K, V>, key: K): bool;
    public native fun add<K: copy + drop, V>(m: &mut Map<K, V>, key: K, val: V);
    public native fun remove<K: copy + drop, V>(m: &mut Map<K, V>, key: K): V;
    public native fun borrow<K: copy + drop, V>(m: &Map<K, V>, key: K): &V;
    public native fun borrow_mut<K: copy + drop, V>(m: &mut Map<K, V>, key: K): &mut V;
    public native fun new_from<K: copy + drop, V: store>(keys: vector<K>, values: vector<V>): Map<K, V>;
    public native fun add_all<K: copy + drop, V>(m: &mut Map<K, V>, keys: vector<K>, values: vector<V>);
    public native fun upsert_all<K: copy + drop, V: drop>(m: &mut Map<K, V>, keys: vector<K>, values: vector<V>);
    public native fun append<K: copy + drop, V>(m: &mut Map<K, V>, other: Map<K, V>);
    public native fun append_disjoint<K: copy + drop, V>(m: &mut Map<K, V>, other: Map<K, V>);
    public native fun trim<K: copy + drop, V>(m: &mut Map<K, V>, at: u64): Map<K, V>;
    public native fun replace_key_inplace<K: copy + drop, V>(m: &mut Map<K, V>, old_key: &K, new_key: K);

    spec native fun spec_len<K, V>(m: Map<K, V>): num;
    spec native fun spec_contains<K, V>(m: Map<K, V>, k: K): bool;
    spec native fun spec_set<K, V>(m: Map<K, V>, k: K, v: V): Map<K, V>;
    spec native fun spec_remove<K, V>(m: Map<K, V>, k: K): Map<K, V>;
    spec native fun spec_get<K, V>(m: Map<K, V>, k: K): V;
    spec native fun spec_preserved<K, V>(m_new: Map<K, V>, m_old: Map<K, V>): bool;

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
    // must PRESERVE the slot: a value write is not a structural mutation.
    fun borrow_write(m: &mut Map<u64, u64>) {
        let r = borrow_mut(m, 1);
        *r = 9;
    }
    spec borrow_write {
        aborts_if !spec_contains(m, 1);
        ensures spec_get(m, 1) == 9;
        ensures spec_len(m) == spec_len(old(m));
        ensures spec_preserved(m, old(m));
    }

    fun mk(): Map<u64, u64> {
        new()
    }
    spec mk {
        ensures spec_len(result) == 0;
    }

    // Structural mutation havocs the slot: preservation is not provable.
    fun add_preserved_bad(m: &mut Map<u64, u64>) {
        add(m, 2, 2);
    }
    spec add_preserved_bad {
        aborts_if spec_contains(m, 2);
        ensures spec_preserved(m, old(m)); // FAILS: mutation havocs the slot
    }

    // Bulk operations, which model post-state via a fresh raw table wrapped
    // into the carrier at each use site. Membership facts phrased over
    // concrete vector literals are trigger-fragile in the shared templates
    // (same verdicts with the ghost field removed), so the cells below only
    // assert what verifies identically without the carrier.
    fun mk_from(): Map<u64, u64> {
        new_from(vector[1, 2], vector[7, 8])
    }
    spec mk_from {
        aborts_if false;
        ensures spec_len(result) == 2;
        ensures forall k: u64: spec_contains(result, k) ==> (k == 1 || k == 2);
    }

    fun addall(m: &mut Map<u64, u64>) {
        add_all(m, vector[1, 2], vector[7, 8]);
    }
    spec addall {
        ensures spec_len(m) == spec_len(old(m)) + 2;
    }

    fun addall_preserved_bad(m: &mut Map<u64, u64>) {
        add_all(m, vector[1, 2], vector[7, 8]);
    }
    spec addall_preserved_bad {
        ensures spec_preserved(m, old(m)); // FAILS: mutation havocs the slot
    }

    fun upsertall(m: &mut Map<u64, u64>) {
        upsert_all(m, vector[1, 2], vector[7, 8]);
    }
    spec upsertall {
        aborts_if false;
        ensures spec_len(m) >= spec_len(old(m));
        ensures spec_len(m) <= spec_len(old(m)) + 2;
    }

    fun app(m: &mut Map<u64, u64>, other: Map<u64, u64>) {
        append(m, other);
    }
    spec app {
        aborts_if false;
        ensures spec_len(m) >= spec_len(old(m));
        ensures spec_len(m) >= spec_len(other);
        ensures spec_len(m) <= spec_len(old(m)) + spec_len(other);
        ensures forall k: u64: spec_contains(m, k) <==>
            (spec_contains(old(m), k) || spec_contains(other, k));
    }

    fun app_disjoint(m: &mut Map<u64, u64>, other: Map<u64, u64>) {
        append_disjoint(m, other);
    }
    spec app_disjoint {
        aborts_if exists k: u64: spec_contains(m, k) && spec_contains(other, k);
        ensures spec_len(m) == spec_len(old(m)) + spec_len(other);
        ensures forall k: u64: spec_contains(m, k) <==>
            (spec_contains(old(m), k) || spec_contains(other, k));
    }

    fun tr(m: &mut Map<u64, u64>): Map<u64, u64> {
        trim(m, 1)
    }
    spec tr {
        aborts_if spec_len(m) < 1;
        ensures spec_len(m) == 1;
        ensures spec_len(result) == spec_len(old(m)) - 1;
        ensures forall k: u64: spec_contains(old(m), k) <==>
            (spec_contains(m, k) || spec_contains(result, k));
        ensures forall k: u64: !(spec_contains(m, k) && spec_contains(result, k));
    }

    // May also abort nondeterministically (models the Move-level abort on the
    // new key violating the map's key order), so the aborts spec stays partial.
    fun rk(m: &mut Map<u64, u64>) {
        replace_key_inplace(m, &1, 2)
    }
    spec rk {
        pragma aborts_if_is_partial = true;
        aborts_if !spec_contains(m, 1);
        ensures !spec_contains(m, 1);
        ensures spec_contains(m, 2);
        ensures spec_len(m) == spec_len(old(m));
        ensures forall k: u64: (k != 1 && k != 2) ==>
            (spec_contains(m, k) == spec_contains(old(m), k));
    }
}

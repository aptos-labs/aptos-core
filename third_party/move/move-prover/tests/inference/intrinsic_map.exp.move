// Test that intrinsic map Move functions are inlined as pure spec calls during
// spec inference, rather than becoming behavior predicates.
//
// `SimpleMap::contains_key`, `length`, and `create` are intrinsic Move functions that map
// to `spec_contains_key`, `spec_len`, and `spec_new` respectively via the IntrinsicDecl
// pairing table. Before the fix, try_as_pure_spec_call returned None for these (they have
// no `$name` spec function body), making them behavior predicates and producing
// `result_of<contains_key>(...)` in inferred specs instead of `spec_contains_key(...)`.
//
// Note: `borrow` (→ spec_get) is excluded from the return-value test because it returns
// a reference, but abort behavior is now covered via spec_aborts_borrow for all aborting
// intrinsic map operations (destroy_empty, add, del, borrow, borrow_mut).
module 0x42::intrinsic_map {
    use aptos_std::simple_map::{Self, SimpleMap};

    // Wraps contains_key — inference should inline spec_contains_key, not use result_of.
    fun has(m: &SimpleMap<u64, u64>, k: u64): bool {
        simple_map::contains_key(m, &k)
    }
    spec has(m: &simple_map::SimpleMap<u64, u64>, k: u64): bool {
        use 0x1::simple_map;
        pragma opaque = true;
        ensures [inferred] result == simple_map::spec_contains_key<u64, u64>(m, k);
        aborts_if [inferred] aborts_of<simple_map::contains_key<u64, u64>>(m, k);
    }


    // Wraps length — inference should inline spec_len.
    fun size(m: &SimpleMap<u64, u64>): u64 {
        simple_map::length(m)
    }
    spec size(m: &simple_map::SimpleMap<u64, u64>): u64 {
        use 0x1::simple_map;
        pragma opaque = true;
        ensures [inferred] result == simple_map::spec_len<u64, u64>(m);
        aborts_if [inferred] aborts_of<simple_map::length<u64, u64>>(m);
    }


    // Wraps create — inference should inline spec_new, not use result_of.
    fun make(): SimpleMap<u64, u64> {
        simple_map::create()
    }
    spec make(): simple_map::SimpleMap<u64, u64> {
        use 0x1::simple_map;
        pragma opaque = true;
        ensures [inferred] result == simple_map::spec_new<u64, u64>();
        aborts_if [inferred] aborts_of<simple_map::create<u64, u64>>();
    }


    // Wraps destroy_empty — aborts if the map is non-empty.
    // aborts_of<destroy_empty> delegates to spec_aborts_destroy_empty, which is
    // axiomatized as `LenTable(t) != 0` in Boogie, so verification succeeds.
    fun drop(m: SimpleMap<u64, u64>) {
        simple_map::destroy_empty(m)
    }
    spec drop(m: simple_map::SimpleMap<u64, u64>) {
        pragma opaque = true;
        ensures [inferred] ensures_of<simple_map::destroy_empty<u64, u64>>(m);
        aborts_if [inferred] aborts_of<simple_map::destroy_empty<u64, u64>>(m);
    }


    // Wraps borrow — inference should inline spec_get (via the spec_fun pairing in
    // IntrinsicFunDef / INTRINSIC_TYPE_MAP_ASSOC_FUNCTIONS), not produce result_of<borrow>.
    // Also tests that aborts_of<borrow> delegates to spec_aborts_borrow via the
    // abort_spec_fun pairing in IntrinsicFunDef.
    fun get_value(m: &SimpleMap<u64, u64>, k: u64): u64 {
        *simple_map::borrow(m, &k)
    }
    spec get_value(m: &simple_map::SimpleMap<u64, u64>, k: u64): u64 {
        use 0x1::simple_map;
        pragma opaque = true;
        ensures [inferred] result == simple_map::spec_get<u64, u64>(m, k);
        aborts_if [inferred] aborts_of<simple_map::borrow<u64, u64>>(m, k);
    }

}
/*
Verification: Succeeded.
*/

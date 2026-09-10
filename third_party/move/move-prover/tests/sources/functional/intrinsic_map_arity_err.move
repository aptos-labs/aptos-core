// An intrinsic map type must have exactly two type parameters: the templates
// and monomorphization index the instantiation as (key, value), so any other
// arity would previously panic with an out-of-bounds read once an instance
// was registered. Rejected at the intrinsic declaration instead.
module 0x42::intrinsic_map_arity_err {
    struct Map<phantom K: copy + drop> has store, drop {}
    spec Map {
        pragma intrinsic = map,
            map_new = new;
    }
    native fun new<K: copy + drop>(): Map<K>;

    fun mk(): Map<u64> {
        new()
    }
}

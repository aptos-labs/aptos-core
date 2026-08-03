// Ghost fields on intrinsic MAP types are accepted: the backend represents
// such maps with a carrier datatype so the ghosts have constructor arguments
// to live in. A map without role bindings cannot reach its values in the
// model; declaring one with a ghost must not disturb anything (in particular,
// data-invariant instrumentation has nothing to propagate and must not
// choke on the missing `map_spec_get` binding).
module 0x42::ghost_field_intrinsic_map {
    struct MapG<phantom K: copy + drop, phantom V> has store { dummy: bool }
    spec MapG {
        pragma intrinsic = map;
        ghost brand: num;
    }

    fun touch(_m: &mut MapG<u64, u64>) {
    }
    spec touch {
        aborts_if false;
    }
}

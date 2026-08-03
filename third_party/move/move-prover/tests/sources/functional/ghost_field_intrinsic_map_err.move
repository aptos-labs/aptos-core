// Ghost fields of intrinsic map types are managed by the map's backend model
// (e.g. an iterator-validity brand havocked by mutating operations) and are
// read-only from spec blocks. This is a transformation-stage error, so it
// lives in its own file (front-end-error modules never reach that stage).
module 0x42::ghost_field_intrinsic_map_err {
    struct MapG<phantom K: copy + drop, phantom V> has store { dummy: bool }
    spec MapG {
        pragma intrinsic = map;
        ghost brand: num;
    }

    fun imap_update(m: &mut MapG<u64, u64>) {
        spec {
            update m.brand = 1; // error: intrinsic map ghosts are read-only
        };
    }
}

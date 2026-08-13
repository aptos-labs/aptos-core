// Ghost-field updates in inferred contracts: the ghost update must not be
// folded into the runtime pack (ghost offsets index a separate namespace),
// so the inferred ensures preserves the runtime field values.
module 0x42::ghost_field_update {
    struct S has copy, drop { x: u64 }
    spec S {
        ghost g: u64;
    }

    fun pack_and_update_ghost(): S {
        let s = S { x: 0 };
        spec {
            update s.g = 1;
        };
        s
    }
}

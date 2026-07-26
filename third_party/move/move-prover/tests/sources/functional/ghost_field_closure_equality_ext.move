// flag: --vector-theory=SmtArrayExt
// Containers embedding function values under an extensional theory (native
// equality enabled): `has_native_equality` must classify a fun type by its
// closure captures — a ghost-bearing capture disqualifies the container from
// raw equality, like any other ghost. SmtArrayExt is the extensional vehicle
// for the same reason as in ghost_field_map_equality_ext.
module 0x42::ghost_field_closure_equality_ext {
    struct S has copy, drop { x: u64 }
    spec S {
        ghost g: u64;
    }

    fun mk(gv: u64): |u64|u64 has copy + drop {
        let s = S { x: 0 };
        spec {
            update s.g = gv;
        };
        |y| y + s.x
    }

    struct FS has copy, drop { f: |u64|u64 has copy + drop }

    fun vec_eq(): (vector<|u64|u64 has copy + drop>, vector<|u64|u64 has copy + drop>) {
        (vector[mk(1)], vector[mk(2)])
    }
    spec vec_eq {
        ensures result_1 == result_2;
    }

    fun vec_neq(): (vector<|u64|u64 has copy + drop>, vector<|u64|u64 has copy + drop>) {
        (vector[mk(1)], vector[mk(2)])
    }
    spec vec_neq {
        ensures result_1 != result_2; // FAILS: they are Move-equal
    }

    fun fs_eq(): (FS, FS) {
        (FS { f: mk(1) }, FS { f: mk(2) })
    }
    spec fs_eq {
        ensures result_1 == result_2;
    }

    fun fs_neq(): (FS, FS) {
        (FS { f: mk(1) }, FS { f: mk(2) })
    }
    spec fs_neq {
        ensures result_1 != result_2; // FAILS: they are Move-equal
    }
}

// Tests error diagnostics for ghost field declarations and updates.
module 0x42::ghost_field_err {
    struct S has copy, drop { x: u64 }
    spec S {
        ghost x: u64; // error: clashes with a runtime field
        ghost g: u64;
        ghost g: u64; // error: duplicate ghost field
    }

    fun f(s: S): u64 { s.x }
    spec f {
        ghost bad: u64; // error: not a struct spec block
    }

    fun upd(s: &mut S) {
        spec {
            update s.x = 1; // error: not a ghost field
        };
    }

    fun upd_pure(s: S): S {
        spec {
            update pass(s).g = 1; // error: target not a local
        };
        s
    }

    fun pass(s: S): S { s }

    fun upd_imm(s: &S) {
        spec {
            update s.g = 1; // error: behind an immutable reference
        };
    }

    struct RefGhost has copy, drop { x: u64 }
    spec RefGhost {
        ghost r: &mut u64; // error: reference-typed ghost field
    }

    struct RecGhost has copy, drop { x: u64 }
    spec RecGhost {
        ghost cyc: RecGhost; // error: transitively self-referential
    }

    // Mutually recursive ghost types: the cycle runs entirely through ghost
    // fields, so the check must walk intermediate structs' ghosts, not just
    // their runtime fields. The later-declared edge is rejected, breaking
    // the cycle.
    struct MraA has copy, drop { x: u64 }
    struct MraB has copy, drop { y: u64 }
    spec MraA { ghost g: MraB; }
    spec MraB { ghost h: MraA; } // error: transitively self-referential

    // A benign generic instantiation must not shield a recursive one: the
    // cycle GenS -> GenTwo -> Wrap<GenS> -> GenS is only visible when the
    // visited set keys on the full instantiation and runtime layouts are
    // available (post-definition-analysis check).
    struct Wrap<T> has copy, drop { v: T }
    struct GenS has copy, drop { x: u64 }
    struct GenTwo has copy, drop { a: Wrap<u64>, b: Wrap<GenS> }
    spec GenS { ghost g: GenTwo; } // error: transitively self-referential

    // Bitwise operators are rejected in ghost field expressions (both in
    // updates and in initializers), because ghost integer fields are modeled
    // as unbounded integers in Boogie and bitwise ops lower to bitvectors.
    struct BwG has copy, drop { x: u64 }
    spec BwG {
        ghost g: u64 = x | 1; // error: bitwise operator in ghost initializer
    }

    fun bw_upd(s: &mut BwG) {
        spec {
            update s.g = s.g | 1; // error: bitwise operator in ghost update
        };
    }

    // Non-map intrinsic types have no generated Boogie datatype to carry
    // ghost constructor arguments and reject ghosts. (Intrinsic MAP types are
    // the exception: they gain a carrier datatype, used for iterator
    // validity.)
    struct IntrinsicOther has store { dummy: bool }
    spec IntrinsicOther {
        pragma intrinsic;
        ghost g: u64; // error: ghost on (non-map) intrinsic type
    }

    // Intrinsic MAP ghosts are accepted (see ghost_field_intrinsic_map.move)
    // but cannot reference the map's type parameters (the carrier bakes ghost
    // types in once per map type). Read-only enforcement is checked in
    // ghost_field_intrinsic_map_err.move (it is a transformation-stage error,
    // which this module full of front-end errors never reaches).
    struct IntrinsicMapG<K: copy + drop, V> has store { dummy: bool }
    spec IntrinsicMapG {
        pragma intrinsic = map;
        ghost brand: num; // error: ghost on intrinsic type
        ghost bad: K; // error: ghost on intrinsic type
    }


    // int2bv also produces a bitvector and is rejected the same way.
    struct I2bG has copy, drop { x: u64 }
    spec I2bG {
        ghost g: u64 = int2bv(x); // error: int2bv in ghost initializer
    }

    fun i2b_upd(s: &mut I2bG) {
        spec {
            update s.g = int2bv(s.g); // error: int2bv in ghost update
        };
    }

    // The functional `update_field(s, g, rhs)` form rejects bitwise RHS the
    // same way as direct `update s.g = rhs`.
    fun upd_field_bv(s: I2bG): I2bG { s }
    spec upd_field_bv {
        // error: bitwise in functional ghost update
        ensures result == update_field(s, g, s.g | 1);
    }
}

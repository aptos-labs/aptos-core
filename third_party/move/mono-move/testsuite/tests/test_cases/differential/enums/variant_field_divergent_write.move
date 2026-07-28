// RUN: publish --print(micro-ops)
module 0x43::variant_field_divergent_write {
    enum E has drop {
        P0 { lead: u8, x: u8 },
        P1 { lead: u16, x: u8 },
    }

    enum F has drop {
        Fa { x: u8 },
        Fb { x: u8, y: u8 },
    }

    fun make_e(sel: u64): E {
        if (sel == 0) {
            E::P0 { lead: 1, x: 0 }
        } else {
            E::P1 { lead: 2, x: 0 }
        }
    }

    fun set_x(e: &mut E, v: u8) {
        e.x = v + 1;
    }

    fun write_read(sel: u64, v: u8): u8 {
        let e = make_e(sel);
        set_x(&mut e, v);
        e.x
    }

    fun make_f(sel: u64): F {
        if (sel == 0) {
            F::Fa { x: 1 }
        } else {
            F::Fb { x: 1, y: 7 }
        }
    }

    // Divergent read: aborts on `Fa`.
    fun read_y(f: &F): u8 {
        f.y
    }

    // Divergent write: aborts on `Fa`.
    fun set_y(f: &mut F, v: u8) {
        f.y = v + 1;
    }

    fun run_read_y(sel: u64): u8 {
        let f = make_f(sel);
        read_y(&f)
    }

    fun run_write_y(sel: u64, v: u8): u8 {
        let f = make_f(sel);
        set_y(&mut f, v);
        read_y(&f)
    }
}

// Divergent write + read back on both variants.
// RUN: execute 0x43::variant_field_divergent_write::write_read --args 0, 41
// CHECK: results: 42
// RUN: execute 0x43::variant_field_divergent_write::write_read --args 1, 41
// CHECK: results: 42

// Divergent read: value on Fb, variant-mismatch abort on Fa.
// RUN: execute 0x43::variant_field_divergent_write::run_read_y --args 1
// CHECK: results: 7
// RUN: execute 0x43::variant_field_divergent_write::run_read_y --args 0
// CHECK-SUBSTR: STRUCT_VARIANT_MISMATCH

// Divergent write: value on Fb, variant-mismatch abort on Fa.
// RUN: execute 0x43::variant_field_divergent_write::run_write_y --args 1, 9
// CHECK: results: 10
// RUN: execute 0x43::variant_field_divergent_write::run_write_y --args 0, 9
// CHECK-SUBSTR: STRUCT_VARIANT_MISMATCH

// RUN: publish --print(stackless,micro-ops)
module 0x99::chained_variant_field_fusion {
    struct Inner has drop, copy {
        a: u8,
        b: u8,
    }

    enum U has drop {
        Ua { p: Inner },
        Ub { p: Inner },
    }

    enum E has drop {
        P0 { lead: u8, p: Inner },
        P1 { lead: u16, p: Inner },
    }

    enum F has drop {
        Fa { x: u8 },
        Fb { x: u8, y: Inner },
    }

    // Uniform enum-rooted chain.
    fun read_u(u: &U): u8 {
        u.p.b
    }

    // Divergent enum-rooted chain read (both variants present).
    fun read_e(e: &E): u8 {
        e.p.b
    }

    // Divergent enum-rooted chain write (both variants present).
    fun write_e(e: &mut E, v: u8) {
        e.p.b = v + 1;
    }

    // Divergent chain that aborts when the runtime variant lacks the field.
    fun read_f(f: &F): u8 {
        f.y.b
    }

    fun mk_u(sel: u64, b: u8): U {
        if (sel == 0) { U::Ua { p: Inner { a: 7, b } } } else { U::Ub { p: Inner { a: 7, b } } }
    }

    fun mk_e(sel: u64, b: u8): E {
        if (sel == 0) { E::P0 { lead: 1, p: Inner { a: 7, b } } } else {
            E::P1 { lead: 2, p: Inner { a: 7, b } }
        }
    }

    fun run_read_u(sel: u64, b: u8): u8 {
        let u = mk_u(sel, b);
        read_u(&u)
    }

    fun run_read_e(sel: u64, b: u8): u8 {
        let e = mk_e(sel, b);
        read_e(&e)
    }

    fun run_write_e(sel: u64, v: u8): u8 {
        let e = mk_e(sel, 0);
        write_e(&mut e, v);
        read_e(&e)
    }

    fun run_read_f(sel: u64): u8 {
        let f = if (sel == 0) { F::Fa { x: 1 } } else { F::Fb { x: 1, y: Inner { a: 7, b: 9 } } };
        read_f(&f)
    }
    fun imm_ref(holder: &E): &u8 {
        &holder.p.b
    }

    fun mut_ref(holder: &mut E): &mut u8 {
        &mut holder.p.b
    }

    fun read_via_imm_ref(sel: u64, value: u8): u8 {
        let holder = mk_e(sel, value);
        *imm_ref(&holder)
    }

    fun bump_via_mut_ref(sel: u64, value: u8): u8 {
        let holder = mk_e(sel, value);
        let field_ref = mut_ref(&mut holder);
        *field_ref = *field_ref + 1;
        *field_ref
    }
}

// Uniform chain: same result on either variant.
// RUN: execute 0x99::chained_variant_field_fusion::run_read_u --args 0, 42
// CHECK: results: 42
// RUN: execute 0x99::chained_variant_field_fusion::run_read_u --args 1, 42
// CHECK: results: 42

// Divergent chain read: the byte offset differs per variant, the value does not.
// RUN: execute 0x99::chained_variant_field_fusion::run_read_e --args 0, 55
// CHECK: results: 55
// RUN: execute 0x99::chained_variant_field_fusion::run_read_e --args 1, 55
// CHECK: results: 55

// Divergent chain write through &mut, then read back.
// RUN: execute 0x99::chained_variant_field_fusion::run_write_e --args 0, 77
// CHECK: results: 78
// RUN: execute 0x99::chained_variant_field_fusion::run_write_e --args 1, 77
// CHECK: results: 78

// Divergent chain that reads a field present only on Fb.
// RUN: execute 0x99::chained_variant_field_fusion::run_read_f --args 1
// CHECK: results: 9
// Reading f.y.b on an Fa aborts with a variant mismatch on both VMs.
// RUN: execute 0x99::chained_variant_field_fusion::run_read_f --args 0
// CHECK-V1-SUBSTR: STRUCT_VARIANT_MISMATCH
// CHECK-V2-SUBSTR: enum variant mismatch

// RUN: execute 0x99::chained_variant_field_fusion::read_via_imm_ref --args 0, 33
// CHECK: results: 33

// RUN: execute 0x99::chained_variant_field_fusion::read_via_imm_ref --args 1, 33
// CHECK: results: 33

// RUN: execute 0x99::chained_variant_field_fusion::bump_via_mut_ref --args 0, 40
// CHECK: results: 41

// RUN: execute 0x99::chained_variant_field_fusion::bump_via_mut_ref --args 1, 40
// CHECK: results: 41

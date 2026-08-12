// RUN: publish --print(stackless,micro-ops)
module 0x77::variant_field_tail_borrows {
    struct Inner has drop, copy {
        a: u8,
        b: u8,
    }

    enum E has drop {
        P0 { lead: u8, p: Inner },
        P1 { lead: u16, p: Inner },
    }

    // &e.p.b escapes (returned): a divergent variant-field borrow, then an
    // immutable borrow of the tail field.
    fun imm_ref(e: &E): &u8 {
        &e.p.b
    }

    // &mut e.p.b escapes (returned): the mutable form of the same shape.
    fun mut_ref(e: &mut E): &mut u8 {
        &mut e.p.b
    }

    fun mk(sel: u64, b: u8): E {
        if (sel == 0) { E::P0 { lead: 1, p: Inner { a: 7, b } } } else {
            E::P1 { lead: 2, p: Inner { a: 7, b } }
        }
    }

    fun read_via_imm_ref(sel: u64, b: u8): u8 {
        let e = mk(sel, b);
        *imm_ref(&e)
    }

    fun bump_via_mut_ref(sel: u64, b: u8): u8 {
        let e = mk(sel, b);
        let r = mut_ref(&mut e);
        *r = *r + 1;
        *r
    }
}

// RUN: execute 0x77::variant_field_tail_borrows::read_via_imm_ref --args 0, 33
// CHECK: results: 33
// RUN: execute 0x77::variant_field_tail_borrows::read_via_imm_ref --args 1, 33
// CHECK: results: 33
// RUN: execute 0x77::variant_field_tail_borrows::bump_via_mut_ref --args 0, 40
// CHECK: results: 41
// RUN: execute 0x77::variant_field_tail_borrows::bump_via_mut_ref --args 1, 40
// CHECK: results: 41

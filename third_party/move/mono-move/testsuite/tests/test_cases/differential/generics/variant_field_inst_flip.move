// Generic functions that read and write the field `x` shared by both variants of `E<T>`.
// Where `x` sits depends on `T`: at the same place in both variants for some instantiations
// and at a different place per variant for others.

// RUN: publish
module 0x1::variant_field_inst_flip {
    enum E<T> has drop {
        A { a: T, x: u64 },
        B { b: u64, x: u64 },
    }

    fun get_x<T: drop>(item: &E<T>): u64 { item.x }

    fun set_x<T: drop>(item: &mut E<T>, new_x: u64) { item.x = new_x }

    fun bump<T: drop>(item: &mut E<T>) {
        let x_ref = &mut item.x;
        *x_ref = *x_ref + 1
    }

    fun make<T: drop>(sel: u64, payload: T): E<T> {
        if (sel == 0) E::A { a: payload, x: 10 } else E::B { b: 2, x: 20 }
    }

    fun churn<T: drop>(item: E<T>): u64 {
        let next = get_x(&item) + 1;
        set_x(&mut item, next);
        bump(&mut item);
        get_x(&item)
    }

    public fun run_u8(sel: u64): u64 { churn(make(sel, 255u8)) }

    public fun run_u64(sel: u64): u64 { churn(make(sel, 18446744073709551615)) }

    public fun run_u128(sel: u64): u64 { churn(make(sel, 340282366920938463463374607431768211455u128)) }

    // Both instantiations in one transaction.
    public fun run_mixed(sel: u64): u64 {
        churn(make(sel, 7u8)) * 1000 + churn(make(1 - sel, 7u128))
    }

    // The `T` payload must survive the writes to `x`.
    public fun payload_u128(): u128 {
        let item = make(0, 340282366920938463463374607431768211455u128);
        bump(&mut item);
        match (item) {
            E::A { a: payload, x: count } => payload - (count as u128),
            E::B { b: _, x: _ } => 0,
        }
    }
}

// RUN: execute 0x1::variant_field_inst_flip::run_u8 --args 0
// CHECK: results: 12

// RUN: execute 0x1::variant_field_inst_flip::run_u8 --args 1
// CHECK: results: 22

// RUN: execute 0x1::variant_field_inst_flip::run_u64 --args 0
// CHECK: results: 12

// RUN: execute 0x1::variant_field_inst_flip::run_u64 --args 1
// CHECK: results: 22

// RUN: execute 0x1::variant_field_inst_flip::run_u128 --args 0
// CHECK: results: 12

// RUN: execute 0x1::variant_field_inst_flip::run_u128 --args 1
// CHECK: results: 22

// RUN: execute 0x1::variant_field_inst_flip::run_mixed --args 0
// CHECK: results: 12022

// RUN: execute 0x1::variant_field_inst_flip::run_mixed --args 1
// CHECK: results: 22012

// RUN: execute 0x1::variant_field_inst_flip::payload_u128
// CHECK: results: 340282366920938463463374607431768211444

// Cross-module field access covers padded structs, enum variants with
// different field widths, and generic structs.

// RUN: publish
module 0x1::shapes {
    // Padded: u64 between two u8s.
    public struct Wide has drop { a: u8, b: u64, c: u8 }

    // The variants use different widths for x; Big also has a u128 field.
    public enum Shape has drop {
        Tiny { x: u8 },
        Big { x: u64, y: u128 },
    }

    public struct G<phantom T> has drop { lead: u8, x: u64 }

    public fun make(): Wide { Wide { a: 1, b: 256, c: 3 } }

    public fun make_shape(sel: u64): Shape {
        if (sel == 0) { Shape::Tiny { x: 7 } } else { Shape::Big { x: 5, y: 2 } }
    }

    public fun make_g<T>(): G<T> { G { lead: 1, x: 42 } }
}

module 0x1::reader {
    // Field reads must use the defining module's struct layout.
    public fun run(): u64 {
        let w = 0x1::shapes::make();
        (w.a as u64) + w.b + (w.c as u64)
    }

    // Cross-module enum match with variant payloads of different widths.
    public fun run_shape(sel: u64): u64 {
        let s = 0x1::shapes::make_shape(sel);
        match (s) {
            0x1::shapes::Shape::Tiny { x } => (x as u64),
            0x1::shapes::Shape::Big { x, y } => x + (y as u64),
        }
    }

    // The caller's field offsets must match those used by make_g<u128>.
    public fun run_g(): u64 {
        let g = 0x1::shapes::make_g<u128>();
        (g.lead as u64) + g.x
    }

    // Phantom type arguments leave the field layout unchanged.
    public fun run_g2(): u64 {
        let a = 0x1::shapes::make_g<u8>();
        let b = 0x1::shapes::make_g<u256>();
        (a.lead as u64) + a.x + (b.lead as u64) + b.x
    }
}

// RUN: execute 0x1::reader::run
// CHECK: results: 260

// RUN: execute 0x1::reader::run_shape --args 0
// CHECK: results: 7

// RUN: execute 0x1::reader::run_shape --args 1
// CHECK: results: 7

// RUN: execute 0x1::reader::run_g
// CHECK: results: 43

// RUN: execute 0x1::reader::run_g2
// CHECK: results: 86

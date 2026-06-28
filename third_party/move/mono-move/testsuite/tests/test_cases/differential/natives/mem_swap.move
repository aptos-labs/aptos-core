// Differential test for `mem::swap`.

// RUN: publish
module 0x1::main {
    use std::mem;

    struct Pair has drop { a: u64, b: u64 }

    // Swap, then combine both values so the result reflects the swap.
    public fun swap_u64(a: u64, b: u64): u64 {
        mem::swap(&mut a, &mut b);
        a * 1000000 + b
    }

    public fun swap_address(a: address, b: address): address {
        mem::swap(&mut a, &mut b);
        a
    }

    // Swap two structs and read both back, confirming the whole value moved.
    public fun swap_struct(): u64 {
        let x = Pair { a: 1, b: 2 };
        let y = Pair { a: 3, b: 4 };
        mem::swap(&mut x, &mut y);
        x.a * 1000 + x.b * 100 + y.a * 10 + y.b
    }

    // Swap two fields of the same struct via field references.
    public fun swap_fields(): u64 {
        let p = Pair { a: 5, b: 8 };
        mem::swap(&mut p.a, &mut p.b);
        p.a * 1000 + p.b
    }

    // Swap two byte vectors (heap-allocated) and read one back.
    public fun swap_vector(): vector<u8> {
        let x = b"ab";
        let y = b"cd";
        mem::swap(&mut x, &mut y);
        x
    }
}

// RUN: execute 0x1::main::swap_u64 --args 7, 9
// CHECK: results: 9000007

// RUN: execute 0x1::main::swap_address --args 0xaa, 0xbb
// CHECK: results: 0xbb

// RUN: execute 0x1::main::swap_struct
// CHECK: results: 3412

// RUN: execute 0x1::main::swap_fields
// CHECK: results: 8005

// RUN: execute 0x1::main::swap_vector
// CHECK: results: 0x6364

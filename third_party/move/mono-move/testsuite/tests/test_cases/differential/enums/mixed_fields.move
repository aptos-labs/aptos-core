// RUN: publish
module 0x42::enums_mixed {
    // Variants with narrow-int fields.
    enum Packet has drop {
        Small { a: u8, b: u16 },
        Large { a: u8, b: u16, c: u32, d: u64 },
    }

    fun small_b(a: u8, b: u16): u16 {
        let p = Packet::Small { a, b };
        match (p) {
            Packet::Small { a: _, b } => b,
            Packet::Large { a: _, b, c: _, d: _ } => b,
        }
    }

    fun large_sum(a: u8, b: u16, c: u32, d: u64): u64 {
        let p = Packet::Large { a, b, c, d };
        match (p) {
            Packet::Small { a, b } => (a as u64) + (b as u64),
            Packet::Large { a, b, c, d } => (a as u64) + (b as u64) + (c as u64) + d,
        }
    }
}

// RUN: execute 0x42::enums_mixed::small_b --args 200, 40000
// CHECK: results: 40000

// RUN: execute 0x42::enums_mixed::large_sum --args 1, 2, 3, 4000000000
// CHECK: results: 4000000006

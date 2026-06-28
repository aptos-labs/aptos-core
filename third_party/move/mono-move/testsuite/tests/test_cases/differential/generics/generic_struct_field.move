// RUN: publish --print(stackless,micro-ops)
module 0x42::generic_struct_field {
    struct Pair<A, B> has copy, drop {
        first: A,
        second: B,
    }

    fun read_via_ref(x: u8, y: u64): u64 {
        let p = Pair<u8, u64> { first: x, second: y };
        let r = &p;
        (r.first as u64) + r.second
    }

    fun read_wide(x: u64, y: u64): u64 {
        let p = Pair<u128, u64> { first: (x as u128), second: y };
        let r = &p;
        (r.first as u64) + r.second
    }

    fun local_field_ops(x: u8, y: u64): u64 {
        let p = Pair<u8, u64> { first: x, second: y };
        p.second = p.second + 1;
        p.first = p.first + 1;
        (p.first as u64) + p.second
    }

    fun write_via_mut_ref(x: u8, y: u64): u64 {
        let p = Pair<u8, u64> { first: x, second: y };
        let r = &mut p;
        r.second = r.second + 10;
        (p.first as u64) + p.second
    }
}

// RUN: execute 0x42::generic_struct_field::read_via_ref --args 3, 1000
// CHECK: results: 1003

// RUN: execute 0x42::generic_struct_field::read_wide --args 3, 1000
// CHECK: results: 1003

// RUN: execute 0x42::generic_struct_field::local_field_ops --args 3, 1000
// CHECK: results: 1005

// RUN: execute 0x42::generic_struct_field::write_via_mut_ref --args 3, 1000
// CHECK: results: 1013

// A variant test (`is`) through a reference, on an enum with a `String` field and on one
// with a `u64` field. The --print step cannot resolve `String`, which is from another
// module: the golden pins why it skips `check` (holds the enum by value) and `is_a` (holds
// it only by reference), and shows the `u64` versions lowered. When run, both VMs agree on
// each variant.

// RUN: publish --print(micro-ops)
module 0x42::enum_skip {
    use std::string::{Self, String};

    enum E has drop { A { text: String }, B }
    enum L has drop { A { num: u64 }, B }

    fun is_a(value: &E): bool { value is E::A }

    fun is_local_a(value: &L): bool { value is L::A }

    public fun check(selector: u64): bool {
        let value = if (selector == 0) E::A { text: string::utf8(b"x") } else E::B;
        is_a(&value)
    }

    public fun check_local(selector: u64): bool {
        let value = if (selector == 0) L::A { num: selector } else L::B;
        is_local_a(&value)
    }
}

// RUN: execute 0x42::enum_skip::check --args 0
// CHECK: results: true

// RUN: execute 0x42::enum_skip::check --args 1
// CHECK: results: false

// RUN: execute 0x42::enum_skip::check_local --args 0
// CHECK: results: true

// RUN: execute 0x42::enum_skip::check_local --args 1
// CHECK: results: false

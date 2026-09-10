// RUN: publish
module 0x1::aggregator_v2 {
    use std::string::String;
    use std::vector;

    struct AggregatorSnapshot<IntElement> has store, drop {
        value: IntElement,
    }

    struct DerivedStringSnapshot has store, drop {
        value: String,
        padding: vector<u8>,
    }

    public native fun create_snapshot<IntElement>(value: IntElement): AggregatorSnapshot<IntElement>;
    public native fun create_derived_string(value: String): DerivedStringSnapshot;
    public native fun read_derived_string(self: &DerivedStringSnapshot): String;
    public native fun derive_string_concat<IntElement>(before: String, snapshot: &AggregatorSnapshot<IntElement>, after: String): DerivedStringSnapshot;

    // `read_derived_string` drops the padding, so expose `padding` through an
    // accessor the harness can observe.
    public fun derived_padding_len(self: &DerivedStringSnapshot): u64 {
        vector::length(&self.padding)
    }
}

module 0x1::main {
    use 0x1::aggregator_v2;
    use std::string::{Self, String};

    public fun create_hello(): String {
        let d = aggregator_v2::create_derived_string(string::utf8(b"hello"));
        aggregator_v2::read_derived_string(&d)
    }

    public fun create_hello_padding(): u64 {
        let d = aggregator_v2::create_derived_string(string::utf8(b"hello"));
        aggregator_v2::derived_padding_len(&d)
    }

    public fun create_empty(): String {
        let d = aggregator_v2::create_derived_string(string::utf8(b""));
        aggregator_v2::read_derived_string(&d)
    }

    public fun concat_u64(v: u64): String {
        let snap = aggregator_v2::create_snapshot<u64>(v);
        let d = aggregator_v2::derive_string_concat(string::utf8(b"before"), &snap, string::utf8(b"after"));
        aggregator_v2::read_derived_string(&d)
    }

    public fun concat_u64_padding(v: u64): u64 {
        let snap = aggregator_v2::create_snapshot<u64>(v);
        let d = aggregator_v2::derive_string_concat(string::utf8(b"before"), &snap, string::utf8(b"after"));
        aggregator_v2::derived_padding_len(&d)
    }

    public fun concat_u128(v: u128): String {
        let snap = aggregator_v2::create_snapshot<u128>(v);
        let d = aggregator_v2::derive_string_concat(string::utf8(b"x"), &snap, string::utf8(b"y"));
        aggregator_v2::read_derived_string(&d)
    }
}

// RUN: execute 0x1::main::create_hello
// CHECK-V2: results: "hello"

// "hello" is 5 bytes: width = max(bcs_size(5) + 1, 22) = 22, so
// padding = 22 - bcs_size(5) - 1 = 22 - 6 - 1 = 15.
// RUN: execute 0x1::main::create_hello_padding
// CHECK-V2: results: 15

// RUN: execute 0x1::main::create_empty
// CHECK-V2: results: ""

// RUN: execute 0x1::main::concat_u64 --args 42
// CHECK-V2: results: "before42after"

// width = bcs_size(6 + 5 + 20) + 1 = bcs_size(31) + 1 = 33; output "before42after"
// is 13 bytes, so padding = 33 - bcs_size(13) - 1 = 33 - 14 - 1 = 18.
// RUN: execute 0x1::main::concat_u64_padding --args 42
// CHECK-V2: results: 18

// RUN: execute 0x1::main::concat_u128 --args 99
// CHECK-V2: results: "x99y"

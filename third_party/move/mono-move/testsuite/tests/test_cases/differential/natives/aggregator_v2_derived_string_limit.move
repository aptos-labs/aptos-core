// The 1024-byte input limit of the derived-string natives: 1024 bytes is accepted and 1025
// aborts, for `create_derived_string` and for the prefix plus suffix of `derive_string_concat`
// (the snapshot's digit does not count, so the accepted string is 1025 bytes long). Checked
// on MonoMove only: the V1 side of the harness has no aggregator support.

// RUN: publish
module 0x1::aggregator_v2 {
    use std::string::{Self, String};

    struct AggregatorSnapshot<IntElement> has store, drop {
        value: IntElement,
    }

    struct DerivedStringSnapshot has store, drop {
        value: String,
        padding: vector<u8>,
    }

    public native fun create_snapshot<IntElement>(value: IntElement): AggregatorSnapshot<IntElement>;
    public native fun create_derived_string(value: String): DerivedStringSnapshot;
    public native fun derive_string_concat<IntElement>(before: String, snapshot: &AggregatorSnapshot<IntElement>, after: String): DerivedStringSnapshot;

    public fun shape(self: &DerivedStringSnapshot): (u64, u64) {
        (string::bytes(&self.value).length(), self.padding.length())
    }
}
module 0x42::main {
    use std::string::{String, utf8};
    use 0x1::aggregator_v2;

    fun text(len: u64): String {
        let bytes = vector[];
        let index = 0;
        while (index < len) {
            bytes.push_back(97);
            index += 1;
        };
        utf8(bytes)
    }

    public fun create(len: u64): (u64, u64) {
        aggregator_v2::shape(&aggregator_v2::create_derived_string(text(len)))
    }

    public fun concat(before: u64, after: u64): (u64, u64) {
        let snap = aggregator_v2::create_snapshot<u128>(7);
        aggregator_v2::shape(&aggregator_v2::derive_string_concat(text(before), &snap, text(after)))
    }
}

// RUN: execute 0x42::main::create --args 1024
// CHECK-V2: results: 1024, 0

// RUN: execute 0x42::main::create --args 1025
// CHECK-V2: aborted: code 196616 (Derived string snapshot input length (1025) exceeds maximum 1024) in 0x1::aggregator_v2

// RUN: execute 0x42::main::concat --args 1000, 24
// CHECK-V2: results: 1025, 38

// RUN: execute 0x42::main::concat --args 1000, 25
// CHECK-V2: aborted: code 196616 (Derived string snapshot input length (1000 + 25) exceeds maximum 1024) in 0x1::aggregator_v2

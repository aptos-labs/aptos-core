// `create_derived_string` and `derive_string_concat` with a collection forced inside the
// native, after it allocates the string value and before the padding: the `_fill` steps show
// the value's allocation fits, so the padding's is the one that collects. With `more` = 6800
// two more collections run before the value and padding are read back. Checked on MonoMove
// only: the V1 side of the harness has no aggregator support.

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

    public fun contents(self: &DerivedStringSnapshot): (vector<u8>, vector<u8>) {
        (*string::bytes(&self.value), self.padding)
    }

    // Reads without allocating.
    public fun shape(self: &DerivedStringSnapshot): (u64, u8, u64) {
        let bytes = string::bytes(&self.value);
        (bytes.length(), bytes[0], self.padding.length())
    }
}
module 0x42::main {
    use std::string;
    use 0x1::aggregator_v2;

    fun churn(rounds: u64) {
        let counter = 0;
        while (counter < rounds) {
            let junk = vector[counter, counter, counter, counter];
            counter = counter + junk.length();
        };
    }

    // A step finer than one churn round, so the heap can end up with room for the value but
    // not the padding.
    fun pad(): u64 {
        let junk = vector[1u64];
        junk.length()
    }

    public fun derived(rounds: u64): (u64, u8, u64) {
        churn(rounds);
        let _ = pad();
        let snapshot = aggregator_v2::create_derived_string(string::utf8(b"hello"));
        aggregator_v2::shape(&snapshot)
    }

    public fun derived_full(rounds: u64, more: u64): (vector<u8>, vector<u8>) {
        churn(rounds);
        let _ = pad();
        let snapshot = aggregator_v2::create_derived_string(string::utf8(b"hello"));
        churn(more);
        aggregator_v2::contents(&snapshot)
    }

    public fun derived_fill(rounds: u64): u64 {
        churn(rounds);
        let _ = pad();
        let value = string::utf8(b"hello");
        let first = b"abc";
        string::length(&value) + first.length()
    }

    // The value is "x" ++ "12345" ++ "y".
    public fun concat(rounds: u64): (u64, u8, u64) {
        let snap = aggregator_v2::create_snapshot<u64>(12345);
        churn(rounds);
        let snapshot = aggregator_v2::derive_string_concat(string::utf8(b"x"), &snap, string::utf8(b"y"));
        aggregator_v2::shape(&snapshot)
    }

    public fun concat_full(rounds: u64, more: u64): (vector<u8>, vector<u8>) {
        let snap = aggregator_v2::create_snapshot<u64>(12345);
        churn(rounds);
        let snapshot = aggregator_v2::derive_string_concat(string::utf8(b"x"), &snap, string::utf8(b"y"));
        churn(more);
        aggregator_v2::contents(&snapshot)
    }

    public fun concat_fill(rounds: u64): u64 {
        let snap = aggregator_v2::create_snapshot<u64>(12345);
        churn(rounds);
        let before = string::utf8(b"x");
        let after = string::utf8(b"y");
        let first = b"abc";
        let _ = snap;
        string::length(&before) + string::length(&after) + first.length()
    }
}

// RUN: execute 0x42::main::derived_fill --args 2724 --heap-size 32768
// CHECK-V2: results: 8
// CHECK-GC-COUNT: 0

// RUN: execute 0x42::main::derived --args 2724 --heap-size 32768
// CHECK-V2: results: 5, 104, 15
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::main::derived_full --args 2724, 0 --heap-size 32768
// CHECK-V2: results: 0x68656c6c6f, 0x000000000000000000000000000000
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::main::derived_full --args 2724, 6800 --heap-size 32768
// CHECK-V2: results: 0x68656c6c6f, 0x000000000000000000000000000000
// CHECK-GC-COUNT: 3

// RUN: execute 0x42::main::concat_fill --args 2724 --heap-size 32768
// CHECK-V2: results: 5
// CHECK-GC-COUNT: 0

// RUN: execute 0x42::main::concat --args 2724 --heap-size 32768
// CHECK-V2: results: 7, 120, 15
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::main::concat_full --args 2724, 0 --heap-size 32768
// CHECK-V2: results: 0x78313233343579, 0x000000000000000000000000000000
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::main::concat_full --args 2724, 6800 --heap-size 32768
// CHECK-V2: results: 0x78313233343579, 0x000000000000000000000000000000
// CHECK-GC-COUNT: 3

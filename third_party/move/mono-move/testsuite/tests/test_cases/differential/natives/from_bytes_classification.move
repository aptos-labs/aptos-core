// `from_bcs::from_bytes` on complete and cut encodings of a vector<u64>, a nested vector, a
// struct (u8, vector<u64>, u128), an enum (also with an invalid tag) and a String through
// `to_string`. Malformed input aborts with code 65537, a String that is not UTF-8 with code 1.
// Each case also runs with a collection forced inside the decode, as the `stop` and `_fill`
// steps show; an invalid tag is rejected without collecting. With `more` = 6800 two more
// collections run before the value is read back.

// RUN: publish
module 0x1::from_bcs {
    use std::string::{Self, String};

    const EINVALID_UTF8: u64 = 0x1;

    public native fun from_bytes<T>(bytes: vector<u8>): T;

    // Same body as the framework's `from_bcs::to_string`.
    public fun to_string(bytes: vector<u8>): String {
        let text = from_bytes<String>(bytes);
        assert!(string::internal_check_utf8(text.bytes()), EINVALID_UTF8);
        text
    }
}
module 0x1::main {
    use std::string::String;

    struct P has drop { a: u8, v: vector<u64>, w: u128 }

    enum E has drop { A { x: u64 }, B { v: vector<u8>, y: u64 } }

    // [1, 2, 3].
    const U64S: vector<u8> = x"03010000000000000002000000000000000300000000000000";
    const U64S_TRUNC: vector<u8> = x"030100000000000000020000000000000003000000000000";
    // [[0x61], [0x62, 0x63], [0x64, 0x65, 0x66]]; the truncated copy cuts the third child.
    const NESTED: vector<u8> = x"03016102626303646566";
    const NESTED_TRUNC: vector<u8> = x"030161026263036465";
    // P { a: 7, v: [9, 10], w: 2^100 + 5 }.
    const PADDED: vector<u8> = x"070209000000000000000a0000000000000005000000000000000000000010000000";
    const PADDED_TRUNC: vector<u8> = x"070209000000000000000a00000000000000050000000000000000000000100000";
    // E::B { v: b"ab", y: 7 }.
    const ENUM_B: vector<u8> = x"010261620700000000000000";
    const ENUM_B_TRUNC: vector<u8> = x"0102616207000000000000";
    const ENUM_BAD_TAG: vector<u8> = x"020261620700000000000000";

    fun live_set(count: u64): vector<vector<u64>> {
        let values = vector[];
        let index = 0;
        while (index < count) {
            values.push_back(vector[index, index, index, index]);
            index += 1;
        };
        values
    }

    fun churn(rounds: u64) {
        let counter = 0;
        while (counter < rounds) {
            let junk = vector[counter, counter, counter, counter];
            counter = counter + junk.length();
        };
    }

    public fun u64s(live: u64, rounds: u64, trunc: bool, stop: bool): (vector<u64>, u64) {
        let bytes = if (trunc) U64S_TRUNC else U64S;
        let keep = live_set(live);
        churn(rounds);
        if (stop) abort 99;
        let values = 0x1::from_bcs::from_bytes<vector<u64>>(bytes);
        (values, keep.length())
    }

    public fun nested(live: u64, rounds: u64, trunc: bool, stop: bool): (u64, u8, u8, u64) {
        let bytes = if (trunc) NESTED_TRUNC else NESTED;
        let keep = live_set(live);
        churn(rounds);
        if (stop) abort 99;
        let decoded = 0x1::from_bcs::from_bytes<vector<vector<u8>>>(bytes);
        (decoded.length(), decoded[1][1], decoded[2][2], keep.length())
    }

    public fun padded(live: u64, rounds: u64, trunc: bool, stop: bool): (u8, vector<u64>, u128, u64) {
        let bytes = if (trunc) PADDED_TRUNC else PADDED;
        let keep = live_set(live);
        churn(rounds);
        if (stop) abort 99;
        let P { a: byte, v: values, w: large } = 0x1::from_bcs::from_bytes<P>(bytes);
        (byte, values, large, keep.length())
    }

    // 0 = complete, 1 = truncated field, 2 = invalid tag.
    public fun enum_b(live: u64, rounds: u64, which: u8, stop: bool): (vector<u8>, u64, u64) {
        let bytes = if (which == 0) ENUM_B else if (which == 1) ENUM_B_TRUNC else ENUM_BAD_TAG;
        let keep = live_set(live);
        churn(rounds);
        if (stop) abort 99;
        match (0x1::from_bcs::from_bytes<E>(bytes)) {
            E::A { x: number } => (vector[], number, keep.length()),
            E::B { v: payload, y: number } => (payload, number, keep.length()),
        }
    }

    // Moves each child out, so every byte is read without copying.
    public fun nested_churned(rounds: u64, more: u64): (u64, vector<u8>, vector<u8>, vector<u8>) {
        let bytes = NESTED;
        churn(rounds);
        let decoded = 0x1::from_bcs::from_bytes<vector<vector<u8>>>(bytes);
        churn(more);
        let count = decoded.length();
        let third = decoded.pop_back();
        let second = decoded.pop_back();
        let first = decoded.pop_back();
        (count, first, second, third)
    }

    public fun nested_fill(rounds: u64): (u64, u64) {
        let bytes = NESTED;
        churn(rounds);
        let first = vector[1u64, 2, 3];
        (bytes.length(), first.length())
    }

    public fun enum_b_churned(rounds: u64, more: u64): (vector<u8>, u64) {
        let bytes = ENUM_B;
        churn(rounds);
        let decoded = 0x1::from_bcs::from_bytes<E>(bytes);
        churn(more);
        match (decoded) {
            E::A { x: number } => (vector[], number),
            E::B { v: payload, y: number } => (payload, number),
        }
    }

    public fun enum_b_fill(rounds: u64): (u64, u64) {
        let bytes = ENUM_B;
        churn(rounds);
        let first = vector[1u64, 2];
        (bytes.length(), first.length())
    }

    // 0 = "abc", 1 = truncated, 2 = complete but not UTF-8.
    public fun string(live: u64, rounds: u64, which: u8, stop: bool): (String, u64) {
        let bytes = if (which == 0) x"03616263" else if (which == 1) x"036162" else x"02ff61";
        let keep = live_set(live);
        churn(rounds);
        if (stop) abort 99;
        (0x1::from_bcs::to_string(bytes), keep.length())
    }
}

// RUN: execute 0x1::main::u64s --args 0, 0, false, false
// CHECK: results: [1, 2, 3], 0

// RUN: execute 0x1::main::u64s --args 0, 0, true, false
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::nested --args 0, 0, false, false
// CHECK: results: 3, 99, 102, 0

// RUN: execute 0x1::main::nested --args 0, 0, true, false
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::padded --args 0, 0, false, false
// CHECK: results: 7, [9, 10], 1267650600228229401496703205381, 0

// RUN: execute 0x1::main::padded --args 0, 0, true, false
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::enum_b --args 0, 0, 0, false
// CHECK: results: 0x6162, 7, 0

// RUN: execute 0x1::main::enum_b --args 0, 0, 1, false
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::enum_b --args 0, 0, 2, false
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::string --args 0, 0, 0, false
// CHECK: results: "abc", 0

// RUN: execute 0x1::main::string --args 0, 0, 1, false
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::string --args 0, 0, 2, false
// CHECK: aborted: code 1 in 0x1::from_bcs

// RUN: execute 0x1::main::u64s --args 0, 80, true, true --heap-size 1024
// CHECK: aborted: code 99 in 0x1::main
// CHECK-GC-COUNT: 0

// RUN: execute 0x1::main::u64s --args 0, 80, true, false --heap-size 1024
// CHECK: aborted: code 65537 in 0x1::from_bcs
// CHECK-GC-COUNT: 1

// RUN: execute 0x1::main::u64s --args 0, 80, false, false --heap-size 1024
// CHECK: results: [1, 2, 3], 0
// CHECK-GC-COUNT: 1

// RUN: execute 0x1::main::nested --args 0, 80, true, true --heap-size 1024
// CHECK: aborted: code 99 in 0x1::main
// CHECK-GC-COUNT: 0

// RUN: execute 0x1::main::nested --args 0, 80, true, false --heap-size 1024
// CHECK: aborted: code 65537 in 0x1::from_bcs
// CHECK-GC-COUNT: 1

// RUN: execute 0x1::main::nested --args 0, 80, false, false --heap-size 1024
// CHECK: results: 3, 99, 102, 0
// CHECK-GC-COUNT: 1

// RUN: execute 0x1::main::padded --args 0, 80, true, true --heap-size 1024
// CHECK: aborted: code 99 in 0x1::main
// CHECK-GC-COUNT: 0

// RUN: execute 0x1::main::padded --args 0, 80, true, false --heap-size 1024
// CHECK: aborted: code 65537 in 0x1::from_bcs
// CHECK-GC-COUNT: 1

// RUN: execute 0x1::main::padded --args 0, 80, false, false --heap-size 1024
// CHECK: results: 7, [9, 10], 1267650600228229401496703205381, 0
// CHECK-GC-COUNT: 1

// RUN: execute 0x1::main::enum_b --args 0, 80, 1, true --heap-size 1024
// CHECK: aborted: code 99 in 0x1::main
// CHECK-GC-COUNT: 0

// RUN: execute 0x1::main::enum_b --args 0, 80, 1, false --heap-size 1024
// CHECK: aborted: code 65537 in 0x1::from_bcs
// CHECK-GC-COUNT: 1

// RUN: execute 0x1::main::enum_b --args 0, 80, 0, false --heap-size 1024
// CHECK: results: 0x6162, 7, 0
// CHECK-GC-COUNT: 1

// RUN: execute 0x1::main::enum_b --args 0, 80, 2, false --heap-size 1024
// CHECK: aborted: code 65537 in 0x1::from_bcs
// CHECK-GC-COUNT: 0

// RUN: execute 0x1::main::string --args 0, 84, 1, true --heap-size 1032
// CHECK: aborted: code 99 in 0x1::main
// CHECK-GC-COUNT: 0

// RUN: execute 0x1::main::string --args 0, 84, 1, false --heap-size 1032
// CHECK: aborted: code 65537 in 0x1::from_bcs
// CHECK-GC-COUNT: 1

// RUN: execute 0x1::main::string --args 0, 84, 2, false --heap-size 1032
// CHECK: aborted: code 1 in 0x1::from_bcs
// CHECK-GC-COUNT: 1

// RUN: execute 0x1::main::string --args 0, 84, 0, false --heap-size 1032
// CHECK: results: "abc", 0
// CHECK-GC-COUNT: 1

// RUN: execute 0x1::main::nested_fill --args 2724 --heap-size 32768
// CHECK: results: 10, 3
// CHECK-GC-COUNT: 0

// RUN: execute 0x1::main::nested_churned --args 2724, 0 --heap-size 32768
// CHECK: results: 3, 0x61, 0x6263, 0x646566
// CHECK-GC-COUNT: 1

// RUN: execute 0x1::main::nested_churned --args 2724, 6800 --heap-size 32768
// CHECK: results: 3, 0x61, 0x6263, 0x646566
// CHECK-GC-COUNT: 3

// RUN: execute 0x1::main::enum_b_fill --args 2724 --heap-size 32768
// CHECK: results: 12, 2
// CHECK-GC-COUNT: 0

// RUN: execute 0x1::main::enum_b_churned --args 2724, 0 --heap-size 32768
// CHECK: results: 0x6162, 7
// CHECK-GC-COUNT: 1

// RUN: execute 0x1::main::enum_b_churned --args 2724, 6800 --heap-size 32768
// CHECK: results: 0x6162, 7
// CHECK-GC-COUNT: 3

// RUN: execute 0x1::main::enum_b --args 32, 0, 2, true --heap-size 1852
// CHECK: aborted: code 99 in 0x1::main
// CHECK-GC-COUNT: 1

// RUN: execute 0x1::main::enum_b --args 32, 0, 2, false --heap-size 1852
// CHECK: aborted: code 65537 in 0x1::from_bcs
// CHECK-GC-COUNT: 1

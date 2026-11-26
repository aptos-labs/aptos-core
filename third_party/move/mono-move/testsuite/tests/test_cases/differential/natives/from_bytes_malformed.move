// `from_bcs::from_bytes` on malformed encodings, each aborting with code 65537: a bool byte of 2,
// bad enum tags, non-minimal and overflowing ULEB128 lengths (two read by an overflow-blind
// decoder as a length the payload satisfies, each after its canonical control), lengths at and
// past the BCS sequence limit (2^31 - 1, 2^31, 2^32), and lengths longer than the input, one past
// the single-allocation cap. A non-UTF-8 `String` decodes unchecked. `nested_after_churn` collects
// in the decode after its first allocation (see `nested_fill`); `more` = 6800 adds two more.

// RUN: publish
module 0x1::from_bcs {
    public native fun from_bytes<T>(bytes: vector<u8>): T;
}
module 0x1::main {
    use std::string::{Self, String};

    enum Two has drop { X, Y { v: u8 } }

    struct Wrap has drop { items: vector<vector<u8>> }

    public fun bool_two(): bool {
        0x1::from_bcs::from_bytes<bool>(x"02")
    }

    public fun enum_tag_past_end() {
        0x1::from_bcs::from_bytes<Two>(x"05");
    }

    // Tag 1 encoded in two bytes.
    public fun enum_tag_non_minimal() {
        0x1::from_bcs::from_bytes<Two>(x"810007");
    }

    // Length 0 encoded in two bytes.
    public fun uleb_non_minimal(): u64 {
        0x1::from_bcs::from_bytes<vector<u8>>(x"8000").length()
    }

    // A ten-byte ULEB128 value: overflows a u64.
    public fun uleb_overflow(): u64 {
        0x1::from_bcs::from_bytes<vector<u8>>(x"ffffffffffffffffff7f").length()
    }

    public fun canonical_length(): vector<u8> {
        0x1::from_bcs::from_bytes<vector<u8>>(x"0161")
    }

    // The same payload after a ten-byte length: digit 1, eight zero digits, then 2 at bit 63.
    // Dropping the overflowing bit would leave length 1.
    public fun overflowing_length(): vector<u8> {
        0x1::from_bcs::from_bytes<vector<u8>>(x"8180808080808080800261")
    }

    // `prefix` followed by 64 bytes 0x61.
    fun with_payload(prefix: vector<u8>): vector<u8> {
        let bytes = prefix;
        let count = 0;
        while (count < 64) {
            bytes.push_back(0x61);
            count += 1;
        };
        bytes
    }

    public fun canonical_length_64(): u64 {
        0x1::from_bcs::from_bytes<vector<u8>>(with_payload(x"40")).length()
    }

    // Ten zero digits, then 1 at bit 70: a shift taken mod 64 would give length 64.
    public fun eleventh_byte_length(): u64 {
        0x1::from_bcs::from_bytes<vector<u8>>(with_payload(x"8080808080808080808001")).length()
    }

    // Length 2^31 - 1: the BCS sequence limit itself.
    public fun uleb_at_sequence_limit(): u64 {
        0x1::from_bcs::from_bytes<vector<u8>>(x"ffffffff0700").length()
    }

    // Length 2^31: one past the BCS sequence limit.
    public fun uleb_past_sequence_limit(): u64 {
        0x1::from_bcs::from_bytes<vector<u8>>(x"8080808008").length()
    }

    // Length 2^32: does not fit the u32 a BCS length decodes into.
    public fun uleb_past_u32(): u64 {
        0x1::from_bcs::from_bytes<vector<u8>>(x"8080808010").length()
    }

    // Length 3 MiB with one byte of input: the allocation fits the heap.
    public fun len_3mib_u8(): u64 {
        0x1::from_bcs::from_bytes<vector<u8>>(x"8080c00100").length()
    }

    // Length 500000 u64s (4000000 bytes) with one byte of input.
    public fun len_500k_u64(): u64 {
        0x1::from_bcs::from_bytes<vector<u64>>(x"a0c21e00").length()
    }

    // Length 500000 vectors: the first element decodes as empty, the second
    // hits the end of input.
    public fun len_500k_nested(): u64 {
        0x1::from_bcs::from_bytes<vector<vector<u8>>>(x"a0c21e00").length()
    }

    // Length 5000000: past the single-allocation cap.
    public fun len_5m_u8(): u64 {
        0x1::from_bcs::from_bytes<vector<u8>>(x"c096b10200").length()
    }

    // Neither VM checks UTF-8 when decoding a `String` here.
    public fun string_not_utf8(): vector<u8> {
        let text: String = 0x1::from_bcs::from_bytes<String>(x"02ff61");
        *string::bytes(&text)
    }

    fun churn(rounds: u64) {
        let counter = 0;
        while (counter < rounds) {
            let junk = vector[counter, counter, counter, counter];
            counter = counter + junk.length();
        };
    }

    // Each item is moved out, so every byte is read without copying.
    public fun nested_after_churn(
        rounds: u64,
        more: u64,
    ): (u64, vector<u8>, vector<u8>, vector<u8>, vector<u8>, vector<u8>) {
        churn(rounds);
        let decoded = 0x1::from_bcs::from_bytes<Wrap>(
            x"0503616263000201020461626364016a"
        );
        churn(more);
        let Wrap { items } = decoded;
        let count = items.length();
        let fifth = items.pop_back();
        let fourth = items.pop_back();
        let third = items.pop_back();
        let second = items.pop_back();
        let first = items.pop_back();
        (count, first, second, third, fourth, fifth)
    }

    public fun nested_fill(rounds: u64): (u64, u64) {
        churn(rounds);
        let bytes = x"0503616263000201020461626364016a";
        let first = vector[1u64, 2, 3, 4, 5];
        (bytes.length(), first.length())
    }
}

// RUN: execute 0x1::main::bool_two
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::enum_tag_past_end
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::enum_tag_non_minimal
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::uleb_non_minimal
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::uleb_overflow
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::canonical_length
// CHECK: results: 0x61

// RUN: execute 0x1::main::overflowing_length
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::canonical_length_64
// CHECK: results: 64

// RUN: execute 0x1::main::eleventh_byte_length
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::uleb_at_sequence_limit
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::uleb_past_sequence_limit
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::uleb_past_u32
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::len_3mib_u8
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::len_500k_u64
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::len_500k_nested
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::len_5m_u8
// CHECK: aborted: code 65537 in 0x1::from_bcs

// RUN: execute 0x1::main::string_not_utf8
// CHECK: results: 0xff61

// RUN: execute 0x1::main::nested_fill --args 2720 --heap-size 32768
// CHECK: results: 16, 5
// CHECK-GC-COUNT: 0

// RUN: execute 0x1::main::nested_after_churn --args 2720, 0 --heap-size 32768
// CHECK: results: 5, 0x616263, 0x, 0x0102, 0x61626364, 0x6a
// CHECK-GC-COUNT: 1

// RUN: execute 0x1::main::nested_after_churn --args 2720, 6800 --heap-size 32768
// CHECK: results: 5, 0x616263, 0x, 0x0102, 0x61626364, 0x6a
// CHECK-GC-COUNT: 3

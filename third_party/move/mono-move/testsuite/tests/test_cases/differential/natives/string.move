// Differential tests for the `0x1::string` natives: internal_check_utf8,
// internal_index_of, internal_sub_string, internal_is_char_boundary. The Move
// stdlib is pre-published into both VMs, so the private natives are exercised
// through their public wrappers (utf8, index_of, sub_string). is_char_boundary
// has no standalone wrapper; it is exercised by sub_string, whose boundary
// asserts call it for both endpoints.
//
// "héllo" is the six-byte sequence [0x68, 0xc3, 0xa9, 0x6c, 0x6c, 0x6f]: 'h',
// then 'é' (two bytes 0xc3 0xa9), then "llo". Byte indices 0, 1, 3, 4, 5, 6 are
// char boundaries; 2 (the 'é' continuation byte) is not.

// RUN: publish
module 0x1::main {
    use std::string::{Self, String};

    // --- internal_check_utf8 (via utf8) ---

    public fun ascii(): String {
        string::utf8(b"hello")
    }

    public fun empty(): String {
        string::utf8(b"")
    }

    // "é" encoded as the two-byte sequence 0xC3 0xA9.
    public fun multibyte(): String {
        string::utf8(vector[0xc3u8, 0xa9u8])
    }

    // 0xFF never appears in valid UTF-8.
    public fun invalid_byte(): String {
        string::utf8(vector[0xffu8])
    }

    // Lone continuation byte without a leading byte.
    public fun lone_continuation(): String {
        string::utf8(vector[0x80u8])
    }

    // Truncated two-byte sequence: leading byte with no continuation.
    public fun truncated(): String {
        string::utf8(vector[0xc3u8])
    }

    // --- internal_index_of (via index_of) ---

    public fun index_found(): u64 {
        let s = string::utf8(b"hello world");
        let r = string::utf8(b"world");
        string::index_of(&s, &r)
    }

    // Returns the index of the first occurrence.
    public fun index_first_of_many(): u64 {
        let s = string::utf8(b"hello hello");
        let r = string::utf8(b"hello");
        string::index_of(&s, &r)
    }

    // No occurrence: returns the length of the searched string.
    public fun index_not_found(): u64 {
        let s = string::utf8(b"hello");
        let r = string::utf8(b"xyz");
        string::index_of(&s, &r)
    }

    // The empty pattern is found at index 0.
    public fun index_empty_needle(): u64 {
        let s = string::utf8(b"hello");
        let r = string::utf8(b"");
        string::index_of(&s, &r)
    }

    // "llo" begins at byte index 3 of "héllo".
    public fun index_multibyte(): u64 {
        let s = string::utf8(vector[0x68u8, 0xc3u8, 0xa9u8, 0x6cu8, 0x6cu8, 0x6fu8]);
        let r = string::utf8(b"llo");
        string::index_of(&s, &r)
    }

    // --- internal_sub_string (via sub_string) ---

    // `i`/`j` are byte indices into "hello world" (all ASCII, every index is a
    // char boundary). Aborts with EINVALID_INDEX (2) if `j > 11` or `i > j`.
    public fun sub(i: u64, j: u64): String {
        let s = string::utf8(b"hello world");
        string::sub_string(&s, i, j)
    }

    // `i`/`j` are byte indices into "héllo". `sub_string` asserts both are char
    // boundaries (calling internal_is_char_boundary), aborting with
    // EINVALID_INDEX (2) otherwise (e.g. index 2 splits the 'é').
    public fun sub_multibyte(i: u64, j: u64): String {
        let s = string::utf8(vector[0x68u8, 0xc3u8, 0xa9u8, 0x6cu8, 0x6cu8, 0x6fu8]);
        string::sub_string(&s, i, j)
    }
}

// RUN: execute 0x1::main::ascii
// CHECK: results: "hello"

// RUN: execute 0x1::main::empty
// CHECK: results: ""

// RUN: execute 0x1::main::multibyte
// CHECK: results: "é"

// RUN: execute 0x1::main::invalid_byte
// CHECK: aborted: code 1

// RUN: execute 0x1::main::lone_continuation
// CHECK: aborted: code 1

// RUN: execute 0x1::main::truncated
// CHECK: aborted: code 1

// RUN: execute 0x1::main::index_found
// CHECK: results: 6

// RUN: execute 0x1::main::index_first_of_many
// CHECK: results: 0

// RUN: execute 0x1::main::index_not_found
// CHECK: results: 5

// RUN: execute 0x1::main::index_empty_needle
// CHECK: results: 0

// RUN: execute 0x1::main::index_multibyte
// CHECK: results: 3

// RUN: execute 0x1::main::sub --args 0, 5
// CHECK: results: "hello"

// RUN: execute 0x1::main::sub --args 6, 11
// CHECK: results: "world"

// Empty range yields the empty string.
// RUN: execute 0x1::main::sub --args 2, 2
// CHECK: results: ""

// Full string.
// RUN: execute 0x1::main::sub --args 0, 11
// CHECK: results: "hello world"

// End index past the length aborts with EINVALID_INDEX (2).
// RUN: execute 0x1::main::sub --args 0, 12
// CHECK: aborted: code 2

// Start index after the end aborts with EINVALID_INDEX (2).
// RUN: execute 0x1::main::sub --args 5, 3
// CHECK: aborted: code 2

// Both indices are char boundaries: bytes [0, 3) of "héllo" is "hé".
// RUN: execute 0x1::main::sub_multibyte --args 0, 3
// CHECK: results: "hé"

// Bytes [1, 3) is "é".
// RUN: execute 0x1::main::sub_multibyte --args 1, 3
// CHECK: results: "é"

// End index 2 splits the 'é' code point: not a char boundary, aborts.
// RUN: execute 0x1::main::sub_multibyte --args 0, 2
// CHECK: aborted: code 2

// Start index 2 splits the 'é' code point: not a char boundary, aborts.
// RUN: execute 0x1::main::sub_multibyte --args 2, 3
// CHECK: aborted: code 2

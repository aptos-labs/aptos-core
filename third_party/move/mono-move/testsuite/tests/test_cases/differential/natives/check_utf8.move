// RUN: publish
module 0x1::main {
    use std::string::{Self, String};

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

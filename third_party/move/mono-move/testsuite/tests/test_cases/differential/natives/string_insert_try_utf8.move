// `string::try_utf8` and `string::insert`. try_utf8 returns `none` for bytes
// that differ from a valid input only in the last byte. insert splices at a
// char boundary of "héllo" ("é" is the two bytes 0xC3 0xA9); an index inside
// "é" or past the end aborts with EINVALID_INDEX (2).

// RUN: publish
module 0x42::strings {
    use std::string::{Self, String};

    // Returns whether the bytes decoded, and the decoded length.
    fun try_describe(bytes: vector<u8>): (bool, u64) {
        let decoded = string::try_utf8(bytes);
        if (decoded.is_some()) {
            (true, decoded.destroy_some().length())
        } else {
            (false, 0)
        }
    }

    public fun try_valid(): (bool, u64) {
        try_describe(vector[0x68u8, 0xc3u8, 0xa9u8])
    }

    // 0xC3 followed by a byte that is not a continuation byte.
    public fun try_invalid(): (bool, u64) {
        try_describe(vector[0x68u8, 0xc3u8, 0x28u8])
    }

    public fun try_empty(): (bool, u64) {
        try_describe(vector[])
    }

    public fun insert_at(at: u64): String {
        let text = string::utf8(vector[0x68u8, 0xc3u8, 0xa9u8, 0x6cu8, 0x6cu8, 0x6fu8]);
        text.insert(at, string::utf8(b"-+"));
        text
    }

    public fun insert_empty_at(at: u64): String {
        let text = string::utf8(vector[0x68u8, 0xc3u8, 0xa9u8, 0x6cu8, 0x6cu8, 0x6fu8]);
        text.insert(at, string::utf8(b""));
        text
    }

    public fun insert_into_empty(): String {
        let text = string::utf8(b"");
        text.insert(0, string::utf8(vector[0xc3u8, 0xa9u8]));
        text
    }
}

// RUN: execute 0x42::strings::try_valid
// CHECK: results: true, 3

// RUN: execute 0x42::strings::try_invalid
// CHECK: results: false, 0

// RUN: execute 0x42::strings::try_empty
// CHECK: results: true, 0

// RUN: execute 0x42::strings::insert_at --args 0
// CHECK: results: "-+héllo"

// RUN: execute 0x42::strings::insert_at --args 1
// CHECK: results: "h-+éllo"

// RUN: execute 0x42::strings::insert_at --args 2
// CHECK: aborted: code 2 in 0x1::string

// RUN: execute 0x42::strings::insert_at --args 3
// CHECK: results: "hé-+llo"

// RUN: execute 0x42::strings::insert_at --args 6
// CHECK: results: "héllo-+"

// RUN: execute 0x42::strings::insert_at --args 7
// CHECK: aborted: code 2 in 0x1::string

// RUN: execute 0x42::strings::insert_empty_at --args 3
// CHECK: results: "héllo"

// RUN: execute 0x42::strings::insert_into_empty
// CHECK: results: "é"

// `string_utils::native_format_list` with a format string that is not UTF-8 (a bad last or
// first byte) aborts with EINVALID_FORMAT (2) and no message. The control differs from the
// first failing format only in its last byte.

// RUN: publish
module 0x1::string_utils {
    use std::string::String;

    struct Cons<T, N> has copy, drop, store {
        car: T,
        cdr: N,
    }

    struct NIL has copy, drop, store {}

    public native fun native_format_list<T>(fmt: &vector<u8>, val: &T): String;

    public fun format_one(fmt: vector<u8>, value: u64): String {
        native_format_list(&fmt, &Cons { car: value, cdr: NIL {} })
    }
}
module 0x42::main {
    use std::string::String;

    public fun ascii_tail(): String {
        0x1::string_utils::format_one(x"7b7d21", 5)
    }

    // 0xFF never appears in UTF-8.
    public fun invalid_tail(): String {
        0x1::string_utils::format_one(x"7b7dff", 5)
    }

    // A lone continuation byte ahead of an otherwise valid format.
    public fun invalid_head(): String {
        0x1::string_utils::format_one(x"807b7d", 5)
    }
}

// RUN: execute 0x42::main::ascii_tail
// CHECK: results: "5!"

// RUN: execute 0x42::main::invalid_tail
// CHECK: aborted: code 2 in 0x1::string_utils

// RUN: execute 0x42::main::invalid_head
// CHECK: aborted: code 2 in 0x1::string_utils

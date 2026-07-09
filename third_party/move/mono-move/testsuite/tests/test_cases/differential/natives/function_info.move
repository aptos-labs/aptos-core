// Differential test for `function_info::is_identifier`.

// RUN: publish
module 0x1::function_info {
    public native fun is_identifier(s: &vector<u8>): bool;
}
module 0x1::main {
    public fun valid(): bool {
        let s = b"ab";
        0x1::function_info::is_identifier(&s)
    }

    public fun digit_start(): bool {
        let s = b"1a";
        0x1::function_info::is_identifier(&s)
    }

    public fun bad_char(): bool {
        let s = b"a.";
        0x1::function_info::is_identifier(&s)
    }

    public fun empty(): bool {
        let s = b"";
        0x1::function_info::is_identifier(&s)
    }
}

// RUN: execute 0x1::main::valid
// CHECK: results: true

// RUN: execute 0x1::main::digit_start
// CHECK: results: false

// RUN: execute 0x1::main::bad_char
// CHECK: results: false

// RUN: execute 0x1::main::empty
// CHECK: results: false

// Differential test for `function_info::is_identifier`.
// `not_utf8`: non-UTF-8 bytes give `false`, not an error (control: `valid`).
// `underscore`: a lone underscore is not an identifier.
// `underscore_letter`: an underscore followed by a letter is.

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

    public fun not_utf8(): bool {
        let bytes = x"61ff";
        0x1::function_info::is_identifier(&bytes)
    }

    public fun underscore(): bool {
        let bytes = b"_";
        0x1::function_info::is_identifier(&bytes)
    }

    public fun underscore_letter(): bool {
        let bytes = b"_a";
        0x1::function_info::is_identifier(&bytes)
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

// RUN: execute 0x1::main::not_utf8
// CHECK: results: false

// RUN: execute 0x1::main::underscore
// CHECK: results: false

// RUN: execute 0x1::main::underscore_letter
// CHECK: results: true

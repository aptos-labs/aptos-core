// `string_utils::native_format` of a String holding invalid UTF-8 (built with
// `from_bcs::from_bytes`, which skips the UTF-8 check), at top level, in a struct field and
// in a vector. Both VMs fail with an invariant violation; the messages differ and V1's
// carries a backtrace, so each side is pinned on its own.

// RUN: publish
module 0x1::from_bcs {
    public native fun from_bytes<T>(bytes: vector<u8>): T;
}
module 0x1::string_utils {
    use std::string::String;

    public native fun native_format<T>(
        value: &T,
        type_tag: bool,
        canonicalize: bool,
        single_line: bool,
        include_int_types: bool,
    ): String;
}
module 0x1::main {
    use std::string::String;

    struct Wrap has drop { s: String, n: u64 }

    fun bad(): String {
        0x1::from_bcs::from_bytes<String>(x"02ff61")
    }

    public fun fmt_top(): String {
        0x1::string_utils::native_format(&bad(), false, false, true, false)
    }

    public fun fmt_field(): String {
        0x1::string_utils::native_format(&Wrap { s: bad(), n: 1 }, false, false, true, false)
    }

    public fun fmt_vector(): String {
        0x1::string_utils::native_format(&vector[bad()], false, false, true, false)
    }
}

// RUN: execute 0x1::main::fmt_top
// CHECK-V1-SUBSTR: VMError with status UNKNOWN_INVARIANT_VIOLATION_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("string_utils") } and message Non-UTF8 string
// CHECK-V2: error: invariant violation: unreachable: `String` must hold UTF-8 bytes

// RUN: execute 0x1::main::fmt_field
// CHECK-V1-SUBSTR: VMError with status UNKNOWN_INVARIANT_VIOLATION_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("string_utils") } and message Non-UTF8 string
// CHECK-V2: error: invariant violation: unreachable: `String` must hold UTF-8 bytes

// RUN: execute 0x1::main::fmt_vector
// CHECK-V1-SUBSTR: VMError with status UNKNOWN_INVARIANT_VIOLATION_ERROR at location Module ModuleId { address: 0000000000000000000000000000000000000000000000000000000000000001, name: Identifier("string_utils") } and message Non-UTF8 string
// CHECK-V2: error: invariant violation: unreachable: `String` must hold UTF-8 bytes

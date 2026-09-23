// RUN: publish
module 0x1::string_utils {
    use std::string::String;

    struct Cons<T, N> has copy, drop, store {
        car: T,
        cdr: N,
    }

    struct NIL has copy, drop, store {}

    public fun cons<T, N>(car: T, cdr: N): Cons<T, N> {
        Cons { car, cdr }
    }

    public fun nil(): NIL {
        NIL {}
    }

    public native fun native_format<T>(
        s: &T,
        type_tag: bool,
        canonicalize: bool,
        single_line: bool,
        include_int_types: bool,
    ): String;

    public native fun native_format_list<T>(fmt: &vector<u8>, val: &T): String;
}

module 0x1::create_signer {
    public native fun create_signer(addr: address): signer;
}

module 0x1::main {
    use std::option;
    use std::string::{Self, String};
    use 0x1::string_utils;

    struct Foo has copy, drop {
        x: u64,
        y: bool,
    }

    struct Bar has copy, drop {
        foo: Foo,
        v: vector<u64>,
    }

    struct Wrap<T> has copy, drop {
        t: T,
    }

    struct Unit has copy, drop {}

    enum Shape has copy, drop {
        Circle { r: u64 },
        Point,
        Wrapper { foo: Foo, s: String, o: option::Option<u64> },
    }

    fun to_string<T>(s: &T): String {
        string_utils::native_format(s, false, false, true, false)
    }

    fun debug_string<T>(s: &T): String {
        string_utils::native_format(s, true, false, false, false)
    }

    fun qualified<T>(s: &T): String {
        string_utils::native_format(s, true, false, true, false)
    }

    fun foo(): Foo {
        Foo { x: 1, y: true }
    }

    // --- primitives ---

    public fun int_plain(): String {
        to_string(&7u8)
    }

    public fun int_typed(): String {
        string_utils::native_format(&7u8, false, false, true, true)
    }

    public fun int_max_u64(): String {
        to_string(&18446744073709551615u64)
    }

    public fun bool_val(): String {
        to_string(&true)
    }

    public fun addr_plain(): String {
        to_string(&@0x1)
    }

    public fun addr_canonical(): String {
        string_utils::native_format(&@0x1, false, true, true, false)
    }

    public fun signer_val(): String {
        let s = 0x1::create_signer::create_signer(@0x123);
        to_string(&s)
    }

    // --- vectors ---

    public fun bytes_hex(): String {
        to_string(&x"00ff10")
    }

    public fun bytes_empty(): String {
        to_string(&x"")
    }

    public fun u64_vec(): String {
        to_string(&vector[1u64, 2u64, 3u64])
    }

    public fun vec_empty(): String {
        to_string(&vector<u64>[])
    }

    // Aggregate elements stay on one line while `single_line` is set.
    public fun vec_nested(): String {
        to_string(&vector[vector[1u64], vector[2u64, 3u64]])
    }

    // --- strings ---

    public fun string_plain(): String {
        to_string(&string::utf8(b"hi"))
    }

    public fun string_escaped(): String {
        to_string(&string::utf8(b"a\"b\\c"))
    }

    // --- structs ---

    public fun struct_flat(): String {
        to_string(&foo())
    }

    public fun struct_nested(): String {
        to_string(&Bar { foo: foo(), v: vector[1u64, 2u64] })
    }

    // An empty struct gets a compiler-inserted `dummy_field`.
    public fun struct_empty(): String {
        to_string(&Unit {})
    }

    public fun struct_qualified(): String {
        qualified(&foo())
    }

    public fun generic_qualified(): String {
        qualified(&Wrap { t: 1u64 })
    }

    public fun generic_nested_qualified(): String {
        qualified(&Wrap { t: foo() })
    }

    // --- multi-line ---

    public fun debug_nested(): String {
        debug_string(&Bar { foo: foo(), v: vector[1u64, 2u64] })
    }

    public fun debug_vec_of_structs(): String {
        debug_string(&vector[foo(), foo()])
    }

    // --- option ---

    public fun option_none(): String {
        to_string(&option::none<u64>())
    }

    public fun option_some(): String {
        to_string(&option::some(1u64))
    }

    public fun option_nested(): String {
        to_string(&option::some(option::some(1u64)))
    }

    // --- enums ---

    public fun enum_fields(): String {
        to_string(&Shape::Circle { r: 7 })
    }

    public fun enum_unit(): String {
        to_string(&Shape::Point)
    }

    public fun enum_qualified(): String {
        qualified(&Shape::Circle { r: 7 })
    }

    public fun enum_subtree(): String {
        qualified(&Shape::Wrapper {
            foo: foo(),
            s: string::utf8(b"hi"),
            o: option::some(5u64),
        })
    }

    // --- format lists ---

    public fun fmt_two(): String {
        let l = string_utils::cons(1u64, string_utils::cons(2u64, string_utils::nil()));
        string_utils::native_format_list(&b"a = {} b = {}", &l)
    }

    public fun fmt_escapes(): String {
        let l = string_utils::cons(1u64, string_utils::nil());
        string_utils::native_format_list(&b"{{}} {}", &l)
    }

    public fun fmt_none(): String {
        string_utils::native_format_list(&b"plain", &string_utils::nil())
    }

    // List elements are always formatted with type tags on one line.
    public fun fmt_struct_elem(): String {
        let l = string_utils::cons(foo(), string_utils::nil());
        string_utils::native_format_list(&b"{}", &l)
    }

    public fun fmt_too_few(): String {
        let l = string_utils::cons(1u64, string_utils::nil());
        string_utils::native_format_list(&b"{} {}", &l)
    }

    public fun fmt_too_many(): String {
        let l = string_utils::cons(1u64, string_utils::cons(2u64, string_utils::nil()));
        string_utils::native_format_list(&b"{}", &l)
    }

    public fun fmt_not_a_list(): String {
        string_utils::native_format_list(&b"{}", &1u64)
    }

    // The chain ends in a `u64` rather than `NIL`.
    public fun fmt_bad_nil(): String {
        let l = string_utils::cons(1u64, 2u64);
        string_utils::native_format_list(&b"{}", &l)
    }

    public fun fmt_unmatched_open(): String {
        string_utils::native_format_list(&b"{x}", &string_utils::nil())
    }

    public fun fmt_unmatched_close(): String {
        string_utils::native_format_list(&b"a}b", &string_utils::nil())
    }

    public fun fmt_unclosed(): String {
        string_utils::native_format_list(&b"a{", &string_utils::nil())
    }
}

// RUN: execute 0x1::main::int_plain
// CHECK: results: "7"

// RUN: execute 0x1::main::int_typed
// CHECK: results: "7u8"

// RUN: execute 0x1::main::int_max_u64
// CHECK: results: "18446744073709551615"

// RUN: execute 0x1::main::bool_val
// CHECK: results: "true"

// RUN: execute 0x1::main::addr_plain
// CHECK: results: "@0x1"

// RUN: execute 0x1::main::addr_canonical
// CHECK: results: "@0000000000000000000000000000000000000000000000000000000000000001"

// RUN: execute 0x1::main::signer_val
// CHECK: results: "signer(@0x123)"

// RUN: execute 0x1::main::bytes_hex
// CHECK: results: "0x00ff10"

// RUN: execute 0x1::main::bytes_empty
// CHECK: results: "0x"

// RUN: execute 0x1::main::u64_vec
// CHECK: results: "[ 1, 2, 3 ]"

// RUN: execute 0x1::main::vec_empty
// CHECK: results: "[]"

// RUN: execute 0x1::main::vec_nested
// CHECK: results: "[ [ 1 ], [ 2, 3 ] ]"

// RUN: execute 0x1::main::string_plain
// CHECK: results: "\"hi\""

// RUN: execute 0x1::main::string_escaped
// CHECK: results: "\"a\\\"b\\\\c\""

// RUN: execute 0x1::main::struct_flat
// CHECK: results: "Foo { x: 1, y: true }"

// RUN: execute 0x1::main::struct_nested
// CHECK: results: "Bar { foo: Foo { x: 1, y: true }, v: [ 1, 2 ] }"

// RUN: execute 0x1::main::struct_empty
// CHECK: results: "Unit { dummy_field: false }"

// RUN: execute 0x1::main::struct_qualified
// CHECK: results: "0x1::main::Foo { x: 1, y: true }"

// RUN: execute 0x1::main::generic_qualified
// CHECK: results: "0x1::main::Wrap<u64> { t: 1 }"

// RUN: execute 0x1::main::generic_nested_qualified
// CHECK: results: "0x1::main::Wrap<0x1::main::Foo> { t: 0x1::main::Foo { x: 1, y: true } }"

// RUN: execute 0x1::main::debug_nested
// CHECK: results: "0x1::main::Bar {\n  foo: 0x1::main::Foo {\n    x: 1,\n    y: true\n  },\n  v: [ 1, 2 ]\n}"

// RUN: execute 0x1::main::debug_vec_of_structs
// CHECK: results: "[\n  0x1::main::Foo {\n    x: 1,\n    y: true\n  },\n  0x1::main::Foo {\n    x: 1,\n    y: true\n  }\n]"

// RUN: execute 0x1::main::option_none
// CHECK: results: "None"

// RUN: execute 0x1::main::option_some
// CHECK: results: "Some(1)"

// RUN: execute 0x1::main::option_nested
// CHECK: results: "Some(Some(1))"

// RUN: execute 0x1::main::enum_fields
// CHECK-V1: results: "#0{ 7 }"
// CHECK-V2: results: "Shape::Circle { r: 7 }"

// RUN: execute 0x1::main::enum_unit
// CHECK-V1: results: "#1{}"
// CHECK-V2: results: "Shape::Point {}"

// RUN: execute 0x1::main::enum_qualified
// CHECK-V1: results: "#0{ 7 }"
// CHECK-V2: results: "0x1::main::Shape::Circle { r: 7 }"

// RUN: execute 0x1::main::enum_subtree
// CHECK-V1: results: "#2{ { 1, true }, { 0x6869 }, #1{ 5 } }"
// CHECK-V2: results: "0x1::main::Shape::Wrapper { foo: 0x1::main::Foo { x: 1, y: true }, s: \"hi\", o: Some(5) }"

// RUN: execute 0x1::main::fmt_two
// CHECK: results: "a = 1 b = 2"

// RUN: execute 0x1::main::fmt_escapes
// CHECK: results: "{} 1"

// RUN: execute 0x1::main::fmt_none
// CHECK: results: "plain"

// RUN: execute 0x1::main::fmt_struct_elem
// CHECK: results: "0x1::main::Foo { x: 1, y: true }"

// EARGS_MISMATCH (1) carries no message on either VM.

// RUN: execute 0x1::main::fmt_too_few
// CHECK: aborted: code 1 in 0x1::string_utils

// RUN: execute 0x1::main::fmt_too_many
// CHECK: aborted: code 1 in 0x1::string_utils

// RUN: execute 0x1::main::fmt_not_a_list
// CHECK: aborted: code 1 in 0x1::string_utils

// RUN: execute 0x1::main::fmt_bad_nil
// CHECK: aborted: code 1 in 0x1::string_utils

// EINVALID_FORMAT (2) does, doubled braces and all.

// RUN: execute 0x1::main::fmt_unmatched_open
// CHECK: aborted: code 2 (Invalid format string: unmatched '{{' bracket) in 0x1::string_utils

// RUN: execute 0x1::main::fmt_unmatched_close
// CHECK: aborted: code 2 (Invalid format string: unmatched '}}' bracket) in 0x1::string_utils

// RUN: execute 0x1::main::fmt_unclosed
// CHECK: aborted: code 2 (Invalid format string: unclosed brackets) in 0x1::string_utils

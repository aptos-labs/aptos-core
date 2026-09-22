// Differential test for `type_info::type_name`.

// RUN: publish
module 0x1::type_info {
    use std::string::String;
    public native fun type_name<T>(): String;
}
module 0x1::main {
    use std::string::String;

    struct Foo has drop {}
    struct Bar<phantom T> has drop {}

    public fun u64_name(): String {
        0x1::type_info::type_name<u64>()
    }

    public fun address_name(): String {
        0x1::type_info::type_name<address>()
    }

    public fun vector_name(): String {
        0x1::type_info::type_name<vector<u8>>()
    }

    public fun struct_name(): String {
        0x1::type_info::type_name<Foo>()
    }

    public fun generic_struct_name(): String {
        0x1::type_info::type_name<Bar<u64>>()
    }

    public fun nested_struct_name(): String {
        0x1::type_info::type_name<Bar<Bar<u64>>>()
    }

    enum Choice<T> has drop {
        A,
        B { value: T },
    }

    public fun enum_name(): String {
        0x1::type_info::type_name<Choice<u64>>()
    }

    public fun nested_enum_name(): String {
        0x1::type_info::type_name<Bar<Choice<address>>>()
    }

    // Canonical function type names always parenthesize results.

    public fun function_name(): String {
        0x1::type_info::type_name<|u64|u64>()
    }

    public fun multi_result_function_name(): String {
        0x1::type_info::type_name<|u8|(u8, u16)>()
    }

    public fun no_arg_function_name(): String {
        0x1::type_info::type_name<||u8>()
    }

    public fun no_result_function_name(): String {
        0x1::type_info::type_name<|u8|()>()
    }

    public fun ref_arg_function_name(): String {
        0x1::type_info::type_name<|&u8, &mut u64|bool>()
    }

    public fun function_in_struct_name(): String {
        0x1::type_info::type_name<Bar<|u8|u8 has copy + drop>>()
    }

    // Result parentheses distinguish a function returning `||u8` from one
    // taking `||()` and returning `u8`.
    public fun function_in_function_name(): String {
        0x1::type_info::type_name<||(||u8)>()
    }

    public fun vector_of_functions_name(): String {
        0x1::type_info::type_name<vector<|Foo|Bar<u64>>>()
    }
}

// RUN: execute 0x1::main::u64_name
// CHECK: results: "u64"

// RUN: execute 0x1::main::address_name
// CHECK: results: "address"

// RUN: execute 0x1::main::vector_name
// CHECK: results: "vector<u8>"

// RUN: execute 0x1::main::struct_name
// CHECK: results: "0x1::main::Foo"

// RUN: execute 0x1::main::generic_struct_name
// CHECK: results: "0x1::main::Bar<u64>"

// RUN: execute 0x1::main::nested_struct_name
// CHECK: results: "0x1::main::Bar<0x1::main::Bar<u64>>"

// RUN: execute 0x1::main::enum_name
// CHECK: results: "0x1::main::Choice<u64>"

// RUN: execute 0x1::main::nested_enum_name
// CHECK: results: "0x1::main::Bar<0x1::main::Choice<address>>"

// RUN: execute 0x1::main::function_name
// CHECK: results: "|u64|(u64)"

// RUN: execute 0x1::main::multi_result_function_name
// CHECK: results: "|u8|(u8, u16)"

// RUN: execute 0x1::main::no_arg_function_name
// CHECK: results: "||(u8)"

// RUN: execute 0x1::main::no_result_function_name
// CHECK: results: "|u8|()"

// RUN: execute 0x1::main::ref_arg_function_name
// CHECK: results: "|&u8, &mut u64|(bool)"

// RUN: execute 0x1::main::function_in_struct_name
// CHECK: results: "0x1::main::Bar<|u8|(u8) has copy + drop>"

// RUN: execute 0x1::main::function_in_function_name
// CHECK: results: "||(||(u8))"

// RUN: execute 0x1::main::vector_of_functions_name
// CHECK: results: "vector<|0x1::main::Foo|(0x1::main::Bar<u64>)>"

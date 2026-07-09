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

    // TODO: cover function types once the specializer supports them.
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

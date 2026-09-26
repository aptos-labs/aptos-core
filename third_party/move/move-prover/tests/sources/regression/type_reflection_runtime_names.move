// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// The prover's type-reflection values must be the bytes the runtime produces.
//
// Every reflection native renders types with `TypeTag::to_canonical_string`. For a struct
// that is `0x<address with leading zeroes trimmed>::module::Name<arg0, arg1>`, and a
// struct's `struct_name` in `type_info::type_of` is `Name<arg0, arg1>`. The prover
// disagreed in two ways, each of which let a specification comparing against the wrong
// bytes verify:
//
// - `type_info::type_of` dropped the type arguments from `struct_name`, so every
//   instantiation of a generic struct became one value: `type_of<G<u8>>() ==
//   type_of<G<u64>>()` verified.
// - `std::type_name::get` rendered the address as 32 zero-padded digits with no `0x`.
//
// `type_info::type_name` already rendered the runtime form; it is pinned here as a
// control. Each claim sits in its own function so that one failure cannot mask another.

module extensions::type_info {
    use std::string;

    struct TypeInfo has copy, drop, store {
        account_address: address,
        module_name: vector<u8>,
        struct_name: vector<u8>,
    }

    public native fun type_of<T>(): TypeInfo;
    public native fun type_name<T>(): string::String;
}

module 0x42::type_reflection_runtime_names {
    use extensions::type_info;
    use std::string;
    use std::ascii;
    use std::type_name;

    struct G<phantom T> has drop {}
    struct H has drop {}

    // -- type_of keeps the type arguments -------------------------------------------

    /// Must fail: different instantiations are different type identities.
    public fun instantiations_differ(): type_info::TypeInfo {
        type_info::type_of<G<u8>>()
    }
    spec instantiations_differ {
        ensures result == type_info::type_of<G<u64>>();
    }

    /// Must fail: the runtime writes the type arguments into `struct_name`.
    public fun struct_name_without_args(): type_info::TypeInfo {
        type_info::type_of<G<u8>>()
    }
    spec struct_name_without_args {
        ensures result.struct_name == b"G";
    }

    /// Must verify.
    public fun struct_name_with_args(): type_info::TypeInfo {
        type_info::type_of<G<u8>>()
    }
    spec struct_name_with_args {
        ensures result.struct_name == b"G<u8>";
    }

    /// Must verify: a struct type argument is rendered in the runtime's form too.
    public fun struct_name_with_struct_arg(): type_info::TypeInfo {
        type_info::type_of<G<H>>()
    }
    spec struct_name_with_struct_arg {
        ensures result.struct_name == b"G<0x42::type_reflection_runtime_names::H>";
    }

    /// Must verify: distinct non-generic structs stay distinct.
    public fun distinct_concrete_control(): bool {
        type_info::type_of<G<u8>>() != type_info::type_of<H>()
    }
    spec distinct_concrete_control {
        ensures result == true;
    }

    /// Must verify: a type parameter nested inside a type argument. Rendering the arguments
    /// puts the parameter's name into `struct_name`, so a spec function reflecting `G<T>`
    /// must receive the parameter's type info, not only one reflecting a bare `T`.
    public fun nested_type_param<T>(): type_info::TypeInfo {
        type_info::type_of<G<T>>()
    }
    spec nested_type_param {
        ensures result == info_of_g<T>();
    }
    spec fun info_of_g<T>(): type_info::TypeInfo {
        type_info::type_of<G<T>>()
    }

    /// Must verify: a function-type argument is rendered as
    /// `FunctionTag::to_canonical_string` does, `|args|(results)` then the abilities.
    public fun function_type_arg(): type_info::TypeInfo {
        type_info::type_of<G<|u8|u8 has drop>>()
    }
    spec function_type_arg {
        ensures result.struct_name == b"G<|u8|(u8) has drop>";
    }

    // -- type_info::type_name (already correct; control) ------------------------------

    /// Must verify.
    public fun type_info_name(): string::String {
        type_info::type_name<G<u8>>()
    }
    spec type_info_name {
        ensures result.bytes == b"0x42::type_reflection_runtime_names::G<u8>";
    }

    // -- std::type_name renders the runtime form ------------------------------------

    /// Must fail: the runtime does not pad the address to 32 digits.
    public fun std_name_padded(): ascii::String {
        type_name::into_string(type_name::get<H>())
    }
    spec std_name_padded {
        ensures result.bytes == b"00000000000000000000000000000042::type_reflection_runtime_names::H";
    }

    /// Must verify.
    public fun std_name_runtime_form(): ascii::String {
        type_name::into_string(type_name::get<H>())
    }
    spec std_name_runtime_form {
        ensures result.bytes == b"0x42::type_reflection_runtime_names::H";
    }
}

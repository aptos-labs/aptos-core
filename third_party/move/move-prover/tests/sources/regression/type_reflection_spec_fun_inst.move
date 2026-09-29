// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A spec function using type reflection translates at vector, struct, concrete and repeated
// type arguments.

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

module 0x42::type_reflection_spec_fun_inst {
    use extensions::type_info;

    struct W<phantom T> has drop {}

    spec fun info<X>(): type_info::TypeInfo { type_info::type_of<X>() }
    spec fun same<X, Y>(): bool { type_info::type_of<X>() == type_info::type_of<Y>() }

    /// Must verify: a vector of a type parameter.
    public fun vector_arg<T>(): bool { true }
    spec vector_arg { ensures info<vector<T>>() == type_info::type_of<vector<T>>(); }

    /// Must verify: a struct over a type parameter.
    public fun struct_arg<T>(): bool { true }
    spec struct_arg { ensures info<W<T>>() == type_info::type_of<W<T>>(); }

    /// Must verify: a concrete type next to a type parameter.
    public fun mixed_args<T>(): bool { true }
    spec mixed_args { ensures same<u64, T>() == (type_info::type_of<u64>() == type_info::type_of<T>()); }

    /// Must verify: the same type parameter twice.
    public fun repeated_arg<T>(): bool { true }
    spec repeated_arg { ensures same<T, T>(); }

    /// Must fail: false at `T = u64`.
    public fun differs_from_u64<T>(): bool { true }
    spec differs_from_u64 { ensures info<vector<T>>() != info<vector<u64>>(); }
}

// `reflect::resolve` with a non-function type argument. Existence and access are checked
// before the type: a public target gives FunctionIncompatibleType (3), a private one
// FunctionNotAccessible (2) and a missing one FunctionNotFound (1). Resolving at the
// script pseudo-module address finds nothing (1).

// RUN: publish
module 0x42::target {
    public fun add(left: u64, right: u64): u64 { left + right }
    fun private_add(left: u64, right: u64): u64 { left + right }
    public fun call_private(): u64 { private_add(1, 2) }
}

module 0x42::m {
    use std::reflect;
    use std::string::utf8;

    public fun not_a_function(): u64 {
        reflect::resolve<u64>(@0x42, &utf8(b"target"), &utf8(b"add")).unwrap_err().error_code()
    }

    public fun not_a_function_private(): u64 {
        reflect::resolve<u64>(@0x42, &utf8(b"target"), &utf8(b"private_add")).unwrap_err().error_code()
    }

    public fun not_a_function_missing(): u64 {
        reflect::resolve<vector<u8>>(@0x42, &utf8(b"target"), &utf8(b"missing")).unwrap_err().error_code()
    }

    public fun script_pseudo_module(): u64 {
        reflect::resolve<|u64, u64|u64 has copy + drop>(
            @0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff, &utf8(b"main"), &utf8(b"main")
        ).unwrap_err().error_code()
    }
}

// RUN: execute 0x42::m::not_a_function
// CHECK: results: 3

// RUN: execute 0x42::m::not_a_function_private
// CHECK: results: 2

// RUN: execute 0x42::m::not_a_function_missing
// CHECK: results: 1

// RUN: execute 0x42::m::script_pseudo_module
// CHECK: results: 1

// Resource state keys include function type arguments in the struct tag.

// RUN: publish
module 0x42::function_type_arg {
    struct Tagged<phantom T> has key { value: u64 }

    public fun publish_and_read(owner: signer, addr: address): u64 {
        move_to(&owner, Tagged<|u8|(u8, u16) has copy + drop> { value: 7 });
        borrow_global<Tagged<|u8|(u8, u16) has copy + drop>>(addr).value
    }

    public fun publish_and_take(owner: signer, addr: address): bool {
        move_to(&owner, Tagged<|&u8|u8 has copy + drop + store> { value: 9 });
        let Tagged { value: _ } = move_from<Tagged<|&u8|u8 has copy + drop + store>>(addr);
        exists<Tagged<|&u8|u8 has copy + drop + store>>(addr)
    }

    // Different function type arguments produce distinct resource keys.
    public fun distinct_instantiations(owner: signer, addr: address): bool {
        move_to(&owner, Tagged<|u8|u8 has copy + drop> { value: 1 });
        exists<Tagged<|u8|(u8, u16) has copy + drop>>(addr)
    }
}

// RUN: execute 0x42::function_type_arg::publish_and_read --args 0x42, 0x42
// CHECK: results: 7

// RUN: execute 0x42::function_type_arg::publish_and_take --args 0x43, 0x43
// CHECK: results: false

// RUN: execute 0x42::function_type_arg::distinct_instantiations --args 0x44, 0x44
// CHECK: results: false

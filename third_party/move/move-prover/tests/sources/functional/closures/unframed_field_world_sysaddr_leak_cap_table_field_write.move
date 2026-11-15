// A capability for a hash-derived address is kept in a table and redirected to @0x1 by a write to its
// field through the table; the table is handed out: the rule is off.
module 0x1::system_addresses {
    use std::signer;
    public fun assert_aptos_framework(s: &signer) { assert!(signer::address_of(s) == @0x1, 3) }
    public fun assert_core_resource(s: &signer) { assert!(signer::address_of(s) == @0xA550C18, 2) }
}

module 0x1::create_signer {
    friend 0x1::account;
    public(friend) native fun create_signer(addr: address): signer;
    spec create_signer { pragma opaque; }
}

// Stand-in for the framework's table: values live behind the handle, reached through natives.
module 0x1::table {
    struct Table<phantom K: copy + drop, phantom V> has store { handle: address }
    struct Box<V> has key, drop, store { val: V }
    native fun new_table_handle<K, V>(): address;
    native fun add_box<K: copy + drop, V, B>(table: &mut Table<K, V>, key: K, val: Box<V>);
    native fun borrow_box_mut<K: copy + drop, V, B>(table: &mut Table<K, V>, key: K): &mut Box<V>;
    public fun new<K: copy + drop, V: store>(): Table<K, V> { Table { handle: new_table_handle<K, V>() } }
    public fun add<K: copy + drop, V>(self: &mut Table<K, V>, key: K, val: V) {
        add_box<K, V, Box<V>>(self, key, Box { val })
    }
    public fun borrow_mut<K: copy + drop, V>(self: &mut Table<K, V>, key: K): &mut V {
        &mut borrow_box_mut<K, V, Box<V>>(self, key).val
    }
    spec Table {
        pragma intrinsic = map, map_new = new, map_add_no_override = add, map_borrow_mut = borrow_mut;
    }
    spec new { pragma intrinsic; }
    spec add { pragma intrinsic; }
    spec borrow_mut { pragma intrinsic; }
}

// Stand-in for the framework's `account` capability.
module 0x1::account {
    use 0x1::create_signer;
    use 0x1::table::{Self, Table};
    struct SignerCapability has drop, store { account: address }
    public fun create_signer_with_capability(capability: &SignerCapability): signer {
        create_signer::create_signer(capability.account)
    }
    public fun create_resource_address(source: &address, seed: vector<u8>): address {
        if (*source == @0x0 && seed == vector[]) @0x1235 else @0x1234
    }
    // A capability for a hash-derived address kept in a table, then redirected through its field.
    public fun redirected(): Table<u64, SignerCapability> {
        let t = table::new();
        t.add(0, SignerCapability { account: create_resource_address(&@0x5, vector[1u8]) });
        t.borrow_mut(0).account = @0x1;
        t
    }
}
module 0x1::sa_gated {
    use std::signer;
    use 0x1::system_addresses;
    struct R has key { v: u64 }
    public fun init(fw: &signer) { system_addresses::assert_aptos_framework(fw); move_to(fw, R { v: 0 }) }
    public fun set(fw: &signer, v: u64) acquires R {
        system_addresses::assert_aptos_framework(fw);
        R[signer::address_of(fw)].v = v;
    }
}

module 0x60::user {
    use 0x1::sa_gated;
    struct Cb has drop, copy { f: |u64| u64 has copy + drop }

    // The callback may obtain a signer for @0x1 through `account::redirected` and its table. Rejected.
    public fun gated_invoke(c: &Cb, x: u64): u64 { (c.f)(x) }
    spec gated_invoke {
        requires exists<sa_gated::R>(@0x1);
        ensures global<sa_gated::R>(@0x1) == old(global<sa_gated::R>(@0x1));
    }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean

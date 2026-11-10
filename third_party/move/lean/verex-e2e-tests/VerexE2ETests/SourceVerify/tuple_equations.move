// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// An equation at a row of values is stated at its components, whichever its
/// sides: a pattern equated to a map's read, and a callee's tuple result equated
/// to an uninterpreted function's.
module 0x1::tuple_equations {
    struct Map<phantom K: copy + drop, phantom V> has store {}

    spec Map {
        pragma intrinsic = map,
            map_has_key = contains,
            map_del_return_key = take,
            map_spec_has_key = spec_contains,
            map_spec_get = spec_get,
            map_spec_set = spec_set,
            map_spec_del = spec_remove;
    }

    native fun contains<K: copy + drop, V>(self: &Map<K, V>, key: K): bool;
    native fun take<K: copy + drop, V>(self: &mut Map<K, V>, key: &K): (K, V);

    spec native fun spec_contains<K, V>(t: Map<K, V>, k: K): bool;
    spec native fun spec_get<K, V>(t: Map<K, V>, k: K): V;
    spec native fun spec_set<K, V>(t: Map<K, V>, k: K, v: V): Map<K, V>;
    spec native fun spec_remove<K, V>(t: Map<K, V>, k: K): Map<K, V>;

    struct Name has copy, drop, store {
        bytes: vector<u8>,
    }

    struct Packed has store, drop {
        name: Name,
        data: vector<u8>,
    }

    struct Pending has key {
        configs: Map<Name, Packed>,
    }

    native fun name_of<T>(): Name;
    spec native fun spec_name_of<T>(): Name;
    spec name_of {
        pragma opaque;
        aborts_if false;
        ensures result == spec_name_of<T>();
    }

    native fun decode<T>(bytes: vector<u8>): T;
    spec native fun spec_decode<T>(bytes: vector<u8>): T;
    spec native fun spec_decodable<T>(bytes: vector<u8>): bool;
    spec decode {
        pragma opaque;
        aborts_if !spec_decodable<T>(bytes);
        ensures result == spec_decode<T>(bytes);
    }

    /// A value read back from the map, unpacked and handed on to an opaque callee:
    /// what the clauses state of its read, the path states of the pattern's locals.
    fun unpacked<T>(): T {
        let configs = &mut Pending[@0x1];
        let key = name_of<T>();
        let (_, packed) = take(&mut configs.configs, &key);
        let Packed { name, data } = packed;
        assert!(name == name_of<T>(), 1);
        decode<T>(data)
    }
    spec unpacked {
        let key = spec_name_of<T>();
        let stored = spec_get(global<Pending>(@0x1).configs, key);
        modifies global<Pending>(@0x1);
        aborts_if !exists<Pending>(@0x1);
        aborts_if !spec_contains(global<Pending>(@0x1).configs, key);
        aborts_if stored.name != key;
        aborts_if !spec_decodable<T>(stored.data);
        ensures result == spec_decode<T>(stored.data);
        ensures !spec_contains(global<Pending>(@0x1).configs, key);
    }

    /// The key of the pair is dropped; the value is a struct read back from the
    /// map, whose name is compared with the key's.
    fun extract<T>(): vector<u8> {
        let configs = &mut Pending[@0x1];
        let key = name_of<T>();
        let (_, packed) = take(&mut configs.configs, &key);
        let Packed { name, data } = packed;
        assert!(name == name_of<T>(), 1);
        data
    }
    spec extract {
        let key = spec_name_of<T>();
        let stored = spec_get(global<Pending>(@0x1).configs, key);
        aborts_if !exists<Pending>(@0x1);
        aborts_if !spec_contains(global<Pending>(@0x1).configs, key);
        aborts_if stored.name != key;
        ensures result == stored.data;
        ensures !spec_contains(global<Pending>(@0x1).configs, key);
    }

    fun extract_wrong<T>(): vector<u8> {
        let configs = &mut Pending[@0x1];
        let key = name_of<T>();
        let (_, packed) = take(&mut configs.configs, &key);
        let Packed { name: _, data } = packed;
        data
    }
    spec extract_wrong {
        let post key = spec_name_of<T>();
        pragma aborts_if_is_partial;
        ensures result == spec_get(global<Pending>(@0x1).configs, key).data; // error: read after the removal
    }

    /// An opaque callee whose tuple result is an uninterpreted function's.
    spec native fun split_of(x: u64): (u64, u64);

    native fun split(x: u64): (u64, u64);
    spec split {
        pragma opaque;
        aborts_if false;
        ensures (result_1, result_2) == split_of(x);
    }

    fun swapped(x: u64): (u64, u64) {
        let (a, b) = split(x);
        (b, a)
    }
    spec swapped {
        aborts_if false;
        ensures (result_2, result_1) == split_of(x);
    }

    fun swapped_wrong(x: u64): (u64, u64) {
        let (a, b) = split(x);
        (b, a)
    }
    spec swapped_wrong {
        aborts_if false;
        ensures (result_1, result_2) == split_of(x); // error: the components are swapped
    }

    /// Two opaque tuples compared.
    spec native fun split_copy_of(x: u64): (u64, u64);

    native fun split_copy(x: u64): (u64, u64);
    spec split_copy {
        pragma opaque;
        aborts_if false;
        ensures split_copy_of(x) == split_of(x);
        ensures (result_1, result_2) == split_copy_of(x);
    }

    fun copied(x: u64): (u64, u64) {
        split_copy(x)
    }
    spec copied {
        aborts_if false;
        ensures (result_1, result_2) == split_of(x);
    }

    /// A component's read of the same result equated at a second call.
    fun twice(x: u64): u64 {
        let (a, _) = split(x);
        let (c, _) = split(x);
        a - c
    }
    spec twice {
        aborts_if false;
        ensures result == 0;
    }
}

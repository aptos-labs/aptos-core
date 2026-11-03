// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// An equation a generic callee's contract states about its type parameter's
// values, at a caller's closed type: a result is the value it equals, whether
// the type is a vector, an enum or a struct of them.
module 0x42::generic_contract_equation {
    struct Pair has copy, drop {
        left: address,
        bytes: vector<u8>,
    }

    enum Choice<T> has copy, drop {
        Nothing,
        Just { value: T },
    }

    spec fun spec_just<T>(value: T): Choice<T> {
        Choice::Just { value }
    }

    spec fun spec_nothing<T>(): Choice<T> {
        Choice::Nothing
    }

    fun id<T>(value: T): T {
        value
    }
    spec id {
        pragma opaque;
        aborts_if false;
        ensures result == value;
    }

    fun just<T>(value: T): Choice<T> {
        Choice::Just { value }
    }
    spec just {
        pragma opaque;
        aborts_if false;
        ensures result == spec_just(value);
    }

    fun nothing<T>(): Choice<T> {
        Choice::Nothing
    }
    spec nothing {
        pragma opaque;
        aborts_if false;
        ensures result == spec_nothing<T>();
    }

    fun vector_value(bytes: vector<u8>): vector<u8> {
        id(bytes)
    }
    spec vector_value {
        ensures result == bytes;
    }

    fun struct_value(pair: Pair): Pair {
        id(pair)
    }
    spec struct_value {
        ensures result == pair;
        ensures result.bytes == pair.bytes;
    }

    fun choice(key: address, some: bool): Choice<address> {
        if (some) just(key) else nothing()
    }
    spec choice {
        ensures some ==> result == spec_just(key);
        ensures !some ==> result == spec_nothing<address>();
    }

    fun nested(bytes: vector<u8>, some: bool): Choice<vector<u8>> {
        if (some) just(id(bytes)) else nothing()
    }
    spec nested {
        ensures some ==> result == spec_just(bytes);
    }

    struct Holder has key {
        key: address,
        bytes: vector<u8>,
    }

    // The value a specification reads from memory and the callee's argument
    // read from the same memory: equal once the result is rewritten.
    fun held_key(owner: address): Choice<address> {
        just(Holder[owner].key)
    }
    spec held_key {
        aborts_if !exists<Holder>(owner);
        ensures result == spec_just(global<Holder>(owner).key);
    }

    fun held_bytes(owner: address): vector<u8> {
        id(Holder[owner].bytes)
    }
    spec held_bytes {
        aborts_if !exists<Holder>(owner);
        ensures result == global<Holder>(owner).bytes;
    }

    // The equation is of these values, not of any others.
    fun wrong_value(bytes: vector<u8>, other: vector<u8>): vector<u8> {
        id(bytes)
    }
    spec wrong_value {
        ensures result == other; // error: `id` returns `bytes`
    }

    fun wrong_variant(key: address, some: bool): Choice<address> {
        if (some) just(key) else nothing()
    }
    spec wrong_variant {
        ensures result == spec_just(key); // error: without `some` the result is `Nothing`
    }
}

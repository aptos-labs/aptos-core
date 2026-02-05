// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Collection module demonstrating stdlib usage.
module test_addr::collection {
    use std::vector;
    use std::option::{Self, Option};

    /// A collection of items with optional metadata.
    struct Collection<T: store> has key, store {
        items: vector<T>,
        name: Option<vector<u8>>,
    }

    /// Error code for empty collection.
    const E_EMPTY_COLLECTION: u64 = 1;

    /// Error code for index out of bounds.
    const E_INDEX_OUT_OF_BOUNDS: u64 = 2;

    /// Create a new empty collection.
    public fun new<T: store>(): Collection<T> {
        Collection {
            items: vector::empty<T>(),
            name: option::none(),
        }
    }

    /// Create a new collection with a name.
    public fun new_named<T: store>(name: vector<u8>): Collection<T> {
        Collection {
            items: vector::empty<T>(),
            name: option::some(name),
        }
    }

    /// Add an item to the collection.
    public fun add<T: store>(collection: &mut Collection<T>, item: T) {
        vector::push_back(&mut collection.items, item);
    }

    /// Remove and return the last item.
    public fun pop<T: store>(collection: &mut Collection<T>): T {
        assert!(!vector::is_empty(&collection.items), E_EMPTY_COLLECTION);
        vector::pop_back(&mut collection.items)
    }

    /// Get the length of the collection.
    public fun length<T: store>(collection: &Collection<T>): u64 {
        vector::length(&collection.items)
    }

    /// Check if collection is empty.
    public fun is_empty<T: store>(collection: &Collection<T>): bool {
        vector::is_empty(&collection.items)
    }

    /// Get the name if set.
    public fun get_name<T: store>(collection: &Collection<T>): Option<vector<u8>> {
        collection.name
    }

    /// Set the collection name.
    public fun set_name<T: store>(collection: &mut Collection<T>, name: vector<u8>) {
        collection.name = option::some(name);
    }

    /// Check if collection has a name.
    public fun has_name<T: store>(collection: &Collection<T>): bool {
        option::is_some(&collection.name)
    }

    /// Borrow an item at index.
    public fun borrow<T: store>(collection: &Collection<T>, index: u64): &T {
        assert!(index < vector::length(&collection.items), E_INDEX_OUT_OF_BOUNDS);
        vector::borrow(&collection.items, index)
    }

    /// Borrow a mutable reference to an item at index.
    public fun borrow_mut<T: store>(collection: &mut Collection<T>, index: u64): &mut T {
        assert!(index < vector::length(&collection.items), E_INDEX_OUT_OF_BOUNDS);
        vector::borrow_mut(&mut collection.items, index)
    }
}

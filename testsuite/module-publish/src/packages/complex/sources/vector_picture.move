// Copyright Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0xABCD::vector_picture {
    use std::error;
    use std::signer;
    use std::vector;
    use aptos_std::object;

    /// The caller tried to mutate an item outside the bounds of the vector.
    const E_INDEX_OUT_OF_BOUNDS: u64 = 1;

    /// The caller tried to create palette for non-initialized account.
    const E_ALL_PALETTES_NOT_INIT: u64 = 2;

    struct AllPalettes has key {
        all: vector<address>,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct Palette has key {
        vec: vector<Color>,
    }

    struct Color has copy, drop, store {
        r: u8,
        g: u8,
        b: u8,
    }

    fun init_module(publisher: &signer) {
        move_to<AllPalettes>(
            publisher,
            AllPalettes {
                all: vector::empty(),
            },
        );
    }

    /// Create a new Palette
    public entry fun create(
        caller: &signer,
        // Length of the vector to create.
        length: u64,
    ) acquires AllPalettes {
        let caller_addr = signer::address_of(caller);

        // Just a dummy color.
        let color = Color {
            r: 124,
            g: 213,
            b: 37,
        };

        // Build vec and palette.
        let vec = vector::empty();
        let i = 0;
        while (i < length) {
            vector::push_back(&mut vec, color);
            i = i + 1;
        };
        let palette = Palette { vec };

        // Create the object we'll store the Palette in.
        let constructor_ref = object::create_object(caller_addr);

        // Move the Palette resource into the object.
        let object_signer = object::generate_signer(&constructor_ref);
        move_to(&object_signer, palette);

        assert!(exists<AllPalettes>(caller_addr), error::invalid_argument(E_ALL_PALETTES_NOT_INIT));

        let all_palettes = borrow_global_mut<AllPalettes>(caller_addr);
        vector::push_back(&mut all_palettes.all, object::address_from_constructor_ref(&constructor_ref));
    }

    /// Update an element in the vector.
    public entry fun update(
        palette_addr: address,
        palette_index: u64,
        index: u64,
        r: u8,
        g: u8,
        b: u8,
    ) acquires Palette, AllPalettes {
        let all_palettes = borrow_global<AllPalettes>(palette_addr);
        let palette_addr = vector::borrow(&all_palettes.all, palette_index);

        let palette_ = borrow_global_mut<Palette>(*palette_addr);

        // Confirm the index is not out of bounds.
        assert!(index < vector::length(&palette_.vec), error::invalid_argument(E_INDEX_OUT_OF_BOUNDS));

        // Write the pixel.
        let color = Color { r, g, b };
        *vector::borrow_mut(&mut palette_.vec, index) = color;
    }
}

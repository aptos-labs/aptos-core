// Copyright Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0xABCD::smart_table_picture {
    use std::signer;
    use std::vector;
    use aptos_std::object;
    use aptos_std::smart_table::{Self, SmartTable};

    /// The caller tried to mutate an item outside the bounds of the vector.
    const E_INDEX_OUT_OF_BOUNDS: u64 = 1;

    struct AllPalettes has key {
        all: vector<address>,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct Palette has key {
        pixels: SmartTable<u32, u8>,
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
    ) acquires AllPalettes {
        let caller_addr = signer::address_of(caller);

        let palette = Palette { pixels: smart_table::new() };

        // Create the object we'll store the Palette in.
        let constructor_ref = object::create_object(caller_addr);

        // Move the Palette resource into the object.
        let object_signer = object::generate_signer(&constructor_ref);
        move_to(&object_signer, palette);

        if (!exists<AllPalettes>(caller_addr)) {
            let vec = vector::empty();
            vector::push_back(&mut vec, object::address_from_constructor_ref(&constructor_ref));

            move_to<AllPalettes>(
                caller,
                AllPalettes {
                    all: vec,
                },
            );
        } else {
            let all_palettes = borrow_global_mut<AllPalettes>(caller_addr);
            vector::push_back(&mut all_palettes.all, object::address_from_constructor_ref(&constructor_ref));
        }
    }

    /// Update an element in the vector.
    public entry fun update(
        palette_addr: address,
        palette_index: u64,
        indices: vector<u64>,
        colors: vector<u8>,
    ) acquires Palette, AllPalettes {
        let all_palettes = borrow_global<AllPalettes>(palette_addr);
        let palette_addr = vector::borrow(&all_palettes.all, palette_index);

        let palette = borrow_global_mut<Palette>(*palette_addr);

        assert!(
            vector::length(&indices) == vector::length(&colors),
            E_INDEX_OUT_OF_BOUNDS,
        );

        let i = 0;
        let len = vector::length(&indices);
        while (i < len) {
            assert!(!vector::is_empty(&indices), E_INDEX_OUT_OF_BOUNDS);
            let index = (vector::pop_back(&mut indices) as u32);
            assert!(!vector::is_empty(&colors), E_INDEX_OUT_OF_BOUNDS);
            let color = vector::pop_back(&mut colors);

            smart_table::upsert(&mut palette.pixels, index, color);
            i = i + 1;
        };
    }
}

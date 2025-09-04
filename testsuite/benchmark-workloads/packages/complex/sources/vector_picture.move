// Copyright Velor Foundation
// SPDX-License-Identifier: Apache-2.0

module 0xABCD::vector_picture {
    use std::error;
    use std::signer;
    use std::vector;
    use velor_std::object;

    /// The caller tried to mutate an item outside the bounds of the vector.
    const E_INDEX_OUT_OF_BOUNDS: u64 = 1;

    /// Color checked is max.
    const E_MAX_COLOR: u64 = 3;

    struct AllPalettes has key {
        all: vector<address>,
    }

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
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
            vec.push_back(color);
            i = i + 1;
        };
        let palette = Palette { vec };

        // Create the object we'll store the Palette in.
        let constructor_ref = object::create_object(caller_addr);

        // Move the Palette resource into the object.
        let object_signer = object::generate_signer(&constructor_ref);
        move_to(&object_signer, palette);

        if (!exists<AllPalettes>(caller_addr)) {
            let vec = vector::empty();
            vec.push_back(object::address_from_constructor_ref(&constructor_ref));

            move_to<AllPalettes>(
                caller,
                AllPalettes {
                    all: vec,
                },
            );
        } else {
            let all_palettes = borrow_global_mut<AllPalettes>(caller_addr);
            all_palettes.all.push_back(object::address_from_constructor_ref(&constructor_ref));
        }
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
        let palette_addr = all_palettes.all.borrow(palette_index);

        let palette = borrow_global_mut<Palette>(*palette_addr);

        // Confirm the index is not out of bounds.
        assert!(index < palette.vec.length(), error::invalid_argument(E_INDEX_OUT_OF_BOUNDS));

        // Write the pixel.
        let color = Color { r, g, b };
        *palette.vec.borrow_mut(index) = color;
    }

    public entry fun check(
        palette_addr: address,
        palette_index: u64,
        index: u64,
    ) acquires Palette, AllPalettes {
        let all_palettes = borrow_global<AllPalettes>(palette_addr);
        let palette_addr = all_palettes.all.borrow(palette_index);

        let palette = borrow_global_mut<Palette>(*palette_addr);

        // Confirm the index is not out of bounds.
        assert!(index < palette.vec.length(), error::invalid_argument(E_INDEX_OUT_OF_BOUNDS));

        let color = palette.vec.borrow(index);
        assert!(color.r != 255 || color.g != 255 || color.b != 255, error::invalid_argument(E_MAX_COLOR));
    }
}

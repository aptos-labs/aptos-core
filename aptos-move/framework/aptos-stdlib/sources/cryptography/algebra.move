module aptos_std::algebra {
    use std::option::Option;

    struct BLS12_381_Fr {}
    struct BLS12_381_Fq {}
    struct BLS12_381_Fq2 {}
    struct BLS12_381_Fq6 {}
    struct BLS12_381_Fq12 {}
    struct BLS12_381_G1 {}
    struct BLS12_381_G2 {}
    struct BLS12_381_Gt {}

    struct Element<phantom S> has copy, drop {
        handle: u64,
    }

    // Common operations.
    public fun deserialize_compressed_checked<S>(bytes: &vector<u8>): Option<Element<S>> {
        let (succeeded, handle) = deserialize_internal<S>(*bytes, true, true);
        if (succeeded) {
            std::option::some(Element<S>{ handle })
        } else {
            std::option::none<Element<S>>()
        }
    }

    public fun deserialize_compressed_unchecked<S>(bytes: &vector<u8>): Option<Element<S>> {
        let (succeeded, handle) = deserialize_internal<S>(*bytes, true, false);
        if (succeeded) {
            std::option::some(Element<S>{ handle })
        } else {
            std::option::none<Element<S>>()
        }
    }

    public fun deserialize_uncompressed_checked<S>(bytes: &vector<u8>): Option<Element<S>> {
        let (succeeded, handle) = deserialize_internal<S>(*bytes, false, true);
        if (succeeded) {
            std::option::some(Element<S>{ handle })
        } else {
            std::option::none<Element<S>>()
        }
    }

    public fun deserialize_uncompressed_unchecked<S>(bytes: &vector<u8>): Option<Element<S>> {
        let (succeeded, handle) = deserialize_internal<S>(*bytes, false, false);
        if (succeeded) {
            std::option::some(Element<S>{ handle })
        } else {
            std::option::none<Element<S>>()
        }
    }

    public fun serialize_compressed<S>(element: &Element<S>): vector<u8> {
        serialize_internal<S>(element.handle, true)
    }

    public fun serialize_uncompressed<S>(element: &Element<S>): vector<u8> {
        serialize_internal<S>(element.handle, false)
    }

    public fun hash_to<S>(bytes: &vector<u8>): Element<S> {
        Element<S> {
            handle: hash_to_internal<S>(*bytes)
        }
    }

    public fun validate<S>(element: &Element<S>): bool {
        validate_internal<S>(element.handle)
    }

    // Field operations.
    public fun field_add<F>(element_0: &Element<F>, element_1: &Element<F>): Element<F> {
        Element<F> {
            handle: field_add_internal<F>(element_0.handle, element_1.handle)
        }
    }

    public fun field_zero<F>(): Element<F> {
        Element<F> {
            handle: field_zero_internal<F>()
        }
    }

    public fun field_div<F>(element_0: &Element<F>, element_1: &Element<F>): Option<Element<F>> {
        let (succeeded, handle) = field_div_internal<F>(element_0.handle, element_1.handle);
        if (succeeded) {
            std::option::some(Element<F>{ handle })
        } else {
            std::option::none<Element<F>>()
        }
    }

    public fun field_eq<F>(element_0: &Element<F>, element_1: &Element<F>): bool {
        field_eq_internal<F>(element_0.handle, element_1.handle)
    }

    public fun field_inv<F>(element: &Element<F>): Option<Element<F>> {
        let (succeeded, handle) = field_inv_internal<F>(element.handle);
        if (succeeded) {
            std::option::some(Element<F>{ handle })
        } else {
            std::option::none<Element<F>>()
        }
    }

    public fun field_mul<F>(element_0: &Element<F>, element_1: &Element<F>): Element<F> {
        Element<F> {
            handle: field_mul_internal<F>(element_0.handle, element_1.handle)
        }
    }

    public fun field_one<F>(): Element<F> {
        Element<F> {
            handle: field_one_internal<F>()
        }
    }

    public fun field_neg<F>(element: &Element<F>): Element<F> {
        Element<F> {
            handle: field_neg_internal<F>(element.handle)
        }
    }

    public fun field_sub<F>(element_0: &Element<F>, element_1: &Element<F>): Element<F> {
        Element<F> {
            handle: field_sub_internal<F>(element_0.handle, element_1.handle)
        }
    }

    public fun field_element_from_u64<F>(val: u64): Element<F> {
        Element<F> {
            handle: field_element_from_u64_internal<F>(val)
        }
    }

    public fun scalar_from_field_element<F>(_element: &Element<F>): vector<u8> {
        //TODO
        vector[3, 4, 5]
    }

    // Group operations.
    public fun group_add<G>(element_0: &Element<G>, element_1: &Element<G>): Element<G> {
        Element<G> {
            handle: group_add_internal<G>(element_0.handle, element_1.handle)
        }
    }

    public fun group_eq<G>(element_1: &Element<G>, element_2: &Element<G>): bool {
        group_eq_internal<G>(element_1.handle, element_2.handle)
    }

    public fun group_generator<G>(): Element<G> {
        Element<G> {
            handle: group_generator_internal<G>()
        }
    }

    public fun group_identity<G>(): Element<G> {
        Element<G> {
            handle: group_identity_internal<G>()
        }
    }

    public fun group_multi_scalar_mul<G>(_element: &vector<Element<G>>, _scalar: &vector<vector<u8>>): Element<G> {
        //TODO
        Element<G> { handle: 0 }
    }

    public fun group_neg<G>(): Element<G> {
        //TODO
        Element<G> { handle: 0 }
    }

    public fun group_scalar_mul<G>(_element: &Element<G>, _scalar: &vector<u8>): Element<G> {
        //TODO
        Element<G> { handle: 0 }
    }

    public fun pairing<G1,G2,Gt>(_element_1: &Element<G1>, _element_2: &Element<G2>): Element<Gt> {
        //TODO
        Element<Gt> { handle: 0 }
    }

    public fun pairing_product<G1,G2,Gt>(_g1_elements: &vector<Element<G1>>, _g2_elements: &vector<Element<G2>>): Element<Gt> {
        //TODO
        Element<Gt> { handle: 0 }
    }

    // Natives.
    native fun deserialize_internal<S>(bytes: vector<u8>, compressed: bool, checked: bool): (bool, u64);
    native fun field_add_internal<S>(handle_0: u64, handle_1: u64): u64;
    native fun field_div_internal<F>(handle_0: u64, handle_1: u64): (bool, u64);
    native fun field_eq_internal<F>(handle_0: u64, handle_1: u64): bool;
    native fun field_inv_internal<F>(handle: u64): (bool, u64);
    native fun field_mul_internal<F>(handle_0: u64, handle_1: u64): u64;
    native fun field_neg_internal<F>(handle: u64): u64;
    native fun field_one_internal<F>(): u64;
    native fun field_sub_internal<F>(handle_0: u64, handle_1: u64): u64;
    native fun field_zero_internal<F>(): u64;
    native fun field_element_from_u64_internal<F>(val: u64): u64;
    native fun group_add_internal<S>(handle_0: u64, handle_1: u64): u64;
    native fun group_eq_internal<F>(handle_0: u64, handle_1: u64): bool;
    native fun group_generator_internal<F>(): u64;
    native fun group_identity_internal<F>(): u64;
    native fun hash_to_internal<S>(bytes: vector<u8>): u64;
    native fun serialize_internal<S>(handle: u64, compressed: bool): vector<u8>;
    native fun validate_internal<S>(handle: u64): bool;

    #[test]
    fun test_BLS12_381_G1() {
        let g1_gen = group_generator<BLS12_381_G1>();
        let g2_gen = group_generator<BLS12_381_G2>();
        let fr_5 = field_element_from_u64<BLS12_381_Fr>(5);
        let fr_7 = field_element_from_u64<BLS12_381_Fr>(7);
        let scalar_5 = scalar_from_field_element<BLS12_381_Fr>(&fr_5);
        let scalar_7 = scalar_from_field_element<BLS12_381_Fr>(&fr_7);
        let g1_5 = group_scalar_mul(&g1_gen, &scalar_5);
        let g1_7 = group_scalar_mul(&g1_gen, &scalar_7);
        let g2_5 = group_scalar_mul(&g2_gen, &scalar_5);
        let g2_7 = group_scalar_mul(&g2_gen, &scalar_7);
        let gt_35 = pairing<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(&g1_5, &g2_7);
        let gt_35_another = pairing<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(&g1_7, &g2_5);
        assert!(group_eq(&gt_35, &gt_35_another), 1);
    }
}

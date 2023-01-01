module aptos_std::curves {
    use std::option::Option;

    // Error codes
    const E_NATIVE_FUN_NOT_AVAILABLE: u64 = 1;

    /// A phantom type that represents the 1st pairing input group `G1` in BLS12-381 pairing.
    ///
    /// In BLS12-381, a finite field `Fq` is used.
    /// q equals to 0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab.
    /// A curve `E(Fq)` is defined as `y^2=x^3+4` over `Fq`.
    /// `G1` is formed by a subset of points on `E(Fq)`.
    /// `G1` has a prime order `r` with value 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.
    ///
    /// A `Scalar<BLS12_381_G1>` is an integer between 0 and `r-1`.
    ///
    /// Function `scalar_from_bytes<BLS12_381_G1>` and `scalar_to_bytes<BLS12_381_G1>`
    /// assumes a 32-byte little-endian encoding of a `Scalar<BLS12_381_G1>`.
    ///
    /// An `Element<BLS12_381_G1>` is an element in `G1`.
    ///
    /// Function `serialize_element_uncompressed<BLS12_381_G1>` and `deserialize_element_uncompressed<BLS12_381_G1>`
    /// assumes a 96-byte encoding `[b_0, ..., b_95]` of an `Element<BLS12_381_G1>`, with the following rules.
    /// - `b_95 & 0x40` is the infinity flag.
    /// - The infinity flag is 1 if and only if the element is the point at infinity.
    /// - The infinity flag is 0 if and only if the element is a point `(x,y)` on curve, with the following rules.
    ///     - `[b_0, ..., b_47 & 0x3f]` is a 48-byte little-endian encoding of `x`.
    ///     - `[b_48, ..., b_95 & 0x3f]` is a 48-byte little-endian encoding of 'y'.
    ///
    /// Function `serialize_element_compressed<BLS12_381_G1>` and `deserialize_element_compressed<BLS12_381_G1>`
    /// assumes a 48-byte encoding `[b_0, ..., b_47]` of an `Element<BLS12_381_G1>` with the following rules.
    /// - `b_47 & 0x40` is the infinity flag.
    /// - The infinity flag is 1 if and only if the element is the point at infinity.
    /// - The infinity flag is 0 if and only if the element is a point `(x,y)` on curve, with the following rules.
    ///     - `[b_0, ..., b_47 & 0x3f]` is a 48-byte little-endian encoding of `x`.
    ///     - `b_47 & 0x80` is the positiveness flag.
    ///     - The positiveness flag is 1 if and only if `y > -y`.
    struct BLS12_381_G1 {}

    /// This is a phantom type that represents the 2nd pairing input group `G2` in BLS12-381 pairing.
    /// TODO: describe the encoding.
    struct BLS12_381_G2 {}

    /// This is a phantom type that represents the pairing output group `Gt` in BLS12-381 pairing.
    /// TODO: describe the encoding.
    struct BLS12_381_Gt {}

    /// This struct represents a scalar, usually an integer between 0 and `r-1`,
    /// where `r` is the prime order of a group, where the group is determined by the type argument `G`.
    /// See the comments on the specific `G` for more details about `Scalar<G>`.
    struct Scalar<phantom G> has copy, drop {
        handle: u64
    }

    /// This struct represents a group element, usually a point in an elliptic curve.
    /// The group is determined by the type argument `G`.
    /// See the comments on the specific `G` for more details about `Element<G>`.
    struct Element<phantom G> has copy, drop {
        handle: u64
    }

    /// Perform a bilinear mapping.
    public fun pairing<G1,G2,Gt>(point_1: &Element<G1>, point_2: &Element<G2>): Element<Gt> {
        abort_if_feature_disabled();
        if (!std::features::generic_curves_enabled()) {
            abort(std::error::invalid_state(1))
        };
        Element<Gt> {
            handle: pairing_internal<G1,G2,Gt>(point_1.handle, point_2.handle)
        }
    }

    /// Compute the product of multiple pairing: `e(p1_1,p2_1) * ... * e(p1_n,p2_n)`.
    public fun multi_pairing<G1,G2,Gt>(g1_elements: &vector<Element<G1>>, g2_elements: &vector<Element<G2>>): Element<Gt> {
        abort_if_feature_disabled();
        let num_g1 = std::vector::length(g1_elements);
        let num_g2 = std::vector::length(g2_elements);
        assert!(num_g1 == num_g2, std::error::invalid_argument(1));
        let g1_handles = vector[];
        let g2_handles = vector[];
        let i = 0;
        while (i < num_g2) {
            std::vector::push_back(&mut g1_handles, std::vector::borrow(g1_elements, i).handle);
            std::vector::push_back(&mut g2_handles, std::vector::borrow(g2_elements, i).handle);
            i = i + 1;
        };

        Element<Gt> {
            handle: multi_pairing_internal<G1,G2,Gt>(num_g1, g1_handles, num_g2, g2_handles)
        }
    }

    public fun scalar_from_u64<G>(value: u64): Scalar<G> {
        abort_if_feature_disabled();
        if (!std::features::generic_curves_enabled()) {
            abort(std::error::invalid_state(1))
        };
        Scalar<G> {
            handle: scalar_from_u64_internal<G>(value)
        }
    }

    public fun scalar_neg<G>(scalar_1: &Scalar<G>): Scalar<G> {
        abort_if_feature_disabled();
        if (!std::features::generic_curves_enabled()) {
            abort(std::error::invalid_state(1))
        };
        Scalar<G> {
            handle: scalar_neg_internal<G>(scalar_1.handle)
        }
    }

    public fun scalar_add<G>(scalar_1: &Scalar<G>, scalar_2: &Scalar<G>): Scalar<G> {
        abort_if_feature_disabled();
        if (!std::features::generic_curves_enabled()) {
            abort(std::error::invalid_state(1))
        };
        Scalar<G> {
            handle: scalar_add_internal<G>(scalar_1.handle, scalar_2.handle)
        }
    }

    public fun scalar_mul<G>(scalar_1: &Scalar<G>, scalar_2: &Scalar<G>): Scalar<G> {
        abort_if_feature_disabled();
        Scalar<G> {
            handle: scalar_mul_internal<G>(scalar_1.handle, scalar_2.handle)
        }
    }

    public fun scalar_inv<G>(scalar: &Scalar<G>): Option<Scalar<G>> {
        abort_if_feature_disabled();
        let (succeeded, handle) = scalar_inv_internal<G>(scalar.handle);
        if (succeeded) {
            let scalar = Scalar<G> { handle };
            std::option::some(scalar)
        } else {
            std::option::none()
        }
    }

    public fun scalar_eq<G>(scalar_1: &Scalar<G>, scalar_2: &Scalar<G>): bool {
        abort_if_feature_disabled();
        scalar_eq_internal<G>(scalar_1.handle, scalar_2.handle)
    }

    // Point basics.
    public fun identity<G>(): Element<G> {
        abort_if_feature_disabled();
        Element<G> {
            handle: identity_internal<G>()
        }
    }

    public fun generator<G>(): Element<G> {
        abort_if_feature_disabled();
        Element<G> {
            handle: generator_internal<G>()
        }
    }

    public fun element_neg<G>(point: &Element<G>): Element<G> {
        abort_if_feature_disabled();
        Element<G> {
            handle: element_neg_internal<G>(point.handle)
        }
    }

    public fun element_add<G>(point_1: &Element<G>, point_2: &Element<G>): Element<G> {
        abort_if_feature_disabled();
        Element<G> {
            handle: element_add_internal<G>(point_1.handle, point_2.handle)
        }
    }

    public fun element_mul<G>(_scalar: &Scalar<G>, _point: &Element<G>): Element<G> {
        abort_if_feature_disabled();
        Element<G> {
            handle: element_mul_internal<G>(_scalar.handle, _point.handle)
        }
    }

    public fun simul_point_mul<G>(scalars: &vector<Scalar<G>>, points: &vector<Element<G>>): Element<G> {
        abort_if_feature_disabled();
        //TODO: replace the naive implementation.
        let result = identity<G>();
        let num_points = std::vector::length(points);
        let num_scalars = std::vector::length(scalars);
        assert!(num_points == num_scalars, 1);
        let i = 0;
        while (i < num_points) {
            let scalar = std::vector::borrow(scalars, i);
            let point = std::vector::borrow(points, i);
            result = element_add(&result, &element_mul(scalar, point));
            i = i + 1;
        };
        result
    }

    /// Decode a `Scalar<G>` from a byte array.
    /// See the comments on the actual type `G` for the format details.
    public fun scalar_from_bytes<G>(bytes: &vector<u8>): Option<Scalar<G>> {
        abort_if_feature_disabled();
        let (succeeded, handle) = scalar_from_bytes_internal<G>(*bytes);
        if (succeeded) {
            let scalar = Scalar<G> {
                handle
            };
            std::option::some(scalar)
        } else {
            std::option::none()
        }
    }

    /// Encode a `Scalar<G>` to a byte array.
    /// See the comments on the actual type `G` for the format details.
    public fun scalar_to_bytes<G>(scalar: &Scalar<G>): vector<u8> {
        abort_if_feature_disabled();
        scalar_to_bytes_internal<G>(scalar.handle)
    }

    /// Encode an `Element<G>` to a byte array with an uncompressed format.
    /// See the comments on the actual type `G` for the format details.
    public fun serialize_element_uncompressed<G>(point: &Element<G>): vector<u8> {
        abort_if_feature_disabled();
        serialize_element_uncompressed_internal<G>(point.handle)
    }

    /// Encode an `Element<G>` to a byte array with a compressed format.
    /// See the comments on the actual type `G` for the format details.
    public fun serialize_element_compressed<G>(point: &Element<G>): vector<u8> {
        abort_if_feature_disabled();
        serialize_element_compressed_internal<G>(point.handle)
    }

    /// Decode an `Element<G>` from a byte array with an uncompressed format.
    /// See the comments on the actual type `G` for the format details.
    public fun deserialize_element_uncompressed<G>(bytes: vector<u8>): Option<Element<G>> {
        abort_if_feature_disabled();
        let (succ, handle) = deserialize_element_uncompressed_internal<G>(bytes);
        if (succ) {
            std::option::some(Element<G> { handle })
        } else {
            std::option::none()
        }
    }

    /// Decode an `Element<G>` from a byte array with a compressed format.
    /// See the comments on the actual type `G` for the format details.
    public fun deserialize_element_compressed<G>(bytes: vector<u8>): Option<Element<G>> {
        abort_if_feature_disabled();
        let (succ, handle) = deserialize_element_compressed_internal<G>(bytes);
        if (succ) {
            std::option::some(Element<G> { handle })
        } else {
            std::option::none()
        }
    }

    public fun element_eq<G>(point_1: &Element<G>, point_2: &Element<G>): bool {
        abort_if_feature_disabled();
        element_eq_internal<G>(point_1.handle, point_2.handle)
    }

    fun abort_if_feature_disabled() {
        if (!std::features::generic_curves_enabled()) {
            abort(std::error::invalid_state(E_NATIVE_FUN_NOT_AVAILABLE))
        };
    }
    // Native functions.
    native fun deserialize_element_uncompressed_internal<G>(bytes: vector<u8>): (bool, u64);
    native fun deserialize_element_compressed_internal<G>(bytes: vector<u8>): (bool, u64);
    native fun scalar_from_u64_internal<G>(value: u64): u64;
    native fun scalar_from_bytes_internal<G>(bytes: vector<u8>): (bool, u64);
    native fun scalar_neg_internal<G>(handle: u64): u64;
    native fun scalar_add_internal<G>(handle_1: u64, handle_2: u64): u64;
    native fun scalar_mul_internal<G>(handle_1: u64, handle_2: u64): u64;
    native fun scalar_inv_internal<G>(handle: u64): (bool, u64);
    native fun scalar_eq_internal<G>(handle_1: u64, handle_2: u64): bool;
    native fun scalar_to_bytes_internal<G>(h: u64): vector<u8>;
    native fun element_add_internal<G>(handle_1: u64, handle_2: u64): u64;
    native fun element_eq_internal<G>(handle_1: u64, handle_2: u64): bool;
    native fun identity_internal<G>(): u64;
    native fun generator_internal<G>(): u64;
    native fun element_mul_internal<G>(scalar_handle: u64, point_handle: u64): u64;
    native fun element_neg_internal<G>(handle: u64): u64;
    native fun serialize_element_uncompressed_internal<G>(handle: u64): vector<u8>;
    native fun serialize_element_compressed_internal<G>(handle: u64): vector<u8>;
    native fun pairing_internal<G1,G2,Gt>(g1_handle: u64, g2_handle: u64): u64;
    ///TODO: Remove `g1_handle_count` and `g2_handle_count` once working with `vector<u64>` in rust is well supported.
    native fun multi_pairing_internal<G1,G2,Gt>(g1_handle_count: u64, g1_handles: vector<u64>, g2_handle_count: u64, g2_handles: vector<u64>): u64;

    #[test(fx = @std)]
    fun test_bls12_381_g1(fx: signer) {
        std::features::change_feature_flags(&fx, vector[std::features::get_generic_curves_feature()], vector[]);
        // Scalar encoding/decoding.
        let scalar_7 = scalar_from_u64<BLS12_381_G1>(7);
        let scalar_7_another = std::option::extract(&mut scalar_from_bytes<BLS12_381_G1>(&x"0700000000000000000000000000000000000000000000000000000000000000"));
        assert!(scalar_eq(&scalar_7, &scalar_7_another), 1);
        assert!( x"0700000000000000000000000000000000000000000000000000000000000000" == scalar_to_bytes(&scalar_7), 1);
        assert!(std::option::is_none(&scalar_from_bytes<BLS12_381_G1>(&x"ffff")), 1);

        // Scalar negation.
        let scalar_minus_7 = scalar_neg(&scalar_7);
        assert!(x"fafffffffefffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73" == scalar_to_bytes(&scalar_minus_7), 1);

        // Scalar addition.
        let scalar_9 = scalar_from_u64<BLS12_381_G1>(9);
        let scalar_2 = scalar_from_u64<BLS12_381_G1>(2);
        let scalar_2_calc = scalar_add(&scalar_minus_7, &scalar_9);
        assert!(scalar_eq(&scalar_2, &scalar_2_calc), 1);

        // Scalar multiplication.
        let scalar_63_calc = scalar_mul(&scalar_7, &scalar_9);
        let scalar_63 = scalar_from_u64<BLS12_381_G1>(63);
        assert!(scalar_eq(&scalar_63, &scalar_63_calc), 1);

        // Scalar inversion.
        let scalar_7_inv_calc = std::option::extract(&mut scalar_inv(&scalar_7));
        assert!(scalar_eq(&scalar_9, &scalar_mul(&scalar_63, &scalar_7_inv_calc)), 1);
        let scalar_0 = scalar_from_u64<BLS12_381_G1>(0);
        assert!(std::option::is_none(&scalar_inv(&scalar_0)), 1);

        // Point encoding/decoding.
        let point_g = generator<BLS12_381_G1>();
        assert!(x"bbc622db0af03afbef1a7af93fe8556c58ac1b173f3a4ea105b974974f8c68c30faca94f8c63952694d79731a7d3f117e1e7c5462923aa0ce48a88a244c73cd0edb3042ccb18db00f60ad0d595e0f5fce48a1d74ed309ea0f1a0aae381f4b308" == serialize_element_uncompressed(&point_g), 1);
        assert!(x"bbc622db0af03afbef1a7af93fe8556c58ac1b173f3a4ea105b974974f8c68c30faca94f8c63952694d79731a7d3f117" == serialize_element_compressed(&point_g), 1);
        let point_g_from_uncomp = std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G1>(x"bbc622db0af03afbef1a7af93fe8556c58ac1b173f3a4ea105b974974f8c68c30faca94f8c63952694d79731a7d3f117e1e7c5462923aa0ce48a88a244c73cd0edb3042ccb18db00f60ad0d595e0f5fce48a1d74ed309ea0f1a0aae381f4b308"));
        let point_g_from_comp = std::option::extract(&mut deserialize_element_compressed<BLS12_381_G1>(x"bbc622db0af03afbef1a7af93fe8556c58ac1b173f3a4ea105b974974f8c68c30faca94f8c63952694d79731a7d3f117"));
        assert!(element_eq(&point_g, &point_g_from_comp), 1);
        assert!(element_eq(&point_g, &point_g_from_uncomp), 1);
        let inf = identity<BLS12_381_G1>();
        assert!(x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040" == serialize_element_uncompressed(&inf), 1);
        assert!(x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040" == serialize_element_compressed(&inf), 1);
        let inf_from_uncomp = std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G1>(x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040"));
        let inf_from_comp = std::option::extract(&mut deserialize_element_compressed<BLS12_381_G1>(x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040"));
        assert!(element_eq(&inf, &inf_from_comp), 1);
        assert!(element_eq(&inf, &inf_from_uncomp), 1);
        let point_7g_from_uncomp = std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G1>(x"b7fc7e62705aef542dbcc5d4bce62a7bf22eef1691bef30dac121fb200ca7dc9a4403b90da4501cfee1935b9bef328191c1a98287eec115a8cb0a1cf4968c6fd101ca4593938d73918dd8e81471d8a3ac4b38930aed539564436b6a4baad8d10"));
        let point_7g_from_comp = std::option::extract(&mut deserialize_element_compressed<BLS12_381_G1>(x"b7fc7e62705aef542dbcc5d4bce62a7bf22eef1691bef30dac121fb200ca7dc9a4403b90da4501cfee1935b9bef32899"));
        assert!(element_eq(&point_7g_from_comp, &point_7g_from_uncomp), 1);

        // Point multiplication by scalar.
        let point_7g_calc = element_mul(&scalar_7, &point_g);
        assert!(element_eq(&point_7g_calc, &point_7g_from_comp), 1);
        assert!(x"b7fc7e62705aef542dbcc5d4bce62a7bf22eef1691bef30dac121fb200ca7dc9a4403b90da4501cfee1935b9bef328191c1a98287eec115a8cb0a1cf4968c6fd101ca4593938d73918dd8e81471d8a3ac4b38930aed539564436b6a4baad8d10" == serialize_element_uncompressed(&point_7g_calc), 1);
        assert!(x"b7fc7e62705aef542dbcc5d4bce62a7bf22eef1691bef30dac121fb200ca7dc9a4403b90da4501cfee1935b9bef32899" == serialize_element_compressed(&point_7g_calc), 1);

        // Point negation.
        let point_minus_7g_calc = element_neg(&point_7g_calc);
        assert!(x"b7fc7e62705aef542dbcc5d4bce62a7bf22eef1691bef30dac121fb200ca7dc9a4403b90da4501cfee1935b9bef32819" == serialize_element_compressed(&point_minus_7g_calc), 1);
        assert!(x"b7fc7e62705aef542dbcc5d4bce62a7bf22eef1691bef30dac121fb200ca7dc9a4403b90da4501cfee1935b9bef328198f9067d78113ed5f734fb2e1b497e52013da0c9d679a592da735f6713d2eed2913f9c11208d2e1f455b0c9942f647309" == serialize_element_uncompressed(&point_minus_7g_calc), 1);

        // Point addition.
        let point_9g = element_mul(&scalar_9, &point_g);
        let point_2g = element_mul(&scalar_2, &point_g);
        let point_2g_calc = element_add(&point_minus_7g_calc, &point_9g);
        assert!(element_eq(&point_2g, &point_2g_calc), 1);

        // Simultaneous point multiplication.
        let point_14g = element_mul(&scalar_from_u64<BLS12_381_G1>(14), &point_g);
        let scalar_1 = scalar_from_u64<BLS12_381_G1>(1);
        let scalar_2 = scalar_from_u64<BLS12_381_G1>(2);
        let scalar_3 = scalar_from_u64<BLS12_381_G1>(3);
        let point_2g = element_mul(&scalar_2, &point_g);
        let point_3g = element_mul(&scalar_3, &point_g);
        let scalars = vector[scalar_1, scalar_2, scalar_3];
        let points = vector[point_g, point_2g, point_3g];
        let point_14g_calc = simul_point_mul(&scalars, &points);
        assert!(element_eq(&point_14g, &point_14g_calc), 1);
    }

    #[test(fx = @std)]
    fun test_bls12_381_g2(fx: signer) {
        std::features::change_feature_flags(&fx, vector[std::features::get_generic_curves_feature()], vector[]);
        //TODO
    }

    #[test(fx = @std)]
    fun test_bls12_381_gt(fx: signer) {
        std::features::change_feature_flags(&fx, vector[std::features::get_generic_curves_feature()], vector[]);
        //TODO
    }

    #[test(fx = @std)]
    fun test_bls12381_pairing(fx: signer) {
        std::features::change_feature_flags(&fx, vector[std::features::get_generic_curves_feature()], vector[]);
        let gt_point_1 = pairing<BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt>(
            &element_mul(&scalar_from_u64(5), &generator<BLS12_381_G1>()),
            &element_mul(&scalar_from_u64(7), &generator<BLS12_381_G2>()),
        );
        let gt_point_2 = pairing<BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt>(
            &element_mul(&scalar_from_u64(1), &generator()),
            &element_mul(&scalar_from_u64(35), &generator()),
        );
        let gt_point_3 = pairing<BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt>(
            &element_mul(&scalar_from_u64(35), &generator<BLS12_381_G1>()),
            &element_mul(&scalar_from_u64(1), &generator<BLS12_381_G2>()),
        );
        assert!(element_eq(&gt_point_1, &gt_point_2), 1);
        assert!(element_eq(&gt_point_1, &gt_point_3), 1);
    }

    #[test(fx = @std)]
    fun test_bls12381_multi_pairing(fx: signer) {
        std::features::change_feature_flags(&fx, vector[std::features::get_generic_curves_feature()], vector[]);
        let g1_point_1 = generator<BLS12_381_G1>();
        let g2_point_1 = generator<BLS12_381_G2>();
        let g1_point_2 = element_mul(&scalar_from_u64<BLS12_381_G1>(5), &g1_point_1);
        let g2_point_2 = element_mul(&scalar_from_u64<BLS12_381_G2>(2), &g2_point_1);
        let g1_point_3 = element_mul(&scalar_from_u64<BLS12_381_G1>(20), &g1_point_1);
        let g2_point_3 = element_mul(&scalar_from_u64<BLS12_381_G2>(5), &g2_point_1);
        let expected = element_mul(&scalar_from_u64<BLS12_381_Gt>(111), &pairing<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(&g1_point_1, &g2_point_1));
        let actual = multi_pairing<BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt>(&vector[g1_point_1, g1_point_2, g1_point_3], &vector[g2_point_1, g2_point_2, g2_point_3]);
        assert!(element_eq(&expected, &actual), 1);
    }
}

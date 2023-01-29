module aptos_std::groups {
    use std::option::Option;
    use aptos_std::type_info::type_of;

    // Error codes
    const E_NOT_IMPLEMENTED: u64 = 2;

    /// `BLS12_381_G1` represents a group used in BLS12-381 pairing.
    /// `Fq` is a finite field with `q=0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab`.
    /// `E(Fq)` is an elliptic curve `y^2=x^3+4` defined over `Fq`.
    /// `BLS12_381_G1` is constructed by a subset of the points on `E(Fq)` and the point at infinity, under point addition. (A subgroup of prime order on `E(Fq)`.)
    /// The prime order `r` of `BLS12_381_G1` is 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.
    /// The identity of `BLS12_381_G1` is the point at infinity.
    /// There exists a bilinear mapping from `(BLS12_381_G1, BLS12_381_G2)` to `BLS12_381_Gt`.
    ///
    /// A `Scalar<BLS12_381_G1>` is an integer between 0 and `r-1`.
    ///
    /// An `Element<BLS12_381_G1>` represents an element in group `BLS12_381_G1`.
    /// Scalar multiplication on `Element<BLS12_381_G1>` requires a `Scalar<BLS12_381_Fr>`.
    ///
    /// Functions `serialize_element_uncompressed<BLS12_381_G1>` and `deserialize_element_uncompressed<BLS12_381_G1>`
    /// assume a 96-byte encoding `[b_0, ..., b_95]` for `Element<BLS12_381_G1>`, with the following rules.
    /// - `b_95 & 0x40` is the infinity flag.
    /// - The infinity flag is 1 if and only if the element is the point at infinity.
    /// - The infinity flag is 0 if and only if the element is a point `(x,y)` on curve `E(Fq)`, with the following rules.
    ///     - `[b_0, ..., b_47 & 0x3f]` is a 48-byte little-endian encoding of `x`.
    ///     - `[b_48, ..., b_95 & 0x3f]` is a 48-byte little-endian encoding of 'y'.
    ///
    /// Functions `serialize_element_compressed<BLS12_381_G1>` and `deserialize_element_compressed<BLS12_381_G1>`
    /// assume a 48-byte encoding `[b_0, ..., b_47]` of an `Element<BLS12_381_G1>` with the following rules.
    /// - `b_47 & 0x40` is the infinity flag.
    /// - The infinity flag is 1 if and only if the element is the point at infinity.
    /// - The infinity flag is 0 if and only if the element is a point `(x,y)` on curve, with the following rules.
    ///     - `[b_0, ..., b_47 & 0x3f]` is a 48-byte little-endian encoding of `x`.
    ///     - `b_47 & 0x80` is the positiveness flag.
    ///     - The positiveness flag is 1 if and only if `y > -y`.
    struct BLS12_381_G1 {}

    /// `BLS12_381_G2` represents a group used in BLS12-381 pairing.
    /// `Fq` is a finite field with `q=0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab`.
    /// `Fq2` is an extension field of `Fq`, constructed as `Fq2=Fq[u]/(u^2+1)`.
    /// `E(Fq2)` is an elliptic curve `y^2=x^3+4(u+1)` defined over `Fq2`.
    /// `BLS12_381_G2` is constructed by a subset of the points on `E(Fq2)` and the point at infinity, under point addition. (A subgroup of prime order on `E(Fq2)`.)
    /// The prime order `r` of `BLS12_381_G2` is 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001, same as `BLS12_381_G1`.
    /// The identity of `BLS12_381_G2` is the point at infinity.
    /// There exists a bilinear mapping from `(BLS12_381_G1, BLS12_381_G2)` to `BLS12_381_Gt`.
    ///
    /// An `Element<BLS12_381_G2>` is an element in group `BLS12_381_G2`.
    /// Scalar multiplication on `Element<BLS12_381_G2>` requires a `Scalar<BLS12_381_Fr>`.
    ///
    /// Functions `serialize_element_uncompressed<BLS12_381_G2>` and `deserialize_element_uncompressed<BLS12_381_G2>`
    /// assume a 192-byte encoding `[b_0, ..., b_191]` of an `Element<BLS12_381_G2>`, with the following rules.
    /// - `b_191 & 0x40` is the infinity flag.
    /// - The infinity flag is 1 if and only if the element is the point at infinity.
    /// - The infinity flag is 0 if and only if the element is a point `(x,y)` on curve `E(Fq2)`, with the following rules.
    ///     - `[b_0, ..., b_95]` is a 96-byte encoding of `x=(x_0+x_1*u)`.
    ///         - `[b_0, ..., b_47]` is a 48-byte little-endian encoding of `x_0`.
    ///         - `[b_48, ..., b_95]` is a 48-byte little-endian encoding of `x_1`.
    ///     - `[b_96, ..., b_191 & 0x3f]` is a 96-byte encoding of 'y=(y_0+y_1*u)'.
    ///         - `[b_96, ..., b_143]` is a 48-byte little-endian encoding of `y_0`.
    ///         - `[b_144, ..., b_191 & 0x3f]` is a 48-byte little-endian encoding of `y_1`.
    ///
    /// Functions `serialize_element_compressed<BLS12_381_G2>` and `deserialize_element_compressed<BLS12_381_G2>`
    /// assume a 96-byte encoding `[b_0, ..., b_95]` of an `Element<BLS12_381_G2>` with the following rules.
    /// - `b_95 & 0x40` is the infinity flag.
    /// - The infinity flag is 1 if and only if the element is the point at infinity.
    /// - The infinity flag is 0 if and only if the element is a point `(x,y)` on curve `E(Fq2)`, with the following rules.
    ///     - `[b_0, ..., b_95 & 0x3f]` is a 96-byte little-endian encoding of `x=(x_0+x_1*u)`.
    ///         - `[b_0, ..., b_47]` is a 48-byte little-endian encoding of `x_0`.
    ///         - `[b_48, ..., b_95 & 0x3f]` is a 48-byte little-endian encoding of `x_1`.
    ///     - `b_95 & 0x80` is the positiveness flag.
    ///     - The positiveness flag is 1 if and only if `y > -y`.
    ///         - Here `a=(a_0+a_1*u)` is considered greater than `b=(b_0+b_1*u)` if `a_1>b_1 OR (a_1=b_1 AND a_0>b_0)`.
    struct BLS12_381_G2 {}

    /// `BLS12_381_Gt` represents the target group of the pairing defined over the BLS12-381 curves.
    /// `Fq` is a finite field with `q=0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab`.
    /// `Fq2` is an extension field of `Fq`, constructed as `Fq2=Fq[u]/(u^2+1)`.
    /// `Fq6` is an extension field of `Fq2`, constructed as `Fq6=Fq2[v]/(v^2-u-1)`.
    /// `Fq12` is an extension field of `Fq6`, constructed as `Fq12=Fq6[w]/(w^2-v)`.
    /// `BLS12_381_Gt` is a multiplicative subgroup of `Fq12`.
    /// The order `r` of `BLS12_381_Gt` is 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001. (Same as `BLS12_381_G1` and `BLS12_381_G2`.)
    /// The identity of `BLS12_381_G2` is 1.
    /// There exists a bilinear mapping from `(BLS12_381_G1, BLS12_381_G2)` to `BLS12_381_Gt`.
    ///
    /// An `Element<BLS12_381_Gt>` is an element in group `BLS12_381_Gt`.
    /// Scalar multiplication on `Element<BLS12_381_Gt>` requires a `Scalar<BLS12_381_Fr>`.
    ///
    /// Functions `serialize_element_uncompressed<BLS12_381_Gt>` and `deserialize_element_uncompressed<BLS12_381_Gt>`,
    /// as well as `serialize_element_ompressed<BLS12_381_Gt>` and `deserialize_element_compressed<BLS12_381_Gt>`,
    /// assume a 576-byte encoding `[b_0, ..., b_575]` of an `Element<BLS12_381_Gt>`, with the following rules.
    ///     - Assume the given element is `e=c_0+c_1*w` where `c_i=c_i0+c_i1*v+c_i2*v^2 for i=0..1` and `c_ij=c_ij0+c_ij1*u for j=0..2`.
    ///     - `[b_0, ..., b_575]` is a concatenation of 12 encoded `Fq` elements: `c_000, c_001, c_010, c_011, c_020, c_021, c_100, c_101, c_110, c_111, c_120, c_121`.
    ///     - Every `c_ijk` uses a 48-byte little-endian encoding.
    struct BLS12_381_Gt {}

    /// The scalar field for groups `BLS12_381_G1`, `BLS12_381_G2` and `BLS12_381_Gt`.
    /// A `Scalar<BLS12_381_Fr>` is an integer between 0 and `r-1` where `r` is the order of `BLS12_381_G1`/`BLS12_381_G2`/`BLS12_381_Gt`.
    /// Functions `deserialize_scalar<BLS12_381_Fr>` and `serialize_scalar<BLS12_381_Fr>`
    /// assume a 32-byte little-endian encoding of a `Scalar<BLS12_381_Gt>`.
    struct BLS12_381_Fr {}

    struct SHA256 {}

    /// This struct represents an integer between 0 and `r-1`, where `r` is the order of group `G`.
    struct Scalar<phantom G> has copy, drop {
        handle: u64
    }

    /// This struct represents an element of the group `G`, where `G` is a type argument.
    struct Element<phantom G> has copy, drop {
        handle: u64
    }

    /// Computes a pairing function (a.k.a., bilinear map) on `element_1` and `element_2`.
    /// Returns an element in the target group `Gt`.
    public fun pairing<G1,G2,Gt>(element_1: &Element<G1>, element_2: &Element<G2>): Element<Gt> {
        Element<Gt> {
            handle: pairing_product_internal<G1,G2,Gt>(vector[element_1.handle], vector[element_2.handle])
        }
    }

    /// Compute the product of multiple pairings.
    public fun pairing_product<G1, G2, Gt>(g1_elements: &vector<Element<G1>>, g2_elements: &vector<Element<G2>>): Element<Gt> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G1>();
        abort_unless_structure_enabled<G2>();
        abort_unless_structure_enabled<Gt>();
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
            handle: pairing_product_internal<G1,G2,Gt>(g1_handles, g2_handles)
        }
    }

    /// Convert a u64 to a scalar.
    public fun scalar_from_u64<S>(value: u64): Scalar<S> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<S>();
        Scalar<S> {
            handle: scalar_from_u64_internal<S>(value)
        }
    }

    /// Compute `-x` for scalar `x`.
    public fun scalar_neg<S>(x: &Scalar<S>): Scalar<S> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<S>();
        Scalar<S> {
            handle: scalar_neg_internal<S>(x.handle)
        }
    }

    /// Compute `x + y` for scalars `x` and `y`.
    public fun scalar_add<S>(x: &Scalar<S>, y: &Scalar<S>): Scalar<S> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<S>();
        Scalar<S> {
            handle: scalar_add_internal<S>(x.handle, y.handle)
        }
    }

    /// Compute `x * y` for scalars `x` and `y`.
    public fun scalar_mul<S>(x: &Scalar<S>, y: &Scalar<S>): Scalar<S> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<S>();
        Scalar<S> {
            handle: scalar_mul_internal<S>(x.handle, y.handle)
        }
    }

    /// Compute `x^(-1)` for scalar `x`, if defined.
    public fun scalar_inv<S>(x: &Scalar<S>): Option<Scalar<S>> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<S>();
        let (succeeded, handle) = scalar_inv_internal<S>(x.handle);
        if (succeeded) {
            let scalar = Scalar<S> { handle };
            std::option::some(scalar)
        } else {
            std::option::none()
        }
    }

    /// Check if `x == y` for scalars `x` and `y`.
    public fun scalar_eq<S>(x: &Scalar<S>, y: &Scalar<S>): bool {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<S>();
        scalar_eq_internal<S>(x.handle, y.handle)
    }

    /// Get the identity of group `G`.
    public fun group_identity<G>(): Element<G> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G>();
        Element<G> {
            handle: group_identity_internal<G>()
        }
    }

    /// Get the generator of group `G`.
    public fun group_generator<G>(): Element<G> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G>();
        Element<G> {
            handle: group_generator_internal<G>()
        }
    }

    /// Compute `-P` for group element `P`.
    public fun element_neg<G>(element_p: &Element<G>): Element<G> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G>();
        Element<G> {
            handle: element_neg_internal<G>(element_p.handle)
        }
    }

    /// Compute `P + Q` for group element `P` and `Q`.
    public fun element_add<G>(element_p: &Element<G>, element_q: &Element<G>): Element<G> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G>();
        Element<G> {
            handle: element_add_internal<G>(element_p.handle, element_q.handle)
        }
    }

    /// Compute `2P` for group element `P`.
    public fun element_double<G>(element_p: &Element<G>): Element<G> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G>();
        Element<G> {
            handle: element_double_internal<G>(element_p.handle)
        }
    }

    /// Compute `k*P` for scalar `k` and group element `P`.
    public fun element_scalar_mul<G, S>(element_p: &Element<G>, scalar_k: &Scalar<S>): Element<G> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G>();
        abort_unless_structure_enabled<S>();
        Element<G> {
            handle: element_mul_internal<G, S>(element_p.handle, scalar_k.handle)
        }
    }

    /// Hash bytes to a group element.
    public fun hash_to_element<H, G>(bytes: vector<u8>): Element<G> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_hash_alg_enabled<H>();
        abort_unless_structure_enabled<G>();
        Element<G> {
            handle: hash_to_element_internal<H, G>(bytes)
        }
    }

    /// Compute `k[0]*P[0]+...+k[n-1]*P[n-1]` for a list of scalars `k[]` and a list of group elements `P[]`, both of size `n`.
    /// This function is much faster and cheaper than calling `element_scalar_mul` and adding up the results using `scalar_add`.
    public fun element_multi_scalar_mul<G, S>(elements: &vector<Element<G>>, scalars: &vector<Scalar<S>>): Element<G> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G>();
        abort_unless_structure_enabled<S>();
        let num_scalars = std::vector::length(scalars);
        let scalar_handles = vector[];
        let i = 0;
        while (i < num_scalars) {
            std::vector::push_back(&mut scalar_handles, std::vector::borrow(scalars, i).handle);
            i = i + 1;
        };

        let num_elements = std::vector::length(elements);
        let element_handles = vector[];
        let i = 0;
        while (i < num_elements) {
            std::vector::push_back(&mut element_handles, std::vector::borrow(elements, i).handle);
            i = i + 1;
        };

        Element<G> {
            handle: element_multi_scalar_mul_internal<G, S>(element_handles, scalar_handles)
        }

    }

    /// Scalar deserialization.
    public fun scalar_deserialize<S>(bytes: &vector<u8>): Option<Scalar<S>> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<S>();
        let (succeeded, handle) = scalar_deserialize_internal<S>(*bytes);
        if (succeeded) {
            let scalar = Scalar<S> {
                handle
            };
            std::option::some(scalar)
        } else {
            std::option::none()
        }
    }

    /// Scalar serialization.
    public fun scalar_serialize<S>(scalar: &Scalar<S>): vector<u8> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<S>();
        scalar_serialize_internal<S>(scalar.handle)
    }

    /// Group element serialization with an uncompressed format.
    public fun serialize_element_uncompressed<G>(element: &Element<G>): vector<u8> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G>();
        element_serialize_uncompressed_internal<G>(element.handle)
    }

    /// Group element serialization with a compressed format.
    public fun serialize_element_compressed<G>(element: &Element<G>): vector<u8> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G>();
        element_serialize_compressed_internal<G>(element.handle)
    }

    /// Group element deserialization with an uncompressed format.
    public fun deserialize_element_uncompressed<G>(bytes: vector<u8>): Option<Element<G>> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G>();
        let (succ, handle) = element_deserialize_uncompressed_internal<G>(bytes);
        if (succ) {
            std::option::some(Element<G> { handle })
        } else {
            std::option::none()
        }
    }

    /// Group element deserialization with a compressed format.
    public fun deserialize_element_compressed<G>(bytes: vector<u8>): Option<Element<G>> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G>();
        let (succ, handle) = element_deserialize_compressed_internal<G>(bytes);
        if (succ) {
            std::option::some(Element<G> { handle })
        } else {
            std::option::none()
        }
    }

    /// Check if `P == Q` for group elements `P` and `Q`.
    public fun element_eq<G>(element_p: &Element<G>, element_q: &Element<G>): bool {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G>();
        element_eq_internal<G>(element_p.handle, element_q.handle)
    }

    /// Get the order of group `G`, little-endian encoded as a byte array.
    public fun group_order<G>(): vector<u8> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G>();
        group_order_internal<G>()
    }

    #[test_only]
    /// Generate a random group element.
    public fun random_element<G>(): Element<G> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G>();
        Element<G> {
            handle: random_element_internal<G>()
        }
    }

    #[test_only]
    /// Generate a random scalar.
    public fun random_scalar<S>(): Scalar<S> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<S>();
        Scalar<S> {
            handle: random_scalar_internal<S>()
        }
    }

    fun abort_if_generic_group_basic_operations_disabled() {
        if (!std::features::generic_group_basic_operations_enabled()) {
            abort(std::error::not_implemented(0))
        }
    }

    fun abort_unless_structure_enabled<S>() {
        let type = type_of<S>();
        if ((type == type_of<BLS12_381_G1>() || type == type_of<BLS12_381_G2>() || type == type_of<BLS12_381_Gt>() || type == type_of<BLS12_381_Fr>())
            && std::features::bls12_381_groups_enabled()
        ) {
            // Let go.
        } else {
            abort(std::error::not_implemented(0))
        }
    }

    fun abort_unless_hash_alg_enabled<S>() {
        let type = type_of<S>();
        if (type == type_of<SHA256>() && std::features::sha256_to_group_enabled()) {
            // Let go.
        } else {
            abort(std::error::not_implemented(0))
        }
    }

    // Native functions.
    native fun element_deserialize_uncompressed_internal<G>(bytes: vector<u8>): (bool, u64);
    native fun element_deserialize_compressed_internal<G>(bytes: vector<u8>): (bool, u64);
    native fun scalar_from_u64_internal<G>(value: u64): u64;
    native fun scalar_deserialize_internal<G>(bytes: vector<u8>): (bool, u64);
    native fun scalar_neg_internal<G>(handle: u64): u64;
    native fun scalar_add_internal<G>(handle_1: u64, handle_2: u64): u64;
    native fun scalar_double_internal<G>(handle: u64): u64;
    native fun scalar_mul_internal<G>(handle_1: u64, handle_2: u64): u64;
    native fun scalar_inv_internal<G>(handle: u64): (bool, u64);
    native fun scalar_eq_internal<G>(handle_1: u64, handle_2: u64): bool;
    native fun scalar_serialize_internal<G>(h: u64): vector<u8>;
    native fun element_add_internal<G>(handle_1: u64, handle_2: u64): u64;
    native fun element_eq_internal<G>(handle_1: u64, handle_2: u64): bool;
    native fun group_identity_internal<G>(): u64;
    native fun group_order_internal<G>(): vector<u8>;
    native fun group_generator_internal<G>(): u64;
    native fun element_mul_internal<G, S>(scalar_handle: u64, element_handle: u64): u64;
    native fun element_double_internal<G>(element_handle: u64): u64;
    native fun element_neg_internal<G>(handle: u64): u64;
    native fun element_serialize_uncompressed_internal<G>(handle: u64): vector<u8>;
    native fun element_serialize_compressed_internal<G>(handle: u64): vector<u8>;
    native fun element_multi_scalar_mul_internal<G, S>(element_handles: vector<u64>, scalar_handles: vector<u64>): u64;
    native fun pairing_product_internal<G1,G2,Gt>(g1_handles: vector<u64>, g2_handles: vector<u64>): u64;
    native fun hash_to_element_internal<H, G>(bytes: vector<u8>): u64;
    #[test_only]
    native fun random_element_internal<G>(): u64;
    #[test_only]
    native fun random_scalar_internal<G>(): u64;

    #[test(fx = @std)]
    fun test_bls12_381_fr(fx: signer) {
        std::features::change_feature_flags(&fx, vector[std::features::get_generic_group_basic_operations_feature(), std::features::get_bls12_381_groups_feature()], vector[]);
        // Scalar encoding/decoding.
        let scalar_7 = scalar_from_u64<BLS12_381_Fr>(7);
        let scalar_7_another = std::option::extract(&mut scalar_deserialize<BLS12_381_Fr>(&x"0700000000000000000000000000000000000000000000000000000000000000"));
        assert!(scalar_eq(&scalar_7, &scalar_7_another), 1);
        assert!( x"0700000000000000000000000000000000000000000000000000000000000000" == scalar_serialize(&scalar_7), 1);
        assert!(std::option::is_none(&scalar_deserialize<BLS12_381_Fr>(&x"ffff")), 1);

        // Scalar negation.
        let scalar_minus_7 = scalar_neg(&scalar_7);
        assert!(x"fafffffffefffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73" == scalar_serialize(&scalar_minus_7), 1);

        // Scalar addition.
        let scalar_9 = scalar_from_u64<BLS12_381_Fr>(9);
        let scalar_2 = scalar_from_u64<BLS12_381_Fr>(2);
        let scalar_2_calc = scalar_add(&scalar_minus_7, &scalar_9);
        assert!(scalar_eq(&scalar_2, &scalar_2_calc), 1);

        // Scalar multiplication.
        let scalar_63_calc = scalar_mul(&scalar_7, &scalar_9);
        let scalar_63 = scalar_from_u64<BLS12_381_Fr>(63);
        assert!(scalar_eq(&scalar_63, &scalar_63_calc), 1);

        // Scalar inversion.
        let scalar_7_inv_calc = std::option::extract(&mut scalar_inv(&scalar_7));
        assert!(scalar_eq(&scalar_9, &scalar_mul(&scalar_63, &scalar_7_inv_calc)), 1);
        let scalar_0 = scalar_from_u64<BLS12_381_Fr>(0);
        assert!(std::option::is_none(&scalar_inv(&scalar_0)), 1);
    }

    #[test(fx = @std)]
    fun test_bls12_381_g1(fx: signer) {
        std::features::change_feature_flags(&fx, vector[std::features::get_generic_group_basic_operations_feature(), std::features::get_bls12_381_groups_feature(), std::features::get_sha256_to_group_feature()], vector[]);
        // Group info.
        assert!(x"01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73" == group_order<BLS12_381_G1>(), 1);

        // Point encoding/decoding.
        let point_g = group_generator<BLS12_381_G1>();
        assert!(x"bbc622db0af03afbef1a7af93fe8556c58ac1b173f3a4ea105b974974f8c68c30faca94f8c63952694d79731a7d3f117e1e7c5462923aa0ce48a88a244c73cd0edb3042ccb18db00f60ad0d595e0f5fce48a1d74ed309ea0f1a0aae381f4b308" == serialize_element_uncompressed(&point_g), 1);
        assert!(x"bbc622db0af03afbef1a7af93fe8556c58ac1b173f3a4ea105b974974f8c68c30faca94f8c63952694d79731a7d3f117" == serialize_element_compressed(&point_g), 1);
        let point_g_from_uncomp = std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G1>(x"bbc622db0af03afbef1a7af93fe8556c58ac1b173f3a4ea105b974974f8c68c30faca94f8c63952694d79731a7d3f117e1e7c5462923aa0ce48a88a244c73cd0edb3042ccb18db00f60ad0d595e0f5fce48a1d74ed309ea0f1a0aae381f4b308"));
        let point_g_from_comp = std::option::extract(&mut deserialize_element_compressed<BLS12_381_G1>(x"bbc622db0af03afbef1a7af93fe8556c58ac1b173f3a4ea105b974974f8c68c30faca94f8c63952694d79731a7d3f117"));
        assert!(element_eq(&point_g, &point_g_from_comp), 1);
        assert!(element_eq(&point_g, &point_g_from_uncomp), 1);
        let inf = group_identity<BLS12_381_G1>();
        assert!(x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040" == serialize_element_uncompressed(&inf), 1);
        assert!(x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040" == serialize_element_compressed(&inf), 1);
        let inf_from_uncomp = std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G1>(x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040"));
        let inf_from_comp = std::option::extract(&mut deserialize_element_compressed<BLS12_381_G1>(x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040"));
        assert!(element_eq(&inf, &inf_from_comp), 1);
        assert!(element_eq(&inf, &inf_from_uncomp), 1);
        let point_7g_from_uncomp = std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G1>(x"b7fc7e62705aef542dbcc5d4bce62a7bf22eef1691bef30dac121fb200ca7dc9a4403b90da4501cfee1935b9bef328191c1a98287eec115a8cb0a1cf4968c6fd101ca4593938d73918dd8e81471d8a3ac4b38930aed539564436b6a4baad8d10"));
        let point_7g_from_comp = std::option::extract(&mut deserialize_element_compressed<BLS12_381_G1>(x"b7fc7e62705aef542dbcc5d4bce62a7bf22eef1691bef30dac121fb200ca7dc9a4403b90da4501cfee1935b9bef32899"));
        assert!(element_eq(&point_7g_from_comp, &point_7g_from_uncomp), 1);

        // Point scalar multiplication.
        let scalar_7 = scalar_from_u64<BLS12_381_Fr>(7);
        let point_7g_calc = element_scalar_mul(&point_g, &scalar_7);
        assert!(element_eq(&point_7g_calc, &point_7g_from_comp), 1);
        assert!(x"b7fc7e62705aef542dbcc5d4bce62a7bf22eef1691bef30dac121fb200ca7dc9a4403b90da4501cfee1935b9bef328191c1a98287eec115a8cb0a1cf4968c6fd101ca4593938d73918dd8e81471d8a3ac4b38930aed539564436b6a4baad8d10" == serialize_element_uncompressed(&point_7g_calc), 1);
        assert!(x"b7fc7e62705aef542dbcc5d4bce62a7bf22eef1691bef30dac121fb200ca7dc9a4403b90da4501cfee1935b9bef32899" == serialize_element_compressed(&point_7g_calc), 1);

        // Point double.
        let scalar_2 = scalar_from_u64<BLS12_381_Fr>(2);
        let point_2g = element_scalar_mul(&point_g, &scalar_2);
        let point_double_g = element_double(&point_g);
        assert!(element_eq(&point_2g, &point_double_g), 1);

        // Point negation.
        let point_minus_7g_calc = element_neg(&point_7g_calc);
        assert!(x"b7fc7e62705aef542dbcc5d4bce62a7bf22eef1691bef30dac121fb200ca7dc9a4403b90da4501cfee1935b9bef32819" == serialize_element_compressed(&point_minus_7g_calc), 1);
        assert!(x"b7fc7e62705aef542dbcc5d4bce62a7bf22eef1691bef30dac121fb200ca7dc9a4403b90da4501cfee1935b9bef328198f9067d78113ed5f734fb2e1b497e52013da0c9d679a592da735f6713d2eed2913f9c11208d2e1f455b0c9942f647309" == serialize_element_uncompressed(&point_minus_7g_calc), 1);

        // Point addition.
        let scalar_9 = scalar_from_u64<BLS12_381_Fr>(9);
        let point_9g = element_scalar_mul(&point_g, &scalar_9);
        let point_2g = element_scalar_mul(&point_g, &scalar_2);
        let point_2g_calc = element_add(&point_minus_7g_calc, &point_9g);
        assert!(element_eq(&point_2g, &point_2g_calc), 1);

        // Multi-scalar multiplication.
        let scalar_a = random_scalar<BLS12_381_Fr>();
        let scalar_b = random_scalar<BLS12_381_Fr>();
        let scalar_c = random_scalar<BLS12_381_Fr>();
        let point_p = random_element<BLS12_381_G1>();
        let point_q = random_element<BLS12_381_G1>();
        let point_r = random_element<BLS12_381_G1>();
        let naive = group_identity<BLS12_381_G1>();
        naive = element_add(&naive, &element_scalar_mul(&point_p, &scalar_a));
        naive = element_add(&naive, &element_scalar_mul(&point_q, &scalar_b));
        naive = element_add(&naive, &element_scalar_mul(&point_r, &scalar_c));
        let fast = element_multi_scalar_mul(&vector[point_p, point_q, point_r], &vector[scalar_a, scalar_b, scalar_c]);
        assert!(element_eq(&naive, &fast), 1);

        // Hash to group.
        let _point = hash_to_element<SHA256, BLS12_381_G1>(x"1234");
    }

    #[test(fx = @std)]
    fun test_bls12_381_g2(fx: signer) {
        std::features::change_feature_flags(&fx, vector[std::features::get_generic_group_basic_operations_feature(), std::features::get_bls12_381_groups_feature(), std::features::get_sha256_to_group_feature()], vector[]);
        // Group info.
        assert!(x"01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73" == group_order<BLS12_381_G2>(), 1);

        // Point encoding/decoding.
        let point_g = group_generator<BLS12_381_G2>();
        assert!(x"b8bd21c1c85680d4efbb05a82603ac0b77d1e37a640b51b4023b40fad47ae4c65110c52d27050826910a8ff0b2a24a027e2b045d057dace5575d941312f14c3349507fdcbb61dab51ab62099d0d06b59654f2788a0d3ac7d609f7152602be0130128b808865493e189a2ac3bccc93a922cd16051699a426da7d3bd8caa9bfdad1a352edac6cdc98c116e7d7227d5e50cbe795ff05f07a9aaa11dec5c270d373fab992e57ab927426af63a7857e283ecb998bc22bb0d2ac32cc34a72ea0c40606" == serialize_element_uncompressed(&point_g), 1);
        assert!(x"b8bd21c1c85680d4efbb05a82603ac0b77d1e37a640b51b4023b40fad47ae4c65110c52d27050826910a8ff0b2a24a027e2b045d057dace5575d941312f14c3349507fdcbb61dab51ab62099d0d06b59654f2788a0d3ac7d609f7152602be013" == serialize_element_compressed(&point_g), 1);
        let point_g_from_uncomp = std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G2>(x"b8bd21c1c85680d4efbb05a82603ac0b77d1e37a640b51b4023b40fad47ae4c65110c52d27050826910a8ff0b2a24a027e2b045d057dace5575d941312f14c3349507fdcbb61dab51ab62099d0d06b59654f2788a0d3ac7d609f7152602be0130128b808865493e189a2ac3bccc93a922cd16051699a426da7d3bd8caa9bfdad1a352edac6cdc98c116e7d7227d5e50cbe795ff05f07a9aaa11dec5c270d373fab992e57ab927426af63a7857e283ecb998bc22bb0d2ac32cc34a72ea0c40606"));
        let point_g_from_comp = std::option::extract(&mut deserialize_element_compressed<BLS12_381_G2>(x"b8bd21c1c85680d4efbb05a82603ac0b77d1e37a640b51b4023b40fad47ae4c65110c52d27050826910a8ff0b2a24a027e2b045d057dace5575d941312f14c3349507fdcbb61dab51ab62099d0d06b59654f2788a0d3ac7d609f7152602be013"));
        assert!(element_eq(&point_g, &point_g_from_comp), 1);
        assert!(element_eq(&point_g, &point_g_from_uncomp), 1);
        let inf = group_identity<BLS12_381_G2>();
        assert!(x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040" == serialize_element_uncompressed(&inf), 1);
        assert!(x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040" == serialize_element_compressed(&inf), 1);
        let inf_from_uncomp = std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G2>(x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040"));
        let inf_from_comp = std::option::extract(&mut deserialize_element_compressed<BLS12_381_G2>(x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040"));
        assert!(element_eq(&inf, &inf_from_comp), 1);
        assert!(element_eq(&inf, &inf_from_uncomp), 1);
        let point_7g_from_uncomp = std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_G2>(x"3c8dd3f68a360f9c5ba81fad2be3408bdc3070619bc7bf3794851bd623685a5036ef5f1388c0541e58c3d2b2dbd19c04c83472247446b1bdd44416ad1c1f929a3f01ed345be35b9b4ba20f17ccf2b5208e3dec8380d6b8c337ed31bff673020dddcc1399cdf852dab1e2c8dc3b0ce819362f3a12da56f37aee93d3881ca760e467942c92428864a6172c80bf4daeb7082070fa8e8937746ae82d57ec8b639977f8ceaef21a11375de52b02e145dc39021bf4cab7eeaa955688a1b75436f9ec05"));
        let point_7g_from_comp = std::option::extract(&mut deserialize_element_compressed<BLS12_381_G2>(x"3c8dd3f68a360f9c5ba81fad2be3408bdc3070619bc7bf3794851bd623685a5036ef5f1388c0541e58c3d2b2dbd19c04c83472247446b1bdd44416ad1c1f929a3f01ed345be35b9b4ba20f17ccf2b5208e3dec8380d6b8c337ed31bff673020d"));
        assert!(element_eq(&point_7g_from_comp, &point_7g_from_uncomp), 1);

        // Point scalar multiplication.
        let scalar_7 = scalar_from_u64<BLS12_381_Fr>(7);
        let point_7g_calc = element_scalar_mul(&point_g, &scalar_7);
        assert!(element_eq(&point_7g_calc, &point_7g_from_comp), 1);
        assert!(x"3c8dd3f68a360f9c5ba81fad2be3408bdc3070619bc7bf3794851bd623685a5036ef5f1388c0541e58c3d2b2dbd19c04c83472247446b1bdd44416ad1c1f929a3f01ed345be35b9b4ba20f17ccf2b5208e3dec8380d6b8c337ed31bff673020dddcc1399cdf852dab1e2c8dc3b0ce819362f3a12da56f37aee93d3881ca760e467942c92428864a6172c80bf4daeb7082070fa8e8937746ae82d57ec8b639977f8ceaef21a11375de52b02e145dc39021bf4cab7eeaa955688a1b75436f9ec05" == serialize_element_uncompressed(&point_7g_calc), 1);
        assert!(x"3c8dd3f68a360f9c5ba81fad2be3408bdc3070619bc7bf3794851bd623685a5036ef5f1388c0541e58c3d2b2dbd19c04c83472247446b1bdd44416ad1c1f929a3f01ed345be35b9b4ba20f17ccf2b5208e3dec8380d6b8c337ed31bff673020d" == serialize_element_compressed(&point_7g_calc), 1);

        // Point double.
        let scalar_2 = scalar_from_u64<BLS12_381_Fr>(2);
        let point_2g = element_scalar_mul(&point_g, &scalar_2);
        let point_double_g = element_double(&point_g);
        assert!(element_eq(&point_2g, &point_double_g), 1);

        // Point negation.
        let point_minus_7g_calc = element_neg(&point_7g_calc);
        assert!(x"3c8dd3f68a360f9c5ba81fad2be3408bdc3070619bc7bf3794851bd623685a5036ef5f1388c0541e58c3d2b2dbd19c04c83472247446b1bdd44416ad1c1f929a3f01ed345be35b9b4ba20f17ccf2b5208e3dec8380d6b8c337ed31bff673028d" == serialize_element_compressed(&point_minus_7g_calc), 1);
        assert!(x"3c8dd3f68a360f9c5ba81fad2be3408bdc3070619bc7bf3794851bd623685a5036ef5f1388c0541e58c3d2b2dbd19c04c83472247446b1bdd44416ad1c1f929a3f01ed345be35b9b4ba20f17ccf2b5208e3dec8380d6b8c337ed31bff673020dceddeb663207acdf4d1d8bd4c2f3c304eec676e4c67b3decd07eb16a68a416806f181fb1731fb7a482baff799c6349118b3a057176c88a4f17d2fcc4729c12a72b27020486c1f909dae682123f6f3d62bcb8808bc7fc85f41145c8e4b3181414" == serialize_element_uncompressed(&point_minus_7g_calc), 1);

        // Point addition.
        let scalar_9 = scalar_from_u64<BLS12_381_Fr>(9);
        let point_9g = element_scalar_mul(&point_g, &scalar_9);
        let point_2g = element_scalar_mul(&point_g, &scalar_2);
        let point_2g_calc = element_add(&point_minus_7g_calc, &point_9g);
        assert!(element_eq(&point_2g, &point_2g_calc), 1);

        // Multi-scalar multiplication.
        let scalar_a = random_scalar<BLS12_381_Fr>();
        let scalar_b = random_scalar<BLS12_381_Fr>();
        let scalar_c = random_scalar<BLS12_381_Fr>();
        let point_p = random_element<BLS12_381_G2>();
        let point_q = random_element<BLS12_381_G2>();
        let point_r = random_element<BLS12_381_G2>();
        let naive = group_identity<BLS12_381_G2>();
        naive = element_add(&naive, &element_scalar_mul(&point_p, &scalar_a));
        naive = element_add(&naive, &element_scalar_mul(&point_q, &scalar_b));
        naive = element_add(&naive, &element_scalar_mul(&point_r, &scalar_c));
        let fast = element_multi_scalar_mul(&vector[point_p, point_q, point_r], &vector[scalar_a, scalar_b, scalar_c]);
        assert!(element_eq(&naive, &fast), 1);

        // Hash to group.
        let _point = hash_to_element<SHA256, BLS12_381_G2>(x"1234");
    }

    #[test(fx = @std)]
    fun test_bls12_381_gt(fx: signer) {
        std::features::change_feature_flags(&fx, vector[std::features::get_generic_group_basic_operations_feature(), std::features::get_bls12_381_groups_feature()], vector[]);
        // Group info.
        assert!(x"01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73" == group_order<BLS12_381_Gt>(), 1);

        // Element encoding/decoding.
        let element_g = group_generator<BLS12_381_Gt>();
        assert!(x"b68917caaa0543a808c53908f694d1b6e7b38de90ce9d83d505ca1ef1b442d2727d7d06831d8b2a7920afc71d8eb50120f17a0ea982a88591d9f43503e94a8f1abaf2e4589f65aafb7923c484540a868883432a5c60e75860b11e5465b1c9a08873ec29e844c1c888cb396933057ffdd541b03a5220eda16b2b3a6728ea678034ce39c6839f20397202d7c5c44bb68134f93193cec215031b17399577a1de5ff1f5b0666bdd8907c61a7651e4e79e0372951505a07fa73c25788db6eb8023519a5aa97b51f1cad1d43d8aabbff4dc319c79a58cafc035218747c2f75daf8f2fb7c00c44da85b129113173d4722f5b201b6b4454062e9ea8ba78c5ca3cadaf7238b47bace5ce561804ae16b8f4b63da4645b8457a93793cbd64a7254f150781019de87ee42682940f3e70a88683d512bb2c3fb7b2434da5dedbb2d0b3fb8487c84da0d5c315bdd69c46fb05d23763f2191aabd5d5c2e12a10b8f002ff681bfd1b2ee0bf619d80d2a795eb22f2aa7b85d5ffb671a70c94809f0dafc5b73ea2fb0657bae23373b4931bc9fa321e8848ef78894e987bff150d7d671aee30b3931ac8c50e0b3b0868effc38bf48cd24b4b811a2995ac2a09122bed9fd9fa0c510a87b10290836ad06c8203397b56a78e9a0c61c77e56ccb4f1bc3d3fcaea7550f3503efe30f2d24f00891cb45620605fcfaa4292687b3a7db7c1c0554a93579e889a121fd8f72649b2402996a084d2381c5043166673b3849e4fd1e7ee4af24aa8ed443f56dfd6b68ffde4435a92cd7a4ac3bc77e1ad0cb728606cf08bf6386e5410f" == serialize_element_uncompressed(&element_g), 1);
        let element_g_from_uncomp = std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_Gt>(x"b68917caaa0543a808c53908f694d1b6e7b38de90ce9d83d505ca1ef1b442d2727d7d06831d8b2a7920afc71d8eb50120f17a0ea982a88591d9f43503e94a8f1abaf2e4589f65aafb7923c484540a868883432a5c60e75860b11e5465b1c9a08873ec29e844c1c888cb396933057ffdd541b03a5220eda16b2b3a6728ea678034ce39c6839f20397202d7c5c44bb68134f93193cec215031b17399577a1de5ff1f5b0666bdd8907c61a7651e4e79e0372951505a07fa73c25788db6eb8023519a5aa97b51f1cad1d43d8aabbff4dc319c79a58cafc035218747c2f75daf8f2fb7c00c44da85b129113173d4722f5b201b6b4454062e9ea8ba78c5ca3cadaf7238b47bace5ce561804ae16b8f4b63da4645b8457a93793cbd64a7254f150781019de87ee42682940f3e70a88683d512bb2c3fb7b2434da5dedbb2d0b3fb8487c84da0d5c315bdd69c46fb05d23763f2191aabd5d5c2e12a10b8f002ff681bfd1b2ee0bf619d80d2a795eb22f2aa7b85d5ffb671a70c94809f0dafc5b73ea2fb0657bae23373b4931bc9fa321e8848ef78894e987bff150d7d671aee30b3931ac8c50e0b3b0868effc38bf48cd24b4b811a2995ac2a09122bed9fd9fa0c510a87b10290836ad06c8203397b56a78e9a0c61c77e56ccb4f1bc3d3fcaea7550f3503efe30f2d24f00891cb45620605fcfaa4292687b3a7db7c1c0554a93579e889a121fd8f72649b2402996a084d2381c5043166673b3849e4fd1e7ee4af24aa8ed443f56dfd6b68ffde4435a92cd7a4ac3bc77e1ad0cb728606cf08bf6386e5410f"));
        assert!(element_eq(&element_g, &element_g_from_uncomp), 1);
        let inf = group_identity<BLS12_381_Gt>();
        assert!(x"010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000" == serialize_element_uncompressed(&inf), 1);
        let inf_from_uncomp = std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_Gt>(x"010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"));
        assert!(element_eq(&inf, &inf_from_uncomp), 1);
        let element_7g_from_uncomp = std::option::extract(&mut deserialize_element_uncompressed<BLS12_381_Gt>(x"2041ea7b66c19680e2c0bb23245a71918753220b31f88a925aa9b1e192e7c188a0b365cb994b3ec5e809206117c6411242b940b10caa37ce734496b3b7c63578a0e3c076f9b31a7ca13a716262e0e4cda4ac994efb9e19893cbfe4d464b9210d099d808a08b3c4c3846e7529984899478639c4e6c46152ef49a04af9c8e6ff442d286c4613a3dac6a4bee4b40e1f6b030f2871dabe4223b250c3181ecd3bc6819004745aeb6bac567407f2b9c7d1978c45ee6712ae46930bc00638383f6696158bad488cbe7663d681c96c035481dbcf78e7a7fbaec3799163aa6914cef3365156bdc3e533a7c883d5974e3462ac6f19e3f9ce26800ae248a45c5f0dd3a48a185969224e6cd6af9a048241bdcac9800d94aeee970e08488fb961e36a769b6c185d185b4605dc9808517196bba9d00a3e37bca466c19187486db104ee03962d39fe473e276355618e44c965f05082bb027a7baa4bcc6d8c0775c1e8a481e77df36ddad91e75a982302937f543a11fe71922dcd4f46fe8f951f91cde412b359507f2b3b6df0374bfe55c9a126ad31ce254e67d64194d32d7955ec791c9555ea5a917fc47aba319e909de82da946eb36e12aff936708402228295db2712f2fc807c95092a86afd71220699df13e2d2fdf2857976cb1e605f72f1b2edabadba3ff05501221fe81333c13917c85d725ce92791e115eb0289a5d0b3330901bb8b0ed146abeb81381b7331f1c508fb14e057b05d8b0190a9e74a3d046dcd24e7ab747049945b3d8a120c4f6d88e67661b55573aa9b361367488a1ef7dffd967d64a1518"));
        assert!(std::option::is_none(&deserialize_element_uncompressed<BLS12_381_Gt>(x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")), 1);

        // Element scalar multiplication.
        let scalar_7 = scalar_from_u64<BLS12_381_Fr>(7);
        let element_7g_calc = element_scalar_mul(&element_g, &scalar_7);
        assert!(element_eq(&element_7g_calc, &element_7g_from_uncomp), 1);
        assert!(x"2041ea7b66c19680e2c0bb23245a71918753220b31f88a925aa9b1e192e7c188a0b365cb994b3ec5e809206117c6411242b940b10caa37ce734496b3b7c63578a0e3c076f9b31a7ca13a716262e0e4cda4ac994efb9e19893cbfe4d464b9210d099d808a08b3c4c3846e7529984899478639c4e6c46152ef49a04af9c8e6ff442d286c4613a3dac6a4bee4b40e1f6b030f2871dabe4223b250c3181ecd3bc6819004745aeb6bac567407f2b9c7d1978c45ee6712ae46930bc00638383f6696158bad488cbe7663d681c96c035481dbcf78e7a7fbaec3799163aa6914cef3365156bdc3e533a7c883d5974e3462ac6f19e3f9ce26800ae248a45c5f0dd3a48a185969224e6cd6af9a048241bdcac9800d94aeee970e08488fb961e36a769b6c185d185b4605dc9808517196bba9d00a3e37bca466c19187486db104ee03962d39fe473e276355618e44c965f05082bb027a7baa4bcc6d8c0775c1e8a481e77df36ddad91e75a982302937f543a11fe71922dcd4f46fe8f951f91cde412b359507f2b3b6df0374bfe55c9a126ad31ce254e67d64194d32d7955ec791c9555ea5a917fc47aba319e909de82da946eb36e12aff936708402228295db2712f2fc807c95092a86afd71220699df13e2d2fdf2857976cb1e605f72f1b2edabadba3ff05501221fe81333c13917c85d725ce92791e115eb0289a5d0b3330901bb8b0ed146abeb81381b7331f1c508fb14e057b05d8b0190a9e74a3d046dcd24e7ab747049945b3d8a120c4f6d88e67661b55573aa9b361367488a1ef7dffd967d64a1518" == serialize_element_uncompressed(&element_7g_calc), 1);

        // Element negation.
        let element_minus_7g_calc = element_neg(&element_7g_calc);
        assert!(x"2041ea7b66c19680e2c0bb23245a71918753220b31f88a925aa9b1e192e7c188a0b365cb994b3ec5e809206117c6411242b940b10caa37ce734496b3b7c63578a0e3c076f9b31a7ca13a716262e0e4cda4ac994efb9e19893cbfe4d464b9210d099d808a08b3c4c3846e7529984899478639c4e6c46152ef49a04af9c8e6ff442d286c4613a3dac6a4bee4b40e1f6b030f2871dabe4223b250c3181ecd3bc6819004745aeb6bac567407f2b9c7d1978c45ee6712ae46930bc00638383f6696158bad488cbe7663d681c96c035481dbcf78e7a7fbaec3799163aa6914cef3365156bdc3e533a7c883d5974e3462ac6f19e3f9ce26800ae248a45c5f0dd3a48a185969224e6cd6af9a048241bdcac9800d94aeee970e08488fb961e36a769b6c184e92a4b9fa2366b1ae8ebdf5542fa1e0ec390c90df40a91e5261800581b5492bd9640d1c5352babc551d1a49998f4517312f55b4339272b28a3e6b0c7d182e2bb61bd7d72b29ae3696db8fafe32b904ab5d0764e46bf21f9a0c9a1f7bedc6b12b9f64820fc8b3fd4a26541472be3c9c93d784cdd53a059d1604bf3292fedd1babfb00398128e3241bc63a5a47b5e9207fcb0c88f7bfddc376a242c9f0c032ba28eec8670f1fa1d47567593b4571c983b8015df91cfa1241b7fb8a57e0e6e01145b98de017eccc2a66e83ced9d83119a505e552467838d35b8ce2f4d7cc9a894f6dee922f35f0e72b7e96f0879b0c8614d3f9e5f5618b5be9b82381628448641a8bb0fd1dffb16c70e6831d8d69f61f2a2ef9e90c421f7a5b1ce7a5d113c7eb01" == serialize_element_uncompressed(&element_minus_7g_calc), 1);

        // Element addition.
        let scalar_9 = scalar_from_u64<BLS12_381_Fr>(9);
        let element_9g = element_scalar_mul(&element_g, &scalar_9);
        let scalar_2 = scalar_from_u64<BLS12_381_Fr>(2);
        let element_2g = element_scalar_mul(&element_g, &scalar_2);
        let element_2g_calc = element_add(&element_minus_7g_calc, &element_9g);
        assert!(element_eq(&element_2g, &element_2g_calc), 1);
    }

    #[test(fx = @std)]
    fun test_bls12381_pairing(fx: signer) {
        std::features::change_feature_flags(&fx, vector[std::features::get_generic_group_basic_operations_feature(), std::features::get_bls12_381_groups_feature()], vector[]);

        // Single pairing.
        let gt_point_1 = pairing<BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt>(
            &element_scalar_mul(&group_generator<BLS12_381_G1>(), &scalar_from_u64<BLS12_381_Fr>(5)),
            &element_scalar_mul(&group_generator<BLS12_381_G2>(), &scalar_from_u64<BLS12_381_Fr>(7)),
        );
        let gt_point_2 = pairing<BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt>(
            &element_scalar_mul(&group_generator(), &scalar_from_u64<BLS12_381_Fr>(1)),
            &element_scalar_mul(&group_generator(), &scalar_from_u64<BLS12_381_Fr>(35)),
        );
        let gt_point_3 = pairing<BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt>(
            &element_scalar_mul(&group_generator<BLS12_381_G1>(), &scalar_from_u64<BLS12_381_Fr>(35)),
            &element_scalar_mul(&group_generator<BLS12_381_G2>(), &scalar_from_u64<BLS12_381_Fr>(1)),
        );
        assert!(element_eq(&gt_point_1, &gt_point_2), 1);
        assert!(element_eq(&gt_point_1, &gt_point_3), 1);

        // Pairing with random points.
        let g1_point = random_element<BLS12_381_G1>();
        let g2_point = random_element<BLS12_381_G2>();
        // e(k1*P1, k2*P2)
        let k1 = random_scalar<BLS12_381_Fr>();
        let k2 = random_scalar<BLS12_381_Fr>();
        let gt_element = pairing<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(&element_scalar_mul(&g1_point, &k1), &element_scalar_mul(&g2_point, &k2));
        // e(P1,P2)^(k1*k2)
        let gt_element_another = element_scalar_mul(&pairing<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(&g1_point, &g2_point), &scalar_mul(&k1, &k2));
        assert!(element_eq(&gt_element, &gt_element_another), 1);

        // Multiple pairing.
        let g1_point_1 = group_generator<BLS12_381_G1>();
        let g2_point_1 = group_generator<BLS12_381_G2>();
        let g1_point_2 = element_scalar_mul(&g1_point_1, &scalar_from_u64<BLS12_381_Fr>(5));
        let g2_point_2 = element_scalar_mul(&g2_point_1, &scalar_from_u64<BLS12_381_Fr>(2));
        let g1_point_3 = element_scalar_mul(&g1_point_1, &scalar_from_u64<BLS12_381_Fr>(20));
        let g2_point_3 = element_scalar_mul(&g2_point_1, &scalar_from_u64<BLS12_381_Fr>(5));
        let expected = element_scalar_mul(&pairing<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(&g1_point_1, &g2_point_1), &scalar_from_u64<BLS12_381_Fr>(111));
        let actual = pairing_product<BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt>(&vector[g1_point_1, g1_point_2, g1_point_3], &vector[g2_point_1, g2_point_2, g2_point_3]);
        assert!(element_eq(&expected, &actual), 1);
    }

    #[test_only]
    struct UnknownGroup {}

    #[test(fx = @std)]
    #[expected_failure(abort_code = 0x0c0000, location = Self)]
    fun test_unknown_group(fx: signer) {
        std::features::change_feature_flags(&fx, vector[std::features::get_generic_group_basic_operations_feature(), std::features::get_bls12_381_groups_feature()], vector[]);
        let _ = group_order<UnknownGroup>();
    }

    #[test(fx = @std)]
    #[expected_failure(abort_code = 0x0c0000, location = Self)]
    fun test_unknown_pairing(fx: signer) {
        std::features::change_feature_flags(&fx, vector[std::features::get_generic_group_basic_operations_feature(), std::features::get_bls12_381_groups_feature()], vector[]);
        // Attempt an invalid pairing: (G2, G1) -> Gt
        pairing<BLS12_381_G2, BLS12_381_G1, BLS12_381_Gt>(
            &element_scalar_mul(&group_generator<BLS12_381_G2>(), &scalar_from_u64<BLS12_381_Fr>(7)),
            &element_scalar_mul(&group_generator<BLS12_381_G1>(), &scalar_from_u64<BLS12_381_Fr>(5)),
        );
    }
}

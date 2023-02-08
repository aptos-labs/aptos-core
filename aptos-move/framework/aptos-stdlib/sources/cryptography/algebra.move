module aptos_std::algebra {
    use std::option::{Option, some, none};
    use aptos_std::type_info::type_of;

    /// A finite field used BLS12-381 curves.
    /// It has a prime order `q=0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab`.
    struct BLS12_381_Fq {}

    /// A serialization scheme where a `BLS12_381_Fq` element is represented by a byte array `b[]` of size 48 using little-endian byte order.
    public fun bls12_381_fq_format(): vector<u8> { x"01" }

    /// A serialization scheme where a `BLS12_381_Fq` element is represented by a byte array `b[]` of size 48 using big-endian byte order.
    public fun bls12_381_fq_bendian_format(): vector<u8> { x"0101" }


    /// An extension field of `BLS12_381_Fq`, constructed as `Fq2=Fq[u]/(u^2+1)`.
    struct BLS12_381_Fq2 {}

    /// A serialization scheme where a `BLS12_381_Fq2` element in form `(c_0+c_1*u)` is represented by a byte array `b[]` of size 96.
    /// `b[0..48]` is `c_0` serialized in `bls12_381_fq_format`.
    /// `b[48..96]` is `c_1` serialized in `bls12_381_fq_format`.
    public fun bls12_381_fq2_format(): vector<u8> { x"02" }

    /// An extension field of `BLS12_381_Fq2`, constructed as `Fq6=Fq2[v]/(v^3-u-1)`.
    struct BLS12_381_Fq6 {}

    /// A serialization scheme where a `BLS12_381_Fq6` element in form `(c_0+c_1*v+c_2*v^2)` is represented by a byte array `b[]` of size 288.
    /// `b[0..96]` is `c_0` serialized in `bls12_381_fq2_format`.
    /// `b[96..192]` is `c_1` serialized in `bls12_381_fq2_format`.
    /// `b[192..288]` is `c_2` serialized in `bls12_381_fq2_format`.
    public fun bls12_381_fq6_format(): vector<u8> { x"03" }

    /// An extension field of `BLS12_381_Fq6`, constructed as `Fq12=Fq6[w]/(w^2-v)`.
    struct BLS12_381_Fq12 {}

    /// A serialization scheme where a `BLS12_381_Fq12` element in form `(c_0+c_1*w)` is represented by a byte array `b[]` of size 576.
    /// `b[0..288]` is `c_0` serialized in `bls12_381_fq6_format`.
    /// `b[288..576]` is `c_1` serialized in `bls12_381_fq6_format`.
    /// Also used in `ark_bls12_381::Fq12::deserialize()`.
    public fun bls12_381_fq12_format(): vector<u8> { x"04" }

    /// A group constructed by the points on a curve `E(Fq)` and the point at inifinity under the elliptic curve point addition.
    /// `E(Fq)` is an elliptic curve `y^2=x^3+4` defined over `BLS12_381_Fq`.
    /// The identity of `BLS12_381_G1` is the point at infinity.
    struct BLS12_381_G1 {}

    /// A serialization scheme where an `BLS12_381_G1` element is represented by a byte array `b[]` of size 96.
    /// `b[95] & 0x40` is the infinity flag.
    /// The infinity flag is 1 if and only if the element is the point at infinity.
    /// The infinity flag is 0 if and only if the element is a point `(x,y)` on curve `E(Fq)`,
    /// `[b[0], ..., b[47] & 0x3f]` is `x` serialized in `bls12_381_fq_format`, and
    /// `[b[48], ..., b[95] & 0x3f]` is `y` serialized in `bls12_381_fq_format`.
    public fun bls12_381_g1_uncompressed_format(): vector<u8> { x"05" }

    /// A serialization scheme where an `BLS12_381_G1` element is represented by a byte array `b[]` of size 48.
    /// `b[47] & 0x40` is the infinity flag.
    /// The infinity flag is 1 if and only if the element is the point at infinity.
    /// The infinity flag is 0 if and only if the element is a point `(x,y)` on curve `E(Fq)`,
    /// `[b[0], ..., b[47] & 0x3f]` is `x` serialized in `bls12_381_fq_format`, and
    /// the positiveness flag `b_47 & 0x80` is 1 if and only if `y > -y` (as unsigned integers).
    public fun bls12_381_g1_compressed_format(): vector<u8> { x"0501" }

    /// A subgroup of `BLS12_381_G1`.
    /// It has a prime order `r=0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001`.
    /// A bilinear map from `(BLS12_381_G1_SUB, BLS12_381_G2_SUB)` to `BLS12_381_Gt` exists.
    struct BLS12_381_G1_SUB {}

    /// Effectively `bls12_381_g1_uncompressed_format` but only applicable to `BLS12_381_G1_SUB` elements.
    public fun bls12_381_g1_sub_uncompressed_format(): vector<u8> { x"06" }

    /// Effectively `bls12_381_g1_compressed_format` but only applicable to `BLS12_381_G1_SUB` elements.
    public fun bls12_381_g1_sub_compressed_format(): vector<u8> { x"0601" }


    /// A group constructed by the points on a curve `E(Fq2)` and the point at inifinity under the elliptic curve point addition.
    /// `E(Fq2)` is an elliptic curve `y^2=x^3+4(u+1)` defined over `BLS12_381_Fq2`.
    /// The identity of `BLS12_381_G1` is the point at infinity.
    struct BLS12_381_G2 {}

    /// A serialization scheme where a `BLS12-381-G2` element is represented by a byte array `b[]` of size 192.
    /// `b[191] & 0x40` is the infinity flag.
    /// The infinity flag is 1 if and only if the element is the point at infinity.
    /// The infinity flag is 0 if and only if the element is a point `(x,y)` on curve `E(Fq2)`,
    /// `b[0..96]` is `x` serialized in `bls12_381_fq2_format`, and
    /// `[b[96], ..., b[191] & 0x3f]` is `y` serialized in `bls12_381_fq2_format`.
    public fun bls12_381_g2_uncompressed_format(): vector<u8> { x"07" }

    /// A serialization scheme where a `BLS12-381-G2` element is represented by a byte array `b[]` of size 96.
    /// `b[95] & 0x40` is the infinity flag.
    /// The infinity flag is 1 if and only if the element is the point at infinity.
    /// The infinity flag is 0 if and only if the element is a point `(x,y)` on curve `E(Fq2)`,
    /// `[b[0], ..., b[95] & 0x3f]` is `x` serialized in `bls12_381_fq2_format`, and
    /// the positiveness flag `b[95] & 0x80` is 1 if and only if `y > -y` (as unsigned integers).
    public fun bls12_381_g2_compressed_format(): vector<u8> { x"0701" }

    /// A subgroup of `BLS12_381_G2`.
    /// It has a prime order `r=0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001`.
    /// A bilinear map from `(BLS12_381_G1_SUB, BLS12_381_G2_SUB)` to `BLS12_381_Gt` exists.
    struct BLS12_381_G2_SUB {}

    /// Effectively `bls12_381_g2_uncompressed_format` but only applicable to `BLS12_381_G2_SUB` elements.
    public fun bls12_381_g2_sub_uncompressed_format(): vector<u8> { x"08" }

    /// Effectively `bls12_381_g2_compressed_format` but only applicable to `BLS12_381_G2_SUB` elements.
    public fun bls12_381_g2_sub_compressed_format(): vector<u8> { x"0801" }

    /// `BLS12_381_Gt` represents the target group of the pairing defined over the BLS12-381 curves.
    /// A multiplicative subgroup of `BLS12_381_Fq12`.
    /// It has a prime order `r=0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001`. (Same as `BLS12_381_G1_SUB` and `BLS12_381_G2_SUB`.)
    /// The identity of `BLS12_381_G2` is 1.
    /// A bilinear map from `(BLS12_381_G1_SUB, BLS12_381_G2_SUB)` to `BLS12_381_Gt` exists.
    struct BLS12_381_Gt {}

    /// Effectively `bls12_381_fq12_format()` but only applicable to `BLS12_381_Gt` elements.
    public fun bls12_381_gt_format(): vector<u8> { x"09" }

    /// A finite field thatshares the same prime number `r` with groups `BLS12_381_G1_SUB`, `BLS12_381_G2_SUB` and `BLS12_381_Gt`,
    /// and thus can be their scalar field.
    struct BLS12_381_Fr {}

    /// A serialization scheme where a `BLS12_381_Fr` element is represented by a byte array `b[]` of size 32 using little-endian byte order.
    public fun bls12_381_fr_lendian_format(): vector<u8> { x"0a" }

    /// A serialization scheme where a `BLS12_381_Fr` element is represented by a byte array `b[]` of size 32 using big-endian byte order.
    public fun bls12_381_fr_bendian_format(): vector<u8> { x"0a01" }

    /// This struct represents an ephemeral element of an algebraic structure `S`.
    struct Element<phantom S> has copy, drop {
        handle: u64
    }

    /// Compute a pre-compiled pairing function (a.k.a., bilinear map) on `element_1` and `element_2`.
    /// Return an element in the target group `Gt`.
    public fun pairing<G1,G2,Gt>(element_1: &Element<G1>, element_2: &Element<G2>): Element<Gt> {
        Element<Gt> {
            handle: pairing_product_internal<G1,G2,Gt>(std::vector::singleton(element_1.handle), std::vector::singleton(element_2.handle))
        }
    }

    /// Compute `pairing(a[0], b[0]) + ... + pairing(a[n-1], b[n-1])` for `n` elements of group `G1` and `n` elements of group `G2`.
    /// This is faster and cheaper than calling `pairing()` separately then aggregating with `group_add`.
    public fun multi_pairing<G1, G2, Gt>(g1_elements: &vector<Element<G1>>, g2_elements: &vector<Element<G2>>): Element<Gt> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G1>();
        abort_unless_structure_enabled<G2>();
        abort_unless_structure_enabled<Gt>();
        let num_g1 = std::vector::length(g1_elements);
        let num_g2 = std::vector::length(g2_elements);
        assert!(num_g1 == num_g2, std::error::invalid_argument(1));
        let g1_handles = std::vector::empty();
        let g2_handles = std::vector::empty();
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

    /// Convert a u64 to an element of an algebraic structure `S`.
    public fun from_u64<S>(value: u64): Element<S> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<S>();
        Element<S> {
            handle: from_u64_internal<S>(value)
        }
    }

    /// Return the additive identity of a field `S`.
    public fun field_zero<S>(): Element<S> {
        Element<S> {
            handle: field_zero_internal<S>()
        }
    }

    /// Return the multiplicative identity of a field `S`.
    public fun field_one<S>(): Element<S> {
        Element<S> {
            handle: field_one_internal<S>()
        }
    }

    /// Compute `-x` for an element `x` of a field `S`.
    public fun field_neg<S>(x: &Element<S>): Element<S> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<S>();
        Element<S> {
            handle: field_neg_internal<S>(x.handle)
        }
    }

    /// Compute `x + y` for elements `x` and `y` of a field `S`.
    public fun field_add<S>(x: &Element<S>, y: &Element<S>): Element<S> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<S>();
        Element<S> {
            handle: field_add_internal<S>(x.handle, y.handle)
        }
    }

    /// Compute `x - y` for elements `x` and `y` of a field `S`.
    public fun field_sub<S>(x: &Element<S>, y: &Element<S>): Element<S> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<S>();
        Element<S> {
            handle: field_sub_internal<S>(x.handle, y.handle)
        }
    }

    /// Compute `x * y` for elements `x` and `y` of a field `S`.
    public fun field_mul<S>(x: &Element<S>, y: &Element<S>): Element<S> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<S>();
        Element<S> {
            handle: field_mul_internal<S>(x.handle, y.handle)
        }
    }

    /// Compute `x / y` for elements `x` and `y` of a field `S`.
    /// Return none if y is the additive identity of field `S`.
    public fun field_div<S>(x: &Element<S>, y: &Element<S>): Option<Element<S>> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<S>();
        let (succ, handle) = field_div_internal<S>(x.handle, y.handle);
        if (succ) {
            some(Element<S> { handle })
        } else {
            none()
        }

    }

    /// Compute `x^(-1)` for an element `x` of a field `S`.
    /// Return none if `x` is the additive identity of field `S`.
    public fun field_inv<S>(x: &Element<S>): Option<Element<S>> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<S>();
        let (succeeded, handle) = field_inv_internal<S>(x.handle);
        if (succeeded) {
            let scalar = Element<S> { handle };
            std::option::some(scalar)
        } else {
            std::option::none()
        }
    }

    /// Compute `b^e` for an element `b` of a field `S` and an integer `e` in little-endian encoding.
    public fun field_pow<S>(b: &Element<S>, e: &vector<u8>): Element<S> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<S>();
        Element<S> {
            handle: field_pow_internal<S>(b.handle, *e)
        }
    }

    /// Check if `x == y` for elements `x` and `y` of an algebraic structure `S`.
    public fun eq<S>(x: &Element<S>, y: &Element<S>): bool {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<S>();
        eq_internal<S>(x.handle, y.handle)
    }

    /// Get the identity of a group `G`.
    public fun group_identity<G>(): Element<G> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G>();
        Element<G> {
            handle: group_identity_internal<G>()
        }
    }

    /// Get the fixed generator of a cyclic group `G`.
    public fun group_generator<G>(): Element<G> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G>();
        Element<G> {
            handle: group_generator_internal<G>()
        }
    }

    /// Compute `-P` for an element `P` of a group `G`.
    public fun group_neg<G>(element_p: &Element<G>): Element<G> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G>();
        Element<G> {
            handle: element_neg_internal<G>(element_p.handle)
        }
    }

    /// Compute `P + Q` for elements `P` and `Q` of a group `G`.
    public fun group_add<G>(element_p: &Element<G>, element_q: &Element<G>): Element<G> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G>();
        Element<G> {
            handle: element_add_internal<G>(element_p.handle, element_q.handle)
        }
    }

    /// Compute `2*P` for an element `P` of a group `G`.
    public fun group_double<G>(element_p: &Element<G>): Element<G> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G>();
        Element<G> {
            handle: element_double_internal<G>(element_p.handle)
        }
    }

    /// Compute `k*p`, where `p` is an element of a group `G` and `k` is an element of the scalar field `S` of group `G`.
    public fun group_scalar_mul<G, S>(element_p: &Element<G>, scalar_k: &Element<S>): Element<G> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G>();
        abort_unless_structure_enabled<S>();
        Element<G> {
            handle: element_mul_internal<G, S>(element_p.handle, scalar_k.handle)
        }
    }

    /// Compute `k[0]*P[0]+...+k[n-1]*P[n-1]` where
    /// `P[]` are elements of group `G`,
    /// `k[]` are elements of the scalar field `S` of group `G`,
    /// and both `P[]` and `k[]` have the same size `n`.
    /// Abort if the number of elements and that of scalars do not match.
    /// This function is much faster and cheaper than calling `group_scalar_mul` and adding up the results using `group_add`.
    public fun group_multi_scalar_mul<G, S>(elements: &vector<Element<G>>, scalars: &vector<Element<S>>): Element<G> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G>();
        abort_unless_structure_enabled<S>();
        let num_scalars = std::vector::length(scalars);
        let scalar_handles = std::vector::empty();
        let i = 0;
        while (i < num_scalars) {
            std::vector::push_back(&mut scalar_handles, std::vector::borrow(scalars, i).handle);
            i = i + 1;
        };

        let num_elements = std::vector::length(elements);
        let element_handles = std::vector::empty();
        let i = 0;
        while (i < num_elements) {
            std::vector::push_back(&mut element_handles, std::vector::borrow(elements, i).handle);
            i = i + 1;
        };

        Element<G> {
            handle: element_multi_scalar_mul_internal<G, S>(element_handles, scalar_handles)
        }

    }

    /// Deserializate a byte array to an element of an algebraic structure `S` with a given scheme.
    public fun deserialize<S>(scheme_id: vector<u8>, bytes: &vector<u8>): Option<Element<S>> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<S>();
        let (succeeded, handle) = deserialize_internal<S>(scheme_id, *bytes);
        if (succeeded) {
            let scalar = Element<S> {
                handle
            };
            std::option::some(scalar)
        } else {
            std::option::none()
        }
    }

    /// Serialize an element of an algebraic structure `S` to a byte array with a given scheme.
    public fun serialize<S>(scheme_id: vector<u8>, scalar: &Element<S>): vector<u8> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<S>();
        serialize_internal<S>(scheme_id, scalar.handle)
    }

    /// Get the order of group `G`, little-endian encoded as a byte array.
    public fun group_order<G>(): vector<u8> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<G>();
        group_order_internal<G>()
    }

    /// Cast an element of a structure `S` to a parent structure `L`.
    public fun upcast<S,L>(element: &Element<S>): Element<L> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<S>();
        abort_unless_structure_enabled<L>();
        Element<L> {
            handle: upcast_internal<S,L>(element.handle)
        }
    }

    /// Cast an element of a structure `L` to a sub structure `S`.
    public fun downcast<L,S>(element: &Element<L>): Option<Element<S>> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<S>();
        abort_unless_structure_enabled<L>();
        let (succ, new_handle) = downcast_internal<L,S>(element.handle);
        if (succ) {
            some(Element<S> { handle: new_handle })
        } else {
            none()
        }
    }

    #[test_only]
    /// Generate a random element of an algebraic structure `S`.
    public fun insecure_random_element<S>(): Element<S> {
        abort_if_generic_group_basic_operations_disabled();
        abort_unless_structure_enabled<S>();
        Element<S> {
            handle: insecure_random_element_internal<S>()
        }
    }

    fun abort_if_generic_group_basic_operations_disabled() {
        if (!std::features::generic_group_basic_operations_enabled()) {
            abort(std::error::not_implemented(0))
        }
    }

    fun abort_unless_structure_enabled<S>() {
        let type = type_of<S>();
        if ((type == type_of<BLS12_381_G1_SUB>() || type == type_of<BLS12_381_G2_SUB>() || type == type_of<BLS12_381_Gt>() || type == type_of<BLS12_381_Fr>())
            && std::features::bls12_381_structures_enabled()
        ) {
            // Let go.
        } else {
            abort(std::error::not_implemented(0))
        }
    }

    // Native functions.
    native fun deserialize_internal<G>(scheme_id: vector<u8>, bytes: vector<u8>): (bool, u64);
    native fun serialize_internal<G>(scheme_id: vector<u8>, h: u64): vector<u8>;
    native fun from_u64_internal<S>(value: u64): u64;
    native fun field_add_internal<F>(handle_1: u64, handle_2: u64): u64;
    native fun field_div_internal<F>(handle_1: u64, handle_2: u64): (bool, u64);
    native fun field_inv_internal<F>(handle: u64): (bool, u64);
    native fun field_mul_internal<F>(handle_1: u64, handle_2: u64): u64;
    native fun field_neg_internal<F>(handle: u64): u64;
    native fun field_one_internal<S>(): u64;
    native fun field_pow_internal<F>(handle: u64, e: vector<u8>): u64;
    native fun field_sub_internal<G>(handle_1: u64, handle_2: u64): u64;
    native fun field_zero_internal<S>(): u64;
    native fun element_add_internal<G>(handle_1: u64, handle_2: u64): u64;
    native fun eq_internal<S>(handle_1: u64, handle_2: u64): bool;
    native fun group_identity_internal<G>(): u64;
    native fun group_order_internal<G>(): vector<u8>;
    native fun group_generator_internal<G>(): u64;
    native fun element_mul_internal<G, S>(scalar_handle: u64, element_handle: u64): u64;
    native fun element_double_internal<G>(element_handle: u64): u64;
    native fun element_neg_internal<G>(handle: u64): u64;
    native fun element_multi_scalar_mul_internal<G, S>(element_handles: vector<u64>, scalar_handles: vector<u64>): u64;
    native fun pairing_product_internal<G1,G2,Gt>(g1_handles: vector<u64>, g2_handles: vector<u64>): u64;
    native fun upcast_internal<S,L>(handle: u64): u64;
    native fun downcast_internal<L,S>(handle: u64): (bool, u64);

    #[test_only]
    native fun insecure_random_element_internal<G>(): u64;

    #[test_only]
    public fun enable_initial_generic_algebraic_operations(fx: &signer) {
        std::features::change_feature_flags(fx, std::vector::singleton(std::features::get_generic_group_basic_operations_feature()), std::vector::empty());
    }

    #[test_only]
    public fun enable_bls12_381_structures(fx: &signer) {
        std::features::change_feature_flags(fx, std::vector::singleton(std::features::get_bls12_381_groups_feature()), std::vector::empty());
    }

    const BLS12_381_FR_VAL_7_SERIALIZED_LENDIAN: vector<u8> = x"0700000000000000000000000000000000000000000000000000000000000000";
    const BLS12_381_FR_VAL_7_SERIALIZED_BENDIAN: vector<u8> = x"0000000000000000000000000000000000000000000000000000000000000007";
    const BLS12_381_FR_VAL_7_NEG_SERIALIZED_LENDIAN: vector<u8> = x"fafffffffefffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73";

    #[test(fx = @std)]
    fun test_bls12_381_fr(fx: signer) {
        enable_initial_generic_algebraic_operations(&fx);
        enable_bls12_381_structures(&fx);

        // Serialization/deserialization.
        let val_7 = from_u64<BLS12_381_Fr>(7);
        let val_7_2nd = std::option::extract(&mut deserialize<BLS12_381_Fr>(bls12_381_fr_lendian_format(), &BLS12_381_FR_VAL_7_SERIALIZED_LENDIAN));
        let val_7_3rd = std::option::extract(&mut deserialize<BLS12_381_Fr>(bls12_381_fr_bendian_format(), &BLS12_381_FR_VAL_7_SERIALIZED_BENDIAN));
        assert!(eq(&val_7, &val_7_2nd), 1);
        assert!(eq(&val_7, &val_7_3rd), 1);
        assert!(BLS12_381_FR_VAL_7_SERIALIZED_LENDIAN == serialize(bls12_381_fr_lendian_format(), &val_7), 1);
        assert!(BLS12_381_FR_VAL_7_SERIALIZED_BENDIAN == serialize(bls12_381_fr_bendian_format(), &val_7), 1);
        assert!(std::option::is_none(&deserialize<BLS12_381_Fr>(bls12_381_fr_lendian_format(), &x"ffff")), 1);
        assert!(std::option::is_none(&deserialize<BLS12_381_Fr>(bls12_381_fr_bendian_format(), &x"ffff")), 1);

        // Negation.
        let val_minus_7 = field_neg(&val_7);
        assert!(BLS12_381_FR_VAL_7_NEG_SERIALIZED_LENDIAN == serialize(bls12_381_fr_lendian_format(), &val_minus_7), 1);

        // Addition.
        let val_9 = from_u64<BLS12_381_Fr>(9);
        let val_2 = from_u64<BLS12_381_Fr>(2);
        assert!(eq(&val_2, &field_add(&val_minus_7, &val_9)), 1);

        // Subtraction.
        assert!(eq(&val_9, &field_sub(&val_2, &val_minus_7)), 1);

        // Multiplication.
        let val_63 = from_u64<BLS12_381_Fr>(63);
        assert!(eq(&val_63, &field_mul(&val_7, &val_9)), 1);

        // division.
        let val_0 = from_u64<BLS12_381_Fr>(0);
        assert!(eq(&val_7, &std::option::extract(&mut field_div(&val_63, &val_9))), 1);
        assert!(std::option::is_none(&field_div(&val_63, &val_0)), 1);

        // Inversion.
        assert!(eq(&val_minus_7, &field_neg(&val_7)), 1);
        assert!(std::option::is_none(&field_inv(&val_0)), 1);

        // Exponentiation.
        let val_3 = from_u64<BLS12_381_Fr>(3);
        let val_81 = from_u64<BLS12_381_Fr>(81);
        assert!(eq(&val_81, &field_pow(&val_3, &x"04")), 1);
    }

//    const BLS12_381_FQ12_VAL_7_SERIALIZED: vector<u8> = x"070000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
//    const BLS12_381_FQ12_VAL_7_NEG_SERIALIZED: vector<u8> = x"a4aafffffffffeb9ffff53b1feffab1e24f6b0f6a0d23067bf1285f3844b7764d7ac4b43b6a71b4b9ae67f39ea11011a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

//    #[test(fx = @std)]
//    fun test_bls12_381_fq12(fx: signer) {
//        enable_initial_generic_algebraic_operations(&fx);
//        enable_bls12_381_structures(&fx);
//
//        // Serialization/deserialization.
//        let val_7 = from_u64<BLS12_381_Fq12>(7);
//        let val_7_another = std::option::extract(&mut deserialize<BLS12_381_Fq12>(bls12_381_fq12_format(), &BLS12_381_FQ12_VAL_7_SERIALIZED));
//        assert!(eq(&val_7, &val_7_another), 1);
//        assert!(BLS12_381_FQ12_VAL_7_SERIALIZED == serialize(bls12_381_fq12_format(), &val_7), 1);
//        assert!(std::option::is_none(&deserialize<BLS12_381_Fq12>(bls12_381_fq12_format(), &x"ffff")), 1);
//
//        // Negation.
//        let val_minus_7 = field_neg(&val_7);
//        assert!(BLS12_381_FQ12_VAL_7_NEG_SERIALIZED == serialize(bls12_381_fq12_format(), &val_minus_7), 1);
//
//        // Addition.
//        let val_9 = from_u64<BLS12_381_Fq12>(9);
//        let val_2 = from_u64<BLS12_381_Fq12>(2);
//        assert!(eq(&val_2, &field_add(&val_minus_7, &val_9)), 1);
//
//        // Subtraction.
//        assert!(eq(&val_9, &field_sub(&val_2, &val_minus_7)), 1);
//
//        // Multiplication.
//        let val_63 = from_u64<BLS12_381_Fq12>(63);
//        assert!(eq(&val_63, &field_mul(&val_7, &val_9)), 1);
//
//        // division.
//        let val_0 = from_u64<BLS12_381_Fq12>(0);
//        assert!(eq(&val_7, &std::option::extract(&mut field_div(&val_63, &val_9))), 1);
//        assert!(std::option::is_none(&field_div(&val_63, &val_0)), 1);
//
//        // Inversion.
//        assert!(eq(&val_minus_7, &field_neg(&val_7)), 1);
//        assert!(std::option::is_none(&field_inv(&val_0)), 1);
//
//        // Exponentiation.
//        let val_3 = from_u64<BLS12_381_Fq12>(3);
//        let val_81 = from_u64<BLS12_381_Fq12>(81);
//        assert!(eq(&val_81, &field_pow(&val_3, &x"04")), 1);
//    }

//    const BLS12_381_G1_INF_SERIALIZED_COMP: vector<u8> = x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040";
//    const BLS12_381_G1_INF_SERIALIZED_UNCOMP: vector<u8> = x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040";
//    const BLS12_381_G1_GENERATOR_SERIALIZED_COMP: vector<u8> = x"bbc622db0af03afbef1a7af93fe8556c58ac1b173f3a4ea105b974974f8c68c30faca94f8c63952694d79731a7d3f117";
//    const BLS12_381_G1_GENERATOR_SERIALIZED_UNCOMP: vector<u8> = x"bbc622db0af03afbef1a7af93fe8556c58ac1b173f3a4ea105b974974f8c68c30faca94f8c63952694d79731a7d3f117e1e7c5462923aa0ce48a88a244c73cd0edb3042ccb18db00f60ad0d595e0f5fce48a1d74ed309ea0f1a0aae381f4b308";
//    const BLS12_381_G1_GENERATOR_MUL_BY_7_SERIALIZED_COMP: vector<u8> = x"b7fc7e62705aef542dbcc5d4bce62a7bf22eef1691bef30dac121fb200ca7dc9a4403b90da4501cfee1935b9bef328191c1a98287eec115a8cb0a1cf4968c6fd101ca4593938d73918dd8e81471d8a3ac4b38930aed539564436b6a4baad8d10";
//    const BLS12_381_G1_GENERATOR_MUL_BY_7_SERIALIZED_UNCOMP: vector<u8> = x"b7fc7e62705aef542dbcc5d4bce62a7bf22eef1691bef30dac121fb200ca7dc9a4403b90da4501cfee1935b9bef32899";
//    const BLS12_381_G1_GENERATOR_MUL_BY_7_NEG_SERIALIZED_COMP: vector<u8> = x"b7fc7e62705aef542dbcc5d4bce62a7bf22eef1691bef30dac121fb200ca7dc9a4403b90da4501cfee1935b9bef32819";
//    const BLS12_381_G1_GENERATOR_MUL_BY_7_NEG_SERIALIZED_UNCOMP: vector<u8> = x"b7fc7e62705aef542dbcc5d4bce62a7bf22eef1691bef30dac121fb200ca7dc9a4403b90da4501cfee1935b9bef328198f9067d78113ed5f734fb2e1b497e52013da0c9d679a592da735f6713d2eed2913f9c11208d2e1f455b0c9942f647309";
//
//    #[test(fx = @std)]
//    fun test_bls12_381_g1(fx: signer) {
//        enable_initial_generic_algebraic_operations(&fx);
//        enable_bls12_381_structures(&fx);
//
//        // Group info.
//        assert!(BLS12_381_R == group_order<BLS12_381_G1>(), 1);
//
//        // Serialization/deserialization.
//        let point_g = group_generator<BLS12_381_G1>();
//        assert!(BLS12_381_G1_GENERATOR_SERIALIZED_UNCOMP == serialize(bls12_381_g1_serialization_scheme_uncompressed(), &point_g), 1);
//        assert!(BLS12_381_G1_GENERATOR_SERIALIZED_COMP == serialize(bls12_381_g1_serialization_scheme_compressed(), &point_g), 1);
//        let point_g_from_uncomp = std::option::extract(&mut deserialize<BLS12_381_G1>(bls12_381_g1_serialization_scheme_uncompressed(), &BLS12_381_G1_GENERATOR_SERIALIZED_UNCOMP));
//        let point_g_from_comp = std::option::extract(&mut deserialize<BLS12_381_G1>(bls12_381_g1_serialization_scheme_compressed(), &BLS12_381_G1_GENERATOR_SERIALIZED_COMP));
//        assert!(eq(&point_g, &point_g_from_comp), 1);
//        assert!(eq(&point_g, &point_g_from_uncomp), 1);
//        let inf = group_identity<BLS12_381_G1>();
//        assert!(BLS12_381_G1_INF_SERIALIZED_UNCOMP == serialize(bls12_381_g1_serialization_scheme_uncompressed(), &inf), 1);
//        assert!(BLS12_381_G1_INF_SERIALIZED_COMP == serialize(bls12_381_g1_serialization_scheme_compressed(), &inf), 1);
//        let inf_from_uncomp = std::option::extract(&mut deserialize<BLS12_381_G1>(bls12_381_g1_serialization_scheme_uncompressed(), &BLS12_381_G1_INF_SERIALIZED_UNCOMP));
//        let inf_from_comp = std::option::extract(&mut deserialize<BLS12_381_G1>(bls12_381_g1_serialization_scheme_compressed(), &BLS12_381_G1_INF_SERIALIZED_COMP));
//        assert!(eq(&inf, &inf_from_comp), 1);
//        assert!(eq(&inf, &inf_from_uncomp), 1);
//        let point_7g_from_uncomp = std::option::extract(&mut deserialize<BLS12_381_G1>(bls12_381_g1_serialization_scheme_uncompressed(), &x"b7fc7e62705aef542dbcc5d4bce62a7bf22eef1691bef30dac121fb200ca7dc9a4403b90da4501cfee1935b9bef328191c1a98287eec115a8cb0a1cf4968c6fd101ca4593938d73918dd8e81471d8a3ac4b38930aed539564436b6a4baad8d10"));
//        let point_7g_from_comp = std::option::extract(&mut deserialize<BLS12_381_G1>(bls12_381_g1_serialization_scheme_compressed(), &x"b7fc7e62705aef542dbcc5d4bce62a7bf22eef1691bef30dac121fb200ca7dc9a4403b90da4501cfee1935b9bef32899"));
//        assert!(eq(&point_7g_from_comp, &point_7g_from_uncomp), 1);
//
//        // Scalar multiplication.
//        let scalar_7 = from_u64<BLS12_381_Fr>(7);
//        let point_7g_calc = group_scalar_mul(&point_g, &scalar_7);
//        assert!(eq(&point_7g_calc, &point_7g_from_comp), 1);
//        assert!(BLS12_381_G1_GENERATOR_MUL_BY_7_SERIALIZED_UNCOMP == serialize(bls12_381_g1_serialization_scheme_uncompressed(), &point_7g_calc), 1);
//        assert!(BLS12_381_G1_GENERATOR_MUL_BY_7_SERIALIZED_COMP == serialize(bls12_381_g1_serialization_scheme_compressed(), &point_7g_calc), 1);
//
//        // Doubling.
//        let scalar_2 = from_u64<BLS12_381_Fr>(2);
//        let point_2g = group_scalar_mul(&point_g, &scalar_2);
//        let point_double_g = group_double(&point_g);
//        assert!(eq(&point_2g, &point_double_g), 1);
//
//        // Negation.
//        let point_minus_7g_calc = group_neg(&point_7g_calc);
//        assert!(BLS12_381_G1_GENERATOR_MUL_BY_7_NEG_SERIALIZED_COMP == serialize(bls12_381_g1_serialization_scheme_compressed(), &point_minus_7g_calc), 1);
//        assert!(BLS12_381_G1_GENERATOR_MUL_BY_7_NEG_SERIALIZED_UNCOMP == serialize(bls12_381_g1_serialization_scheme_uncompressed(), &point_minus_7g_calc), 1);
//
//        // Addition.
//        let scalar_9 = from_u64<BLS12_381_Fr>(9);
//        let point_9g = group_scalar_mul(&point_g, &scalar_9);
//        let point_2g = group_scalar_mul(&point_g, &scalar_2);
//        let point_2g_calc = group_add(&point_minus_7g_calc, &point_9g);
//        assert!(eq(&point_2g, &point_2g_calc), 1);
//
//        // Multi-scalar multiplication.
//        let scalar_a = insecure_random_element<BLS12_381_Fr>();
//        let scalar_b = insecure_random_element<BLS12_381_Fr>();
//        let scalar_c = insecure_random_element<BLS12_381_Fr>();
//        let point_p = insecure_random_element<BLS12_381_G1>();
//        let point_q = insecure_random_element<BLS12_381_G1>();
//        let point_r = insecure_random_element<BLS12_381_G1>();
//        let naive = group_identity<BLS12_381_G1>();
//        naive = group_add(&naive, &group_scalar_mul(&point_p, &scalar_a));
//        naive = group_add(&naive, &group_scalar_mul(&point_q, &scalar_b));
//        naive = group_add(&naive, &group_scalar_mul(&point_r, &scalar_c));
//        let fast = group_multi_scalar_mul(&triplet(point_p, point_q, point_r), &triplet(scalar_a, scalar_b, scalar_c));
//        assert!(eq(&naive, &fast), 1);
//    }
//
//    const BLS12_381_G2_INF_SERIALIZED_UNCOMP: vector<u8> = x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040";
//    const BLS12_381_G2_INF_SERIALIZED_COMP: vector<u8> = x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040";
//    const BLS12_381_G2_GENERATOR_SERIALIZED_UNCOMP: vector<u8> = x"b8bd21c1c85680d4efbb05a82603ac0b77d1e37a640b51b4023b40fad47ae4c65110c52d27050826910a8ff0b2a24a027e2b045d057dace5575d941312f14c3349507fdcbb61dab51ab62099d0d06b59654f2788a0d3ac7d609f7152602be0130128b808865493e189a2ac3bccc93a922cd16051699a426da7d3bd8caa9bfdad1a352edac6cdc98c116e7d7227d5e50cbe795ff05f07a9aaa11dec5c270d373fab992e57ab927426af63a7857e283ecb998bc22bb0d2ac32cc34a72ea0c40606";
//    const BLS12_381_G2_GENERATOR_SERIALIZED_COMP: vector<u8> = x"b8bd21c1c85680d4efbb05a82603ac0b77d1e37a640b51b4023b40fad47ae4c65110c52d27050826910a8ff0b2a24a027e2b045d057dace5575d941312f14c3349507fdcbb61dab51ab62099d0d06b59654f2788a0d3ac7d609f7152602be013";
//    const BLS12_381_G2_GENERATOR_MUL_BY_7_SERIALIZED_UNCOMP: vector<u8> = x"3c8dd3f68a360f9c5ba81fad2be3408bdc3070619bc7bf3794851bd623685a5036ef5f1388c0541e58c3d2b2dbd19c04c83472247446b1bdd44416ad1c1f929a3f01ed345be35b9b4ba20f17ccf2b5208e3dec8380d6b8c337ed31bff673020dddcc1399cdf852dab1e2c8dc3b0ce819362f3a12da56f37aee93d3881ca760e467942c92428864a6172c80bf4daeb7082070fa8e8937746ae82d57ec8b639977f8ceaef21a11375de52b02e145dc39021bf4cab7eeaa955688a1b75436f9ec05";
//    const BLS12_381_G2_GENERATOR_MUL_BY_7_SERIALIZED_COMP: vector<u8> = x"3c8dd3f68a360f9c5ba81fad2be3408bdc3070619bc7bf3794851bd623685a5036ef5f1388c0541e58c3d2b2dbd19c04c83472247446b1bdd44416ad1c1f929a3f01ed345be35b9b4ba20f17ccf2b5208e3dec8380d6b8c337ed31bff673020d";
//    const BLS12_381_G2_GENERATOR_MUL_BY_7_NEG_SERIALIZED_UNCOMP: vector<u8> = x"3c8dd3f68a360f9c5ba81fad2be3408bdc3070619bc7bf3794851bd623685a5036ef5f1388c0541e58c3d2b2dbd19c04c83472247446b1bdd44416ad1c1f929a3f01ed345be35b9b4ba20f17ccf2b5208e3dec8380d6b8c337ed31bff673020dceddeb663207acdf4d1d8bd4c2f3c304eec676e4c67b3decd07eb16a68a416806f181fb1731fb7a482baff799c6349118b3a057176c88a4f17d2fcc4729c12a72b27020486c1f909dae682123f6f3d62bcb8808bc7fc85f41145c8e4b3181414";
//    const BLS12_381_G2_GENERATOR_MUL_BY_7_NEG_SERIALIZED_COMP: vector<u8> = x"3c8dd3f68a360f9c5ba81fad2be3408bdc3070619bc7bf3794851bd623685a5036ef5f1388c0541e58c3d2b2dbd19c04c83472247446b1bdd44416ad1c1f929a3f01ed345be35b9b4ba20f17ccf2b5208e3dec8380d6b8c337ed31bff673028d";
//
//    #[test(fx = @std)]
//    fun test_bls12_381_g2(fx: signer) {
//        enable_initial_generic_algebraic_operations(&fx);
//        enable_bls12_381_structures(&fx);
//
//        // Group info.
//        assert!(BLS12_381_R == group_order<BLS12_381_G2>(), 1);
//
//        // Serialization/deserialization.
//        let point_g = group_generator<BLS12_381_G2>();
//        assert!(BLS12_381_G2_GENERATOR_SERIALIZED_UNCOMP == serialize(bls12_381_g2_serialization_scheme_uncompressed(), &point_g), 1);
//        assert!(BLS12_381_G2_GENERATOR_SERIALIZED_COMP == serialize(bls12_381_g2_serialization_scheme_compressed(), &point_g), 1);
//        let point_g_from_uncomp = std::option::extract(&mut deserialize<BLS12_381_G2>(bls12_381_g2_serialization_scheme_uncompressed(), &BLS12_381_G2_GENERATOR_SERIALIZED_UNCOMP));
//        let point_g_from_comp = std::option::extract(&mut deserialize<BLS12_381_G2>(bls12_381_g2_serialization_scheme_compressed(), &BLS12_381_G2_GENERATOR_SERIALIZED_COMP));
//        assert!(eq(&point_g, &point_g_from_comp), 1);
//        assert!(eq(&point_g, &point_g_from_uncomp), 1);
//        let inf = group_identity<BLS12_381_G2>();
//        assert!(BLS12_381_G2_INF_SERIALIZED_UNCOMP == serialize(bls12_381_g2_serialization_scheme_uncompressed(), &inf), 1);
//        assert!(BLS12_381_G2_INF_SERIALIZED_COMP == serialize(bls12_381_g2_serialization_scheme_compressed(), &inf), 1);
//        let inf_from_uncomp = std::option::extract(&mut deserialize<BLS12_381_G2>(bls12_381_g2_serialization_scheme_uncompressed(), &BLS12_381_G2_INF_SERIALIZED_UNCOMP));
//        let inf_from_comp = std::option::extract(&mut deserialize<BLS12_381_G2>(bls12_381_g2_serialization_scheme_compressed(), &BLS12_381_G2_INF_SERIALIZED_COMP));
//        assert!(eq(&inf, &inf_from_comp), 1);
//        assert!(eq(&inf, &inf_from_uncomp), 1);
//        let point_7g_from_uncomp = std::option::extract(&mut deserialize<BLS12_381_G2>(bls12_381_g2_serialization_scheme_uncompressed(), &BLS12_381_G2_GENERATOR_MUL_BY_7_SERIALIZED_UNCOMP));
//        let point_7g_from_comp = std::option::extract(&mut deserialize<BLS12_381_G2>(bls12_381_g2_serialization_scheme_compressed(), &BLS12_381_G2_GENERATOR_MUL_BY_7_SERIALIZED_COMP));
//        assert!(eq(&point_7g_from_comp, &point_7g_from_uncomp), 1);
//
//        // Scalar multiplication.
//        let scalar_7 = from_u64<BLS12_381_Fr>(7);
//        let point_7g_calc = group_scalar_mul(&point_g, &scalar_7);
//        assert!(eq(&point_7g_calc, &point_7g_from_comp), 1);
//        assert!(BLS12_381_G2_GENERATOR_MUL_BY_7_SERIALIZED_UNCOMP == serialize(bls12_381_g2_serialization_scheme_uncompressed(), &point_7g_calc), 1);
//        assert!(BLS12_381_G2_GENERATOR_MUL_BY_7_SERIALIZED_COMP == serialize(bls12_381_g2_serialization_scheme_compressed(), &point_7g_calc), 1);
//
//        // Doubling.
//        let scalar_2 = from_u64<BLS12_381_Fr>(2);
//        let point_2g = group_scalar_mul(&point_g, &scalar_2);
//        let point_double_g = group_double(&point_g);
//        assert!(eq(&point_2g, &point_double_g), 1);
//
//        // Negation.
//        let point_minus_7g_calc = group_neg(&point_7g_calc);
//        assert!(BLS12_381_G2_GENERATOR_MUL_BY_7_NEG_SERIALIZED_COMP == serialize(bls12_381_g2_serialization_scheme_compressed(), &point_minus_7g_calc), 1);
//        assert!(BLS12_381_G2_GENERATOR_MUL_BY_7_NEG_SERIALIZED_UNCOMP == serialize(bls12_381_g2_serialization_scheme_uncompressed(), &point_minus_7g_calc), 1);
//
//        // Addition.
//        let scalar_9 = from_u64<BLS12_381_Fr>(9);
//        let point_9g = group_scalar_mul(&point_g, &scalar_9);
//        let point_2g = group_scalar_mul(&point_g, &scalar_2);
//        let point_2g_calc = group_add(&point_minus_7g_calc, &point_9g);
//        assert!(eq(&point_2g, &point_2g_calc), 1);
//
//        // Multi-scalar multiplication.
//        let scalar_a = insecure_random_element<BLS12_381_Fr>();
//        let scalar_b = insecure_random_element<BLS12_381_Fr>();
//        let scalar_c = insecure_random_element<BLS12_381_Fr>();
//        let point_p = insecure_random_element<BLS12_381_G2>();
//        let point_q = insecure_random_element<BLS12_381_G2>();
//        let point_r = insecure_random_element<BLS12_381_G2>();
//        let naive = group_identity<BLS12_381_G2>();
//        naive = group_add(&naive, &group_scalar_mul(&point_p, &scalar_a));
//        naive = group_add(&naive, &group_scalar_mul(&point_q, &scalar_b));
//        naive = group_add(&naive, &group_scalar_mul(&point_r, &scalar_c));
//        let fast = group_multi_scalar_mul(&triplet(point_p, point_q, point_r), &triplet(scalar_a, scalar_b, scalar_c));
//        assert!(eq(&naive, &fast), 1);
//    }
//
//    const BLS12_381_R: vector<u8> = x"01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73";
//    const BLS12_381_FQ12_ONE_SERIALIZED: vector<u8> = x"010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
//    const BLS12_381_GT_GENERATOR_SERIALIZED: vector<u8> = x"b68917caaa0543a808c53908f694d1b6e7b38de90ce9d83d505ca1ef1b442d2727d7d06831d8b2a7920afc71d8eb50120f17a0ea982a88591d9f43503e94a8f1abaf2e4589f65aafb7923c484540a868883432a5c60e75860b11e5465b1c9a08873ec29e844c1c888cb396933057ffdd541b03a5220eda16b2b3a6728ea678034ce39c6839f20397202d7c5c44bb68134f93193cec215031b17399577a1de5ff1f5b0666bdd8907c61a7651e4e79e0372951505a07fa73c25788db6eb8023519a5aa97b51f1cad1d43d8aabbff4dc319c79a58cafc035218747c2f75daf8f2fb7c00c44da85b129113173d4722f5b201b6b4454062e9ea8ba78c5ca3cadaf7238b47bace5ce561804ae16b8f4b63da4645b8457a93793cbd64a7254f150781019de87ee42682940f3e70a88683d512bb2c3fb7b2434da5dedbb2d0b3fb8487c84da0d5c315bdd69c46fb05d23763f2191aabd5d5c2e12a10b8f002ff681bfd1b2ee0bf619d80d2a795eb22f2aa7b85d5ffb671a70c94809f0dafc5b73ea2fb0657bae23373b4931bc9fa321e8848ef78894e987bff150d7d671aee30b3931ac8c50e0b3b0868effc38bf48cd24b4b811a2995ac2a09122bed9fd9fa0c510a87b10290836ad06c8203397b56a78e9a0c61c77e56ccb4f1bc3d3fcaea7550f3503efe30f2d24f00891cb45620605fcfaa4292687b3a7db7c1c0554a93579e889a121fd8f72649b2402996a084d2381c5043166673b3849e4fd1e7ee4af24aa8ed443f56dfd6b68ffde4435a92cd7a4ac3bc77e1ad0cb728606cf08bf6386e5410f";
//    const BLS12_381_GT_GENERATOR_MUL_BY_7_SERIALIZED: vector<u8> = x"2041ea7b66c19680e2c0bb23245a71918753220b31f88a925aa9b1e192e7c188a0b365cb994b3ec5e809206117c6411242b940b10caa37ce734496b3b7c63578a0e3c076f9b31a7ca13a716262e0e4cda4ac994efb9e19893cbfe4d464b9210d099d808a08b3c4c3846e7529984899478639c4e6c46152ef49a04af9c8e6ff442d286c4613a3dac6a4bee4b40e1f6b030f2871dabe4223b250c3181ecd3bc6819004745aeb6bac567407f2b9c7d1978c45ee6712ae46930bc00638383f6696158bad488cbe7663d681c96c035481dbcf78e7a7fbaec3799163aa6914cef3365156bdc3e533a7c883d5974e3462ac6f19e3f9ce26800ae248a45c5f0dd3a48a185969224e6cd6af9a048241bdcac9800d94aeee970e08488fb961e36a769b6c185d185b4605dc9808517196bba9d00a3e37bca466c19187486db104ee03962d39fe473e276355618e44c965f05082bb027a7baa4bcc6d8c0775c1e8a481e77df36ddad91e75a982302937f543a11fe71922dcd4f46fe8f951f91cde412b359507f2b3b6df0374bfe55c9a126ad31ce254e67d64194d32d7955ec791c9555ea5a917fc47aba319e909de82da946eb36e12aff936708402228295db2712f2fc807c95092a86afd71220699df13e2d2fdf2857976cb1e605f72f1b2edabadba3ff05501221fe81333c13917c85d725ce92791e115eb0289a5d0b3330901bb8b0ed146abeb81381b7331f1c508fb14e057b05d8b0190a9e74a3d046dcd24e7ab747049945b3d8a120c4f6d88e67661b55573aa9b361367488a1ef7dffd967d64a1518";
//    const BLS12_381_GT_GENERATOR_MUL_BY_7_NEG_SERIALIZED: vector<u8> = x"1b44a9da32966426891f1c02c4505df254842dfe51d0bf218d9e143cc7fe225d2fd4ec860dd0c7f9b51c4f2958e4ca0645938a8b6cbeb5320385b06ada8b861036e4ce003eb4109f5c1ea207ac818a8134ba2180579fcae80b2fa2908273df0c3e2ee7e3ff90a88c712206619448e4a40d90dba80f630e2b464cc8d51bedf4bb2a770671c600256325f25bcbea071f01cdb76a7640cb017aa0a25a09ffd468437d150a133aa734d0a94b8b3b9f9046f5c0f2c7a2adc386bd906aee3ae7e131029d4005a382411df341ad084167286ff9537b24b720dd4f4e3e0a4b91fe01e433a393bef0a094cbed53bd06f022d4d310f84f9596ca68070f2bde9bc743984412e016986da381150be40bb63cdbc59719c84adf0225cc3a89054eed4eafa8cc19490d98f0a5a6080ea4e645d6e6ded27b4756f3434a7aea1a43533d4ce46a0db2d36500aafcdb60ac3e1ca45925667e091423b4ecbbcce5f85a2cd6feec6ff828406027c42bc87969cb091ebc19330510967253d11acff4b3f2e378f472603a0e2375ef80959e9c1ac8bb81275c214942d439663c23d91391495bc80199d6a79354c1df1452801886d20d46b6f773420d4de48d972c832e37eaf53917ab9db79fe02cf506bac7ca7b5c0d1577be5e082f54d2d7cc8e42d8fb1aef022c9aaf580e6d1225ddf1a809a23060aa925066c94a4fb6220bee4718200b6b49ee2c048db40be384e5147b9cbe679f91c20a7bb013137861463a117c0d659788a0021d0268c80dac43c9945a228409d8c9281c65b36fbcd8f8709f06bae250af319488f50e";
//
//    #[test(fx = @std)]
//    fun test_bls12_381_gt(fx: signer) {
//        enable_initial_generic_algebraic_operations(&fx);
//        enable_bls12_381_structures(&fx);
//
//        // Group info.
//        assert!(BLS12_381_R == group_order<BLS12_381_Gt>(), 1);
//
//        // Serialization/deserialization.
//        let element_g = group_generator<BLS12_381_Gt>();
//        assert!(BLS12_381_GT_GENERATOR_SERIALIZED == serialize(bls12_381_gt_serialization_scheme(), &element_g), 1);
//        let element_g_from_uncomp = std::option::extract(&mut deserialize<BLS12_381_Gt>(bls12_381_gt_serialization_scheme(), &BLS12_381_GT_GENERATOR_SERIALIZED));
//        assert!(eq(&element_g, &element_g_from_uncomp), 1);
//        let inf = group_identity<BLS12_381_Gt>();
//        assert!(BLS12_381_FQ12_ONE_SERIALIZED == serialize(bls12_381_gt_serialization_scheme(), &inf), 1);
//        let inf_from_uncomp = std::option::extract(&mut deserialize<BLS12_381_Gt>(bls12_381_gt_serialization_scheme(), &BLS12_381_FQ12_ONE_SERIALIZED));
//        assert!(eq(&inf, &inf_from_uncomp), 1);
//        let element_7g_from_uncomp = std::option::extract(&mut deserialize<BLS12_381_Gt>(bls12_381_gt_serialization_scheme(), &BLS12_381_GT_GENERATOR_MUL_BY_7_SERIALIZED));
//        assert!(std::option::is_none(&deserialize<BLS12_381_Gt>(bls12_381_gt_serialization_scheme(), &x"ffff")), 1);
//
//        // Element scalar multiplication.
//        let scalar_7 = from_u64<BLS12_381_Fr>(7);
//        let element_7g_calc = group_scalar_mul(&element_g, &scalar_7);
//        assert!(eq(&element_7g_calc, &element_7g_from_uncomp), 1);
//        assert!(BLS12_381_GT_GENERATOR_MUL_BY_7_NEG_SERIALIZED == serialize(bls12_381_gt_serialization_scheme(), &element_7g_calc), 1);
//
//        // Element negation.
//        let element_minus_7g_calc = group_neg(&element_7g_calc);
//        assert!(BLS12_381_GT_GENERATOR_MUL_BY_7_NEG_SERIALIZED == serialize(bls12_381_gt_serialization_scheme(), &element_minus_7g_calc), 1);
//
//        // Element addition.
//        let scalar_9 = from_u64<BLS12_381_Fr>(9);
//        let element_9g = group_scalar_mul(&element_g, &scalar_9);
//        let scalar_2 = from_u64<BLS12_381_Fr>(2);
//        let element_2g = group_scalar_mul(&element_g, &scalar_2);
//        let element_2g_calc = group_add(&element_minus_7g_calc, &element_9g);
//        assert!(eq(&element_2g, &element_2g_calc), 1);
//
//        // Upcasting to BLS12_381_Fq12.
//        let _fq12_element_from_cast = upcast<BLS12_381_Gt, BLS12_381_Fq12>(&element_minus_7g_calc);
//    }
//
//    #[test(fx = @std)]
//    fun test_bls12381_pairing(fx: signer) {
//        enable_initial_generic_algebraic_operations(&fx);
//        enable_bls12_381_structures(&fx);
//
//        // Single pairing.
//        let gt_point_1 = pairing<BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt>(
//            &group_scalar_mul(&group_generator<BLS12_381_G1>(), &from_u64<BLS12_381_Fr>(5)),
//            &group_scalar_mul(&group_generator<BLS12_381_G2>(), &from_u64<BLS12_381_Fr>(7)),
//        );
//        let gt_point_2 = pairing<BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt>(
//            &group_scalar_mul(&group_generator(), &from_u64<BLS12_381_Fr>(1)),
//            &group_scalar_mul(&group_generator(), &from_u64<BLS12_381_Fr>(35)),
//        );
//        let gt_point_3 = pairing<BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt>(
//            &group_scalar_mul(&group_generator<BLS12_381_G1>(), &from_u64<BLS12_381_Fr>(35)),
//            &group_scalar_mul(&group_generator<BLS12_381_G2>(), &from_u64<BLS12_381_Fr>(1)),
//        );
//        assert!(eq(&gt_point_1, &gt_point_2), 1);
//        assert!(eq(&gt_point_1, &gt_point_3), 1);
//
//        // Pairing with random points.
//        let g1_point = insecure_random_element<BLS12_381_G1>();
//        let g2_point = insecure_random_element<BLS12_381_G2>();
//        // e(k1*P1, k2*P2)
//        let k1 = insecure_random_element<BLS12_381_Fr>();
//        let k2 = insecure_random_element<BLS12_381_Fr>();
//        let gt_element = pairing<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(&group_scalar_mul(&g1_point, &k1), &group_scalar_mul(&g2_point, &k2));
//        // e(P1,P2)^(k1*k2)
//        let gt_element_another = group_scalar_mul(&pairing<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(&g1_point, &g2_point), &field_mul(&k1, &k2));
//        assert!(eq(&gt_element, &gt_element_another), 1);
//
//        // Multiple pairing.
//        let g1_point_1 = group_generator<BLS12_381_G1>();
//        let g2_point_1 = group_generator<BLS12_381_G2>();
//        let g1_point_2 = group_scalar_mul(&g1_point_1, &from_u64<BLS12_381_Fr>(5));
//        let g2_point_2 = group_scalar_mul(&g2_point_1, &from_u64<BLS12_381_Fr>(2));
//        let g1_point_3 = group_scalar_mul(&g1_point_1, &from_u64<BLS12_381_Fr>(20));
//        let g2_point_3 = group_scalar_mul(&g2_point_1, &from_u64<BLS12_381_Fr>(5));
//        let expected = group_scalar_mul(&pairing<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(&g1_point_1, &g2_point_1), &from_u64<BLS12_381_Fr>(111));
//        let actual = pairing_product<BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt>(&triplet(g1_point_1, g1_point_2, g1_point_3), &triplet(g2_point_1, g2_point_2, g2_point_3));
//        assert!(eq(&expected, &actual), 1);
//    }
//
//    #[test_only]
//    struct UnknownGroup {}
//
//    #[test(fx = @std)]
//    #[expected_failure(abort_code = 0x0c0000, location = Self)]
//    fun test_unknown_group(fx: signer) {
//        enable_initial_generic_algebraic_operations(&fx);
//        enable_bls12_381_structures(&fx);
//        let _ = group_order<UnknownGroup>();
//    }
//
//    #[test(fx = @std)]
//    #[expected_failure(abort_code = 0x0c0000, location = Self)]
//    fun test_unknown_pairing(fx: signer) {
//        enable_initial_generic_algebraic_operations(&fx);
//        enable_bls12_381_structures(&fx);
//        // Attempt an invalid pairing: (G2, G1) -> Gt
//        pairing<BLS12_381_G2, BLS12_381_G1, BLS12_381_Gt>(&insecure_random_element<BLS12_381_G2>(), &insecure_random_element<BLS12_381_G1>());
//    }

    #[test_only]
    fun triplet<T>(a: T, b: T, c: T): vector<T> {
        let ret = std::vector::empty();
        std::vector::push_back(&mut ret, a);
        std::vector::push_back(&mut ret, b);
        std::vector::push_back(&mut ret, c);
        ret
    }
}

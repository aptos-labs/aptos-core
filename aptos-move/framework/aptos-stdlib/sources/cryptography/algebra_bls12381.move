/// This module defines marker types, constants and test cases for working with BLS12-381 curves
/// using generic API defined in `algebra.move`.
///
/// Below are the BLS12-381 structures currently supported.
/// - Field `Fq12`.
/// - Group `G1Affine`.
/// - Group `G2Affine`.
/// - Group `Gt`.
/// - Field `Fr`.
module aptos_std::algebra_bls12381 {
    // Marker types (and their serialization schemes) begin.

    /// The finite field $F_r$ that can be used as the scalar fields
    /// for the groups $G_1$, $G_2$, $G_t$ in BLS12-381-based pairing.
    struct Fr {}

    /// A serialization scheme for `Fr` elements,
    /// where an element is represented by a byte array `b[]` of size 32 with the least significant byte coming first.
    ///
    /// NOTE: the same scheme is also used in other implementations (e.g., ark-bls12-381-0.4.0, blst-0.3.7).
    public fun format_bls12381fr_lsb(): u64 { 0x0a00000000000000 }

    /// A serialization scheme for `Fr` elements,
    /// where an element is represented by a byte array `b[]` of size 32 with the most significant byte coming first.
    ///
    /// NOTE: the same scheme is also used in other implementations (e.g., ark-bls12-381-0.4.0, blst-0.3.7).
    public fun format_bls12381fr_msb(): u64 { 0x0a01000000000000 }

    // Marker types (and their serialization schemes) end.

    // Tests begin.
    #[test_only]
    use aptos_std::algebra::{field_zero, field_one, field_is_zero, field_is_one, from_u64, eq, deserialize, serialize, field_neg, field_add, field_sub, field_mul, field_div, field_inv, insecure_random_element, field_sqr, enable_initial_generic_algebraic_operations};

    #[test_only]
    const BLS12_381_FR_VAL_7_SERIALIZED_LENDIAN: vector<u8> = x"0700000000000000000000000000000000000000000000000000000000000000";
    #[test_only]
    const BLS12_381_FR_VAL_7_SERIALIZED_BENDIAN: vector<u8> = x"0000000000000000000000000000000000000000000000000000000000000007";
    #[test_only]
    const BLS12_381_FR_VAL_7_NEG_SERIALIZED_LENDIAN: vector<u8> = x"fafffffffefffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73";

    #[test(fx = @std)]
    fun test_fr(fx: signer) {
        enable_initial_generic_algebraic_operations(&fx);

        // Special elements and checks.
        let val_0 = field_zero<Fr>();
        let val_1 = field_one<Fr>();
        assert!(field_is_zero(&val_0), 1);
        assert!(!field_is_zero(&val_1), 1);
        assert!(!field_is_one(&val_0), 1);
        assert!(field_is_one(&val_1), 1);

        // Serialization/deserialization.
        let val_7 = from_u64<Fr>(7);
        let val_7_2nd = std::option::extract(&mut deserialize<Fr>(
            format_bls12381fr_lsb(), &BLS12_381_FR_VAL_7_SERIALIZED_LENDIAN));
        let val_7_3rd = std::option::extract(&mut deserialize<Fr>(
            format_bls12381fr_msb(), &BLS12_381_FR_VAL_7_SERIALIZED_BENDIAN));
        assert!(eq(&val_7, &val_7_2nd), 1);
        assert!(eq(&val_7, &val_7_3rd), 1);
        assert!(BLS12_381_FR_VAL_7_SERIALIZED_LENDIAN == serialize(format_bls12381fr_lsb(), &val_7), 1);
        assert!(BLS12_381_FR_VAL_7_SERIALIZED_BENDIAN == serialize(format_bls12381fr_msb(), &val_7), 1);

        // Deserialization: byte array of right size but the value is not a member.
        assert!(std::option::is_none(&deserialize<Fr>(
            format_bls12381fr_lsb(), &x"01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73")), 1);
        assert!(std::option::is_none(&deserialize<Fr>(
            format_bls12381fr_msb(), &x"73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001")), 1);

        // Deserialization: byte array of wrong size.
        assert!(std::option::is_none(&deserialize<Fr>(
            format_bls12381fr_lsb(), &x"01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed7300")), 1);
        assert!(std::option::is_none(&deserialize<Fr>(
            format_bls12381fr_msb(), &x"0073eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001")), 1);
        assert!(std::option::is_none(&deserialize<Fr>(format_bls12381fr_lsb(), &x"ffff")), 1);
        assert!(std::option::is_none(&deserialize<Fr>(format_bls12381fr_msb(), &x"ffff")), 1);

        // Negation.
        let val_minus_7 = field_neg(&val_7);
        assert!(BLS12_381_FR_VAL_7_NEG_SERIALIZED_LENDIAN == serialize<Fr>(
            format_bls12381fr_lsb(), &val_minus_7), 1);

        // Addition.
        let val_9 = from_u64<Fr>(9);
        let val_2 = from_u64<Fr>(2);
        assert!(eq(&val_2, &field_add(&val_minus_7, &val_9)), 1);

        // Subtraction.
        assert!(eq(&val_9, &field_sub(&val_2, &val_minus_7)), 1);

        // Multiplication.
        let val_63 = from_u64<Fr>(63);
        assert!(eq(&val_63, &field_mul(&val_7, &val_9)), 1);

        // division.
        let val_0 = from_u64<Fr>(0);
        assert!(eq(&val_7, &std::option::extract(&mut field_div(&val_63, &val_9))), 1);
        assert!(std::option::is_none(&field_div(&val_63, &val_0)), 1);

        // Inversion.
        assert!(eq(&val_minus_7, &field_neg(&val_7)), 1);
        assert!(std::option::is_none(&field_inv(&val_0)), 1);

        // Squaring.
        let val_x = insecure_random_element<Fr>();
        assert!(eq(&field_mul(&val_x, &val_x), &field_sqr(&val_x)), 1);
    }

    // Tests end.
}

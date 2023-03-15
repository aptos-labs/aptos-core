/// This module defines marker types, constants and test cases for working with BLS12-381 curves
/// using generic API defined in `algebra.move`.
///
/// Currently supported BLS12-381 structures include field `Fr`.
module aptos_std::algebra_bls12381 {
    //
    // Marker types and their serialization schemes begin.
    //

    /// The finite field $F_r$ that can be used as the scalar fields
    /// for the groups $G_1$, $G_2$, $G_t$ in BLS12-381-based pairing.
    struct Fr {}

    /// A serialization format for `Fr` elements,
    /// where an element is represented by a byte array `b[]` of size 32 with the least significant byte coming first.
    ///
    /// NOTE: the same scheme is also used in other implementations (e.g., ark-bls12-381-0.4.0, blst-0.3.7).
    struct FrFormatLsb {}

    //
    // (Marker types and their serialization schemes end here).
    // Tests begin.
    //

    #[test_only]
    use aptos_std::algebra::{deserialize, serialize, add, enable_cryptography_algebra_natives};

    #[test_only]
    const BLS12_381_FR_VAL_0_SERIALIZED_LSB: vector<u8> = x"0000000000000000000000000000000000000000000000000000000000000000";
    #[test_only]
    const BLS12_381_FR_VAL_1_SERIALIZED_LSB: vector<u8> = x"0100000000000000000000000000000000000000000000000000000000000000";

    #[test(fx = @std)]
    fun test_fr(fx: signer) {
        enable_cryptography_algebra_natives(&fx);
        let val_0 = std::option::extract(&mut deserialize<Fr, FrFormatLsb>(
            &BLS12_381_FR_VAL_0_SERIALIZED_LSB));
        let val_1 = std::option::extract(&mut deserialize<Fr, FrFormatLsb>(
            &BLS12_381_FR_VAL_1_SERIALIZED_LSB));
        let sum = add(&val_0, &val_1);
        assert!(BLS12_381_FR_VAL_1_SERIALIZED_LSB == serialize<Fr, FrFormatLsb>(&sum), 1);
    }

    //
    // (Tests end here.)
    //
}

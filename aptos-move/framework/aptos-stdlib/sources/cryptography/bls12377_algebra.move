/// This module defines marker types and serialization format markers for working with BLS12-377
/// using the generic API defined in `aptos_std::crypto_algebra`.
///
/// The supported structures mirror those of `bls12381_algebra`:
/// - `Fr`: scalar field
/// - `Fq12`: target field of the pairing
/// - `G1`, `G2`, `Gt`: pairing-friendly groups
/// and their serialization format marker types.
module aptos_std::bls12377_algebra {
    //
    // Marker types + serialization formats begin.
    //

    /// The finite field F_{q^12} used in BLS12-377 curves.
    struct Fq12 {}

    /// A serialization format for `Fq12` matching arkworks little-endian, least-significant-coefficient-first layout.
    /// Byte length: 576 (12 limbs of 48 bytes).
    struct FormatFq12LscLsb {}

    /// The group G1 in BLS12-377 pairing.
    struct G1 {}
    /// Uncompressed G1 serialization (MSB format, 96 bytes).
    struct FormatG1Uncompr {}
    /// Compressed G1 serialization (MSB format, 48 bytes).
    struct FormatG1Compr {}

    /// The group G2 in BLS12-377 pairing.
    struct G2 {}
    /// Uncompressed G2 serialization (MSB format, 192 bytes).
    struct FormatG2Uncompr {}
    /// Compressed G2 serialization (MSB format, 96 bytes).
    struct FormatG2Compr {}

    /// The target group Gt in BLS12-377 pairing (a prime-order subgroup of `Fq12`).
    struct Gt {}
    /// Serialization format for `Gt` (delegates to `FormatFq12LscLsb`).
    struct FormatGt {}

    /// The scalar field Fr for the BLS12-377 pairing groups.
    struct Fr {}
    /// Little-endian 32-byte serialization for `Fr`.
    struct FormatFrLsb {}
    /// Big-endian 32-byte serialization for `Fr`.
    struct FormatFrMsb {}
}


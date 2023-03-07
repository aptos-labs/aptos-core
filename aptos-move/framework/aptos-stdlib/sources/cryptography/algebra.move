/// Module `algebra` provides structs/functions for doing arithmetic and other common operations
/// on algebraic structures (mostly groups and fields) that are widely used in cryptographic systems.
///
/// Different from existing modules like `ristretto255.move`, the functions here are generic.
/// Typically, each function represent an operation defined for ANY group/field
/// and require 1 (or 2+) marker type(s) which represents the actual structure(s) to work with.
/// See the test cases in `algebra.move` for more examples.
///
/// The generic APIs should allow Move developers to build generic cryptographic schemes on top of them
/// and use the schemes with different underlying algebraic structures by simply changing some type parameters.
/// E.g., Groth16 proof verifier that accepts a generic pairing is now possible.
///
/// Below are the structures currently supported.
/// - BLS12-381 structures.
///   - Group `BLS12_381_G1`.
///   - Group `BLS12_381_G2`.
///   - Group `BLS12_381_Gt`.
///   - Field `BLS12_381_Fq12`.
///   - Field `BLS12_381_Fr`.
///
/// Below are the operations currently supported.
/// - Serialization/deserialization.
/// - Group operations.
///   - Getting group order.
///   - Getting group identity.
///   - Getting group generator.
///   - Addition.
///   - Subtraction.
///   - Negation.
///   - Efficient sclar multiplication.
///   - Efficient doubling.
///   - Equal-to-identity check.
/// - Field operations.
///   - Getting additive identity.
///   - Getting multiplicative identity.
///   - Conversion from integers.
///   - Addition.
///   - Negation.
///   - Subtraction.
///   - Multiplication.
///   - Inversion.
///   - Division.
///   - Efficient squaring.
///   - Equal-to-additive-identity check.
///   - Equal-to-multiplicative-identity check.
/// - Equality check.
/// - Upcasting/downcasting between structures.
///
/// Note: in `algebra.move` additive group notions are used.
module aptos_std::algebra {
    use std::option::{Option, some, none};
    use aptos_std::type_info::type_of;
    use std::features::{bls12_381_structures_enabled, generic_algebraic_structures_basic_operations_enabled};

    // Marker types (and their serialization schemes) begin.

    /// The finite field $F_q$ used in BLS12-381 curves.
    /// It has a prime order $q$ equal to 0x1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab.
    ///
    /// NOTE: currently information-only and no operations are implemented for this structure.
    struct BLS12_381_Fq {}

    /// Return the ID of a serialization scheme,
    /// where a `BLS12_381_Fq` element is represented by a byte array `b[]` of size 48 using little-endian byte order.
    ///
    /// NOTE: currently information-only, not implemented.
    public fun bls12_381_fq_format(): vector<u8> { x"01" }

    /// A serialization format for `BLS12_381_Fq` elements.
    /// In this format, an element is represented by a byte array `b[]` of size 48 using little-endian byte order.
    ///
    /// NOTE: currently information-only, not implemented.
    struct BLS12_381_Fq_Format {}

    /// Return the ID of a serialization scheme,
    /// where a `BLS12_381_Fq` element is represented by a byte array `b[]` of size 48 using big-endian byte order.
    ///
    /// NOTE: currently information-only, not implemented.
    public fun bls12_381_fq_bendian_format(): vector<u8> { x"0101" }

    /// A serialization format for `BLS12_381_Fq` elements.
    /// In this format, an element is represented by a byte array `b[]` of size 48 using big-endian byte order.
    ///
    /// NOTE: currently information-only, not implemented.
    struct BLS12_381_Fq_Format_BEndian {}

    /// The finite field $F_{q^2}$ used in BLS12-381 curves.
    /// It is an extension field of `BLS12_381_Fq`, constructed as $F_{q^2}=F_q[u]/(u^2+1)$.
    ///
    /// NOTE: currently information-only and no operations are implemented for this structure.
    struct BLS12_381_Fq2 {}

    /// Return the ID of a serialization scheme,
    /// where a `BLS12_381_Fq2` element $(c_0+c_1\cdot u)$ is represented by a byte array `b[]` of size 96.
    /// `b[0..48]` is $c_0$ serialized in `bls12_381_fq_format`.
    /// `b[48..96]` is $c_1$ serialized in `bls12_381_fq_format`.
    ///
    /// NOTE: currently information-only, not implemented.
    public fun bls12_381_fq2_format(): vector<u8> { x"02" }

    /// A serialization scheme for `BLS12_381_Fq2` elements.
    ///In this format, an element in the form $(c_0+c_1\cdot u)$ is represented by a byte array `b[]` of size 96.
    /// `b[0..48]` is $c_0$ serialized in `BLS12_381_Fq_Format`.
    /// `b[48..96]` is $c_1$ serialized in `BLS12_381_Fq_Format`.
    ///
    /// NOTE: currently information-only, not implemented.
    struct BLS12_381_Fq2_Format {}

    /// A serialization scheme for `BLS12_381_Fq2` elements.
    /// In this format, an element in the form $(c_0+c_1\cdot u)$ is represented by a byte array `b[]` of size 96.
    /// `b[0..48]` is $c_0$ serialized in `BLS12_381_Fq_Format_BEndianFq`.
    /// `b[48..96]` is $c_1$ serialized in `BLS12_381_Fq_Format_BEndianFq`.
    ///
    /// NOTE: currently information-only, not implemented.
    struct BLS12_381_Fq2_Format_BEndianFq {}

    /// The finite field $F_{q^6}$ used in BLS12-381 curves.
    /// It is an extension field of `BLS12_381_Fq2`, constructed as $F_{q^6}=F_{q^2}[v]/(v^3-u-1)$.
    ///
    /// NOTE: currently information-only and no operations are implemented for this structure.
    struct BLS12_381_Fq6 {}

    /// Return the ID of a serialization scheme,
    /// where a `BLS12_381_Fq6` element $(c_0+c_1\cdot v+c_2\cdot v^2)$ is represented by a byte array `b[]` of size 288.
    /// `b[0..96]` is $c_0$ serialized in `bls12_381_fq2_format`.
    /// `b[96..192]` is $c_1$ serialized in `bls12_381_fq2_format`.
    /// `b[192..288]` is $c_2$ serialized in `bls12_381_fq2_format`.
    ///
    /// NOTE: currently information-only, not implemented.
    public fun bls12_381_fq6_format(): vector<u8> { x"03" }

    /// A serialization scheme for `BLS12_381_Fq6` elements.
    ///
    /// In this format, an element $(c_0+c_1\cdot v+c_2\cdot v^2)$ is represented by a byte array `b[]` of size 288.
    /// `b[0..96]` is $c_0$ serialized in `BLS12_381_Fq2_Format`.
    /// `b[96..192]` is $c_1$ serialized in `BLS12_381_Fq2_Format`.
    /// `b[192..288]` is $c_2$ serialized in `BLS12_381_Fq2_Format`.
    ///
    /// NOTE: currently information-only, not implemented.
    struct BLS12_381_Fq6_Format {}

    /// The finite field $F_{q^12}$ used in BLS12-381 curves.
    /// It is an extension field of `BLS12_381_Fq6`, constructed as $F_{q^12}=F_{q^6}[w]/(w^2-v)$.
    struct BLS12_381_Fq12 {}

    /// Return the ID of a serialization scheme,
    /// where a `BLS12_381_Fq12` element $(c_0+c_1\cdot w)$ is represented by a byte array `b[]` of size 576.
    /// `b[0..288]` is $c_0$ serialized in `bls12_381_fq6_format`.
    /// `b[288..576]` is $c_1$ serialized in `bls12_381_fq6_format`.
    ///
    /// NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.3.0).
    public fun bls12_381_fq12_format(): vector<u8> { x"04" }

    /// A serialization scheme for `BLS12_381_Fq12` elements.

    /// In this format, an element $(c_0+c_1\cdot w)$ is represented by a byte array `b[]` of size 576.
    /// `b[0..288]` is $c_0$ serialized in `BLS12_381_Fq6_Format`.
    /// `b[288..576]` is $c_1$ serialized in `BLS12_381_Fq6_Format`.
    ///
    /// NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.3.0).
    struct BLS12_381_Fq12_Format {}

    /// A group constructed by the points on the BLS12-381 curve $E(F_q): y^2=x^3+4$ and the point at inifinity,
    /// under the elliptic curve point addition.
    /// It contains the prime-order subgroup $G_1$ used in pairing.
    /// The identity is the point at infinity.
    ///
    /// NOTE: currently information-only and no operations are implemented for this structure.
    struct BLS12_381_G1_Parent {}

    /// Return the ID of a serialization scheme,
    /// where an `BLS12_381_G1_Parent` element is represented by a byte array `b[]` of size 96.
    /// `b[95] & 0x40` is the infinity flag.
    /// The infinity flag is 1 if and only if the element is the point at infinity.
    /// The infinity flag is 0 if and only if the element is a point $(x,y)$ on curve $E(F_q)$,
    /// `[b[0], ..., b[47] & 0x3f]` is $x$ serialized in `bls12_381_fq_format`, and
    /// `[b[48], ..., b[95] & 0x3f]` is $y$ serialized in `bls12_381_fq_format`.
    ///
    /// NOTE: currently information-only, not implemented.
    public fun bls12_381_g1_parent_uncompressed_format(): vector<u8> { x"05" }

    /// A serialization scheme for `BLS12_381_G1_Parent` elements.
    ///
    /// In this format, an element is represented by a byte array `b[]` of size 96.
    /// `b[95] & 0x40` is the infinity flag.
    /// The infinity flag is 1 if and only if the element is the point at infinity.
    /// The infinity flag is 0 if and only if the element is a point $(x,y)$ on curve $E(F_q)$,
    /// `[b[0], ..., b[47] & 0x3f]` is $x$ serialized in `bls12_381_fq_format`, and
    /// `[b[48], ..., b[95] & 0x3f]` is $y$ serialized in `bls12_381_fq_format`.
    ///
    /// NOTE: currently information-only, not implemented.
    struct BLS12_381_G1_Parent_Format_Uncompressed {}

    /// A serialization scheme for `BLS12_381_G1_Parent` elements.
    ///
    /// In this format, an element is represented by a byte array `b[]` of size 96.
    /// `b[95] & 0x40` is the infinity flag.
    /// The infinity flag is 1 if and only if the element is the point at infinity.
    /// The infinity flag is 0 if and only if the element is a point $(x,y)$ on curve $E(F_q)$,
    /// `[b[0], ..., b[47] & 0x3f]` is $x$ serialized in `BLS12_381_Fq_Format_BEndian`, and
    /// `[b[48], ..., b[95] & 0x3f]` is $y$ serialized in `BLS12_381_Fq_Format_BEndian`.
    ///
    /// NOTE: currently information-only, not implemented.
    struct BLS12_381_G1_Parent_Format_Uncompressed_BEndianFq {}

    /// Return the ID of a serialization scheme,
    /// where an `BLS12_381_G1_Parent` element is represented by a byte array `b[]` of size 48.
    /// `b[47] & 0x40` is the infinity flag.
    /// The infinity flag is 1 if and only if the element is the point at infinity.
    /// The infinity flag is 0 if and only if the element is a point $(x,y)$ on curve $E(Fq)$,
    /// `[b[0], ..., b[47] & 0x3f]` is $x$ serialized in `bls12_381_fq_format`, and
    /// the positiveness flag `b_47 & 0x80` is 1 if and only if $y > -y$ ($y$ and $-y$ treated as unsigned integers).
    ///
    /// NOTE: currently information-only, not implemented.
    public fun bls12_381_g1_parent_compressed_format(): vector<u8> { x"0501" }


    /// A serialization scheme for `BLS12_381_G1_Parent` elements.
    ///
    /// In this format, an element is represented by a byte array `b[]` of size 48.
    /// `b[47] & 0x40` is the infinity flag.
    /// The infinity flag is 1 if and only if the element is the point at infinity.
    /// The infinity flag is 0 if and only if the element is a point $(x,y)$ on curve $E(Fq)$,
    /// `[b[0], ..., b[47] & 0x3f]` is $x$ serialized in `bls12_381_fq_format`, and
    /// the positiveness flag `b_47 & 0x80` is 1 if and only if $y > -y$ ($y$ and $-y$ treated as unsigned integers).
    ///
    /// NOTE: currently information-only, not implemented.
    struct BLS12_381_G1_Parent_Format_Compressed {}

    /// A serialization scheme for `BLS12_381_G1_Parent` elements.
    ///
    /// In this format, an element is represented by a byte array `b[]` of size 48.
    /// `b[47] & 0x40` is the infinity flag.
    /// The infinity flag is 1 if and only if the element is the point at infinity.
    /// The infinity flag is 0 if and only if the element is a point $(x,y)$ on curve $E(Fq)$,
    /// `[b[0], ..., b[47] & 0x3f]` is $x$ serialized in `BLS12_381_Fq_Format_BEndian`, and
    /// the positiveness flag `b_47 & 0x80` is 1 if and only if $y > -y$ ($y$ and $-y$ treated as unsigned integers).
    ///
    /// NOTE: currently information-only, not implemented.
    struct BLS12_381_G1_Parent_Format_Compressed_BEndianFq {}

    /// The group $G_1$ in BLS12-381-based pairing $G_1 \times G_2 \rightarrow G_t$.
    /// It is subgroup of `BLS12_381_G1_Parent`.
    /// It has a prime order $r$ equal to 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.
    /// (so `BLS12_381_Fr` is the scalar field).
    struct BLS12_381_G1 {}

    /// Return the ID of a serialization scheme,
    /// which is essentially `bls12_381_g1_parent_uncompressed_format()` but only applicable to `BLS12_381_G1` elements.
    ///
    /// NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.3.0).
    public fun bls12_381_g1_uncompressed_format(): vector<u8> { x"06" }

    /// A serialization scheme for `BLS12_381_G1` elements.
    ///
    /// Essentially `BLS12_381_G1_Parent_Format_Uncompressed` but only applied to `BLS12_381_G1` elements.
    struct BLS12_381_G1_Format_Uncompressed {}

    /// A serialization scheme for `BLS12_381_G1` elements.
    ///
    /// Essentially `BLS12_381_G1_Parent_Format_Uncompressed_BEndianFq` but only applied to `BLS12_381_G1` elements.
    struct BLS12_381_G1_Format_Uncompressed_BEndianFq {}

    /// Return the ID of a serialization scheme,
    /// which is essentially `bls12_381_g1_parent_compressed_format()` but only applicable to `BLS12_381_G1` elements.
    ///
    /// NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.3.0).
    public fun bls12_381_g1_compressed_format(): vector<u8> { x"0601" }

    /// A serialization scheme for `BLS12_381_G1` elements.
    ///
    /// Essentially `BLS12_381_G1_Parent_Format_Compressed` but only applied to `BLS12_381_G1` elements.
    struct BLS12_381_G1_Format_Compressed {}

    /// A serialization scheme for `BLS12_381_G1` elements.
    ///
    /// Essentially `BLS12_381_G1_Parent_Format_Compressed_BEndianFq` but only applied to `BLS12_381_G1` elements.
    struct BLS12_381_G1_Format_Compressed_BEndianFq {}

    /// A group constructed by the points on a curve $E(F_{q^2})$ and the point at inifinity under the elliptic curve point addition.
    /// $E(F_{q^2})$ is an elliptic curve $y^2=x^3+4(u+1)$ defined over $F_{q^2}$.
    /// The identity of `BLS12_381_G2` is the point at infinity.
    ///
    /// NOTE: currently information-only and no operations are implemented for this structure.
    struct BLS12_381_G2_Parent {}

    /// Return the ID of a serialization scheme,
    /// where a `BLS12_381_G2_Parent` element is represented by a byte array `b[]` of size 192.
    /// `b[191] & 0x40` is the infinity flag.
    /// The infinity flag is 1 if and only if the element is the point at infinity.
    /// The infinity flag is 0 if and only if the element is a point $(x,y)$ on curve $E(F_{q^2})$,
    /// `b[0..96]` is $x$ serialized in `BLS12_381_Fq2_Format`, and
    /// `[b[96], ..., b[191] & 0x3f]` is $y$ serialized in `BLS12_381_Fq2_Format`.
    ///
    /// NOTE: currently information-only, not implemented.
    public fun bls12_381_g2_parent_uncompressed_format(): vector<u8> { x"07" }

    /// A serialization scheme for `BLS12_381_G2_Parent` elements.
    ///
    /// In this format, an element is represented by a byte array `b[]` of size 192.
    /// `b[191] & 0x40` is the infinity flag.
    /// The infinity flag is 1 if and only if the element is the point at infinity.
    /// The infinity flag is 0 if and only if the element is a point $(x,y)$ on curve $E(F_{q^2})$,
    /// `b[0..96]` is $x$ serialized in `BLS12_381_Fq2_Format`, and
    /// `[b[96], ..., b[191] & 0x3f]` is $y$ serialized in `BLS12_381_Fq2_Format`.
    ///
    /// NOTE: currently information-only, not implemented.
    struct BLS12_381_G2_Parent_Format_Uncompressed {}

    /// Return the ID of a serialization scheme,
    /// where a `BLS12_381_G2_Parent` element is represented by a byte array `b[]` of size 96.
    /// `b[95] & 0x40` is the infinity flag.
    /// The infinity flag is 1 if and only if the element is the point at infinity.
    /// The infinity flag is 0 if and only if the element is a point $(x,y)$ on curve $E(F_{q^2})$,
    /// `[b[0], ..., b[95] & 0x3f]` is $x$ serialized in `BLS12_381_Fq2_Format`, and
    /// the positiveness flag `b[95] & 0x80` is 1 if and only if $y > -y$ ($y$ and $-y$ treated as unsigned integers).
    ///
    /// NOTE: currently information-only, not implemented.
    public fun bls12_381_g2_parent_compressed_format(): vector<u8> { x"0701" }

    /// A serialization scheme for `BLS12_381_G2_Parent` elements.
    ///
    /// In this format, an element is represented by a byte array `b[]` of size 96.
    /// `b[95] & 0x40` is the infinity flag.
    /// The infinity flag is 1 if and only if the element is the point at infinity.
    /// The infinity flag is 0 if and only if the element is a point $(x,y)$ on curve $E(F_{q^2})$,
    /// `[b[0], ..., b[95] & 0x3f]` is $x$ serialized in `BLS12_381_Fq2_Format`, and
    /// the positiveness flag `b[95] & 0x80` is 1 if and only if $y > -y$ ($y$ and $-y$ treated as unsigned integers).
    ///
    /// NOTE: currently information-only, not implemented.
    struct BLS12_381_G2_Parent_Format_Compressed {}

    /// The group $G_2$ in BLS12-381-based pairing $G_1 \times G_2 \rightarrow G_t$.
    /// It is a subgroup of `BLS12_381_G2_Parent`.
    /// It has a prime order $r$ equal to 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.
    /// (so `BLS12_381_Fr` is the scalar field).
    struct BLS12_381_G2 {}

    /// Return the ID of a serialization scheme,
    /// which is essentially `bls12_381_g2_parent_uncompressed_format()` but only applicable to `BLS12_381_G2` elements.
    ///
    /// NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.3.0).
    public fun bls12_381_g2_uncompressed_format(): vector<u8> { x"08" }

    /// A serialization scheme for `BLS12_381_G2` elements.
    ///
    /// Essentially `BLS12_381_G2_Parent_Format_Uncompressed` but only applicable to `BLS12_381_G2` elements.
    ///
    /// NOTE: currently information-only, not implemented.
    struct BLS12_381_G2_Format_Uncompressed {}

    /// Return the ID of a serialization scheme,
    /// which is essentially `bls12_381_g2_parent_compressed_format()` but only applicable to `BLS12_381_G2` elements.
    ///
    /// NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.3.0).
    public fun bls12_381_g2_compressed_format(): vector<u8> { x"0801" }

    /// A serialization scheme for `BLS12_381_G2` elements.
    ///
    /// Essentially `BLS12_381_G2_Parent_Format_Compressed` but only applicable to `BLS12_381_G2` elements.
    ///
    /// NOTE: currently information-only, not implemented.
    struct BLS12_381_G2_Format_Compressed {}

    /// The group $G_2$ in BLS12-381-based pairing $G_1 \times G_2 \rightarrow G_t$.
    /// It is a multiplicative subgroup of `BLS12_381_Fq12`.
    /// It has a prime order $r$ equal to 0x73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001.
    /// (so `BLS12_381_Fr` is the scalar field).
    /// The identity of `BLS12_381_Gt` is 1.
    struct BLS12_381_Gt {}

    /// Return the ID of a serialization scheme,
    /// which is essentially `bls12_381_fq12_format()` but only applicable to `BLS12_381_Gt` elements.
    ///
    /// NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.3.0).
    public fun bls12_381_gt_format(): vector<u8> { x"09" }

    /// A serialization scheme for `BLS12_381_Gt` elements.
    ///
    /// Essentially `BLS12_381_Fq12_Format` but only applicable to `BLS12_381_Gt` elements.
    ///
    /// NOTE: the same scheme is also used in other implementations (e.g. ark-bls12-381-0.3.0).
    struct BLS12_381_Gt_Format {}

    /// The finite field $F_r$ that can be used as the scalar fields
    /// for the groups $G_1$, $G_2$, $G_t$ in BLS12-381-based pairing.
    struct BLS12_381_Fr {}

    /// Return the ID of a serialization scheme,
    /// where a `BLS12_381_Fr` element is represented by a byte array `b[]` of size 32 using little-endian byte order.
    ///
    /// NOTE: the same scheme is also used in other implementations (e.g., ark-bls12-381-0.3.0, blst-0.3.7).
    public fun bls12_381_fr_lendian_format(): vector<u8> { x"0a" }

    /// A serialization scheme for `BLS12_381_Fr` elements.
    ///
    /// In this format, an element is represented by a byte array `b[]` of size 32 using little-endian byte order.
    ///
    /// NOTE: the same scheme is also used in other implementations (e.g., ark-bls12-381-0.3.0, blst-0.3.7).
    struct BLS12_381_Fr_Format_LEndian {}

    /// Return the ID of a serialization scheme,
    /// where a `BLS12_381_Fr` element is represented by a byte array `b[]` of size 32 using big-endian byte order.
    ///
    /// NOTE: the same scheme is also used in other implementations (e.g., blst-0.3.7).
    public fun bls12_381_fr_bendian_format(): vector<u8> { x"0a01" }

    /// A serialization scheme for `BLS12_381_Fr` elements.
    ///
    /// In this format, an element is represented by a byte array `b[]` of size 32 using big-endian byte order.
    ///
    /// NOTE: the same scheme is also used in other implementations (e.g., ark-bls12-381-0.3.0, blst-0.3.7).
    struct BLS12_381_Fr_Format_BEndian {}

    // Marker types (and their serialization schemes) end.

    // Hash-to-structure suites begin.
    /// A ciphersuite for hashing bytes to a `BLS12_381_G1` element.
    /// Defined in https://datatracker.ietf.org/doc/draft-irtf-cfrg-hash-to-curve/.
    struct HASH_SUITE_BLS12381G1_XMD_SHA_256_SSWU_RO_ {}
    // Hash-to-structure suites end.

    /// This struct represents an element of an algebraic structure `S`.
    struct Element<phantom S> has copy, drop {
        handle: u64
    }

    // Public functions begin.

    /// Compute a pre-compiled pairing function (a.k.a., bilinear map) on `element_1` and `element_2`.
    /// Return an element in the target group `Gt`.
    public fun pairing<G1,G2,Gt>(element_1: &Element<G1>, element_2: &Element<G2>): Element<Gt> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_triplet_enabled_for_pairing<G1,G2,Gt>();
        Element<Gt> {
            handle: pairing_internal<G1,G2,Gt>(element_1.handle, element_2.handle)
        }
    }

    /// Check if `x == y` for elements `x` and `y` of an algebraic structure `S`.
    public fun eq<S>(x: &Element<S>, y: &Element<S>): bool {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_enabled_for_basic_operation<S>();
        eq_internal<S>(x.handle, y.handle)
    }

    /// Convert a u64 to an element of an algebraic structure `S`.
    public fun from_u64<S>(value: u64): Element<S> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_enabled_for_basic_operation<S>();
        Element<S> {
            handle: from_u64_internal<S>(value)
        }
    }

    /// Return the additive identity of a field `S`.
    public fun field_zero<S>(): Element<S> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_enabled_for_basic_operation<S>();
        Element<S> {
            handle: field_zero_internal<S>()
        }
    }

    /// Return the multiplicative identity of a field `S`.
    public fun field_one<S>(): Element<S> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_enabled_for_basic_operation<S>();
        Element<S> {
            handle: field_one_internal<S>()
        }
    }

    /// Compute `-x` for an element `x` of a field `S`.
    public fun field_neg<S>(x: &Element<S>): Element<S> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_enabled_for_basic_operation<S>();
        Element<S> {
            handle: field_neg_internal<S>(x.handle)
        }
    }

    /// Compute `x + y` for elements `x` and `y` of a field `S`.
    public fun field_add<S>(x: &Element<S>, y: &Element<S>): Element<S> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_enabled_for_basic_operation<S>();
        Element<S> {
            handle: field_add_internal<S>(x.handle, y.handle)
        }
    }

    /// Compute `x - y` for elements `x` and `y` of a field `S`.
    public fun field_sub<S>(x: &Element<S>, y: &Element<S>): Element<S> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_enabled_for_basic_operation<S>();
        Element<S> {
            handle: field_sub_internal<S>(x.handle, y.handle)
        }
    }

    /// Compute `x * y` for elements `x` and `y` of a field `S`.
    public fun field_mul<S>(x: &Element<S>, y: &Element<S>): Element<S> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_enabled_for_basic_operation<S>();
        Element<S> {
            handle: field_mul_internal<S>(x.handle, y.handle)
        }
    }

    /// Try computing `x / y` for elements `x` and `y` of a field `S`.
    /// Return none if y is the additive identity of field `S`.
    public fun field_div<S>(x: &Element<S>, y: &Element<S>): Option<Element<S>> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_enabled_for_basic_operation<S>();
        let (succ, handle) = field_div_internal<S>(x.handle, y.handle);
        if (succ) {
            some(Element<S> { handle })
        } else {
            none()
        }
    }

    /// Compute `x^2` for an element `x` of a field `S`.
    ///
    public fun field_sqr<S>(x: &Element<S>): Element<S> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_enabled_for_basic_operation<S>();
        Element<S> {
            handle: field_sqr_internal<S>(x.handle)
        }
    }

    /// Try computing `x^(-1)` for an element `x` of a field `S`.
    /// Return none if `x` is the additive identity of field `S`.
    public fun field_inv<S>(x: &Element<S>): Option<Element<S>> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_enabled_for_basic_operation<S>();
        let (succeeded, handle) = field_inv_internal<S>(x.handle);
        if (succeeded) {
            let scalar = Element<S> { handle };
            some(scalar)
        } else {
            none()
        }
    }

    /// Check if an element `x` is the multiplicative identity of field `S`.
    public fun field_is_one<S>(x: &Element<S>): bool {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_enabled_for_basic_operation<S>();
        field_is_one_internal<S>(x.handle)
    }

    /// Check if an element `x` is the aditive identity of field `S`.
    public fun field_is_zero<S>(x: &Element<S>): bool {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_enabled_for_basic_operation<S>();
        field_is_zero_internal<S>(x.handle)
    }

    /// Compute `P + Q` for elements `P` and `Q` of a group `G`.
    public fun group_add<G>(element_p: &Element<G>, element_q: &Element<G>): Element<G> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_enabled_for_basic_operation<G>();
        Element<G> {
            handle: group_add_internal<G>(element_p.handle, element_q.handle)
        }
    }

    /// Compute `2*P` for an element `P` of a group `G`. Faster and cheaper than `P + P`.
    public fun group_double<G>(element_p: &Element<G>): Element<G> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_enabled_for_basic_operation<G>();
        Element<G> {
            handle: group_double_internal<G>(element_p.handle)
        }
    }

    /// Get the fixed generator of a cyclic group `G`.
    public fun group_generator<G>(): Element<G> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_enabled_for_basic_operation<G>();
        Element<G> {
            handle: group_generator_internal<G>()
        }
    }

    /// Get the identity of a group `G`.
    public fun group_identity<G>(): Element<G> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_enabled_for_basic_operation<G>();
        Element<G> {
            handle: group_identity_internal<G>()
        }
    }

    /// Compute `k[0]*P[0]+...+k[n-1]*P[n-1]` where `P[]` are `n` elements of group `G`,
    /// and `k[]` are `n` scalars represented by a byte array `scalars`.
    /// `k[]` will be parsed assuming `bin(k[0]) || ... || bin(k[n-1]) == scalar_bin[0..w*n]`, where
    /// `w` is the scalar bit length, specified by parameter `scalar_size_in_bits`,
    /// `bin(x)` is the least-significant-bit-first `w`-bit representation of an integer `x`,
    /// `||` is bit array concatenation,
    /// `scalar_bin` is all `bin(scalar[i])` concatenated, then extended at the back with 0s if not long enough.
    ///
    /// NOTE: in some groups, this function is much faster and cheaper than
    /// calling `element_scalar_mul` and adding up the results using `scalar_add`.
    public fun group_multi_scalar_mul<G>(elements: &vector<Element<G>>, scalars: &vector<u8>, scalar_size_in_bits: u64): Element<G> {
        let element_handles = handles_from_elements(elements);
        Element<G> {
            handle: group_multi_scalar_mul_internal<G>(element_handles, scalars, scalar_size_in_bits)
        }
    }

    /// Compute `k[0]*P[0]+...+k[n-1]*P[n-1]` where `P[]` are `n` elements of group `G`
    /// and `k[]` are `n` elements of the scalarfield `S` of group `G`.
    public fun group_multi_scalar_mul_typed<G, S>(elements: &vector<Element<G>>, scalars: &vector<Element<S>>): Element<G> {
        let element_handles = handles_from_elements(elements);
        let scalar_handles = handles_from_elements(scalars);
        Element<G> {
            handle: group_multi_scalar_mul_typed_internal<G>(element_handles, scalar_handles)
        }
    }

    fun handles_from_elements<S>(elements: &vector<Element<S>>): vector<u64> {
        let num_elements = std::vector::length(elements);
        let element_handles = std::vector::empty();
        let i = 0;
        while (i < num_elements) {
            std::vector::push_back(&mut element_handles, std::vector::borrow(elements, i).handle);
            i = i + 1;
        };
        element_handles
    }

    /// Compute `-P` for an element `P` of a group `G`.
    public fun group_neg<G>(element_p: &Element<G>): Element<G> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_enabled_for_basic_operation<G>();
        Element<G> {
            handle: group_neg_internal<G>(element_p.handle)
        }
    }

    /// Compute `k*P`, where `P` is an element of a group `G` and `k` is an element of the scalar field `S` of group `G`.
    public fun group_scalar_mul_typed<G, S>(element_p: &Element<G>, scalar_k: &Element<S>): Element<G> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_pair_enabled_for_group_scalar_mul<G,S>();
        Element<G> {
            handle: group_scalar_mul_typed_internal<G, S>(element_p.handle, scalar_k.handle)
        }
    }

    /// Compute `k*P`, where `P` is an element of a group `G` and `k` is an element of the scalar field `S` of group `G`.
    public fun group_scalar_mul<G>(element_p: &Element<G>, scalar_encoded: &vector<u8>): Element<G> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_enabled_for_basic_operation<G>();
        Element<G> {
            handle: group_scalar_mul_internal<G>(element_p.handle, scalar_encoded)
        }
    }

    /// Compute `P - Q` for elements `P` and `Q` of a group `G`.
    public fun group_sub<G>(element_p: &Element<G>, element_q: &Element<G>): Element<G> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_enabled_for_basic_operation<G>();
        Element<G> {
            handle: group_sub_internal<G>(element_p.handle, element_q.handle)
        }

    }

    /// Try deserializing a byte array to an element of an algebraic structure `S` using a given `scheme`.
    /// Return none if the deserialization failed.
    public fun deserialize<S>(scheme: vector<u8>, bytes: &vector<u8>): Option<Element<S>> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_serialization_scheme_enabled<S>(scheme);
        let (succeeded, handle) = deserialize_internal<S>(scheme, bytes);
        if (succeeded) {
            some(Element<S> { handle })
        } else {
            none()
        }
    }

    /// Try deserializing a byte array to an element of an algebraic structure `S` using a given format `F`.
    /// Return none if the deserialization failed.
    public fun deserialize_v2<S, F>(bytes: &vector<u8>): Option<Element<S>> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_serialization_format_enabled<S, F>();
        let (succeeded, handle) = deserialize_v2_internal<S,F>(bytes);
        if (succeeded) {
            some(Element<S> { handle })
        } else {
            none()
        }
    }

    /// Serialize an element of an algebraic structure `S` to a byte array using a given `scheme`.
    public fun serialize<S>(scheme: vector<u8>, element: &Element<S>): vector<u8> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_serialization_scheme_enabled<S>(scheme);
        serialize_internal<S>(scheme, element.handle)
    }

    /// Serialize an element of an algebraic structure `S` to a byte array using a given format `F`.
    public fun serialize_v2<S, F>(element: &Element<S>): vector<u8> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_serialization_format_enabled<S, F>();
        serialize_v2_internal<S,F>(element.handle)
    }

    /// Get the order of group `G`, a big integer little-endian encoded as a byte array.
    public fun group_order<G>(): vector<u8> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_enabled_for_basic_operation<G>();
        group_order_internal<G>()
    }

    /// Check if an element `x` is the identity of its group `G`.
    public fun group_is_identity<G>(element_x: &Element<G>): bool {
        group_is_identity_internal<G>(element_x.handle)
    }

    /// Cast an element of a structure `S` to a parent structure `L`.
    public fun upcast<S,L>(element: &Element<S>): Element<L> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_pair_enabled_for_upcast<S,L>();
        Element<L> {
            handle: upcast_internal<S,L>(element.handle)
        }
    }

    /// Try casting an element `x` of a structure `L` to a sub-structure `S`.
    /// Return none if `x` is not a member of `S`.
    ///
    /// NOTE: Membership check is performed inside, which can be expensive, depending on the structures `L` and `S`.
    public fun downcast<L,S>(element_x: &Element<L>): Option<Element<S>> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_pair_enabled_for_upcast<S,L>();
        let (succ, new_handle) = downcast_internal<L,S>(element_x.handle);
        if (succ) {
            some(Element<S> { handle: new_handle })
        } else {
            none()
        }
    }

    /// Hash some bytes into structure `Struc` using suite `Suite`.
    public fun hash_to<Struc, Suite>(bytes: &vector<u8>): Element<Struc> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_structure_hashsuite_pair_enabled_for_hash<Struc, Suite>();
        Element {
            handle: hash_to_internal<Struc, Suite>(bytes)
        }
    }

    /// Hash some bytes with domain separation tag `dst` into group `G` using suite `S`.
    public fun hash_to_group<G, S>(dst: &vector<u8>, bytes: &vector<u8>): Element<G> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_structure_hashsuite_pair_enabled_for_hash<G, S>();
        Element {
            handle: hash_to_group_internal<G, S>(dst, bytes)
        }
    }

    #[test_only]
    /// Generate a random element of an algebraic structure `S`.
    public fun insecure_random_element<S>(): Element<S> {
        abort_unless_generic_algebraic_structures_basic_operations_enabled();
        abort_unless_type_enabled_for_basic_operation<S>();
        Element<S> {
            handle: insecure_random_element_internal<S>()
        }
    }

    // Public functions end.

    // Native functions begin.

    native fun deserialize_internal<G>(scheme_id: vector<u8>, bytes: &vector<u8>): (bool, u64);
    native fun deserialize_v2_internal<G,F>(bytes: &vector<u8>): (bool, u64);
    native fun downcast_internal<L,S>(handle: u64): (bool, u64);
    native fun eq_internal<S>(handle_1: u64, handle_2: u64): bool;
    native fun field_add_internal<F>(handle_1: u64, handle_2: u64): u64;
    native fun field_div_internal<F>(handle_1: u64, handle_2: u64): (bool, u64);
    native fun field_inv_internal<F>(handle: u64): (bool, u64);
    native fun field_is_one_internal<F>(handle: u64): bool;
    native fun field_is_zero_internal<F>(handle: u64): bool;
    native fun field_mul_internal<F>(handle_1: u64, handle_2: u64): u64;
    native fun field_neg_internal<F>(handle: u64): u64;
    native fun field_one_internal<S>(): u64;
    native fun field_sqr_internal<G>(handle: u64): u64;
    native fun field_sub_internal<G>(handle_1: u64, handle_2: u64): u64;
    native fun field_zero_internal<S>(): u64;
    native fun from_u64_internal<S>(value: u64): u64;
    native fun group_add_internal<G>(handle_1: u64, handle_2: u64): u64;
    native fun group_double_internal<G>(element_handle: u64): u64;
    native fun group_generator_internal<G>(): u64;
    native fun group_identity_internal<G>(): u64;
    native fun group_is_identity_internal<G>(handle: u64): bool;
    native fun group_multi_scalar_mul_internal<G>(element_handles: vector<u64>, scalars_encoded: &vector<u8>, scalar_size_in_bits: u64): u64;
    native fun group_multi_scalar_mul_typed_internal<G>(element_handles: vector<u64>, scalar_handles: vector<u64>): u64;
    native fun group_neg_internal<G>(handle: u64): u64;
    native fun group_order_internal<G>(): vector<u8>;
    native fun group_scalar_mul_typed_internal<G, S>(element_handle: u64, scalar_handle: u64): u64;
    native fun group_scalar_mul_internal<G>(element_handle: u64, scalar_encoded: &vector<u8>): u64;
    native fun group_sub_internal<G>(handle_1: u64, handle_2: u64): u64;
    native fun hash_to_internal<S,C>(bytes: &vector<u8>): u64;
    native fun hash_to_group_internal<G,C>(dst: &vector<u8>, bytes: &vector<u8>): u64;
    #[test_only]
    native fun insecure_random_element_internal<G>(): u64;
    native fun pairing_internal<G1,G2,Gt>(g1_handle: u64, g2_handle: u64): u64;
    native fun serialize_internal<G>(scheme_id: vector<u8>, h: u64): vector<u8>;
    native fun serialize_v2_internal<G,F>(handle: u64): vector<u8>;
    native fun upcast_internal<S,L>(handle: u64): u64;

    // Native functions end.

    // private functions begin.

    fun abort_unless_structure_hashsuite_pair_enabled_for_hash<S, H>() {
        let structure_type = type_of<S>();
        let hashsuite_type = type_of<H>();
        if (structure_type == type_of<BLS12_381_G1>() && hashsuite_type == type_of<HASH_SUITE_BLS12381G1_XMD_SHA_256_SSWU_RO_>() && bls12_381_structures_enabled()) return;
        abort(std::error::not_implemented(0))
    }

    fun abort_unless_generic_algebraic_structures_basic_operations_enabled() {
        if (generic_algebraic_structures_basic_operations_enabled()) return;
        abort(std::error::not_implemented(0))
    }

    fun abort_unless_type_enabled_for_basic_operation<S>() {
        let type = type_of<S>();
        if ((type == type_of<BLS12_381_G1>() || type == type_of<BLS12_381_G2>() || type == type_of<BLS12_381_Gt>() || type == type_of<BLS12_381_Fr>() || type == type_of<BLS12_381_Fq12>()) && bls12_381_structures_enabled()) return;
        abort(std::error::not_implemented(0))
    }

    fun abort_unless_type_serialization_scheme_enabled<S>(scheme: vector<u8>) {
        let type = type_of<S>();
        if (type == type_of<BLS12_381_Fr>() && (scheme == bls12_381_fr_lendian_format() || scheme == bls12_381_fr_bendian_format()) && bls12_381_structures_enabled()) return;
        if (type == type_of<BLS12_381_Fq12>() && scheme == bls12_381_fq12_format() && bls12_381_structures_enabled()) return;
        if (type == type_of<BLS12_381_G1>() && (scheme == bls12_381_g1_uncompressed_format() || scheme == bls12_381_g1_compressed_format()) && bls12_381_structures_enabled()) return;
        if (type == type_of<BLS12_381_G2>() && (scheme == bls12_381_g2_uncompressed_format() || scheme == bls12_381_g2_compressed_format()) && bls12_381_structures_enabled()) return;
        if (type == type_of<BLS12_381_Gt>() && scheme == bls12_381_gt_format() && bls12_381_structures_enabled()) return;
        abort(std::error::not_implemented(0))
    }

    fun abort_unless_type_serialization_format_enabled<S, F>() {
        let structure_type = type_of<S>();
        let format_type = type_of<F>();
        if (structure_type == type_of<BLS12_381_Fr>() && (format_type == type_of<BLS12_381_Fr_Format_LEndian>() || format_type == type_of<BLS12_381_Fr_Format_BEndian>()) && bls12_381_structures_enabled()) return;
        if (structure_type == type_of<BLS12_381_Fq12>() && format_type == type_of<BLS12_381_Fq12_Format>() && bls12_381_structures_enabled()) return;
        if (structure_type == type_of<BLS12_381_G1>() && (format_type == type_of<BLS12_381_G1_Format_Uncompressed>() || format_type == type_of<BLS12_381_G1_Format_Compressed>()) && bls12_381_structures_enabled()) return;
        if (structure_type == type_of<BLS12_381_G2>() && (format_type == type_of<BLS12_381_G2_Format_Uncompressed>() || format_type == type_of<BLS12_381_G2_Format_Compressed>()) && bls12_381_structures_enabled()) return;
        if (structure_type == type_of<BLS12_381_Gt>() && format_type == type_of<BLS12_381_Gt_Format>() && bls12_381_structures_enabled()) return;
        abort(std::error::not_implemented(0))
    }

    fun abort_unless_type_triplet_enabled_for_pairing<G1, G2, Gt>() {
        let g1_type = type_of<G1>();
        let g2_type = type_of<G2>();
        let gt_type = type_of<Gt>();
        if (g1_type == type_of<BLS12_381_G1>() && g2_type == type_of<BLS12_381_G2>() && gt_type == type_of<BLS12_381_Gt>() && bls12_381_structures_enabled()) return;
        abort(std::error::not_implemented(0))
    }

    fun abort_unless_type_pair_enabled_for_group_scalar_mul<G, S>() {
        let group = type_of<G>();
        let scalar_field = type_of<S>();
        if ((group == type_of<BLS12_381_G1>() || group == type_of<BLS12_381_G2>() || group == type_of<BLS12_381_Gt>()) && scalar_field == type_of<BLS12_381_Fr>() && bls12_381_structures_enabled()) return;
        abort(std::error::not_implemented(0))
    }

    fun abort_unless_type_pair_enabled_for_upcast<S, L>() {
        let super = type_of<L>();
        let sub = type_of<S>();
        if (super == type_of<BLS12_381_Fq12>() && sub == type_of<BLS12_381_Gt>() && bls12_381_structures_enabled()) return;
        abort(std::error::not_implemented(0))
    }

    #[test_only]
    fun enable_initial_generic_algebraic_operations(fx: &signer) {
        std::features::change_feature_flags(fx, std::vector::singleton(std::features::get_generic_agebraic_structures_basic_operations_feature()), std::vector::empty());
    }

    #[test_only]
    fun enable_bls12_381_structures(fx: &signer) {
        std::features::change_feature_flags(fx, std::vector::singleton(std::features::get_bls12_381_strutures_feature()), std::vector::empty());
    }

    // Private functions end.

    // Tests begin.

    #[test_only]
    const BLS12_381_FR_VAL_7_SERIALIZED_LENDIAN: vector<u8> = x"0700000000000000000000000000000000000000000000000000000000000000";
    #[test_only]
    const BLS12_381_FR_VAL_7_SERIALIZED_BENDIAN: vector<u8> = x"0000000000000000000000000000000000000000000000000000000000000007";
    #[test_only]
    const BLS12_381_FR_VAL_7_NEG_SERIALIZED_LENDIAN: vector<u8> = x"fafffffffefffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73";

    #[test(fx = @std)]
    fun test_bls12_381_fr(fx: signer) {
        enable_initial_generic_algebraic_operations(&fx);
        enable_bls12_381_structures(&fx);

        // Special elements and checks.
        let val_0 = field_zero<BLS12_381_Fr>();
        let val_1 = field_one<BLS12_381_Fr>();
        assert!(field_is_zero(&val_0), 1);
        assert!(!field_is_zero(&val_1), 1);
        assert!(!field_is_one(&val_0), 1);
        assert!(field_is_one(&val_1), 1);

        // Serialization/deserialization.
        let val_7 = from_u64<BLS12_381_Fr>(7);
        let val_7_2nd = std::option::extract(&mut deserialize_v2<BLS12_381_Fr, BLS12_381_Fr_Format_LEndian>(&BLS12_381_FR_VAL_7_SERIALIZED_LENDIAN));
        let val_7_3rd = std::option::extract(&mut deserialize_v2<BLS12_381_Fr, BLS12_381_Fr_Format_BEndian>(&BLS12_381_FR_VAL_7_SERIALIZED_BENDIAN));
        assert!(eq(&val_7, &val_7_2nd), 1);
        assert!(eq(&val_7, &val_7_3rd), 1);
        assert!(BLS12_381_FR_VAL_7_SERIALIZED_LENDIAN == serialize_v2<BLS12_381_Fr, BLS12_381_Fr_Format_LEndian>(&val_7), 1);
        assert!(BLS12_381_FR_VAL_7_SERIALIZED_BENDIAN == serialize_v2<BLS12_381_Fr, BLS12_381_Fr_Format_BEndian>(&val_7), 1);

        // Deserialization: byte array of right size but the value is not a member.
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_Fr, BLS12_381_Fr_Format_LEndian>(&x"01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73")), 1);
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_Fr, BLS12_381_Fr_Format_BEndian>(&x"73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001")), 1);

        // Deserialization: byte array of wrong size.
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_Fr, BLS12_381_Fr_Format_LEndian>(&x"01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed7300")), 1);
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_Fr, BLS12_381_Fr_Format_BEndian>(&x"0073eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001")), 1);
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_Fr, BLS12_381_Fr_Format_LEndian>(&x"ffff")), 1);
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_Fr, BLS12_381_Fr_Format_BEndian>(&x"ffff")), 1);

        // Negation.
        let val_minus_7 = field_neg(&val_7);
        assert!(BLS12_381_FR_VAL_7_NEG_SERIALIZED_LENDIAN == serialize_v2<BLS12_381_Fr, BLS12_381_Fr_Format_LEndian>(&val_minus_7), 1);

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

        // Squaring.
        let val_x = insecure_random_element<BLS12_381_Fr>();
        assert!(eq(&field_mul(&val_x, &val_x), &field_sqr(&val_x)), 1);
    }

    #[test_only]
    const BLS12_381_FQ12_VAL_7_SERIALIZED: vector<u8> = x"070000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
    #[test_only]
    const BLS12_381_FQ12_VAL_7_NEG_SERIALIZED: vector<u8> = x"a4aafffffffffeb9ffff53b1feffab1e24f6b0f6a0d23067bf1285f3844b7764d7ac4b43b6a71b4b9ae67f39ea11011a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

    #[test(fx = @std)]
    fun test_bls12_381_fq12(fx: signer) {
        enable_initial_generic_algebraic_operations(&fx);
        enable_bls12_381_structures(&fx);

        // Special elements and checks.
        let val_0 = field_zero<BLS12_381_Fq12>();
        let val_1 = field_one<BLS12_381_Fq12>();
        assert!(field_is_zero(&val_0), 1);
        assert!(!field_is_zero(&val_1), 1);
        assert!(!field_is_one(&val_0), 1);
        assert!(field_is_one(&val_1), 1);

        // Serialization/deserialization.
        let val_7 = from_u64<BLS12_381_Fq12>(7);
        let val_7_another = std::option::extract(&mut deserialize_v2<BLS12_381_Fq12, BLS12_381_Fq12_Format>(&BLS12_381_FQ12_VAL_7_SERIALIZED));
        assert!(eq(&val_7, &val_7_another), 1);
        assert!(BLS12_381_FQ12_VAL_7_SERIALIZED == serialize_v2<BLS12_381_Fq12, BLS12_381_Fq12_Format>(&val_7), 1);
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_Fq12, BLS12_381_Fq12_Format>(&x"ffff")), 1);

        // Negation.
        let val_minus_7 = field_neg(&val_7);
        assert!(BLS12_381_FQ12_VAL_7_NEG_SERIALIZED == serialize_v2<BLS12_381_Fq12, BLS12_381_Fq12_Format>(&val_minus_7), 1);

        // Addition.
        let val_9 = from_u64<BLS12_381_Fq12>(9);
        let val_2 = from_u64<BLS12_381_Fq12>(2);
        assert!(eq(&val_2, &field_add(&val_minus_7, &val_9)), 1);

        // Subtraction.
        assert!(eq(&val_9, &field_sub(&val_2, &val_minus_7)), 1);

        // Multiplication.
        let val_63 = from_u64<BLS12_381_Fq12>(63);
        assert!(eq(&val_63, &field_mul(&val_7, &val_9)), 1);

        // division.
        let val_0 = from_u64<BLS12_381_Fq12>(0);
        assert!(eq(&val_7, &std::option::extract(&mut field_div(&val_63, &val_9))), 1);
        assert!(std::option::is_none(&field_div(&val_63, &val_0)), 1);

        // Inversion.
        assert!(eq(&val_minus_7, &field_neg(&val_7)), 1);
        assert!(std::option::is_none(&field_inv(&val_0)), 1);

        // Squaring.
        let val_x = insecure_random_element<BLS12_381_Fq12>();
        assert!(eq(&field_mul(&val_x, &val_x), &field_sqr(&val_x)), 1);

        // Downcasting.
        assert!(eq(&group_identity<BLS12_381_Gt>(), &std::option::extract(&mut downcast<BLS12_381_Fq12, BLS12_381_Gt>(&val_1))), 1);
    }

    #[test_only]
    const BLS12_381_R: vector<u8> = x"01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73";
    #[test_only]
    const BLS12_381_G1_INF_SERIALIZED_COMP: vector<u8> = x"c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
    #[test_only]
    const BLS12_381_G1_INF_SERIALIZED_UNCOMP: vector<u8> = x"400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
    #[test_only]
    const BLS12_381_G1_GENERATOR_SERIALIZED_COMP: vector<u8> = x"97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb";
    #[test_only]
    const BLS12_381_G1_GENERATOR_SERIALIZED_UNCOMP: vector<u8> = x"17f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb08b3f481e3aaa0f1a09e30ed741d8ae4fcf5e095d5d00af600db18cb2c04b3edd03cc744a2888ae40caa232946c5e7e1";
    #[test_only]
    const BLS12_381_G1_GENERATOR_MUL_BY_7_SERIALIZED_COMP: vector<u8> = x"b928f3beb93519eecf0145da903b40a4c97dca00b21f12ac0df3be9116ef2ef27b2ae6bcd4c5bc2d54ef5a70627efcb7";
    #[test_only]
    const BLS12_381_G1_GENERATOR_MUL_BY_7_SERIALIZED_UNCOMP: vector<u8> = x"1928f3beb93519eecf0145da903b40a4c97dca00b21f12ac0df3be9116ef2ef27b2ae6bcd4c5bc2d54ef5a70627efcb7108dadbaa4b636445639d5ae3089b3c43a8a1d47818edd1839d7383959a41c10fdc66849cfa1b08c5a11ec7e28981a1c";
    #[test_only]
    const BLS12_381_G1_GENERATOR_MUL_BY_7_NEG_SERIALIZED_COMP: vector<u8> = x"9928f3beb93519eecf0145da903b40a4c97dca00b21f12ac0df3be9116ef2ef27b2ae6bcd4c5bc2d54ef5a70627efcb7";
    #[test_only]
    const BLS12_381_G1_GENERATOR_MUL_BY_7_NEG_SERIALIZED_UNCOMP: vector<u8> = x"1928f3beb93519eecf0145da903b40a4c97dca00b21f12ac0df3be9116ef2ef27b2ae6bcd4c5bc2d54ef5a70627efcb70973642f94c9b055f4e1d20812c1f91329ed2e3d71f635a72d599a679d0cda1320e597b4e1b24f735fed1381d767908f";

    #[test(fx = @std)]
    fun test_bls12_381_g1(fx: signer) {
        enable_initial_generic_algebraic_operations(&fx);
        enable_bls12_381_structures(&fx);

        // Special constants and checks.
        assert!(BLS12_381_R == group_order<BLS12_381_G1>(), 1);
        let point_at_infinity = group_identity<BLS12_381_G1>();
        let generator = group_generator<BLS12_381_G1>();
        assert!(group_is_identity(&point_at_infinity), 1);
        assert!(!group_is_identity(&generator), 1);

        // Serialization/deserialization.
        assert!(BLS12_381_G1_GENERATOR_SERIALIZED_UNCOMP == serialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Uncompressed>(&generator), 1);
        assert!(BLS12_381_G1_GENERATOR_SERIALIZED_COMP == serialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Compressed>(&generator), 1);
        let generator_from_comp = std::option::extract(&mut deserialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Compressed>(&BLS12_381_G1_GENERATOR_SERIALIZED_COMP));
        let generator_from_uncomp = std::option::extract(&mut deserialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Uncompressed>(&BLS12_381_G1_GENERATOR_SERIALIZED_UNCOMP));
        assert!(eq(&generator, &generator_from_comp), 1);
        assert!(eq(&generator, &generator_from_uncomp), 1);

        // Deserialization: byte array of correct size but the value is not a member.
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_Fq12, BLS12_381_Fq12_Format>(&x"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")), 1);

        // Deserialization: byte array of wrong size.
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_Fq12, BLS12_381_Fq12_Format>(&x"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")), 1);

        assert!(BLS12_381_G1_INF_SERIALIZED_UNCOMP == serialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Uncompressed>(&point_at_infinity), 1);
        assert!(BLS12_381_G1_INF_SERIALIZED_COMP == serialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Compressed>(&point_at_infinity), 1);
        let inf_from_uncomp = std::option::extract(&mut deserialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Uncompressed>(&BLS12_381_G1_INF_SERIALIZED_UNCOMP));
        let inf_from_comp = std::option::extract(&mut deserialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Compressed>(&BLS12_381_G1_INF_SERIALIZED_COMP));
        assert!(eq(&point_at_infinity, &inf_from_comp), 1);
        assert!(eq(&point_at_infinity, &inf_from_uncomp), 1);

        let point_7g_from_uncomp = std::option::extract(&mut deserialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Uncompressed>(&BLS12_381_G1_GENERATOR_MUL_BY_7_SERIALIZED_UNCOMP));
        let point_7g_from_comp = std::option::extract(&mut deserialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Compressed>(&BLS12_381_G1_GENERATOR_MUL_BY_7_SERIALIZED_COMP));
        assert!(eq(&point_7g_from_comp, &point_7g_from_uncomp), 1);

        // Deserialization: on the curve but not in the prime-order subgroup.
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Uncompressed>(&x"8959e137e0719bf872abb08411010f437a8955bd42f5ba20fca64361af58ce188b1adb96ef229698bb7860b79e24ba12a76e9853b35f5c9b2002d9e5833fd8f9ab4cd3934a4722a06f6055bfca720c91629811e2ecae7f0cf301b6d07898a90f")), 1);
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Compressed>(&x"8959e137e0719bf872abb08411010f437a8955bd42f5ba20fca64361af58ce188b1adb96ef229698bb7860b79e24ba12")), 1);

        // Deserialization: a valid point in (Fq,Fq) but not on the curve.
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Uncompressed>(&x"8959e137e0719bf872abb08411010f437a8955bd42f5ba20fca64361af58ce188b1adb96ef229698bb7860b79e24ba12000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")), 1);

        //TODO: a point on the curve but not in the prime-order subgroup.

        // Deserialization: an invalid point (x not in Fq).
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Uncompressed>(&x"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffa76e9853b35f5c9b2002d9e5833fd8f9ab4cd3934a4722a06f6055bfca720c91629811e2ecae7f0cf301b6d07898a90f")), 1);
        //TODO: Compressed version.
        // assert!(std::option::is_none(&deserialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Compressed>(&x"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")), 1);

        // Deserialization: byte array of wrong size.
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Uncompressed>(&x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ab")), 1);
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Compressed>(&x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ab")), 1);

        // Scalar multiplication.
        let scalar_7 = from_u64<BLS12_381_Fr>(7);
        let point_7g_calc = group_scalar_mul_typed(&generator, &scalar_7);
        assert!(eq(&point_7g_calc, &point_7g_from_comp), 1);
        let point_7g_calc = group_scalar_mul(&generator, &x"07");
        assert!(eq(&point_7g_calc, &point_7g_from_comp), 1);
        assert!(BLS12_381_G1_GENERATOR_MUL_BY_7_SERIALIZED_UNCOMP == serialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Uncompressed>(&point_7g_calc), 1);
        assert!(BLS12_381_G1_GENERATOR_MUL_BY_7_SERIALIZED_COMP == serialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Compressed>( &point_7g_calc), 1);

        // Multi-scalar multiplication.
        let scalar_a = x"0003";
        let scalar_b = x"0104";
        let scalar_c = x"0205";
        let point_p = insecure_random_element<BLS12_381_G1>();
        let point_q = insecure_random_element<BLS12_381_G1>();
        let point_r = insecure_random_element<BLS12_381_G1>();
        let expected = group_identity<BLS12_381_G1>();
        let expected = group_add(&expected, &group_scalar_mul(&point_p, &scalar_a));
        let expected = group_add(&expected, &group_scalar_mul(&point_q, &scalar_b));
        let expected = group_add(&expected, &group_scalar_mul(&point_r, &scalar_c));
        let points = std::vector::empty();
        std::vector::push_back(&mut points, point_p);
        std::vector::push_back(&mut points, point_q);
        std::vector::push_back(&mut points, point_r);
        let scalars_encoded = std::vector::empty();
        std::vector::append(&mut scalars_encoded, scalar_a);
        std::vector::append(&mut scalars_encoded, scalar_b);
        std::vector::append(&mut scalars_encoded, scalar_c);
        let actual = group_multi_scalar_mul(&points, &scalars_encoded, 16);
        assert!(eq(&expected, &actual), 1);
        let scalars = std::vector::empty();
        let scalar_a = from_u64<BLS12_381_Fr>(0x0300);
        let scalar_b = from_u64<BLS12_381_Fr>(0x0401);
        let scalar_c = from_u64<BLS12_381_Fr>(0x0502);
        std::vector::push_back(&mut scalars, scalar_a);
        std::vector::push_back(&mut scalars, scalar_b);
        std::vector::push_back(&mut scalars, scalar_c);
        let actual = group_multi_scalar_mul_typed(&points, &scalars);
        assert!(eq(&expected, &actual), 1);

        // Doubling.
        let scalar_2 = from_u64<BLS12_381_Fr>(2);
        let point_2g = group_scalar_mul_typed(&generator, &scalar_2);
        let point_double_g = group_double(&generator);
        assert!(eq(&point_2g, &point_double_g), 1);

        // Negation.
        let point_minus_7g_calc = group_neg(&point_7g_calc);
        assert!(BLS12_381_G1_GENERATOR_MUL_BY_7_NEG_SERIALIZED_COMP == serialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Compressed>(&point_minus_7g_calc), 1);
        assert!(BLS12_381_G1_GENERATOR_MUL_BY_7_NEG_SERIALIZED_UNCOMP == serialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Uncompressed>(&point_minus_7g_calc), 1);

        // Addition.
        let scalar_9 = from_u64<BLS12_381_Fr>(9);
        let point_9g = group_scalar_mul_typed(&generator, &scalar_9);
        let point_2g = group_scalar_mul_typed(&generator, &scalar_2);
        let point_2g_calc = group_add(&point_minus_7g_calc, &point_9g);
        assert!(eq(&point_2g, &point_2g_calc), 1);

        // Subtraction.
        assert!(eq(&point_9g, &group_sub(&point_2g, &point_minus_7g_calc)), 1);

        // // Hash-to-group.
        // let actual = hash_to_group<BLS12_381_G1, HASH_SUITE_BLS12381G1_XMD_SHA_256_SSWU_RO_>(&b"QUUX-V01-CS02-with-BLS12381G1_XMD:SHA-256_SSWU_RO_", &b"");
        // let expected = std::option::extract(&mut deserialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Uncompressed_BEndianFq>(&x"2c15230b26dbc6fc9a37051158c95b79656e17a1a920b11394ca91c44247d3e48a7a74985cc5c776cdfe4b1f19884970453912e9d31528c060be9ab5c43e8415"));
        // assert!(eq(&expected, &actual), 1);
        // let actual = hash_to_group<BLS12_381_G1, HASH_SUITE_BLS12381G1_XMD_SHA_256_SSWU_RO_>(&b"QUUX-V01-CS02-with-BLS12381G1_XMD:SHA-256_SSWU_RO_", &b"abcdef0123456789");
        // let expected = std::option::extract(&mut deserialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Uncompressed_BEndianFq>(&x"65038ac8f2b1def042a5df0b33b1f4eca6bff7cb0f9c6c1526811864e544ed80cad44d40a656e7aff4002a8de287abc8ae0482b5ae825822bb870d6df9b56ca3"));
        // assert!(eq(&expected, &actual), 1);
    }

    #[test_only]
    const BLS12_381_G2_INF_SERIALIZED_UNCOMP: vector<u8> = x"400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
    #[test_only]
    const BLS12_381_G2_INF_SERIALIZED_COMP: vector<u8> = x"c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
    #[test_only]
    const BLS12_381_G2_GENERATOR_SERIALIZED_UNCOMP: vector<u8> = x"13e02b6052719f607dacd3a088274f65596bd0d09920b61ab5da61bbdc7f5049334cf11213945d57e5ac7d055d042b7e024aa2b2f08f0a91260805272dc51051c6e47ad4fa403b02b4510b647ae3d1770bac0326a805bbefd48056c8c121bdb80606c4a02ea734cc32acd2b02bc28b99cb3e287e85a763af267492ab572e99ab3f370d275cec1da1aaa9075ff05f79be0ce5d527727d6e118cc9cdc6da2e351aadfd9baa8cbdd3a76d429a695160d12c923ac9cc3baca289e193548608b82801";
    #[test_only]
    const BLS12_381_G2_GENERATOR_SERIALIZED_COMP: vector<u8> = x"93e02b6052719f607dacd3a088274f65596bd0d09920b61ab5da61bbdc7f5049334cf11213945d57e5ac7d055d042b7e024aa2b2f08f0a91260805272dc51051c6e47ad4fa403b02b4510b647ae3d1770bac0326a805bbefd48056c8c121bdb8";
    #[test_only]
    const BLS12_381_G2_GENERATOR_MUL_BY_7_SERIALIZED_UNCOMP: vector<u8> = x"0d0273f6bf31ed37c3b8d68083ec3d8e20b5f2cc170fa24b9b5be35b34ed013f9a921f1cad1644d4bdb14674247234c8049cd1dbb2d2c3581e54c088135fef36505a6823d61b859437bfc79b617030dc8b40e32bad1fa85b9c0f368af6d38d3c05ecf93654b7a1885695aaeeb7caf41b0239dc45e1022be55d37111af2aecef87799638bec572de86a7437898efa702008b7ae4dbf802c17a6648842922c9467e460a71c88d393ee7af356da123a2f3619e80c3bdcc8e2b1da52f8cd9913ccdd";
    #[test_only]
    const BLS12_381_G2_GENERATOR_MUL_BY_7_SERIALIZED_COMP: vector<u8> = x"8d0273f6bf31ed37c3b8d68083ec3d8e20b5f2cc170fa24b9b5be35b34ed013f9a921f1cad1644d4bdb14674247234c8049cd1dbb2d2c3581e54c088135fef36505a6823d61b859437bfc79b617030dc8b40e32bad1fa85b9c0f368af6d38d3c";
    #[test_only]
    const BLS12_381_G2_GENERATOR_MUL_BY_7_NEG_SERIALIZED_UNCOMP: vector<u8> = x"0d0273f6bf31ed37c3b8d68083ec3d8e20b5f2cc170fa24b9b5be35b34ed013f9a921f1cad1644d4bdb14674247234c8049cd1dbb2d2c3581e54c088135fef36505a6823d61b859437bfc79b617030dc8b40e32bad1fa85b9c0f368af6d38d3c141418b3e4c84511f485fcc78b80b8bc623d6f3f1282e6da09f9c1860402272ba7129c72c4fcd2174f8ac87671053a8b1149639c79ffba82a4b71f73b11f186f8016a4686ab17ed0ec3d7bc6e476c6ee04c3f3c2d48b1d4ddfac073266ebddce";
    #[test_only]
    const BLS12_381_G2_GENERATOR_MUL_BY_7_NEG_SERIALIZED_COMP: vector<u8> = x"ad0273f6bf31ed37c3b8d68083ec3d8e20b5f2cc170fa24b9b5be35b34ed013f9a921f1cad1644d4bdb14674247234c8049cd1dbb2d2c3581e54c088135fef36505a6823d61b859437bfc79b617030dc8b40e32bad1fa85b9c0f368af6d38d3c";

    #[test(fx = @std)]
    fun test_bls12_381_g2(fx: signer) {
        enable_initial_generic_algebraic_operations(&fx);
        enable_bls12_381_structures(&fx);

        // Special constants and checks.
        assert!(BLS12_381_R == group_order<BLS12_381_G2>(), 1);
        let point_at_infinity = group_identity<BLS12_381_G2>();
        let generator = group_generator<BLS12_381_G2>();
        assert!(group_is_identity(&point_at_infinity), 1);
        assert!(!group_is_identity(&generator), 1);

        // Serialization/deserialization.
        assert!(BLS12_381_G2_GENERATOR_SERIALIZED_COMP == serialize_v2<BLS12_381_G2, BLS12_381_G2_Format_Compressed>(&generator), 1);
        assert!(BLS12_381_G2_GENERATOR_SERIALIZED_UNCOMP == serialize_v2<BLS12_381_G2, BLS12_381_G2_Format_Uncompressed>(&generator), 1);
        let generator_from_uncomp = std::option::extract(&mut deserialize_v2<BLS12_381_G2, BLS12_381_G2_Format_Uncompressed>(&BLS12_381_G2_GENERATOR_SERIALIZED_UNCOMP));
        let generator_from_comp = std::option::extract(&mut deserialize_v2<BLS12_381_G2, BLS12_381_G2_Format_Compressed>(&BLS12_381_G2_GENERATOR_SERIALIZED_COMP));
        assert!(eq(&generator, &generator_from_comp), 1);
        assert!(eq(&generator, &generator_from_uncomp), 1);
        assert!(BLS12_381_G2_INF_SERIALIZED_UNCOMP == serialize_v2<BLS12_381_G2, BLS12_381_G2_Format_Uncompressed>(&point_at_infinity), 1);
        assert!(BLS12_381_G2_INF_SERIALIZED_COMP == serialize_v2<BLS12_381_G2, BLS12_381_G2_Format_Compressed>(&point_at_infinity), 1);
        let inf_from_uncomp = std::option::extract(&mut deserialize_v2<BLS12_381_G2, BLS12_381_G2_Format_Uncompressed>(&BLS12_381_G2_INF_SERIALIZED_UNCOMP));
        let inf_from_comp = std::option::extract(&mut deserialize_v2<BLS12_381_G2, BLS12_381_G2_Format_Compressed>(&BLS12_381_G2_INF_SERIALIZED_COMP));
        assert!(eq(&point_at_infinity, &inf_from_comp), 1);
        assert!(eq(&point_at_infinity, &inf_from_uncomp), 1);
        let point_7g_from_uncomp = std::option::extract(&mut deserialize_v2<BLS12_381_G2, BLS12_381_G2_Format_Uncompressed>(&BLS12_381_G2_GENERATOR_MUL_BY_7_SERIALIZED_UNCOMP));
        let point_7g_from_comp = std::option::extract(&mut deserialize_v2<BLS12_381_G2, BLS12_381_G2_Format_Compressed>(&BLS12_381_G2_GENERATOR_MUL_BY_7_SERIALIZED_COMP));
        assert!(eq(&point_7g_from_comp, &point_7g_from_uncomp), 1);

        // Deserialization: on the curve but not in the prime-order subgroup.
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Uncompressed>(&x"f037d4ccd5ee751eba1c1fd4c7edbb76d2b04c3a1f3f554827cf37c3acbc2dbb7cdb320a2727c2462d6c55ca1f637707b96eeebc622c1dbe7c56c34f93887c8751b42bd04f29253a82251c192ef27ece373993b663f4360505299c5bd18c890ddd862a6308796bf47e2265073c1f7d81afd69f9497fc1403e2e97a866129b43b672295229c21116d4a99f3e5c2ae720a31f181dbed8a93e15f909c20cf69d11a8879adbbe6890740def19814e6d4ed23fb0dcbd79291655caf48b466ac9cae04")), 1);
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Compressed>(&x"f037d4ccd5ee751eba1c1fd4c7edbb76d2b04c3a1f3f554827cf37c3acbc2dbb7cdb320a2727c2462d6c55ca1f637707b96eeebc622c1dbe7c56c34f93887c8751b42bd04f29253a82251c192ef27ece373993b663f4360505299c5bd18c890d")), 1);

        // Deserialization: a valid point in (Fq2,Fq2) but not on the curve.
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Uncompressed>(&x"f037d4ccd5ee751eba1c1fd4c7edbb76d2b04c3a1f3f554827cf37c3acbc2dbb7cdb320a2727c2462d6c55ca1f637707b96eeebc622c1dbe7c56c34f93887c8751b42bd04f29253a82251c192ef27ece373993b663f4360505299c5bd18c890d000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")), 1);

        //TODO: a point on the curve but not in the prime-order subgroup.

        // Deserialization: an invalid point (x not in Fq2).
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Uncompressed>(&x"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffdd862a6308796bf47e2265073c1f7d81afd69f9497fc1403e2e97a866129b43b672295229c21116d4a99f3e5c2ae720a31f181dbed8a93e15f909c20cf69d11a8879adbbe6890740def19814e6d4ed23fb0dcbd79291655caf48b466ac9cae04")), 1);
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Compressed>(&x"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")), 1);

        // Deserialization: byte array of wrong size.
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Uncompressed>(&x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ab")), 1);
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_G1, BLS12_381_G1_Format_Compressed>(&x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ab")), 1);

        // Scalar multiplication.
        let scalar_7 = from_u64<BLS12_381_Fr>(7);
        let point_7g_calc = group_scalar_mul(&generator, &x"07");
        assert!(eq(&point_7g_calc, &point_7g_from_comp), 1);
        let point_7g_calc = group_scalar_mul_typed(&generator, &scalar_7);
        assert!(eq(&point_7g_calc, &point_7g_from_comp), 1);
        assert!(BLS12_381_G2_GENERATOR_MUL_BY_7_SERIALIZED_UNCOMP == serialize_v2<BLS12_381_G2, BLS12_381_G2_Format_Uncompressed>(&point_7g_calc), 1);
        assert!(BLS12_381_G2_GENERATOR_MUL_BY_7_SERIALIZED_COMP == serialize_v2<BLS12_381_G2, BLS12_381_G2_Format_Compressed>(&point_7g_calc), 1);

        // Multi-scalar multiplication.
        let scalar_a = x"0003";
        let scalar_b = x"0104";
        let scalar_c = x"0205";
        let point_p = insecure_random_element<BLS12_381_G2>();
        let point_q = insecure_random_element<BLS12_381_G2>();
        let point_r = insecure_random_element<BLS12_381_G2>();
        let expected = group_identity<BLS12_381_G2>();
        let expected = group_add(&expected, &group_scalar_mul(&point_p, &scalar_a));
        let expected = group_add(&expected, &group_scalar_mul(&point_q, &scalar_b));
        let expected = group_add(&expected, &group_scalar_mul(&point_r, &scalar_c));
        let points = std::vector::empty();
        std::vector::push_back(&mut points, point_p);
        std::vector::push_back(&mut points, point_q);
        std::vector::push_back(&mut points, point_r);
        let scalars_encoded = std::vector::empty();
        std::vector::append(&mut scalars_encoded, scalar_a);
        std::vector::append(&mut scalars_encoded, scalar_b);
        std::vector::append(&mut scalars_encoded, scalar_c);
        let actual = group_multi_scalar_mul(&points, &scalars_encoded, 16);
        assert!(eq(&expected, &actual), 1);
        let scalars = std::vector::empty();
        let scalar_a = from_u64<BLS12_381_Fr>(0x0300);
        let scalar_b = from_u64<BLS12_381_Fr>(0x0401);
        let scalar_c = from_u64<BLS12_381_Fr>(0x0502);
        std::vector::push_back(&mut scalars, scalar_a);
        std::vector::push_back(&mut scalars, scalar_b);
        std::vector::push_back(&mut scalars, scalar_c);
        let actual = group_multi_scalar_mul_typed(&points, &scalars);
        assert!(eq(&expected, &actual), 1);

        // Doubling.
        let scalar_2 = from_u64<BLS12_381_Fr>(2);
        let point_2g = group_scalar_mul_typed(&generator, &scalar_2);
        let point_double_g = group_double(&generator);
        assert!(eq(&point_2g, &point_double_g), 1);

        // Negation.
        let point_minus_7g_calc = group_neg(&point_7g_calc);
        assert!(BLS12_381_G2_GENERATOR_MUL_BY_7_NEG_SERIALIZED_COMP == serialize_v2<BLS12_381_G2, BLS12_381_G2_Format_Compressed>(&point_minus_7g_calc), 1);
        assert!(BLS12_381_G2_GENERATOR_MUL_BY_7_NEG_SERIALIZED_UNCOMP == serialize_v2<BLS12_381_G2, BLS12_381_G2_Format_Uncompressed>(&point_minus_7g_calc), 1);

        // Addition.
        let scalar_9 = from_u64<BLS12_381_Fr>(9);
        let point_9g = group_scalar_mul_typed(&generator, &scalar_9);
        let point_2g = group_scalar_mul_typed(&generator, &scalar_2);
        let point_2g_calc = group_add(&point_minus_7g_calc, &point_9g);
        assert!(eq(&point_2g, &point_2g_calc), 1);

        // Subtraction.
        assert!(eq(&point_9g, &group_sub(&point_2g, &point_minus_7g_calc)), 1);
    }

    #[test_only]
    const BLS12_381_FQ12_ONE_SERIALIZED: vector<u8> = x"010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
    #[test_only]
    const BLS12_381_GT_GENERATOR_SERIALIZED: vector<u8> = x"b68917caaa0543a808c53908f694d1b6e7b38de90ce9d83d505ca1ef1b442d2727d7d06831d8b2a7920afc71d8eb50120f17a0ea982a88591d9f43503e94a8f1abaf2e4589f65aafb7923c484540a868883432a5c60e75860b11e5465b1c9a08873ec29e844c1c888cb396933057ffdd541b03a5220eda16b2b3a6728ea678034ce39c6839f20397202d7c5c44bb68134f93193cec215031b17399577a1de5ff1f5b0666bdd8907c61a7651e4e79e0372951505a07fa73c25788db6eb8023519a5aa97b51f1cad1d43d8aabbff4dc319c79a58cafc035218747c2f75daf8f2fb7c00c44da85b129113173d4722f5b201b6b4454062e9ea8ba78c5ca3cadaf7238b47bace5ce561804ae16b8f4b63da4645b8457a93793cbd64a7254f150781019de87ee42682940f3e70a88683d512bb2c3fb7b2434da5dedbb2d0b3fb8487c84da0d5c315bdd69c46fb05d23763f2191aabd5d5c2e12a10b8f002ff681bfd1b2ee0bf619d80d2a795eb22f2aa7b85d5ffb671a70c94809f0dafc5b73ea2fb0657bae23373b4931bc9fa321e8848ef78894e987bff150d7d671aee30b3931ac8c50e0b3b0868effc38bf48cd24b4b811a2995ac2a09122bed9fd9fa0c510a87b10290836ad06c8203397b56a78e9a0c61c77e56ccb4f1bc3d3fcaea7550f3503efe30f2d24f00891cb45620605fcfaa4292687b3a7db7c1c0554a93579e889a121fd8f72649b2402996a084d2381c5043166673b3849e4fd1e7ee4af24aa8ed443f56dfd6b68ffde4435a92cd7a4ac3bc77e1ad0cb728606cf08bf6386e5410f";
    #[test_only]
    const BLS12_381_GT_GENERATOR_MUL_BY_7_SERIALIZED: vector<u8> = x"2041ea7b66c19680e2c0bb23245a71918753220b31f88a925aa9b1e192e7c188a0b365cb994b3ec5e809206117c6411242b940b10caa37ce734496b3b7c63578a0e3c076f9b31a7ca13a716262e0e4cda4ac994efb9e19893cbfe4d464b9210d099d808a08b3c4c3846e7529984899478639c4e6c46152ef49a04af9c8e6ff442d286c4613a3dac6a4bee4b40e1f6b030f2871dabe4223b250c3181ecd3bc6819004745aeb6bac567407f2b9c7d1978c45ee6712ae46930bc00638383f6696158bad488cbe7663d681c96c035481dbcf78e7a7fbaec3799163aa6914cef3365156bdc3e533a7c883d5974e3462ac6f19e3f9ce26800ae248a45c5f0dd3a48a185969224e6cd6af9a048241bdcac9800d94aeee970e08488fb961e36a769b6c185d185b4605dc9808517196bba9d00a3e37bca466c19187486db104ee03962d39fe473e276355618e44c965f05082bb027a7baa4bcc6d8c0775c1e8a481e77df36ddad91e75a982302937f543a11fe71922dcd4f46fe8f951f91cde412b359507f2b3b6df0374bfe55c9a126ad31ce254e67d64194d32d7955ec791c9555ea5a917fc47aba319e909de82da946eb36e12aff936708402228295db2712f2fc807c95092a86afd71220699df13e2d2fdf2857976cb1e605f72f1b2edabadba3ff05501221fe81333c13917c85d725ce92791e115eb0289a5d0b3330901bb8b0ed146abeb81381b7331f1c508fb14e057b05d8b0190a9e74a3d046dcd24e7ab747049945b3d8a120c4f6d88e67661b55573aa9b361367488a1ef7dffd967d64a1518";
    #[test_only]
    const BLS12_381_GT_GENERATOR_MUL_BY_7_NEG_SERIALIZED: vector<u8> = x"2041ea7b66c19680e2c0bb23245a71918753220b31f88a925aa9b1e192e7c188a0b365cb994b3ec5e809206117c6411242b940b10caa37ce734496b3b7c63578a0e3c076f9b31a7ca13a716262e0e4cda4ac994efb9e19893cbfe4d464b9210d099d808a08b3c4c3846e7529984899478639c4e6c46152ef49a04af9c8e6ff442d286c4613a3dac6a4bee4b40e1f6b030f2871dabe4223b250c3181ecd3bc6819004745aeb6bac567407f2b9c7d1978c45ee6712ae46930bc00638383f6696158bad488cbe7663d681c96c035481dbcf78e7a7fbaec3799163aa6914cef3365156bdc3e533a7c883d5974e3462ac6f19e3f9ce26800ae248a45c5f0dd3a48a185969224e6cd6af9a048241bdcac9800d94aeee970e08488fb961e36a769b6c184e92a4b9fa2366b1ae8ebdf5542fa1e0ec390c90df40a91e5261800581b5492bd9640d1c5352babc551d1a49998f4517312f55b4339272b28a3e6b0c7d182e2bb61bd7d72b29ae3696db8fafe32b904ab5d0764e46bf21f9a0c9a1f7bedc6b12b9f64820fc8b3fd4a26541472be3c9c93d784cdd53a059d1604bf3292fedd1babfb00398128e3241bc63a5a47b5e9207fcb0c88f7bfddc376a242c9f0c032ba28eec8670f1fa1d47567593b4571c983b8015df91cfa1241b7fb8a57e0e6e01145b98de017eccc2a66e83ced9d83119a505e552467838d35b8ce2f4d7cc9a894f6dee922f35f0e72b7e96f0879b0c8614d3f9e5f5618b5be9b82381628448641a8bb0fd1dffb16c70e6831d8d69f61f2a2ef9e90c421f7a5b1ce7a5d113c7eb01";

    #[test(fx = @std)]
    fun test_bls12_381_gt(fx: signer) {
        enable_initial_generic_algebraic_operations(&fx);
        enable_bls12_381_structures(&fx);

        // Special constants and checks.
        assert!(BLS12_381_R == group_order<BLS12_381_Gt>(), 1);
        let identity = group_identity<BLS12_381_Gt>();
        let generator = group_generator<BLS12_381_Gt>();
        assert!(group_is_identity(&identity), 1);
        assert!(!group_is_identity(&generator), 1);

        // Serialization/deserialization.
        assert!(BLS12_381_GT_GENERATOR_SERIALIZED == serialize_v2<BLS12_381_Gt, BLS12_381_Gt_Format>(&generator), 1);
        let generator_from_deser = std::option::extract(&mut deserialize_v2<BLS12_381_Gt, BLS12_381_Gt_Format>(&BLS12_381_GT_GENERATOR_SERIALIZED));
        assert!(eq(&generator, &generator_from_deser), 1);
        assert!(BLS12_381_FQ12_ONE_SERIALIZED == serialize_v2<BLS12_381_Gt, BLS12_381_Gt_Format>(&identity), 1);
        let identity_from_deser = std::option::extract(&mut deserialize_v2<BLS12_381_Gt, BLS12_381_Gt_Format>(&BLS12_381_FQ12_ONE_SERIALIZED));
        assert!(eq(&identity, &identity_from_deser), 1);
        let element_7g_from_deser = std::option::extract(&mut deserialize_v2<BLS12_381_Gt, BLS12_381_Gt_Format>(&BLS12_381_GT_GENERATOR_MUL_BY_7_SERIALIZED));
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_Gt, BLS12_381_Gt_Format>(&x"ffff")), 1);

        // Deserialization: in Fq12 but not in the prime-order subgroup.
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_Gt, BLS12_381_Gt_Format>(&x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")), 1);

        // Deserialization: byte array of wrong size.
        assert!(std::option::is_none(&deserialize_v2<BLS12_381_Gt, BLS12_381_Gt_Format>(&x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ab")), 1);

        // Element scalar multiplication.
        let scalar_7 = from_u64<BLS12_381_Fr>(7);
        let element_7g_calc = group_scalar_mul_typed(&generator, &scalar_7);
        assert!(eq(&element_7g_calc, &element_7g_from_deser), 1);
        let element_7g_calc = group_scalar_mul(&generator, &x"07");
        assert!(eq(&element_7g_calc, &element_7g_from_deser), 1);
        assert!(BLS12_381_GT_GENERATOR_MUL_BY_7_SERIALIZED == serialize_v2<BLS12_381_Gt, BLS12_381_Gt_Format>(&element_7g_calc), 1);

        // Element negation.
        let element_minus_7g_calc = group_neg(&element_7g_calc);
        assert!(BLS12_381_GT_GENERATOR_MUL_BY_7_NEG_SERIALIZED == serialize_v2<BLS12_381_Gt, BLS12_381_Gt_Format>(&element_minus_7g_calc), 1);

        // Element addition.
        let scalar_9 = from_u64<BLS12_381_Fr>(9);
        let element_9g = group_scalar_mul_typed(&generator, &scalar_9);
        let scalar_2 = from_u64<BLS12_381_Fr>(2);
        let element_2g = group_scalar_mul_typed(&generator, &scalar_2);
        let element_2g_calc = group_add(&element_minus_7g_calc, &element_9g);
        assert!(eq(&element_2g, &element_2g_calc), 1);

        // Subtraction.
        assert!(eq(&element_9g, &group_sub(&element_2g, &element_minus_7g_calc)), 1);

        // Upcasting to BLS12_381_Fq12.
        assert!(eq(&field_one<BLS12_381_Fq12>(), &upcast<BLS12_381_Gt, BLS12_381_Fq12>(&identity)), 1);
    }

    // #[test(fx = @std)]
    // fun test_bls12381_pairing(fx: signer) {
    //     enable_initial_generic_algebraic_operations(&fx);
    //     enable_bls12_381_structures(&fx);
    //
    //     // pairing(a*P,b*Q) == (a*b)*pairing(P,Q)
    //     let element_p = insecure_random_element<BLS12_381_G1>();
    //     let element_q = insecure_random_element<BLS12_381_G2>();
    //     let a = insecure_random_element<BLS12_381_Fr>();
    //     let b = insecure_random_element<BLS12_381_Fr>();
    //     let gt_element = pairing<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(&group_scalar_mul_typed(&element_p, &a), &group_scalar_mul_typed(&element_q, &b));
    //     let gt_element_another = group_scalar_mul_typed(&pairing<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(&element_p, &element_q), &field_mul(&a, &b));
    //     assert!(eq(&gt_element, &gt_element_another), 1);
    // }

    #[test_only]
    struct MysteriousGroup {}

    #[test(fx = @std)]
    #[expected_failure(abort_code = 0x0c0000, location = Self)]
    fun test_unknown_group(fx: signer) {
        enable_initial_generic_algebraic_operations(&fx);
        enable_bls12_381_structures(&fx);
        let _ = group_order<MysteriousGroup>();
    }

    #[test(fx = @std)]
    #[expected_failure(abort_code = 0x0c0000, location = Self)]
    fun test_unknown_pairing(fx: signer) {
        enable_initial_generic_algebraic_operations(&fx);
        enable_bls12_381_structures(&fx);
        // Attempt an invalid pairing: (G2, G1) -> Gt
        pairing<BLS12_381_G2, BLS12_381_G1, BLS12_381_Gt>(
            &insecure_random_element<BLS12_381_G2>(),
            &insecure_random_element<BLS12_381_G1>(),
        );
    }

    // Tests end.
}

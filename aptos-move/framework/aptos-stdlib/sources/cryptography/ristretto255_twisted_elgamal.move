/// This module implements a Twisted ElGamal encryption API, over the Ristretto255 curve, designed to work with
/// additional cryptographic constructs such as Bulletproofs.
///
/// A Twisted ElGamal *ciphertext* encrypts a value `v` under a basepoint `G` and a secondary point `H`, 
/// alongside a public key `Y = sk^(-1) * H`, where `sk` is the corresponding secret key. The ciphertext is of the form:
/// `(v * G + r * H, r * Y)`, where `r` is a random scalar.
///
/// The Twisted ElGamal scheme differs from standard ElGamal by introducing a secondary point `H` to enhance 
/// flexibility and functionality in cryptographic protocols. This design still maintains the homomorphic property:
/// `Enc_Y(v, r) + Enc_Y(v', r') = Enc_Y(v + v', r + r')`, where `v, v'` are plaintexts, `Y` is the public key, 
/// and `r, r'` are random scalars.
module aptos_std::ristretto255_twisted_elgamal {
    use std::option::Option;
    use aptos_std::ristretto255::{Self, CompressedRistretto, RistrettoPoint, Scalar};

    //
    // Structs
    //

    /// A Twisted ElGamal ciphertext, consisting of two Ristretto255 points.
    struct Ciphertext has drop {
        left: RistrettoPoint,   // v * G + r * H
        right: RistrettoPoint,  // r * Y
    }

    /// A compressed Twisted ElGamal ciphertext, consisting of two compressed Ristretto255 points.
    struct CompressedCiphertext has store, copy, drop {
        left: CompressedRistretto,
        right: CompressedRistretto,
    }

    /// A Twisted ElGamal public key, represented as a compressed Ristretto255 point.
    struct CompressedPubkey has store, copy, drop {
        point: CompressedRistretto,
    }

    //
    // Public functions
    //

    /// Creates a new public key from a serialized Ristretto255 point.
    /// Returns `Some(CompressedPubkey)` if the deserialization is successful, otherwise `None`.
    public fun new_pubkey_from_bytes(bytes: vector<u8>): Option<CompressedPubkey> {
        let point = ristretto255::new_compressed_point_from_bytes(bytes);
        if (point.is_some()) {
            let pk = CompressedPubkey {
                point: point.extract()
            };
            std::option::some(pk)
        } else {
            std::option::none()
        }
    }

    /// Serializes a Twisted ElGamal public key into its byte representation.
    public fun pubkey_to_bytes(pubkey: &CompressedPubkey): vector<u8> {
        ristretto255::compressed_point_to_bytes(pubkey.point)
    }

    /// Converts a public key into its corresponding `RistrettoPoint`.
    public fun pubkey_to_point(pubkey: &CompressedPubkey): RistrettoPoint {
        ristretto255::point_decompress(&pubkey.point)
    }

    /// Converts a public key into its corresponding `CompressedRistretto` representation.
    public fun pubkey_to_compressed_point(pubkey: &CompressedPubkey): CompressedRistretto {
        pubkey.point
    }

    /// Creates a new ciphertext from a serialized representation, consisting of two 32-byte Ristretto255 points.
    /// Returns `Some(Ciphertext)` if the deserialization succeeds, otherwise `None`.
    public fun new_ciphertext_from_bytes(bytes: vector<u8>): Option<Ciphertext> {
        if (bytes.length() != 64) {
            return std::option::none()
        };

        let bytes_right = bytes.trim(32);

        let left_point = ristretto255::new_point_from_bytes(bytes);
        let right_point = ristretto255::new_point_from_bytes(bytes_right);

        if (left_point.is_some() && right_point.is_some()) {
            std::option::some(Ciphertext {
                left: left_point.extract(),
                right: right_point.extract()
            })
        } else {
            std::option::none()
        }
    }

    /// Creates a ciphertext `(val * G, 0 * G)` where `val` is the plaintext, and the randomness is set to zero.
    public fun new_ciphertext_no_randomness(val: &Scalar): Ciphertext {
        Ciphertext {
            left: ristretto255::basepoint_mul(val),
            right: ristretto255::point_identity(),
        }
    }

    /// Constructs a Twisted ElGamal ciphertext from two `RistrettoPoint`s.
    public fun ciphertext_from_points(left: RistrettoPoint, right: RistrettoPoint): Ciphertext {
        Ciphertext {
            left,
            right,
        }
    }

    /// Constructs a Twisted ElGamal ciphertext from two compressed Ristretto255 points.
    public fun ciphertext_from_compressed_points(
        left: CompressedRistretto,
        right: CompressedRistretto
    ): CompressedCiphertext {
        CompressedCiphertext {
            left,
            right,
        }
    }

    /// Serializes a Twisted ElGamal ciphertext into its byte representation.
    public fun ciphertext_to_bytes(ct: &Ciphertext): vector<u8> {
        let bytes = ristretto255::point_to_bytes(&ristretto255::point_compress(&ct.left));
        bytes.append(ristretto255::point_to_bytes(&ristretto255::point_compress(&ct.right)));
        bytes
    }

    /// Converts a ciphertext into a pair of `RistrettoPoint`s.
    public fun ciphertext_into_points(c: Ciphertext): (RistrettoPoint, RistrettoPoint) {
        let Ciphertext { left, right } = c;
        (left, right)
    }

    /// Returns the two `RistrettoPoint`s representing the ciphertext.
    public fun ciphertext_as_points(c: &Ciphertext): (&RistrettoPoint, &RistrettoPoint) {
        (&c.left, &c.right)
    }

    /// Compresses a Twisted ElGamal ciphertext into its `CompressedCiphertext` representation.
    public fun compress_ciphertext(ct: &Ciphertext): CompressedCiphertext {
        CompressedCiphertext {
            left: ristretto255::point_compress(&ct.left),
            right: ristretto255::point_compress(&ct.right),
        }
    }

    /// Decompresses a `CompressedCiphertext` back into its `Ciphertext` representation.
    public fun decompress_ciphertext(ct: &CompressedCiphertext): Ciphertext {
        Ciphertext {
            left: ristretto255::point_decompress(&ct.left),
            right: ristretto255::point_decompress(&ct.right),
        }
    }

    /// Adds two ciphertexts homomorphically, producing a new ciphertext representing the sum of the two.
    public fun ciphertext_add(lhs: &Ciphertext, rhs: &Ciphertext): Ciphertext {
        Ciphertext {
            left: ristretto255::point_add(&lhs.left, &rhs.left),
            right: ristretto255::point_add(&lhs.right, &rhs.right),
        }
    }

    /// Adds two ciphertexts homomorphically, updating the first ciphertext in place.
    public fun ciphertext_add_assign(lhs: &mut Ciphertext, rhs: &Ciphertext) {
        ristretto255::point_add_assign(&mut lhs.left, &rhs.left);
        ristretto255::point_add_assign(&mut lhs.right, &rhs.right);
    }

    /// Subtracts one ciphertext from another homomorphically, producing a new ciphertext representing the difference.
    public fun ciphertext_sub(lhs: &Ciphertext, rhs: &Ciphertext): Ciphertext {
        Ciphertext {
            left: ristretto255::point_sub(&lhs.left, &rhs.left),
            right: ristretto255::point_sub(&lhs.right, &rhs.right),
        }
    }

    /// Subtracts one ciphertext from another homomorphically, updating the first ciphertext in place.
    public fun ciphertext_sub_assign(lhs: &mut Ciphertext, rhs: &Ciphertext) {
        ristretto255::point_sub_assign(&mut lhs.left, &rhs.left);
        ristretto255::point_sub_assign(&mut lhs.right, &rhs.right);
    }

    /// Creates a copy of the provided ciphertext.
    public fun ciphertext_clone(c: &Ciphertext): Ciphertext {
        Ciphertext {
            left: ristretto255::point_clone(&c.left),
            right: ristretto255::point_clone(&c.right),
        }
    }

    /// Compares two ciphertexts for equality, returning `true` if they encrypt the same value and randomness.
    public fun ciphertext_equals(lhs: &Ciphertext, rhs: &Ciphertext): bool {
        ristretto255::point_equals(&lhs.left, &rhs.left) &&
            ristretto255::point_equals(&lhs.right, &rhs.right)
    }

    /// Returns the `RistrettoPoint` in the ciphertext that contains the encrypted value in the exponent.
    public fun get_value_component(ct: &Ciphertext): &RistrettoPoint {
        &ct.left
    }

    //
    // Test-only functions
    //

    #[test_only]
    /// Derives a public key from a given secret key using the formula `Y = sk^(-1) * H`.
    /// Returns `Some(CompressedPubkey)` if the secret key inversion succeeds, otherwise `None`.
    public fun pubkey_from_secret_key(sk: &Scalar): Option<CompressedPubkey> {
        let sk_invert = ristretto255::scalar_invert(sk);

        if (sk_invert.is_some()) {
            let point = ristretto255::point_mul(
                &ristretto255::hash_to_point_base(),
                &sk_invert.extract()
            );

            std::option::some(CompressedPubkey {
                point: ristretto255::point_compress(&point)
            })
        } else {
            std::option::none()
        }
    }

    #[test_only]
    /// Constructs a ciphertext `(v * point1 + r * point2, r * pubkey)` where `point1` and `point2` are arbitrary points.
    public fun new_ciphertext(
        v: &Scalar,
        point1: &RistrettoPoint,
        r: &Scalar,
        point2: &RistrettoPoint,
        pubkey: &CompressedPubkey
    ): Ciphertext {
        Ciphertext {
            left: ristretto255::double_scalar_mul(v, point1, r, point2),
            right: ristretto255::point_mul(&pubkey_to_point(pubkey), r),
        }
    }

    #[test_only]
    /// Constructs a ciphertext `(v * G + r * H, r * Y)` using the Ristretto255 basepoint `G` and a secondary basepoint `H`.
    public fun new_ciphertext_with_basepoint(v: &Scalar, r: &Scalar, pubkey: &CompressedPubkey): Ciphertext {
        Ciphertext {
            left: ristretto255::double_scalar_mul(
                v,
                &ristretto255::basepoint(),
                r,
                &ristretto255::hash_to_point_base()
            ),
            right: ristretto255::point_mul(&pubkey_to_point(pubkey), r),
        }
    }

    #[test_only]
    /// Generates a random Twisted ElGamal key pair (`sk`, `Y`), where `Y = sk^(-1) * H`.
    public fun generate_twisted_elgamal_keypair(): (ristretto255::Scalar, CompressedPubkey) {
        let sk = ristretto255::random_scalar();
        let pk = pubkey_from_secret_key(&sk);

        (sk, pk.extract())
    }
}

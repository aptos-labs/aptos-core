/// This module implements an ElGamal encryption API that can be used with the Bulletproofs module.
///
/// An ElGamal encryption of a value v under a generator g and public key y is (v * g + r * y, r * g), for a random scalar r. 
/// Note we place the value v in the exponent of g so that ciphertexts are additively homomorphic, so that Enc(v,y) + Enc(v',y) = Enc(v+v', y) where v,v' are encrypted messages, y is a public key, and the same randomness is used across both encryptions. 

module aptos_std::elgamal {
    use aptos_std::ristretto255::{Self, RistrettoPoint, Scalar, CompressedRistretto, point_compress};

    /// An ElGamal ciphertext to some value.
    struct Ciphertext has drop {
        left: RistrettoPoint,
	right: RistrettoPoint,
    }

    /// An ElGamal public key.
    struct PubKey has drop {
        point: RistrettoPoint,
    }

    /// Moves a Ristretto point into an ElGamal ciphertext.
    public fun new_ciphertext_from_point(left: RistrettoPoint, right: RistrettoPoint): Ciphertext {
        Ciphertext {
            left,
	    right,
        }
    }

    /// Deserializes a ciphertext from a compressed Ristretto point.
    public fun new_ciphertext_from_compressed(left: &CompressedRistretto, right: &CompressedRistretto): Ciphertext {
        Ciphertext {
            left: ristretto255::point_decompress(left),
	    right: ristretto255::point_decompress(right),
        }
    }

    /// Returns a ciphertext (val * val_base + r * pub_key, r * val_base) where val_base is the generator.
    public fun new_ciphertext(val: &Scalar, val_base: &RistrettoPoint, rand: &Scalar, pub_key: &PubKey): Ciphertext {
        Ciphertext {
            left: ristretto255::double_scalar_mul(val, val_base, rand, pub_key.point),
	    right: ristretto255::scalar_mul(rand, val_base),
        }
    }

    /// Returns a ciphertext (val * basepoint + r * pub_key, rand * basepoint) where `basepoint` is the Ristretto255 basepoint.
    public fun new_commitment_with_basepoint(val: &Scalar, rand: &Scalar, pub_key: &PubKey): Ciphertext {
        Ciphertext {
            left: ristretto255::basepoint_double_mul(rand, pub_key.point, val),
	    right: ristretto255::basepoint_mul(rand),
        }
    }

    /// Returns lhs + rhs. Useful for re-randomizing the ciphertext or updating the committed value.
    public fun ciphertext_add(lhs: &Ciphertext, rhs: &Ciphertext): Ciphertext {
        Ciphertext {
            left: ristretto255::point_add(&lhs.left, &rhs.left),
	    right: ristretto255::point_add(&lhs.right, &rhs.right),
        }
    }

    /// Sets lhs = lhs + rhs. Useful for re-randomizing the ciphertext or updating the encrypted value.
    public fun ciphertext_add_assign(lhs: &mut Ciphertext, rhs: &Ciphertext) {
        ristretto255::point_add_assign(&mut lhs.left, &rhs.left);
	ristretto255::point_add_assign(&mut lhs.right, &rhs.right);
    }

    /// Returns lhs - rhs. Useful for re-randomizing the ciphertext or updating the encrypted value.
    public fun ciphertext_sub(lhs: &Ciphertext, rhs: &Ciphertext): Ciphertext {
        Ciphertext {
            left: ristretto255::point_sub(&lhs.left, &rhs.left),
	    right: ristretto255::point_sub(&lhs.right, &rhs.right),
        }
    }

    /// Sets lhs = lhs - rhs. Useful for re-randomizing the ciphertext or updating the encrypted value.
    public fun ciphertext_sub_assign(lhs: &mut Ciphertext, rhs: &Ciphertext) {
        ristretto255::point_sub_assign(&mut lhs.left, &rhs.left);
	ristretto255::point_sub_assign(&mut lhs.right, &rhs.right);
    }

    /// Creates a copy of this ciphertext.
    public fun ciphertext_clone(c: &Ciphertext): Ciphertext {
        Ciphertext {
            left: ristretto255::point_clone(&c.left),
	    right: ristretto255::point_clone(&c.right),
        }
    }

    /// Returns true if the two ciphertexts are identical: i.e., same value and same randomness.
    public fun ciphertext_equals(lhs: &Ciphertext, rhs: &Ciphertext): bool {
        ristretto255::point_equals(&lhs.left, &rhs.left) &&
	ristretto255::point_equals(&lhs.right, &rhs.right)
    }

    /// Returns the underlying elliptic curve point representing the ciphertext as a pair of in-memory RistrettoPoints.
    public fun ciphertext_as_points(c: &Ciphertext): (&RistrettoPoint, &RistrettoPoint) {
        (&c.left, &c.right)
    }

    /// Returns the ciphertext as a pair of CompressedRistretto points.
    public fun ciphertext_as_compressed_points(c: &Ciphertext): (CompressedRistretto, CompressedRistretto)   {
        (point_compress(&c.left), point_compress(&c.right))
    }

    /// Moves the ciphertext into a pair of RistrettoPoints.
    public fun ciphertext_into_points(c: Ciphertext): (RistrettoPoint, RistrettoPoint) {
        let Ciphertext { left, right } = c;
        (left, right)
    }

    /// Moves the ciphertext into a pair of CompressedRistretto points.
    public fun ciphertext_into_compressed_points(c: Ciphertext): (CompressedRistretto, CompressedRistretto) {
        (point_compress(&c.left), point_compress(&c.right))
    }

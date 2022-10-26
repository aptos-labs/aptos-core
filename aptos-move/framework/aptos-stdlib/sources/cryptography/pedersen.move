/// This module implements a Pedersen commitment API that can be used with the Bulletproofs module.
///
/// A Pedersen commitment to a value v under a _commitment key_ (g, h) is v * g + r * h, for a random scalar r.

module aptos_std::pedersen {
    use aptos_std::ristretto255::{Self, RistrettoPoint, Scalar, CompressedRistretto, point_compress};

    /// The default Pedersen randomness base used in our underlying Bulletproofs library.
    /// This is obtained by hashing the compressed Ristretto255 basepoint using SHA3-512 (not SHA2-512).
    const BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE : vector<u8> = x"8c9240b456a9e6dc65c377a1048d745f94a08cdb7f44cbcd7b46f34048871134";

    /// A Pedersen commitment to some value with some randomness.
    struct Commitment has drop {
        point: RistrettoPoint,
    }

    /// Moves a Ristretto point into a Pedersen commitment.
    public fun new_commitment_from_point(point: RistrettoPoint): Commitment {
        Commitment {
            point
        }
    }

    /// Deserializes a commitment from a compressed Ristretto point.
    public fun new_commitment_from_compressed(point: &CompressedRistretto): Commitment {
        Commitment {
            point: ristretto255::point_decompress(point)
        }
    }

    /// Returns a commitment val * val_base + r * rand_base where (val_base, rand_base) is the commitment key.
    public fun new_commitment(val: &Scalar, val_base: &RistrettoPoint, rand: &Scalar, rand_base: &RistrettoPoint): Commitment {
        Commitment {
            point: ristretto255::double_scalar_mul(val, val_base, rand, rand_base)
        }
    }

    /// Returns a commitment val * basepoint + r * rand_base where `basepoint` is the Ristretto255 basepoint.
    public fun new_commitment_with_basepoint(val: &Scalar, rand: &Scalar, rand_base: &RistrettoPoint): Commitment {
        Commitment {
            point: ristretto255::basepoint_double_mul(rand, rand_base, val)
        }
    }

    /// Returns a commitment val * basepoint + r * rand_base where `basepoint` is the Ristretto255 basepoint and `rand_base`
    /// is the default randomness based used in the Bulletproof library (i.e., BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE).
    public fun new_commitment_for_bulletproof(val: &Scalar, rand: &Scalar): Commitment {
        let rand_base = ristretto255::new_point_from_bytes(BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE);
        let rand_base = std::option::extract(&mut rand_base);

        Commitment {
            point: ristretto255::basepoint_double_mul(rand, &rand_base, val)
        }
    }

    /// Returns a non-hiding commitment val * basepoint where `basepoint` is the Ristretto255 basepoint.
    public fun new_non_hiding_commitment_for_bulletproof(val: &Scalar): Commitment {
        Commitment {
            point: ristretto255::basepoint_mul(val)
        }
    }

    /// Returns lhs + rhs. Useful for re-randomizing the commitment or updating the committed value.
    public fun commitment_add(lhs: &Commitment, rhs: &Commitment): Commitment {
        Commitment {
            point: ristretto255::point_add(&lhs.point, &rhs.point)
        }
    }

    /// Sets lhs = lhs + rhs. Useful for re-randomizing the commitment or updating the committed value.
    public fun commitment_add_assign(lhs: &mut Commitment, rhs: &Commitment) {
        ristretto255::point_add_assign(&mut lhs.point, &rhs.point);
    }

    /// Returns lhs - rhs. Useful for re-randomizing the commitment or updating the committed value.
    public fun commitment_sub(lhs: &Commitment, rhs: &Commitment): Commitment {
        Commitment {
            point: ristretto255::point_sub(&lhs.point, &rhs.point)
        }
    }

    /// Sets lhs = lhs - rhs. Useful for re-randomizing the commitment or updating the committed value.
    public fun commitment_sub_assign(lhs: &mut Commitment, rhs: &Commitment) {
        ristretto255::point_sub_assign(&mut lhs.point, &rhs.point);
    }

    /// Creates a copy of this commitment.
    public fun commitment_clone(c: &Commitment): Commitment {
        Commitment {
            point: ristretto255::point_clone(&c.point)
        }
    }

    /// Returns true if the two commitments are identical: i.e., same value and same randomness.
    public fun commitment_equals(lhs: &Commitment, rhs: &Commitment): bool {
        ristretto255::point_equals(&lhs.point, &rhs.point)
    }

    /// Returns the underlying elliptic curve point representing the commitment as an in-memory RistrettoPoint.
    public fun commitment_as_point(c: &Commitment): &RistrettoPoint {
        &c.point
    }

    /// Returns the Pedersen commitment as a CompressedRistretto point.
    public fun commitment_as_compressed_point(c: &Commitment): CompressedRistretto {
        point_compress(&c.point)
    }

    /// Moves the Commitment into a CompressedRistretto point.
    public fun commitment_into_point(c: Commitment): RistrettoPoint {
        let Commitment { point } = c;
        point
    }

    /// Moves the Commitment into a CompressedRistretto point.
    public fun commitment_into_compressed_point(c: Commitment): CompressedRistretto {
        point_compress(&c.point)
    }

    /// Returns the randomness base compatible with the Bulletproofs module.
    public fun randomness_base_for_bulletproof(): RistrettoPoint {
        std::option::extract(&mut ristretto255::new_point_from_bytes(BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE))
    }
}

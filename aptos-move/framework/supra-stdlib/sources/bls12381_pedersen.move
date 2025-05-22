/// This module implements a Pedersen commitment API, over the Ristretto255 curve, that can be used with the
/// Bulletproofs module.
///
/// A Pedersen commitment to a value `v` under _commitment key_ `(g, h)` is `v * g + r * h`, for a random scalar `r`.

module supra_std::bls12381_pedersen {
    use std::option::Option;
    use aptos_std::bls12381_algebra::{G1, FormatG1Compr, Fr};
    use aptos_std::crypto_algebra::{Element, deserialize, serialize, scalar_mul, add, one, sub, eq, multi_scalar_mul};

    //
    // Constants
    //

    /// The default Pedersen randomness base `h` used in our underlying Bulletproofs library.
    /// This is obtained by hashing the compressed Bls12381 basepoint using SHA3-512 (not SHA2-512).
    const BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE : vector<u8> = x"ad39c7c86db77b61b160a67568ef58125fb2e843f83a327ba1f822462b5a2c130731d866e119bcc1766910e3f7672878";

    //
    // Structs
    //

    /// A Pedersen commitment to some value with some randomness.
    struct Commitment has copy, drop {
        point: Element<G1>,
    }

    //
    // Public functions
    //

    /// Creates a new public key from a serialized Bls12381 point.
    public fun new_commitment_from_bytes(bytes: vector<u8>): Option<Commitment> {
        let point = deserialize<G1, FormatG1Compr>(&bytes);
        if (std::option::is_some(&mut point)) {
            let comm = Commitment {
                point: std::option::extract(&mut point)
            };
            std::option::some(comm)
        } else {
            std::option::none<Commitment>()
        }
    }

    /// Returns a commitment as a serialized byte array
    public fun commitment_to_bytes(comm: &Commitment): vector<u8> {
        serialize<G1,FormatG1Compr>(&comm.point)
    }

    /// Moves a Ristretto point into a Pedersen commitment.
    public fun commitment_from_point(point: Element<G1>): Commitment {
        Commitment {
            point
        }
    }

    /// Returns a commitment `v * val_base + r * rand_base` where `(val_base, rand_base)` is the commitment key.
    public fun new_commitment(v: &Element<Fr>, val_base: &Element<G1>, r: &Element<Fr>, rand_base: &Element<G1>): Commitment {
        let a = scalar_mul(val_base, v);
        let b = scalar_mul(rand_base, r);
        Commitment {
            point: add(&a, &b)
        }
    }

    /// Returns a commitment `v * G + r * H` where `G` is the Ristretto255 basepoint and `H` is the default randomness
    /// base used in the Bulletproofs library (i.e., `BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE`).
    public fun new_commitment_for_bulletproof(v: &Element<Fr>, r: &Element<Fr>): Commitment {
        let rand_base = deserialize<G1, FormatG1Compr>(&BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE);
        let rand_base = std::option::extract(&mut rand_base);

        Commitment {
            point: multi_scalar_mul(&vector[one<G1>(), rand_base], &vector[*v, *r])
        }
    }

    /// Homomorphically combines two commitments `lhs` and `rhs` as `lhs + rhs`.
    /// Useful for re-randomizing the commitment or updating the committed value.
    public fun commitment_add(lhs: &Commitment, rhs: &Commitment): Commitment {
        Commitment {
            point: add(&lhs.point, &rhs.point)
        }
    }

    /// Like `commitment_add` but assigns `lhs = lhs + rhs`.
    public fun commitment_add_assign(lhs: &mut Commitment, rhs: &Commitment) {
        lhs.point = add(&lhs.point, &rhs.point);
    }

    /// Homomorphically combines two commitments `lhs` and `rhs` as `lhs - rhs`.
    /// Useful for re-randomizing the commitment or updating the committed value.
    public fun commitment_sub(lhs: &Commitment, rhs: &Commitment): Commitment {
        Commitment {
            point: sub(&lhs.point, &rhs.point)
        }
    }

    /// Like `commitment_add` but assigns `lhs = lhs - rhs`.
    public fun commitment_sub_assign(lhs: &mut Commitment, rhs: &Commitment) {
        lhs.point = sub(&lhs.point, &rhs.point);
    }

    /// Returns true if the two commitments are identical: i.e., same value and same randomness.
    public fun commitment_equals(lhs: &Commitment, rhs: &Commitment): bool {
        eq(&lhs.point, &rhs.point)
    }

    /// Returns the underlying elliptic curve point representing the commitment as an in-memory `RistrettoPoint`.
    public fun commitment_as_point(c: &Commitment): &Element<G1> {
        &c.point
    }

    /// Moves the Commitment into a CompressedRistretto point.
    public fun commitment_into_point(c: Commitment): Element<G1> {
        let Commitment { point } = c;
        point
    }

    /// Returns the randomness base compatible with the Bulletproofs module.
    ///
    /// Recal that a Bulletproof range proof attests, in zero-knowledge, that a value `v` inside a Pedersen commitment
    /// `v * g + r * h` is sufficiently "small" (e.g., is 32-bits wide). Here, `h` is referred to as the
    /// "randomness base" of the commitment scheme.
    ///
    /// Bulletproof has a default choice for `g` and `h` and this function returns the default `h` as used in the
    /// Bulletproofs Move module.
    public fun randomness_base_for_bulletproof(): Element<G1> {
        std::option::extract(&mut deserialize<G1, FormatG1Compr>(&BULLETPROOF_DEFAULT_PEDERSEN_RAND_BASE))
    }
}

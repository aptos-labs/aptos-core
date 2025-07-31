/// The Schnorr relation is:
///
///     R(G, Y; x) =?= 1   <=>   Y =?= x G
///
/// This can be framed as a homomorphism check:
///
///     \psi(w)   =?=    f(G, Y)
///
/// where:
///
///   1. The transformation function $f$ is:
///
///     f(G, Y) := Y
///       ^^^^
///        |
///      stmt.points
///
///   2. The homomorphism $\psi$ is
///
///     \psi(w) := w G
///
///
module sigma_protocols::homomorphism_schnorr_example {
    use aptos_std::ristretto255::{RistrettoPoint, Scalar};
    use sigma_protocols::homomorphism::new_secret_witness;

    use aptos_std::ristretto255::scalar_one;
    use sigma_protocols::public_statement::{PublicStatement, new_public_statement};
    use sigma_protocols::homomorphism::{Self, Proof, SecretWitness};
    use sigma_protocols::representation::new_representation;
    use sigma_protocols::representation_vec::{RepresentationVec, new_representation_vec};
    #[test_only]
    use aptos_std::ristretto255::{point_mul, random_point, random_scalar, point_equals, point_clone};
    #[test_only]
    use sigma_protocols::homomorphism::empty_proof;

    /// Application-specific domain-separator
    const DST : vector<u8> = b"My Schnorr test case app";
    /// Protocol-specific domain-separator
    const NAME : vector<u8> = b"Schnorr's ZKPoK of discrete log";

    /// Index of $G$ in the `PublicStatement::points` vector.
    const IDX_G: u64 = 0;

    /// Index of $Y$ in the `PublicStatement::points` vector.
    const IDX_Y: u64 = 1;

    /// Index of $x$ in the `SecretWitness::w` vector.
    const IDX_x: u64 = 0;

    fun new_schnorr_statement(_G: RistrettoPoint, _Y: RistrettoPoint): PublicStatement {
        // [ G, Y ]
        new_public_statement(vector[_G, _Y], vector[])
    }

    fun new_schnorr_witness(x: Scalar): SecretWitness {
        // [ x ]
        new_secret_witness(vector[x])
    }

    fun psi(_stmt: &PublicStatement, w: &SecretWitness): RepresentationVec {
        // [
        //   [ x G ]
        // ]
        new_representation_vec(vector[
            new_representation(vector[IDX_G], vector[*w.get_scalar(IDX_x)])
        ])
    }

    /// Returns $Y$, which is stored in `stmt.points[IDX_Y]`
    fun f(_stmt: &PublicStatement): RepresentationVec {
        // [
        //   [ Y ]
        // ]
        new_representation_vec(vector[
            new_representation(vector[IDX_Y], vector[scalar_one()])
        ])
    }

    public fun schnorr_verify(stmt: &PublicStatement, proof: &Proof): bool {
        homomorphism::verify(
            b"my_test_for_schnorr_domain_separator",
            b"schnorr",
            |_X, w| psi(_X, w),
            |_X| f(_X),
            stmt,
            proof
        )
    }

    #[test]
    fun correctness() {
        let x = random_scalar();
        let _G = random_point();
        let _Y = point_mul(&_G, &x);

        let stmt = new_schnorr_statement(point_clone(&_G), _Y);
        let witn = new_schnorr_witness(x);

        let (proof, randomness) = homomorphism::prove(
            DST, NAME,
            |_X, w| psi(_X, w),
            &stmt,
            &witn
        );

        let _A = &proof.get_commitment()[0];
        let alpha = randomness.get_scalar(IDX_x);

        // Make sure the returned commitment A = \alpha G, as it should be.
        assert!(point_equals(_A, &point_mul(&_G, alpha)), 1);

        // Make sure the sigma protocol proof verifies
        assert!(schnorr_verify(&stmt, &proof), 2);
    }

    #[test]
    #[expected_failure(abort_code=65540, location=sigma_protocols::homomorphism)]
    /// An empty proof should NOT verify!
    fun empty_proof_should_not_verify() {
        let stmt = new_public_statement(vector[], vector[]);
        let proof = empty_proof();

        assert!(schnorr_verify(&stmt, &proof), 1);
    }
}

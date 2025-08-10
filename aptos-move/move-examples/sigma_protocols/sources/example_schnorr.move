/// A Schnorr ZKPoK of $x$ such that $Y = x G$.
///
/// The Schnorr NP relation is:
///
///     R(G, Y; x) =?= 1   <=>   Y =?= x G
///
/// This can be framed as a homomorphism check:
///
///     \psi(x)   =?=    f(G, Y)
///
/// where:
///
///   1. The homomorphism $\psi$ is
///
///     \psi(x) := [ x G ]
///
///   2. The transformation function $f$ is:
///
///     f(G, Y) := [ Y ]
///       ^^^^
///        |
///      stmt.points
///
module sigma_protocols::example_schnorr {
    use std::error;
    use aptos_std::ristretto255::{RistrettoPoint, Scalar};

    use aptos_std::ristretto255::scalar_one;
    use sigma_protocols::secret_witness::{SecretWitness, new_secret_witness};
    use sigma_protocols::public_statement::{PublicStatement, new_public_statement};
    use sigma_protocols::representation::new_representation;
    use sigma_protocols::representation_vec::{RepresentationVec, new_representation_vec};
    #[test_only]
    use aptos_std::ristretto255::{point_mul, random_point, random_scalar, point_clone};
    #[test_only]
    use sigma_protocols::homomorphism::{Self, evaluate_homomorphism};
    #[test_only]
    use sigma_protocols::utils::equal_vec_points;

    /// Application-specific domain-separator
    const DST : vector<u8> = b"My Schnorr test case app";

    /// Index of $G$ in the `PublicStatement::points` vector.
    const IDX_G: u64 = 0;
    /// Index of $Y$ in the `PublicStatement::points` vector.
    const IDX_Y: u64 = 1;

    /// Index of $x$ in the `SecretWitness::w` vector.
    const IDX_x: u64 = 0;

    /// The expected number of points $n_1$ in a PedEq statement is 4.
    const E_WRONG_N_1: u64 = 1;
    /// The expected number of scalars $n_2$ in a PedEq statement is 0.
    const E_WRONG_N_2: u64 = 2;
    /// The expected number of scalars $k$ in a PedEq witness is 3.
    const E_WRONG_K: u64 = 3;
    /// The expected number of points $m$ in the image of the PedEq homomorphism and transformation function is 2.
    const E_WRONG_M: u64 = 4;

    fun new_schnorr_statement(_G: RistrettoPoint, _Y: RistrettoPoint): PublicStatement {
        new_public_statement(vector[_G, _Y], vector[])
    }

    fun new_schnorr_witness(x: Scalar): SecretWitness {
        new_secret_witness(vector[x])
    }


    fun psi(_stmt: &PublicStatement, w: &SecretWitness): RepresentationVec {
        assert!(_stmt.get_points().length() == 2, error::invalid_argument(E_WRONG_N_1));
        assert!(_stmt.get_scalars().length() == 0, error::invalid_argument(E_WRONG_N_2));
        assert!(w.length() == 1, error::invalid_argument(E_WRONG_K));
        // [
        //   x G
        // ]
        let repr_vec = new_representation_vec(vector[
            new_representation(vector[IDX_G], vector[*w.get(IDX_x)])
        ]);

        assert!(repr_vec.length() == 1, error::invalid_argument(E_WRONG_M));

        repr_vec
    }

    fun f(_stmt: &PublicStatement): RepresentationVec {
        // [
        //   Y
        // ]
        let repr_vec = new_representation_vec(vector[
            new_representation(vector[IDX_Y], vector[scalar_one()])
        ]);

        assert!(repr_vec.length() == 1, E_WRONG_M);

        repr_vec
    }

    #[test_only]
    fun random_statement_witness_pair(): (PublicStatement, SecretWitness) {
        let x = random_scalar();
        let _G = random_point();
        let _Y = point_mul(&_G, &x);

        let stmt = new_schnorr_statement(point_clone(&_G), _Y);
        let witn = new_schnorr_witness(x);

        (stmt, witn)
    }

    #[test]
    /// In an abundance of caution, we double-check our homomorphism $\psi$ is implemented correctly by evaluating it
    /// at a random point and testing the evaluation against one computed by hand manually.
    fun psi_correctness() {
        let (_X, w) = random_statement_witness_pair();

        // Expected evaluation, computed by hand manually
        let _G = _X.get_point(IDX_G);
        let x = w.get(IDX_x);
        let expected_psi = vector[ point_mul(_G, x) ];

        // Actual evaluation, computed via our $\psi$ implementation
        let actual_psi = evaluate_homomorphism(|_X, w| psi(_X, w), &_X, &w);

        assert!(equal_vec_points(&actual_psi, &expected_psi), 1);
    }

    #[test]
    fun proof_correctness() {
        let (stmt, witn) = random_statement_witness_pair();

        homomorphism::assert_correctly_computed_proof_verifies(
            DST,
            stmt,
            witn,
            |_X, w| psi(_X, w),
            |_X| f(_X),
        );
    }

    #[test]
    #[expected_failure(abort_code=65538, location=sigma_protocols::homomorphism)]
    fun empty_proof_for_random_statement_test() {
        assert!(
            !homomorphism::empty_proof_verifies(
                DST,
                |_X, w| psi(_X, w),
                |_X| f(_X),
                new_schnorr_statement(random_point(), random_point())
            ), 1);
    }

    #[test]
    #[expected_failure(abort_code=65538, location=sigma_protocols::homomorphism)]
    fun empty_proof_for_empty_statement_test() {
        assert!(
            !homomorphism::empty_proof_verifies(
                DST,
                |_X, w| psi(_X, w),
                |_X| f(_X),
                new_public_statement(vector[], vector[])
            ), 1);
    }
}

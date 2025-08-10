/// A ZKPoK of $m, r_1, r_2$ such that $C_1 = m G + r_1 H$ and $C_2 = m G + r_2 H$.
///
/// The NP relation is:
///
///     R(G, H, C_1, C_2;
///       m, r_1, r_2)    =?= 1   <=>   {  C_1 =?= m G + r_1 H  } AND
///                                     {  C_2 =?= m G + r_2 H  }
///
/// This can be framed as a homomorphism check:
///
///     \psi(m, r_1, r_2)   =?=    f(G, H, C_1, C_2)
///
/// where:
///
///   1. The homomorphism $\psi$ is
///
///     \psi(m, r_1, r_2) := [
///                             m G + r_1 H,
///                             m G + r_2 H
///                          ]
///
///   2. The transformation function $f$ is:
///
///     f(G, H, C_1, C_2) := [
///                             C_1,
///                             C_2
///                          ]
///       ^^^^^^^^^^^^^^
///        |
///      stmt.points
module sigma_protocols::example_pedeq {
    use std::error;
    use aptos_std::ristretto255::{RistrettoPoint, Scalar};

    use aptos_std::ristretto255::scalar_one;
    use sigma_protocols::secret_witness::{SecretWitness, new_secret_witness};
    use sigma_protocols::public_statement::{PublicStatement, new_public_statement};
    use sigma_protocols::representation::new_representation;
    use sigma_protocols::representation_vec::{RepresentationVec, new_representation_vec};
    #[test_only]
    use sigma_protocols::homomorphism::{Self, evaluate_homomorphism};
    #[test_only]
    use aptos_std::ristretto255::{point_mul, random_point, random_scalar, point_add};
    #[test_only]
    use sigma_protocols::utils::equal_vec_points;

    /// Application-specific domain-separator
    const DST : vector<u8> = b"My PedEq test case app";

    /// Index of $G$ in the `PublicStatement::points` vector.
    const IDX_G: u64 = 0;
    /// Index of $H$ in the `PublicStatement::points` vector.
    const IDX_H: u64 = 1;
    /// Index of $C_1$ in the `PublicStatement::points` vector.
    const IDX_C_1: u64 = 2;
    /// Index of $C_2$ in the `PublicStatement::points` vector.
    const IDX_C_2: u64 = 3;

    /// Index of $m$ in the `SecretWitness::w` vector.
    const IDX_m: u64 = 0;
    /// Index of $r_1$ in the `SecretWitness::w` vector.
    const IDX_r_1: u64 = 1;
    /// Index of $r_2$ in the `SecretWitness::w` vector.
    const IDX_r_2: u64 = 2;

    /// The expected number of points $n_1$ in a PedEq statement is 4.
    const E_WRONG_N_1: u64 = 1;
    /// The expected number of scalars $n_2$ in a PedEq statement is 0.
    const E_WRONG_N_2: u64 = 2;
    /// The expected number of scalars $k$ in a PedEq witness is 3.
    const E_WRONG_K: u64 = 3;
    /// The expected number of points $m$ in the image of the PedEq homomorphism and transformation function is 2.
    const E_WRONG_M: u64 = 4;

    /// Creates a new PedEq statement.
    fun new_pedeq_statement(_G: RistrettoPoint, _H: RistrettoPoint,
                            _C_1: RistrettoPoint, _C_2: RistrettoPoint): PublicStatement {
        new_public_statement(vector[_G, _H, _C_1, _C_2], vector[])
    }

    /// Creates a new PedEq witness.
    fun new_pedeq_witness(m: Scalar, r_1: Scalar, r_2: Scalar): SecretWitness {
        new_secret_witness(vector[m, r_1, r_2])
    }

    /// Note: It is good practice to assert your statement, your witness and the homomorphism's output have the right
    /// sizes.
    ///
    /// For the PedEq relation, $n_1, n_2, k, m$ are constants. But it is possible to implement relation "families"
    /// which take a variable number of inputs (e.g., imagine this PedEq generalized to $n$ commitments).
    fun psi(_stmt: &PublicStatement, w: &SecretWitness): RepresentationVec {
        assert!(_stmt.get_points().length() == 4, error::invalid_argument(E_WRONG_N_1));
        assert!(_stmt.get_scalars().length() == 0, error::invalid_argument(E_WRONG_N_2));
        assert!(w.length() == 3, error::invalid_argument(E_WRONG_K));

        // [
        //   m G + r_1 H,
        //   m G + r_2 H
        // ]
        let repr_vec = new_representation_vec(vector[
            new_representation(vector[IDX_G, IDX_H], vector[*w.get(IDX_m), *w.get(IDX_r_1)]),
            new_representation(vector[IDX_G, IDX_H], vector[*w.get(IDX_m), *w.get(IDX_r_2)]),
        ]);

        assert!(repr_vec.length() == 2, error::invalid_argument(E_WRONG_M));

        repr_vec
    }

    /// Note: It is good practice to assert your transformation function's output has the right # of group elements.
    fun f(_stmt: &PublicStatement): RepresentationVec {
        // [
        //   C_1,
        //   C_2
        // ]
        let repr_vec = new_representation_vec(vector[
            new_representation(vector[IDX_C_1], vector[scalar_one()]),
            new_representation(vector[IDX_C_2], vector[scalar_one()])
        ]);

        assert!(repr_vec.length() == 2, E_WRONG_M);

        repr_vec
    }

    #[test_only]
    fun random_statement_witness_pair(): (PublicStatement, SecretWitness) {
        let m = random_scalar();
        let r_1 = random_scalar();
        let r_2 = random_scalar();
        let _G = random_point();
        let _H = random_point();
        let m_G = point_mul(&_G, &m);
        let r_1_H = point_mul(&_H, &r_1);
        let r_2_H = point_mul(&_H, &r_2);
        let _C_1 = point_add(&m_G, &r_1_H);
        let _C_2 = point_add(&m_G, &r_2_H);

        let stmt = new_pedeq_statement(_G, _H, _C_1, _C_2);
        let witn = new_pedeq_witness(m, r_1, r_2);

        (stmt, witn)
    }

    #[test]
    /// In an abundance of caution, we double-check our homomorphism $\psi$ is implemented correctly by evaluating it
    /// at a random point and testing the evaluation against one computed by hand manually.
    fun psi_correctness() {
        let (_X, w) = random_statement_witness_pair();

        // Expected evaluation, computed by hand manually
        let _G = _X.get_point(IDX_G);
        let _H = _X.get_point(IDX_H);
        let m = w.get(IDX_m);
        let r_1 = w.get(IDX_r_1);
        let r_2 = w.get(IDX_r_2);
        let expected_psi = vector[
            point_add(
                &point_mul(_G, m),
                &point_mul(_H, r_1)
            ),
            point_add(
                &point_mul(_G, m),
                &point_mul(_H, r_2)
            )
        ];

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
                new_pedeq_statement(random_point(), random_point(), random_point(), random_point())
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

/// A ZKPoK of $v, r_1, r_2$ such that $C_1 = v G + r_1 H$ and $C_2 = v G + r_2 H$.
///
/// The NP relation is:
/// ```
///     R(G, H, C_1, C_2;
///       v, r_1, r_2)    =?= 1   <=>   {  C_1 =?= v G + r_1 H  } AND
///                                     {  C_2 =?= v G + r_2 H  }
/// ```
/// This can be framed as a homomorphism check:
/// ```
///     \psi(v, r_1, r_2)   =?=    f(G, H, C_1, C_2)
/// ```
/// where:
///
///   1. The homomorphism $\psi$ is
///   ```
///     \psi(v, r_1, r_2) := [
///                             v G + r_1 H,
///                             v G + r_2 H
///                          ]
///   ```
///   2. The transformation function $f$ is:
///   ```
///     f(G, H, C_1, C_2) := [
///                             C_1,
///                             C_2
///                          ]
///       ^^^^^^^^^^^^^^
///        |
///      stmt.points
///   ```
module aptos_experimental::sigma_protocol_pedeq_example {
    use std::error;
    use aptos_std::ristretto255::{RistrettoPoint, Scalar, CompressedRistretto};

    use aptos_std::ristretto255::scalar_one;
    use aptos_experimental::sigma_protocol_fiat_shamir::{DomainSeparator, new_domain_separator};
    use aptos_experimental::sigma_protocol_witness::{Witness, new_secret_witness};
    use aptos_experimental::sigma_protocol_statement::{Statement, new_statement};
    use aptos_experimental::sigma_protocol_representation::new_representation;
    use aptos_experimental::sigma_protocol_representation_vec::{RepresentationVec, new_representation_vec};
    #[test_only]
    use aptos_experimental::sigma_protocol_homomorphism::evaluate_psi;
    #[test_only]
    use aptos_std::ristretto255::{point_mul, random_point, random_scalar, point_add, point_compress};
    #[test_only]
    use aptos_experimental::sigma_protocol;
    #[test_only]
    use aptos_experimental::sigma_protocol_utils::equal_vec_points;

    /// Protocol ID used for domain separation
    const PROTOCOL_ID: vector<u8> = b"My PedEq test case app";

    /// Index of $G$ in the `PublicStatement::points` vector.
    const IDX_G: u64 = 0;
    /// Index of $H$ in the `PublicStatement::points` vector.
    const IDX_H: u64 = 1;
    /// Index of $C_1$ in the `PublicStatement::points` vector.
    const IDX_C_1: u64 = 2;
    /// Index of $C_2$ in the `PublicStatement::points` vector.
    const IDX_C_2: u64 = 3;

    /// Index of $v$ in the `SecretWitness::w` vector.
    const IDX_v: u64 = 0;
    /// Index of $r_1$ in the `SecretWitness::w` vector.
    const IDX_r_1: u64 = 1;
    /// Index of $r_2$ in the `SecretWitness::w` vector.
    const IDX_r_2: u64 = 2;

    /// The number of points $n_1$ in a PedEq public statement
    /// WARNING: Crucial for security.
    const N_1: u64 = 4;
    /// The number of scalars $n_1$ in a PedEq public statement
    /// WARNING: Crucial for security.
    const N_2: u64 = 0;
    /// The number of scalars $k$ in a PedEq secret witness
    /// WARNING: Crucial for security.
    const K: u64 = 3;
    /// The number of points $v$ in the image of the PedEq homomorphism and transformation function
    /// WARNING: Crucial for security.
    const M: u64 = 2;

    /// The expected number of points $n_1$ in a PedEq statement is `N_1 = 4`.
    const E_WRONG_N_1: u64 = 1;
    /// The expected number of scalars $n_2$ in a PedEq statement is `N_2 = 0`.
    const E_WRONG_N_2: u64 = 2;
    /// The expected number of scalars $k$ in a PedEq witness is `K = 3`.
    const E_WRONG_K: u64 = 3;
    /// The expected number of points $v$ in the image of the PedEq homomorphism and transformation function is `M = 2`.
    const E_WRONG_M: u64 = 4;

    fun new_session(session_id: vector<u8>): DomainSeparator {
        new_domain_separator(PROTOCOL_ID, session_id)
    }

    /// Creates a new PedEq statement.
    fun new_pedeq_statement(
        compressed_G: CompressedRistretto, _G: RistrettoPoint,
        compressed_H: CompressedRistretto, _H: RistrettoPoint,
        compressed_C_1: CompressedRistretto, _C_1: RistrettoPoint,
        compressed_C_2: CompressedRistretto, _C_2: RistrettoPoint,
    ): Statement {
        new_statement(
            vector[_G, _H, _C_1, _C_2],
            vector[compressed_G, compressed_H, compressed_C_1, compressed_C_2],
            vector[]
        )
    }

    /// Creates a new PedEq witness.
    fun new_pedeq_witness(v: Scalar, r_1: Scalar, r_2: Scalar): Witness {
        new_secret_witness(vector[v, r_1, r_2])
    }

    /// WARNING: See README.md in the `sigma_protocols/` directory for principles on how to implement this correctly!
    fun psi(stmt: &Statement, w: &Witness): RepresentationVec {
        // WARNING: Crucial for security
        assert!(stmt.get_points().length() == N_1, error::invalid_argument(E_WRONG_N_1));
        // WARNING: Crucial for security
        assert!(stmt.get_scalars().length() == N_2, error::invalid_argument(E_WRONG_N_2));
        // WARNING: Crucial for security
        assert!(w.length() == K, error::invalid_argument(E_WRONG_K));

        // [
        //   v G + r_1 H,
        //   v G + r_2 H
        // ]
        let repr_vec = new_representation_vec(vector[
            new_representation(vector[IDX_G, IDX_H], vector[*w.get(IDX_v), *w.get(IDX_r_1)]),
            new_representation(vector[IDX_G, IDX_H], vector[*w.get(IDX_v), *w.get(IDX_r_2)]),
        ]);

        // WARNING: Crucial for security
        assert!(repr_vec.length() == M, error::invalid_argument(E_WRONG_M));

        repr_vec
    }

    /// WARNING: See README.md in the `sigma_protocols/` directory for principles on how to implement this correctly!
    fun f(stmt: &Statement): RepresentationVec {
        // WARNING: Crucial for security
        assert!(stmt.get_points().length() == N_1, error::invalid_argument(E_WRONG_N_1));
        // WARNING: Crucial for security
        assert!(stmt.get_scalars().length() == N_2, error::invalid_argument(E_WRONG_N_2));

        // [
        //   C_1,
        //   C_2
        // ]
        let repr_vec = new_representation_vec(vector[
            new_representation(vector[IDX_C_1], vector[scalar_one()]),
            new_representation(vector[IDX_C_2], vector[scalar_one()])
        ]);

        // WARNING: Crucial for security
        assert!(repr_vec.length() == M, error::invalid_argument(E_WRONG_M));

        repr_vec
    }

    #[test_only]
    fun random_statement_witness_pair(): (Statement, Witness) {
        let v = random_scalar();
        let r_1 = random_scalar();
        let r_2 = random_scalar();
        let _G = random_point(); // Move linter does not let us use G
        let _H = random_point(); // Move linter does not let us use H
        let v_G = point_mul(&_G, &v);
        let r_1_H = point_mul(&_H, &r_1);
        let r_2_H = point_mul(&_H, &r_2);
        let _C_1 = point_add(&v_G, &r_1_H);  // Move linter does not let us use C_1
        let _C_2 = point_add(&v_G, &r_2_H);  // Move linter does not let us use C_2

        let stmt = new_pedeq_statement(
            point_compress(&_G), _G,
            point_compress(&_H), _H,
            point_compress(&_C_1), _C_1,
            point_compress(&_C_2), _C_2,
        );
        let witn = new_pedeq_witness(v, r_1, r_2);

        (stmt, witn)
    }

    #[test]
    /// In an abundance of caution, we double-check our homomorphism $\psi$ is implemented correctly by evaluating it
    /// at a random point and testing the evaluation against one computed by hand manually.
    fun psi_correctness() {
        let (_X, w) = random_statement_witness_pair();

        // Expected evaluation, computed by hand manually
        let _G = _X.get_point(IDX_G); // Move linter does not let us use G
        let _H = _X.get_point(IDX_H); // Move linter does not let us use H
        let v = w.get(IDX_v);
        let r_1 = w.get(IDX_r_1);
        let r_2 = w.get(IDX_r_2);
        let expected_psi = vector[
            point_add(
                &point_mul(_G, v),
                &point_mul(_H, r_1)
            ),
            point_add(
                &point_mul(_G, v),
                &point_mul(_H, r_2)
            )
        ];

        // Actual evaluation, computed via our $\psi$ implementation
        // TODO(Ugly): Change `|_X, w| psi(_X, w)` to `psi` when public structs ship and allow this.
        let actual_psi = evaluate_psi(|_X, w| psi(_X, w), &_X, &w);

        assert!(equal_vec_points(&actual_psi, &expected_psi), 1);
    }

    #[test]
    fun proof_correctness() {
        let (stmt, witn) = random_statement_witness_pair();

        // TODO(Ugly): Change `|_X, w| psi(_X, w)` to `psi` and `|_X| f(_X)` to `f` when public structs ship and allow this.
        sigma_protocol::assert_correctly_computed_proof_verifies(
            new_session(b"session: test pedeq proving correctness"),
            stmt,
            witn,
            |_X, w| psi(_X, w),
            |_X| f(_X),
        );
    }

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_experimental::sigma_protocol_fiat_shamir)]
    fun empty_proof_for_random_statement_test() {
        let _G = random_point();
        let _H = random_point();
        let _C_1 = random_point();
        let _C_2 = random_point();
        // TODO(Ugly): Change `|_X, w| psi(_X, w)` to `psi` and `|_X| f(_X)` to `f` when public structs ship and allow this.
        assert!(
            !sigma_protocol::empty_proof_verifies(
                new_session(b"session: test empty pedeq proof for random statement does not verify"),
                |_X, w| psi(_X, w),
                |_X| f(_X),
                new_pedeq_statement(
                    point_compress(&_G), _G,
                    point_compress(&_H), _H,
                    point_compress(&_C_1), _C_1,
                    point_compress(&_C_2), _C_2,
                )
            ), 1);
    }

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_experimental::sigma_protocol_fiat_shamir)]
    fun empty_proof_for_empty_statement_test() {
        // TODO(Ugly): Change `|_X, w| psi(_X, w)` to `psi` and `|_X| f(_X)` to `f` when public structs ship and allow this.
        assert!(
            !sigma_protocol::empty_proof_verifies(
                new_session(b"session: test empty pedeq proof for empty statement does not verify"),
                |_X, w| psi(_X, w),
                |_X| f(_X),
                new_statement(vector[], vector[], vector[])
            ), 1);
    }
}

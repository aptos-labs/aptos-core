#[test_only]
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
    use aptos_std::ristretto255::{Scalar, CompressedRistretto};

    use aptos_framework::chain_id;
    use aptos_experimental::sigma_protocol_fiat_shamir::{DomainSeparator, new_domain_separator};
    use aptos_experimental::sigma_protocol_witness::{Witness, new_secret_witness};
    use aptos_experimental::sigma_protocol_statement::Statement;
    use aptos_experimental::sigma_protocol_statement_builder::new_builder;
    use aptos_experimental::sigma_protocol_representation::{repr_point, new_representation};
    use aptos_experimental::sigma_protocol_representation_vec::{RepresentationVec, new_representation_vec};
    #[test_only]
    use aptos_experimental::sigma_protocol_homomorphism::evaluate_psi;
    #[test_only]
    use aptos_std::ristretto255::{random_point, random_scalar};
    #[test_only]
    use aptos_experimental::sigma_protocol;
    #[test_only]
    use aptos_framework::account;
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

    /// Phantom marker type for PedEq statements.
    struct PedEq has drop {}

    /// The expected number of points $n_1$ in a PedEq statement is `N_1 = 4`.
    const E_WRONG_N_1: u64 = 1;
    /// The expected number of scalars $n_2$ in a PedEq statement is `N_2 = 0`.
    const E_WRONG_N_2: u64 = 2;
    /// The expected number of scalars $k$ in a PedEq witness is `K = 3`.
    const E_WRONG_K: u64 = 3;
    /// The expected number of points $v$ in the image of the PedEq homomorphism and transformation function is `M = 2`.
    const E_WRONG_M: u64 = 4;

    fun new_session(session_id: vector<u8>): DomainSeparator {
        new_domain_separator(@aptos_experimental, chain_id::get(), PROTOCOL_ID, session_id)
    }

    /// Creates a new PedEq statement using the builder.
    fun new_pedeq_statement(
        compressed_G: CompressedRistretto,
        compressed_H: CompressedRistretto,
        compressed_C_1: CompressedRistretto,
        compressed_C_2: CompressedRistretto,
    ): Statement<PedEq> {
        let b = new_builder();
        b.add_point(compressed_G);
        b.add_point(compressed_H);
        b.add_point(compressed_C_1);
        b.add_point(compressed_C_2);
        b.build()
    }

    /// Creates a new PedEq witness.
    fun new_pedeq_witness(v: Scalar, r_1: Scalar, r_2: Scalar): Witness {
        new_secret_witness(vector[v, r_1, r_2])
    }

    /// WARNING: See README.md in the `sigma_protocols/` directory for principles on how to implement this correctly!
    fun psi(stmt: &Statement<PedEq>, w: &Witness): RepresentationVec {
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
    fun f(stmt: &Statement<PedEq>): RepresentationVec {
        // WARNING: Crucial for security
        assert!(stmt.get_points().length() == N_1, error::invalid_argument(E_WRONG_N_1));
        // WARNING: Crucial for security
        assert!(stmt.get_scalars().length() == N_2, error::invalid_argument(E_WRONG_N_2));

        // [
        //   C_1,
        //   C_2
        // ]
        let repr_vec = new_representation_vec(vector[
            repr_point(IDX_C_1),
            repr_point(IDX_C_2)
        ]);

        // WARNING: Crucial for security
        assert!(repr_vec.length() == M, error::invalid_argument(E_WRONG_M));

        repr_vec
    }

    #[test_only]
    fun random_statement_witness_pair(): (Statement<PedEq>, Witness) {
        let v = random_scalar();
        let r_1 = random_scalar();
        let r_2 = random_scalar();
        let _G = random_point(); // Move linter does not let us use G
        let _H = random_point(); // Move linter does not let us use H
        let v_G = _G.point_mul(&v);
        let r_1_H = _H.point_mul(&r_1);
        let r_2_H = _H.point_mul(&r_2);
        let _C_1 = v_G.point_add(&r_1_H);  // Move linter does not let us use C_1
        let _C_2 = v_G.point_add(&r_2_H);  // Move linter does not let us use C_2

        let stmt = new_pedeq_statement(
            _G.point_compress(),
            _H.point_compress(),
            _C_1.point_compress(),
            _C_2.point_compress(),
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
            _G.point_mul(v).point_add(
                &_H.point_mul(r_1)
            ),
            _G.point_mul(v).point_add(
                &_H.point_mul(r_2)
            )
        ];

        // Actual evaluation, computed via our $\psi$ implementation
        let actual_psi = evaluate_psi(|_X, w| psi(_X, w), &_X, &w);

        assert!(equal_vec_points(&actual_psi, &expected_psi), 1);
    }

    #[test]
    fun proof_correctness() {
        chain_id::initialize_for_test(&account::create_signer_for_test(@aptos_framework), 4);
        let (stmt, witn) = random_statement_witness_pair();

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
        chain_id::initialize_for_test(&account::create_signer_for_test(@aptos_framework), 4);
        assert!(
            !sigma_protocol::empty_proof_verifies(
                new_session(b"session: test empty pedeq proof for random statement does not verify"),
                |_X, w| psi(_X, w),
                |_X| f(_X),
                new_pedeq_statement(
                    random_point().point_compress(),
                    random_point().point_compress(),
                    random_point().point_compress(),
                    random_point().point_compress(),
                )
            ), 1);
    }

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_experimental::sigma_protocol_fiat_shamir)]
    fun empty_proof_for_empty_statement_test() {
        chain_id::initialize_for_test(&account::create_signer_for_test(@aptos_framework), 4);
        let b = new_builder();
        let stmt: Statement<PedEq> = b.build();
        assert!(
            !sigma_protocol::empty_proof_verifies(
                new_session(b"session: test empty pedeq proof for empty statement does not verify"),
                |_X, w| psi(_X, w),
                |_X| f(_X),
                stmt
            ), 1);
    }
}

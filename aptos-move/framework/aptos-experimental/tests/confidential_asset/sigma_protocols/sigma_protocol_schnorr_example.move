#[test_only]
/// A Schnorr ZKPoK of $s$ such that $Y = s G$.
///
/// The Schnorr NP relation is:
/// ```
///     R(G, Y; s) =?= 1   <=>   Y =?= s G
/// ```
/// This can be framed as a homomorphism check:
/// ```
///     \psi(s)   =?=    f(G, Y)
/// ```
/// where:
///
///   1. The homomorphism $\psi$ is
///   ```
///     \psi(s) := [ s G ]
///   ```
///   2. The transformation function $f$ is:
///   ```
///     f(G, Y) := [ Y ]
///       ^^^^
///        |
///      stmt.points
///   ```
module aptos_experimental::sigma_protocol_schnorr_example {
    use std::error;
    use aptos_std::ristretto255::{Scalar, CompressedRistretto};

    use aptos_framework::chain_id;
    use aptos_experimental::sigma_protocol_fiat_shamir::{DomainSeparator, new_domain_separator};
    use aptos_experimental::sigma_protocol_witness::{Witness, new_secret_witness};
    use aptos_experimental::sigma_protocol_statement::Statement;
    use aptos_experimental::sigma_protocol_statement_builder::new_builder;
    use aptos_experimental::sigma_protocol_representation::{repr_point, repr_scaled};
    use aptos_experimental::sigma_protocol_representation_vec::{RepresentationVec, new_representation_vec};
    #[test_only]
    use aptos_std::ristretto255::{random_point, random_scalar};
    #[test_only]
    use aptos_experimental::sigma_protocol_homomorphism::evaluate_psi;
    #[test_only]
    use aptos_experimental::sigma_protocol;
    #[test_only]
    use aptos_framework::account;
    #[test_only]
    use aptos_experimental::sigma_protocol_utils::equal_vec_points;

    /// Protocol ID used for domain separation
    const PROTOCOL_ID: vector<u8> = b"My Schnorr test case app";

    /// Index of $G$ in the `PublicStatement::points` vector.
    const IDX_G: u64 = 0;
    /// Index of $Y$ in the `PublicStatement::points` vector.
    const IDX_Y: u64 = 1;

    /// Index of $s$ in the `SecretWitness::w` vector.
    const IDX_s: u64 = 0;

    /// The number of points $n_1$ in a Schnorr public statement is 2: $G$ and $Y$
    const N_1: u64 = 2;
    /// The number of scalars $n_1$ in a Schnorr public statement is 0
    const N_2: u64 = 0;
    /// The number of scalars $k$ in a Schnorr secret witness is 1: $s$
    const K: u64 = 1;
    /// The number of points $v$ in the image of the Schnorr homomorphism and transformation function is 1: $G^s$
    const M: u64 = 1;

    /// Phantom marker type for Schnorr statements.
    struct Schnorr has drop {}

    /// The expected number of points $n_1$ in a Schnorr statement is `N_1 = 2`.
    const E_WRONG_N_1: u64 = 1;
    /// The expected number of scalars $n_2$ in a Schnorr statement is `N_2 = 0`.
    const E_WRONG_N_2: u64 = 2;
    /// The expected number of scalars $k$ in a Schnorr witness is `K = 1`.
    const E_WRONG_K: u64 = 3;
    /// The expected number of points $m$ in the image of the Schnorr homomorphism and transformation function is `M = 1`.
    const E_WRONG_M: u64 = 4;

    fun new_session(session_id: vector<u8>): DomainSeparator {
        new_domain_separator(@aptos_experimental, chain_id::get(), PROTOCOL_ID, session_id)
    }

    /// Creates a new Schnorr statement using the builder.
    fun new_schnorr_statement(
        compressed_G: CompressedRistretto,
        compressed_Y: CompressedRistretto,
    ): Statement<Schnorr> {
        let b = new_builder();
        b.add_point(compressed_G);
        b.add_point(compressed_Y);
        b.build()
    }

    fun new_schnorr_witness(s: Scalar): Witness {
        new_secret_witness(vector[s])
    }

    /// WARNING: See README.md in the `sigma_protocols/` directory for principles on how to implement this correctly!
    fun psi(stmt: &Statement<Schnorr>, w: &Witness): RepresentationVec {
        // WARNING: Crucial for security
        assert!(stmt.get_points().length() == N_1, error::invalid_argument(E_WRONG_N_1));
        // WARNING: Crucial for security
        assert!(stmt.get_scalars().length() == N_2, error::invalid_argument(E_WRONG_N_2));
        // WARNING: Crucial for security
        assert!(w.length() == K, error::invalid_argument(E_WRONG_K));

        // [
        //   s G
        // ]
        let repr_vec = new_representation_vec(vector[
            repr_scaled(IDX_G, *w.get(IDX_s))
        ]);

        // WARNING: Crucial for security
        assert!(repr_vec.length() == M, error::invalid_argument(E_WRONG_M));

        repr_vec
    }

    /// WARNING: See README.md in the `sigma_protocols/` directory for principles on how to implement this correctly!
    fun f(stmt: &Statement<Schnorr>): RepresentationVec {
        // WARNING: Crucial for security
        assert!(stmt.get_points().length() == N_1, error::invalid_argument(E_WRONG_N_1));
        // WARNING: Crucial for security
        assert!(stmt.get_scalars().length() == N_2, error::invalid_argument(E_WRONG_N_2));

        // [
        //   Y
        // ]
        let repr_vec = new_representation_vec(vector[
            repr_point(IDX_Y)
        ]);

        // WARNING: Crucial for security
        assert!(repr_vec.length() == M, error::invalid_argument(E_WRONG_M));

        repr_vec
    }

    #[test_only]
    fun random_statement_witness_pair(): (Statement<Schnorr>, Witness) {
        let s = random_scalar();
        let _G = random_point();
        let compressed_G = _G.point_compress(); // Move linter does not let us use G
        let compressed_Y = _G.point_mul(&s).point_compress(); // Move linter does not let us use Y

        let stmt = new_schnorr_statement(
            compressed_G,
            compressed_Y,
        );
        let witn = new_schnorr_witness(s);

        (stmt, witn)
    }

    #[test]
    /// In an abundance of caution, we double-check our homomorphism $\psi$ is implemented correctly by evaluating it
    /// at a random point and testing the evaluation against one computed by hand manually.
    fun psi_correctness() {
        let (_X, w) = random_statement_witness_pair();

        // Expected evaluation, computed by hand manually
        let _G = _X.get_point(IDX_G); // Move linter does not let us use G
        let s = w.get(IDX_s);
        let expected_psi = vector[ _G.point_mul(s) ];

        // Actual evaluation, computed via our $\psi$ implementation
        let actual_psi = evaluate_psi(|_X, w| psi(_X, w), &_X, &w);

        assert!(equal_vec_points(&actual_psi, &expected_psi), 1);
    }

    #[test]
    fun proof_correctness() {
        chain_id::initialize_for_test(&account::create_signer_for_test(@aptos_framework), 4);
        let (stmt, witn) = random_statement_witness_pair();

        sigma_protocol::assert_correctly_computed_proof_verifies(
            new_session(b"session: test schnorr proving correctness"),
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
        let compressed_G = random_point().point_compress();
        let compressed_Y = random_point().point_compress();
        assert!(
            !sigma_protocol::empty_proof_verifies(
                new_session(b"session: test empty schnorr proof for random statement does not verify"),
                |_X, w| psi(_X, w),
                |_X| f(_X),
                new_schnorr_statement(compressed_G, compressed_Y)
            ), 1);
    }

    #[test]
    #[expected_failure(abort_code=65537, location=aptos_experimental::sigma_protocol_fiat_shamir)]
    fun empty_proof_for_empty_statement_test() {
        chain_id::initialize_for_test(&account::create_signer_for_test(@aptos_framework), 4);
        let b = new_builder();
        let stmt: Statement<Schnorr> = b.build();
        assert!(
            !sigma_protocol::empty_proof_verifies(
                new_session(b"session: test empty schnorr proof for empty statement does not verify"),
                |_X, w| psi(_X, w),
                |_X| f(_X),
                stmt
            ), 1);
    }
}

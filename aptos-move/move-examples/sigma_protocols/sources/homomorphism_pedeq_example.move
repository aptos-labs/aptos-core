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
///     f(G, H, C_1, C_2) := [ C_1, C_2 ]
///       ^^^^^^^^^^^^^^
///        |
///      stmt.points
module sigma_protocols::homomorphism_pedeq_example {
    use aptos_std::ristretto255::{RistrettoPoint, Scalar};
    use sigma_protocols::homomorphism::new_secret_witness;

    use aptos_std::ristretto255::scalar_one;
    use sigma_protocols::public_statement::{PublicStatement, new_public_statement};
    use sigma_protocols::homomorphism::{Self, Proof, SecretWitness};
    use sigma_protocols::representation::new_representation;
    use sigma_protocols::representation_vec::{RepresentationVec, new_representation_vec};
    #[test_only]
    use std::string;
    #[test_only]
    use aptos_std::debug;
    #[test_only]
    use aptos_std::ristretto255::{point_mul, random_point, random_scalar, point_add};
    #[test_only]
    use sigma_protocols::homomorphism::empty_proof;

    /// Application-specific domain-separator
    const DST : vector<u8> = b"My PedEq test case app";
    /// Protocol-specific domain-separator
    const NAME : vector<u8> = b"ZKPoK of the same committed message m";

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

    fun new_pedeq_statement(_G: RistrettoPoint, _H: RistrettoPoint,
                            _C_1: RistrettoPoint, _C_2: RistrettoPoint): PublicStatement {
        new_public_statement(vector[_G, _H, _C_1, _C_2], vector[])
    }

    fun new_pedeq_witness(m: Scalar, r_1: Scalar, r_2: Scalar): SecretWitness {
        new_secret_witness(vector[m, r_1, r_2])
    }

    fun psi(_stmt: &PublicStatement, w: &SecretWitness): RepresentationVec {
        new_representation_vec(vector[
            // m G + r_1 H
            new_representation(vector[IDX_G, IDX_H], vector[*w.get_scalar(IDX_m), *w.get_scalar(IDX_r_1)]),
            // m G + r_2 H
            new_representation(vector[IDX_G, IDX_H], vector[*w.get_scalar(IDX_m), *w.get_scalar(IDX_r_2)]),
        ])
    }

    /// Returns $[C_1, C_2]$
    fun f(_stmt: &PublicStatement): RepresentationVec {
        new_representation_vec(vector[
            new_representation(vector[IDX_C_1, IDX_C_2], vector[scalar_one()])
        ])
    }

    public fun pedeq_verify(stmt: &PublicStatement, proof: &Proof): bool {
        homomorphism::verify_slow(
            DST,
            NAME,
            |_X, w| psi(_X, w),
            |_X| f(_X),
            stmt,
            proof
        )
    }

    #[test]
    /// TODO: I think we can aim to write a general correctness test function that takes DSTs and lambdas for:
    ///  new_statement, new_witness, psi and f
    fun correctness() {
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


        debug::print(&string::utf8(b"new_pedeq_witness"));

        let (proof, randomness) = homomorphism::prove(
            DST, NAME,
            |_X, w| psi(_X, w),
            &stmt,
            &witn
        );

        debug::print(&string::utf8(b"prove"));

        // Make sure the sigma protocol proof verifies
        assert!(pedeq_verify(&stmt, &proof), 2);
    }

    #[test]
    #[expected_failure(abort_code=65540, location=sigma_protocols::homomorphism)]
    /// An empty proof should NOT verify!
    fun empty_proof_should_not_verify() {
        let stmt = new_public_statement(vector[], vector[]);
        let proof = empty_proof();

        assert!(pedeq_verify(&stmt, &proof), 1);
    }
}

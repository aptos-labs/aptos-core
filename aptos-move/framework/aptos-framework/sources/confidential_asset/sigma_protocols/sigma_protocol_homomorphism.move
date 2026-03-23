/// This module can be used to build $\Sigma$-protocols for proving knowledge of a pre-image on a homomorphism $\psi$.
///
/// Let $\mathbb{G}^{n_1} \times \mathbb{F}^{n_2}$ denote the set of public statements.
///
/// This module helps you convince a verifier with $X\in S$ that you know a secret $w\in \mathbb{F}^k$ such that
/// $\psi(w) = f(X)$, where:
///
///    $\psi : \mathbb{F}^k \rightarrow \mathbb{G}^m$ is a *homomorphism*, and
///    $f : \mathbb{G}^{n_1} \times \mathbb{F}^{n_2} \rightarrow \mathbb{G}^m$ is a *transformation function*.
///
/// Many useful statements can be proved in ZK by framing them as proving knowledge of a pre-image on a homomorphism:
///
///    e.g., a Schnorr signature is just proving knowledge of $x$ such that $\psi(x) = x G$, where the PK is $x G$.
///
///    e.g., a proof that $C_1, C_2$ both Pedersen-commit to the same $m$ is proving knowledge of $(m, r_1, r_2)$ s.t.
///          $\psi(m, r_1, r_2) = (m G + r_1 H, m G + r_2 H)$
///
/// The sigma protocol is very simple:
///
/// + ------------------  +                                         + ------------------------------------------------ +
/// | Prover has $(X, w)$ |                                         | Verifier has                                     |
/// + ------------------  +                                         | $X \in \mathbb{G}^{n_1} \times \mathbb{F}^{n_2}$ |
///                                                                 + ------------------------------------------------ +
/// 1. Sample $\alpha \in \mathbb{F}^k
/// 2. Compute *commitment* $A \gets \psi(\alpha)$
///
///                                 3. send commitment $A$
///                            ------------------------------->
///
///                                                                  4. Assert $A \in \mathbb{G}^m$
///                                                                  5. Pick *random challenge* $e$
///                                                                     (via Fiat-Shamir on: $(X, A)$ a protocol
///                                                                      identifier and a session identifier)
///                                  6. send challenge $e$
///                            <-------------------------------
///
/// 7. Compute response $\sigma = \alpha + e \cdot w$
///
///                               8. send response $\sigma$
///                            ------------------------------->
///
///                                                                  9. Check $\psi(\sigma) = A + e f(X)$
///
module aptos_framework::sigma_protocol_homomorphism {
    use aptos_framework::sigma_protocol_witness::Witness;
    use aptos_framework::sigma_protocol_statement::Statement;
    use aptos_framework::sigma_protocol_representation_vec::RepresentationVec;
    #[test_only]
    use aptos_std::ristretto255::{RistrettoPoint, multi_scalar_mul};

    friend aptos_framework::sigma_protocol;
    friend aptos_framework::sigma_protocol_registration;
    friend aptos_framework::sigma_protocol_key_rotation;
    friend aptos_framework::sigma_protocol_withdraw;
    friend aptos_framework::sigma_protocol_transfer;
    #[test_only]
    friend aptos_framework::sigma_protocol_pedeq_example;
    #[test_only]
    friend aptos_framework::sigma_protocol_schnorr_example;

    /// The transformation function  $f : \mathbb{G}^{n_1} \times \mathbb{F}^{n_2} \rightarrow \mathbb{G}^m$
    struct TransformationFunction<phantom P>(|&Statement<P>| RepresentationVec);

    /// The homomorphism $\psi : \mathbb{F}^k \rightarrow \mathbb{G}^m$
    struct Homomorphism<phantom P>(|&Statement<P>, &Witness| RepresentationVec);

    #[test_only]
    /// Computes and returns $\psi(X, w) \in \mathbb{G}^m$ given the public statement $X$ and the secret witness $w$.
    public(friend) inline fun evaluate_psi<P>(psi: Homomorphism<P>,
                                   stmt: &Statement<P>,
                                   witn: &Witness): vector<RistrettoPoint> {
        psi(stmt, witn).map_ref(|repr| multi_scalar_mul(&repr.to_points(stmt), repr.get_scalars()))
    }

    #[test_only]
    /// Returns $f(X) \in \mathbb{G}^m$ given the public statement $X$.
    public(friend) inline fun evaluate_f<P>(f: TransformationFunction<P>,
                                 stmt: &Statement<P>): vector<RistrettoPoint> {
        f(stmt).map_ref(|repr| multi_scalar_mul(&repr.to_points(stmt), repr.get_scalars()))
    }
}

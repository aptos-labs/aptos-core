// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    fiat_shamir,
    sigma_protocol::{
        homomorphism::{
            self,
            fixed_base_msms::{self},
        },
        FirstProofItem, Proof,
    },
    Scalar,
};
use anyhow::ensure;
use aptos_crypto::{
    arkworks::{msm::IsMsmInput, random::sample_field_element},
    utils,
};
use ark_ec::CurveGroup;
use ark_ff::{Field, Fp, FpConfig, PrimeField};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use rand_core::{CryptoRng, RngCore};
use serde::Serialize;
use std::fmt::Debug;

// `CurveGroup` is needed here because the code does `into_affine()` // TODO: not anymore!!!
pub trait Trait<C: CurveGroup>:
    fixed_base_msms::Trait<
        Domain: Witness<C::ScalarField>,
        MsmOutput = C,
        Scalar = C::ScalarField,
        MsmInput: IsMsmInput<Base = C::Affine>, // need to be a bit specific because this code multiplies scalars and does into_affine(), etc // TODO: not anymore!!!
    > + Sized
    + CanonicalSerialize
{
    /// Domain-separation tag (DST) used to ensure that all cryptographic hashes and
    /// transcript operations within the protocol are uniquely namespaced
    fn dst(&self) -> Vec<u8>;

    fn prove<Ct: Serialize, R: RngCore + CryptoRng>(
        &self,
        witness: &Self::Domain,
        statement: Self::Codomain,
        cntxt: &Ct, // for SoK purposes
        rng: &mut R,
    ) -> (Proof<<Self as fixed_base_msms::Trait>::Scalar, Self>, <Self as homomorphism::Trait>::CodomainNormalized) { // or C::ScalarField
        prove_homomorphism(self, witness, statement, cntxt, true, rng, &self.dst())
    }

    #[allow(non_snake_case)]
    fn verify<Ct: Serialize, H, R: RngCore + CryptoRng>(
        &self,
        public_statement: &Self::CodomainNormalized,
        proof: &Proof<C::ScalarField, H>, // Would seem natural to set &Proof<E, Self>, but that ties the lifetime of H to that of Self, but we'd like it to be eg static
        cntxt: &Ct,
        rng: &mut R,
    ) -> anyhow::Result<()>
    where
        H: homomorphism::Trait<Domain = Self::Domain, CodomainNormalized = Self::CodomainNormalized>, // need this because `H` is technically different from `Self` due to lifetime changes
    {
        let msm_terms = self.msm_terms_for_verify::<_, H, _>(
            public_statement,
            proof,
            cntxt,
            rng,
        );

        let msm_result = Self::msm_eval(msm_terms);
        ensure!(msm_result == C::ZERO); // or MsmOutput::zero()

        Ok(())
    }

    #[allow(non_snake_case)]
    fn compute_verifier_challenges<Ct, R: RngCore + CryptoRng>(
        &self,
        public_statement: &Self::CodomainNormalized,
        prover_first_message: &Self::CodomainNormalized, // TODO: this input will have to be modified for `compact` proofs; we just need something serializable, could pass `FirstProofItem<F, H>` instead
        cntxt: &Ct,
        number_of_beta_powers: usize,
        rng: &mut R,
    ) -> (C::ScalarField, Vec<C::ScalarField>)
    where
        Ct: Serialize,
        // H: homomorphism::Trait<Domain = Self::Domain, Codomain = Self::Codomain>, // will probably need this if we use `FirstProofItem<F, H>` instead
    {
        // --- Fiat–Shamir challenge c ---
        let c = fiat_shamir_challenge_for_sigma_protocol::<_, C::ScalarField, _>(
            cntxt,
            self,
            &public_statement,
            prover_first_message,
            &self.dst(),
        );

        // --- Random verifier challenge β ---
        let beta = sample_field_element(rng);
        let powers_of_beta = utils::powers(beta, number_of_beta_powers);

        (c, powers_of_beta)
    }

    // Returns the MSM terms that `verify()` needs
    #[allow(non_snake_case)]
    fn msm_terms_for_verify<Ct: Serialize, H, R: RngCore + CryptoRng>(
        &self,
        public_statement: &Self::CodomainNormalized,
        proof: &Proof<C::ScalarField, H>,
        cntxt: &Ct,
        rng: &mut R,
    ) -> Self::MsmInput
    where
        H: homomorphism::Trait<Domain = Self::Domain, CodomainNormalized = Self::CodomainNormalized>, // Need this because the lifetime was changed
    {
        let prover_first_message = match &proof.first_proof_item {
            FirstProofItem::Commitment(A) => A,
            FirstProofItem::Challenge(_) => {
                panic!("Missing implementation - expected commitment, not challenge")
            },
        };

        let number_of_beta_powers = public_statement.clone().into_iter().count(); // TODO: maybe pass the into_iter version in merge_msm_terms?

        let (c, powers_of_beta) = self.compute_verifier_challenges(public_statement, prover_first_message, cntxt, number_of_beta_powers, rng);

        let msm_terms_for_prover_response = self.msm_terms(&proof.z);

        Self::merge_msm_terms(
            msm_terms_for_prover_response.into_iter().collect(),
            prover_first_message,
            public_statement,
            &powers_of_beta,
            c,
        )
    }

    /// The MSM terms of the sigma protocol. Instead of computing the answer, returning the terms in this form.
    /// This is useful for combining with the MSM terms of other protocols, but note that beta powers are already being
    /// added here because it's convenient (and slightly faster) to do that when the c factor is being added
    #[allow(non_snake_case)]
    fn merge_msm_terms(
        msm_terms: Vec<Self::MsmInput>,
        prover_first_message: &Self::CodomainNormalized,
        statement: &Self::CodomainNormalized,
        powers_of_beta: &[C::ScalarField],
        c: C::ScalarField,
    ) -> Self::MsmInput
    {
        let mut final_basis = Vec::new();
        let mut final_scalars = Vec::new();


    for (((term, A), P), beta_power) in msm_terms
        .into_iter()
        .zip(prover_first_message.clone().into_iter())
        .zip(statement.clone().into_iter())
        .zip(powers_of_beta)
    {
            let mut bases = term.bases().to_vec();
            let mut scalars = term.scalars().to_vec();

            // Multiply scalars by βᶦ
            for scalar in scalars.iter_mut() {
                *scalar *= beta_power;
            }

            // Add prover + statement contributions
            bases.push(A); // this is the element `A` from the prover's first message
            bases.push(P); // this is the element `P` from the statement, but we'll need `P^c`

            scalars.push(- (*beta_power));
            scalars.push(-c * beta_power);

            final_basis.extend(bases);
            final_scalars.extend(scalars);
        }

        Self::MsmInput::new(final_basis, final_scalars).expect("Something went wrong constructing MSM input")
    }
}

pub trait Witness<F: Field>: CanonicalSerialize + CanonicalDeserialize + Clone + Eq {
    /// Computes a scaled addition: `self + c * other`. Can take ownership because the
    /// randomness is discarded by the prover afterwards
    fn scaled_add(self, other: &Self, c: F) -> Self;

    /// Samples a random element in the domain. The prover has a witness `w` and calls `w.rand(rng)` to get
    /// the prover's first nonce (of the same "size" as `w`, hence why this cannot be a static method),
    /// which it then uses to compute the prover's first message in the sigma protocol.
    fn rand<R: RngCore + CryptoRng>(&self, rng: &mut R) -> Self;
}

impl<const N: usize, P: FpConfig<N>> Witness<Fp<P, N>> for Fp<P, N> {
    fn scaled_add(self, other: &Self, c: Fp<P, N>) -> Self {
        self + c * other
    }

    fn rand<R: RngCore + CryptoRng>(&self, rng: &mut R) -> Self {
        sample_field_element(rng)
    }
}

impl<F: PrimeField> Witness<F> for Scalar<F> {
    fn scaled_add(self, other: &Self, c: F) -> Self {
        Scalar(self.0 + (c) * other.0)
    }

    fn rand<R: RngCore + CryptoRng>(&self, rng: &mut R) -> Self {
        Scalar(sample_field_element(rng))
    }
}

impl<F: PrimeField, W: Witness<F>> Witness<F> for Vec<W> {
    fn scaled_add(self, other: &Self, c: F) -> Self {
        self.into_iter()
            .zip(other.iter())
            .map(|(a, b)| a.scaled_add(b, c))
            .collect()
    }

    fn rand<R: RngCore + CryptoRng>(&self, rng: &mut R) -> Self {
        self.iter().map(|elem| elem.rand(rng)).collect()
    }
}

// Standard method to get `trait Statement = Canonical Serialize + ...`, because type aliases are experimental in Rust
pub trait Statement: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq {}
impl<T> Statement for T where T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq {}

/// Computes the Fiat–Shamir challenge for a Σ-protocol instance.
///
/// This function derives a non-interactive challenge scalar by appending
/// protocol-specific data to a Merlin transcript. In the abstraction used here,
/// the protocol proves knowledge of a preimage under a homomorphism. Therefore,
/// all public data relevant to that homomorphism (e.g., its MSM bases) and
/// the image under consideration are included in the transcript.
///
/// # Arguments
/// - `cntxt`: Extra "context" material that needs to be hashed for the challenge.
/// - `hom`: The homomorphism structure carrying its public data (e.g., MSM bases).
/// - `statement`: The public statement, i.e. the image of a witness under the homomorphism.
/// - `prover_first_message`: the first message in the Σ-protocol (the prover's commitment)
/// - `dst`: A domain separation tag to ensure unique challenges per protocol.
///
/// # Returns
/// The derived Fiat–Shamir challenge scalar, after incorporating the domain
/// separator, public data, statement, and prover’s first message into the transcript.
pub fn fiat_shamir_challenge_for_sigma_protocol<
    Ct: Serialize,
    F: PrimeField,
    H: homomorphism::Trait + CanonicalSerialize,
>(
    cntxt: &Ct,
    hom: &H,
    statement: &H::CodomainNormalized,
    prover_first_message: &H::CodomainNormalized,
    dst: &[u8],
) -> F
where
    H::Domain: Witness<F>,
    H::CodomainNormalized: Statement,
{
    // Initialise the transcript
    let mut fs_t = merlin::Transcript::new(dst);

    // Append the "context" to the transcript
    <merlin::Transcript as fiat_shamir::SigmaProtocol<F, H>>::append_sigma_protocol_ctxt(
        &mut fs_t, cntxt,
    );

    // Append the MSM bases to the transcript. (If the same hom is used for many proofs, maybe use a single transcript + a boolean to prevent it from repeating?)
    <merlin::Transcript as fiat_shamir::SigmaProtocol<F, H>>::append_sigma_protocol_msm_bases(
        &mut fs_t, hom,
    );

    // Append the public statement (the image of the witness) to the transcript
    <merlin::Transcript as fiat_shamir::SigmaProtocol<F, H>>::append_sigma_protocol_public_statement(
        &mut fs_t,
        &statement,
    );

    // Add the first prover message (the commitment) to the transcript
    <merlin::Transcript as fiat_shamir::SigmaProtocol<F, H>>::append_sigma_protocol_first_prover_message(
        &mut fs_t,
        prover_first_message,
    );

    // Generate the Fiat-Shamir challenge from the updated transcript
    <merlin::Transcript as fiat_shamir::SigmaProtocol<F, H>>::challenge_for_sigma_protocol(
        &mut fs_t,
    )
}

// We're keeping this separate because it only needs the homomorphism property rather than being a bunch of "fixed-base MSMS",
// and moreover in this way it gets reused in the PairingTupleHomomorphism code which has a custom sigma protocol implementation
#[allow(non_snake_case)]
pub fn prove_homomorphism<Ct: Serialize, F: PrimeField, H: homomorphism::Trait, R>(
    homomorphism: &H,
    witness: &H::Domain,
    statement: H::Codomain,
    cntxt: &Ct,
    store_prover_commitment: bool, // true = store prover's commitment, false = store Fiat-Shamir challenge
    rng: &mut R,
    dst: &[u8],
) -> (Proof<F, H>, H::CodomainNormalized)
where
    H::Domain: Witness<F>,
    H::CodomainNormalized: Statement,
    R: RngCore + CryptoRng,
{
    // Step 1: Sample randomness. Here the `witness` is only used to make sure that `r` has the right dimensions
    let r = witness.rand(rng);

    // Step 2: Compute commitment A = Ψ(r)
    let A_proj = homomorphism.apply(&r);
    let A = homomorphism.normalize(A_proj);
    let normalized_statement = homomorphism.normalize(statement); // TODO: combine these two normalisations

    // Step 3: Obtain Fiat-Shamir challenge
    let c = fiat_shamir_challenge_for_sigma_protocol::<_, F, H>(
        cntxt,
        homomorphism,
        &normalized_statement,
        &A,
        dst,
    );

    // Step 4: Compute prover response
    let z = r.scaled_add(&witness, c);

    // Step 5: Pick first **recorded** item
    let first_proof_item = if store_prover_commitment {
        FirstProofItem::Commitment(A)
    } else {
        FirstProofItem::Challenge(c)
    };

    (
        Proof {
            first_proof_item,
            z,
        },
        normalized_statement,
    )
}

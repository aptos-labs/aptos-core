// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    dlog::bsgs,
    pcs::univariate_hiding_kzg,
    pvss::{
        chunky::{
            chunked_elgamal::{self, num_chunks_per_scalar},
            chunks,
            hkzg_chunked_elgamal::{self, HkzgWeightedElgamalWitness},
            input_secret::InputSecret,
            keys,
            public_parameters::PublicParameters,
        },
        traits::{
            self,
            transcript::{Aggregatable, MalleableTranscript, NonAggregatableTranscript},
            HasEncryptionPublicParams,
        },
        Player,
    },
    range_proofs::{dekart_univariate_v2, traits::BatchedRangeProof},
    sigma_protocol::{
        self,
        homomorphism::{tuple::TupleCodomainShape, Trait as _},
        traits::Trait as _,
    },
    traits::transcript::HasAggregatableSubtranscript,
    Scalar,
};
use anyhow::bail;
use aptos_crypto::{
    arkworks::{
        self,
        random::{
            sample_field_element, sample_field_elements, unsafe_random_point, unsafe_random_points,
            UniformRand,
        },
        scrape::LowDegreeTest,
        serialization::{ark_de, ark_se},
    },
    bls12381::{self},
    utils,
    weighted_config::WeightedConfigArkworks,
    CryptoMaterialError, SecretSharingConfig as _, ValidCryptoMaterial,
};
use ark_ec::{
    pairing::{Pairing, PairingOutput},
    AffineRepr, CurveGroup, VariableBaseMSM,
};
use ark_ff::{AdditiveGroup, Fp, FpConfig};
use ark_poly::EvaluationDomain;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use serde::{Deserialize, Serialize};
use std::ops::{Mul, Sub};

/// Domain-separation tag (DST) used to ensure that all cryptographic hashes and
/// transcript operations within the protocol are uniquely namespaced
pub const DST: &[u8; 39] = b"APTOS_WEIGHTED_CHUNKY_FIELD_PVSS_FS_DST";

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Transcript<E: Pairing> {
    dealer: Player,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    /// This is the aggregatable subtranscript
    pub subtrs: SubTranscript<E>,
    /// Proof (of knowledge) showing that the s_{i,j}'s in C are base-B representations (of the s_i's in V, but this is not part of the proof), and that the r_j's in R are used in C
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub sharing_proof: SharingProof<E>,
}

#[allow(non_snake_case)]
#[derive(
    CanonicalSerialize, CanonicalDeserialize, Serialize, Deserialize, Clone, Debug, PartialEq, Eq,
)]
pub struct SubTranscript<E: Pairing> {
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub Vs: Vec<Vec<E::G2>>,
    /// First chunked ElGamal component: C[i][j] = s_{i,j} * G + r_j * ek_i. Here s_i = \sum_j s_{i,j} * B^j // TODO: change notation because B is not a group element?
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub Cs: Vec<Vec<Vec<E::G1>>>, // TODO: maybe make this and the other fields affine? The verifier will have to do it anyway... and we are trying to speed that up
    /// Second chunked ElGamal component: R[j] = r_j * H
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub Rs: Vec<Vec<E::G1>>,
}

impl<E: Pairing> ValidCryptoMaterial for SubTranscript<E> {
    const AIP_80_PREFIX: &'static str = "";

    fn to_bytes(&self) -> Vec<u8> {
        // TODO: using `Result<Vec<u8>>` and `.map_err(|_| CryptoMaterialError::DeserializationError)` would be more consistent here?
        bcs::to_bytes(&self).expect("Unexpected error during PVSS transcript serialization")
    }
}

impl<E: Pairing> TryFrom<&[u8]> for SubTranscript<E> {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        bcs::from_bytes::<SubTranscript<E>>(bytes)
            .map_err(|_| CryptoMaterialError::DeserializationError)
    }
}

/// This is the secret sharing config that will be used for weighted `chunky`
#[allow(type_alias_bounds)]
type SecretSharingConfig<E: Pairing> = WeightedConfigArkworks<E::ScalarField>;

impl<const N: usize, P: FpConfig<N>, E: Pairing<ScalarField = Fp<P, N>>>
    HasAggregatableSubtranscript<SecretSharingConfig<E>> for Transcript<E>
{
    type SubTranscript = SubTranscript<E>;

    fn get_subtranscript(&self) -> Self::SubTranscript {
        self.subtrs.clone()
    }
}

impl<const N: usize, P: FpConfig<N>, E: Pairing<ScalarField = Fp<P, N>>> traits::SubTranscript
    for SubTranscript<E>
{
    type DealtPubKey = keys::DealtPubKey<E>;
    type DealtPubKeyShare = Vec<keys::DealtPubKeyShare<E>>;
    type DealtSecretKeyShare = Vec<keys::DealtSecretKeyShare<E>>;
    type DecryptPrivKey = keys::DecryptPrivKey<E>;
    type EncryptPubKey = keys::EncryptPubKey<E>;
    type PublicParameters = PublicParameters<E>;
    type SecretSharingConfig = SecretSharingConfig<E>;

    #[allow(non_snake_case)]
    fn get_public_key_share(
        &self,
        _sc: &Self::SecretSharingConfig,
        player: &Player,
    ) -> Self::DealtPubKeyShare {
        self.Vs[player.id]
            .iter()
            .map(|V_i| {
                let affine = V_i.into_affine();

                keys::DealtPubKeyShare::<E>::new(keys::DealtPubKey::new(affine))
            })
            .collect()
    }

    fn get_dealt_public_key(&self) -> Self::DealtPubKey {
        Self::DealtPubKey::new(self.Vs.last().expect("V is empty somehow")[0].into_affine())
    }

    #[allow(non_snake_case)]
    fn decrypt_own_share(
        &self,
        sc: &Self::SecretSharingConfig,
        player: &Player,
        dk: &Self::DecryptPrivKey,
        pp: &Self::PublicParameters,
    ) -> (Self::DealtSecretKeyShare, Self::DealtPubKeyShare) {
        let weight = sc.get_player_weight(player);

        let Cs = &self.Cs[player.id];

        // TODO: put an assert here saying that len(Cs) = weight

        let ephemeral_keys: Vec<_> = self
            .Rs
            .iter()
            .take(weight)
            .map(|R_i_vec| R_i_vec.iter().map(|R_i| R_i.mul(dk.dk)).collect::<Vec<_>>())
            .collect();

        if let Some(first_key) = ephemeral_keys.first() {
            debug_assert_eq!(
                first_key.len(),
                Cs[0].len(),
                "Number of ephemeral keys does not match the number of ciphertext chunks"
            );
        }

        let mut sk_shares: Vec<Scalar<E>> = Vec::with_capacity(weight);
        let pk_shares = self.get_public_key_share(sc, player);

        for i in 0..weight {
            // TODO: should really put this in a separate function
            let dealt_encrypted_secret_key_share_chunks: Vec<_> = Cs[i]
                .iter()
                .zip(ephemeral_keys[i].iter())
                .map(|(C_ij, ephemeral_key)| C_ij.sub(ephemeral_key))
                .collect();

            let dealt_chunked_secret_key_share = bsgs::dlog_vec(
                pp.pp_elgamal.G.into_group(),
                &dealt_encrypted_secret_key_share_chunks,
                &pp.table,
                1 << pp.ell as u32,
            )
            .expect("BSGS dlog failed");

            let dealt_chunked_secret_key_share_fr: Vec<E::ScalarField> =
                dealt_chunked_secret_key_share
                    .iter()
                    .map(|&x| E::ScalarField::from(x))
                    .collect();

            let dealt_secret_key_share =
                chunks::le_chunks_to_scalar(pp.ell, &dealt_chunked_secret_key_share_fr);

            sk_shares.push(Scalar(dealt_secret_key_share));
        }

        (
            sk_shares, pk_shares, // TODO: review this formalism... wh ydo we need this here?
        )
    }
}

impl<E: Pairing> Aggregatable<SecretSharingConfig<E>> for SubTranscript<E> {
    fn aggregate_with(&mut self, sc: &SecretSharingConfig<E>, other: &Self) -> anyhow::Result<()> {
        debug_assert_eq!(self.Cs.len(), sc.get_total_num_players());
        debug_assert_eq!(self.Vs.len(), sc.get_total_num_players() + 1);
        debug_assert_eq!(self.Cs.len(), other.Cs.len());
        debug_assert_eq!(self.Rs.len(), other.Rs.len());
        debug_assert_eq!(self.Vs.len(), other.Vs.len());

        for i in 0..sc.get_total_num_players() {
            for j in 0..self.Vs[i].len() {
                self.Vs[i][j] += other.Vs[i][j];
                for k in 0..self.Cs[i][j].len() {
                    self.Cs[i][j][k] += other.Cs[i][j][k];
                }
            }
        }

        self.Vs[sc.get_total_num_players()][0] += other.Vs[sc.get_total_num_players()][0];

        for i in 0..self.Rs.len() {
            for (r_self, r_other) in self.Rs[i].iter_mut().zip(&other.Rs[i]) {
                *r_self += r_other;
            }
        }

        Ok(())
    }
}

#[allow(non_snake_case)]
#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct SharingProof<E: Pairing> {
    /// SoK: the SK is knowledge of `witnesses` s_{i,j} yielding the commitment and the C and the R, their image is the PK, and the signed message is a certain context `cntxt`
    pub SoK: sigma_protocol::Proof<E, hkzg_chunked_elgamal::WeightedHomomorphism<'static, E>>, // static because we don't want the lifetime of the Proof to depend on the Homomorphism TODO: try removing it?
    /// A batched range proof showing that all committed values s_{i,j} lie in some range
    pub range_proof: dekart_univariate_v2::Proof<E>,
    /// A KZG-style commitment to the values s_{i,j} going into the range proof
    pub range_proof_commitment:
        <dekart_univariate_v2::Proof<E> as BatchedRangeProof<E>>::Commitment,
}

impl<E: Pairing> ValidCryptoMaterial for Transcript<E> {
    const AIP_80_PREFIX: &'static str = "";

    fn to_bytes(&self) -> Vec<u8> {
        // TODO: using `Result<Vec<u8>>` and `.map_err(|_| CryptoMaterialError::DeserializationError)` would be more consistent here?
        bcs::to_bytes(&self).expect("Unexpected error during PVSS transcript serialization")
    }
}

impl<E: Pairing> TryFrom<&[u8]> for Transcript<E> {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        bcs::from_bytes::<Transcript<E>>(bytes)
            .map_err(|_| CryptoMaterialError::DeserializationError)
    }
}

// Temporary hack, will deal with this at some point... a struct would be cleaner
#[allow(type_alias_bounds)]
type SokContext<'a, A: Serialize + Clone> = (
    bls12381::PublicKey,
    &'a A,   // This is for the session id
    usize,   // This is for the player id
    Vec<u8>, // This is for the DST
);

use aptos_crypto::weighted_config::unflatten_by_weights;

impl<const N: usize, P: FpConfig<N>, E: Pairing<ScalarField = Fp<P, N>>> traits::Transcript
    for Transcript<E>
{
    type DealtPubKey = keys::DealtPubKey<E>;
    type DealtPubKeyShare = Vec<keys::DealtPubKeyShare<E>>;
    type DealtSecretKey = keys::DealtSecretKey<E>;
    type DealtSecretKeyShare = Vec<keys::DealtSecretKeyShare<E>>;
    type DecryptPrivKey = keys::DecryptPrivKey<E>;
    type EncryptPubKey = keys::EncryptPubKey<E>;
    type InputSecret = InputSecret<E::ScalarField>;
    type PublicParameters = PublicParameters<E>;
    type SecretSharingConfig = SecretSharingConfig<E>;
    type SigningPubKey = bls12381::PublicKey;
    type SigningSecretKey = bls12381::PrivateKey;

    fn scheme_name() -> String {
        "chunky_pvss".to_string()
    }

    /// Fetches the domain-separation tag (DST)
    fn dst() -> Vec<u8> {
        DST.to_vec()
    }

    #[allow(non_snake_case)]
    fn deal<A: Serialize + Clone, R: rand_core::RngCore + rand_core::CryptoRng>(
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        _ssk: &Self::SigningSecretKey,
        spk: &Self::SigningPubKey,
        eks: &[Self::EncryptPubKey],
        s: &Self::InputSecret,
        session_id: &A,
        dealer: &Player,
        rng: &mut R,
    ) -> Self {
        debug_assert_eq!(
            eks.len(),
            sc.get_total_num_players(),
            "Number of encryption keys must equal total weight"
        );

        // Initialize the PVSS SoK context
        let sok_cntxt = (spk.clone(), session_id, dealer.id, DST.to_vec()); // This is a bit hacky; also get rid of DST here and use self.dst? Would require making `self` input of `deal()`

        // Generate the Shamir secret sharing polynomial
        let mut f = vec![*s.get_secret_a()]; // constant term of polynomial
        f.extend(sample_field_elements::<E::ScalarField, _>(
            sc.get_threshold_weight() - 1,
            rng,
        )); // these are the remaining coefficients; total degree is `t - 1`, so the reconstruction threshold is `t`

        // Generate its `n` evaluations (shares) by doing an FFT over the whole domain, then truncating
        let mut f_evals = sc.get_threshold_config().domain.fft(&f);
        f_evals.truncate(sc.get_total_weight());
        debug_assert_eq!(f_evals.len(), sc.get_total_weight());

        // Encrypt the chunked shares and generate the sharing proof
        let (Cs, Rs, sharing_proof) =
            Self::encrypt_chunked_shares(&f_evals, eks, pp, sc, sok_cntxt, rng);

        // Add constant term for the `\mathbb{G}_2` commitment (we're doing this **after** the previous step
        // because we're now mutating `f_evals` by enlarging it; this is an unimportant technicality however,
        // it has no impact on computational complexity whatsoever as we could simply modify the `commit_to_scalars()`
        // function to take another input)
        f_evals.push(f[0]); // or *s.get_secret_a()

        // Commit to polynomial evaluations + constant term
        let G_2 = pp.get_commitment_base();
        let flattened_Vs = arkworks::commit_to_scalars(&G_2, &f_evals);
        debug_assert_eq!(flattened_Vs.len(), sc.get_total_weight() + 1);

        let mut Vs = unflatten_by_weights(&flattened_Vs, sc);
        Vs.push(vec![*flattened_Vs.last().unwrap()]);

        Transcript {
            dealer: *dealer,
            subtrs: SubTranscript { Vs, Cs, Rs },
            sharing_proof,
        }
    }

    fn get_dealers(&self) -> Vec<Player> {
        vec![self.dealer]
    }

    #[allow(non_snake_case)]
    fn get_public_key_share(
        &self,
        _sc: &Self::SecretSharingConfig,
        player: &Player,
    ) -> Self::DealtPubKeyShare {
        self.subtrs.Vs[player.id]
            .iter()
            .map(|V_i| {
                let affine = V_i.into_affine();

                keys::DealtPubKeyShare::<E>::new(keys::DealtPubKey::new(affine))
            })
            .collect()
    }

    fn get_dealt_public_key(&self) -> Self::DealtPubKey {
        Self::DealtPubKey::new(self.subtrs.Vs.last().expect("V is empty somehow")[0].into_affine())
    }

    #[allow(non_snake_case)]
    fn decrypt_own_share(
        &self,
        sc: &Self::SecretSharingConfig,
        player: &Player,
        dk: &Self::DecryptPrivKey,
        pp: &Self::PublicParameters,
    ) -> (Self::DealtSecretKeyShare, Self::DealtPubKeyShare) {
        let weight = sc.get_player_weight(player);

        let Cs = &self.subtrs.Cs[player.id];

        // TODO: put an assert here saying that len(Cs) = weight

        let ephemeral_keys: Vec<_> = self
            .subtrs
            .Rs
            .iter()
            .take(weight)
            .map(|R_i_vec| R_i_vec.iter().map(|R_i| R_i.mul(dk.dk)).collect::<Vec<_>>())
            .collect();

        if let Some(first_key) = ephemeral_keys.first() {
            debug_assert_eq!(
                first_key.len(),
                Cs[0].len(),
                "Number of ephemeral keys does not match the number of ciphertext chunks"
            );
        }

        let mut sk_shares: Vec<Scalar<E>> = Vec::with_capacity(weight);
        let pk_shares = self.get_public_key_share(sc, player);

        for i in 0..weight {
            // TODO: should really put this in a separate function
            let dealt_encrypted_secret_key_share_chunks: Vec<_> = Cs[i]
                .iter()
                .zip(ephemeral_keys[i].iter())
                .map(|(C_ij, ephemeral_key)| C_ij.sub(ephemeral_key))
                .collect();

            let dealt_chunked_secret_key_share = bsgs::dlog_vec(
                pp.pp_elgamal.G.into_group(),
                &dealt_encrypted_secret_key_share_chunks,
                &pp.table,
                1 << pp.ell as u32,
            )
            .expect("BSGS dlog failed");

            let dealt_chunked_secret_key_share_fr: Vec<E::ScalarField> =
                dealt_chunked_secret_key_share
                    .iter()
                    .map(|&x| E::ScalarField::from(x))
                    .collect();

            let dealt_secret_key_share =
                chunks::le_chunks_to_scalar(pp.ell, &dealt_chunked_secret_key_share_fr);

            sk_shares.push(Scalar(dealt_secret_key_share));
        }

        (
            sk_shares, pk_shares, // TODO: review this formalism... wh ydo we need this here?
        )
    }

    #[allow(non_snake_case)]
    fn generate<R>(sc: &Self::SecretSharingConfig, pp: &Self::PublicParameters, rng: &mut R) -> Self
    where
        R: rand_core::RngCore + rand_core::CryptoRng,
    {
        let num_chunks_per_share = num_chunks_per_scalar::<E::ScalarField>(pp.ell) as usize;

        Transcript {
            dealer: sc.get_player(0),
            subtrs: SubTranscript {
                Vs: unflatten_by_weights(
                    &unsafe_random_points::<E::G2, _>(sc.get_total_weight() + 1, rng),
                    sc,
                ),
                Cs: (0..sc.get_total_num_players())
                    .map(|i| {
                        let w = sc.get_player_weight(&sc.get_player(i)); // TODO: combine these functions...
                        (0..w)
                            .map(|_| unsafe_random_points(num_chunks_per_share, rng))
                            .collect() // todo: use vec![vec![]]... like in the generate functions
                    })
                    .collect(),
                Rs: (0..sc.get_max_weight())
                    .map(|_| unsafe_random_points(num_chunks_per_share, rng))
                    .collect(),
            },
            sharing_proof: SharingProof {
                range_proof_commitment: sigma_protocol::homomorphism::TrivialShape(
                    unsafe_random_point(rng),
                ),
                SoK: hkzg_chunked_elgamal::WeightedProof::generate(sc, num_chunks_per_share, rng),
                range_proof: dekart_univariate_v2::Proof::generate(pp.ell, rng),
            },
        }
    }
}

impl<const N: usize, P: FpConfig<N>, E: Pairing<ScalarField = Fp<P, N>>> NonAggregatableTranscript
    for Transcript<E>
{
    #[allow(non_snake_case)]
    fn verify<A: Serialize + Clone>(
        &self,
        sc: &Self::SecretSharingConfig,
        pp: &Self::PublicParameters,
        spks: &[Self::SigningPubKey],
        eks: &[Self::EncryptPubKey],
        sid: &A,
    ) -> anyhow::Result<()> {
        if eks.len() != sc.get_total_num_players() {
            bail!(
                "Expected {} encryption keys, but got {}",
                sc.get_total_num_players(),
                eks.len()
            );
        }
        if self.subtrs.Cs.len() != sc.get_total_num_players() {
            bail!(
                "Expected {} arrays of chunked ciphertexts, but got {}",
                sc.get_total_num_players(),
                self.subtrs.Cs.len()
            );
        }
        if self.subtrs.Vs.len() != sc.get_total_num_players() + 1 {
            bail!(
                "Expected {} commitment elements, but got {}",
                sc.get_total_num_players() + 1,
                self.subtrs.Vs.len()
            );
        }

        // Initialize the **identical** PVSS SoK context
        let sok_cntxt = (
            &spks[self.dealer.id],
            sid.clone(),
            self.dealer.id,
            DST.to_vec(),
        ); // As above, this is a bit hacky... though we have access to `self` now

        {
            // Verify the PoK
            let eks_inner: Vec<_> = eks.iter().map(|ek| ek.ek).collect();
            let hom = hkzg_chunked_elgamal::WeightedHomomorphism::new(
                &pp.pk_range_proof.ck_S.lagr_g1,
                pp.pk_range_proof.ck_S.xi_1,
                &pp.pp_elgamal,
                &eks_inner,
            );
            if let Err(err) = hom.verify(
                &TupleCodomainShape(
                    self.sharing_proof.range_proof_commitment.clone(),
                    chunked_elgamal::WeightedCodomainShape {
                        chunks: self.subtrs.Cs.clone(),
                        randomness: self.subtrs.Rs.clone(),
                    },
                ),
                &self.sharing_proof.SoK,
                &sok_cntxt,
            ) {
                bail!("PoK verification failed: {:?}", err);
            }

            // Verify the range proof
            if let Err(err) = self.sharing_proof.range_proof.verify(
                &pp.pk_range_proof.vk,
                sc.get_total_weight() * num_chunks_per_scalar::<E::ScalarField>(pp.ell) as usize,
                pp.ell as usize,
                &self.sharing_proof.range_proof_commitment,
            ) {
                bail!("Range proof batch verification failed: {:?}", err);
            }
        }

        let mut rng = rand::thread_rng(); // TODO: make `rng` a parameter of fn verify()?

        // Do the SCRAPE LDT
        let ldt = LowDegreeTest::random(
            &mut rng,
            sc.get_threshold_weight(),
            sc.get_total_weight() + 1,
            true,
            &sc.get_threshold_config().domain,
        ); // includes_zero is true here means it includes a commitment to f(0), which is in V[n]
        let Vs_flat: Vec<_> = self.subtrs.Vs.iter().flatten().cloned().collect();
        // could add an assert_eq here with sc.get_total_weight()
        ldt.low_degree_test_group(&Vs_flat)?;

        // Now compute the final MSM // TODO: merge this multi_exp with the PoK verification, as in YOLO YOSO? // TODO2: and use the iterate stuff you developed? it's being forgotten here
        let mut base_vec = Vec::new();
        let mut exp_vec = Vec::new();

        let beta = sample_field_element(&mut rng);
        let powers_of_beta = utils::powers(beta, sc.get_total_weight() + 1);

        let Cs_flat: Vec<_> = self.subtrs.Cs.iter().flatten().cloned().collect();
        assert_eq!(
            Cs_flat.len(),
            sc.get_total_weight(),
            "Number of ciphertexts does not equal number of weights"
        ); // TODO what if zero weight?
           // could add an assert_eq here with sc.get_total_weight()

        for i in 0..Cs_flat.len() {
            for j in 0..Cs_flat[i].len() {
                let base = Cs_flat[i][j];
                let exp = pp.powers_of_radix[j] * powers_of_beta[i];
                base_vec.push(base);
                exp_vec.push(exp);
            }
        }

        let weighted_Cs = E::G1::msm(&E::G1::normalize_batch(&base_vec), &exp_vec)
            .expect("Failed to compute MSM of Cs in chunky");

        let weighted_Vs = E::G2::msm(
            &E::G2::normalize_batch(&Vs_flat[..sc.get_total_weight()]),
            &powers_of_beta[..sc.get_total_weight()],
        )
        .expect("Failed to compute MSM of Vs in chunky");

        let res = E::multi_pairing(
            [
                weighted_Cs.into_affine(),
                *pp.get_encryption_public_params().message_base(),
            ],
            [pp.get_commitment_base(), (-weighted_Vs).into_affine()],
        ); // Making things affine here rather than converting the two bases to group elements, since that's probably what they would be converted to anyway: https://github.com/arkworks-rs/algebra/blob/c1f4f5665504154a9de2345f464b0b3da72c28ec/ec/src/models/bls12/g1.rs#L14

        if PairingOutput::<E>::ZERO != res {
            return Err(anyhow::anyhow!("Expected zero during multi-pairing check"));
        }

        Ok(())
    }
}

impl<const N: usize, P: FpConfig<N>, E: Pairing<ScalarField = Fp<P, N>>> Transcript<E> {
    // why are N and P needed? TODO: maybe integrate into deal()
    #[allow(non_snake_case)]
    pub fn encrypt_chunked_shares<
        'a,
        A: Serialize + Clone,
        R: rand_core::RngCore + rand_core::CryptoRng,
    >(
        f_evals: &[E::ScalarField],
        eks: &[keys::EncryptPubKey<E>],
        pp: &PublicParameters<E>,
        sc: &<Self as traits::Transcript>::SecretSharingConfig, // only for debugging purposes?
        sok_cntxt: SokContext<'a, A>,
        rng: &mut R,
    ) -> (Vec<Vec<Vec<E::G1>>>, Vec<Vec<E::G1>>, SharingProof<E>) {
        // Generate the required randomness
        let hkzg_randomness = univariate_hiding_kzg::CommitmentRandomness::rand(rng);
        let elgamal_randomness = Scalar::vecvec_from_inner(
            (0..sc.get_max_weight())
                .map(|_| {
                    chunked_elgamal::correlated_randomness(
                        rng,
                        1 << pp.ell as u64,
                        num_chunks_per_scalar::<E::ScalarField>(pp.ell),
                    )
                })
                .collect(),
        );

        // Chunk and flatten the shares
        let f_evals_chunked: Vec<Vec<E::ScalarField>> = f_evals
            .iter()
            .map(|f_eval| chunks::scalar_to_le_chunks(pp.ell, f_eval))
            .collect();
        // Flatten it now (for use in the range proof) before `f_evals_chunked` is consumed in the next step
        let f_evals_chunked_flat: Vec<E::ScalarField> =
            f_evals_chunked.iter().flatten().copied().collect();
        // Separately, gather the chunks by weight
        let f_evals_weighted = unflatten_by_weights(&f_evals_chunked, sc);

        // Now generate the encrypted shares and range proof commitment, together with its SoK, so:
        // (1) Set up the witness
        let witness = HkzgWeightedElgamalWitness {
            hkzg_randomness,
            chunked_plaintexts: Scalar::vecvecvec_from_inner(f_evals_weighted),
            elgamal_randomness,
        };
        // (2) Compute its image under the corresponding homomorphism, and produce an SoK
        //   (2a) Set up the tuple homomorphism
        let eks_inner: Vec<_> = eks.iter().map(|ek| ek.ek).collect(); // TODO: this is a bit ugly
        let hom = hkzg_chunked_elgamal::WeightedHomomorphism::new(
            &pp.pk_range_proof.ck_S.lagr_g1,
            pp.pk_range_proof.ck_S.xi_1,
            &pp.pp_elgamal,
            &eks_inner,
        );
        //   (2b) Compute its image (the public statement), so the range proof commitment and chunked_elgamal encryptions
        let statement = hom.apply(&witness);
        //   (2c) Produce the SoK
        let SoK = hom
            .prove(&witness, &statement, &sok_cntxt, rng)
            .change_lifetime(); // Make sure the lifetime of the proof is not coupled to `hom` which has references

        // Destructure the "public statement" of the above sigma protocol
        let TupleCodomainShape(
            range_proof_commitment,
            chunked_elgamal::WeightedCodomainShape {
                chunks: Cs,
                randomness: Rs,
            },
        ) = statement;

        // debug_assert_eq!(
        //     Cs.len(),
        //     sc.get_total_weight(),
        //     "Number of encrypted chunks must equal number of players"
        // );

        // Generate the batch range proof, given the `range_proof_commitment` produced in the PoK
        let range_proof = dekart_univariate_v2::Proof::prove(
            &pp.pk_range_proof,
            &f_evals_chunked_flat,
            pp.ell as usize,
            &range_proof_commitment,
            &hkzg_randomness,
            rng,
        );

        // Assemble the sharing proof
        let sharing_proof = SharingProof {
            SoK,
            range_proof,
            range_proof_commitment,
        };

        (Cs, Rs, sharing_proof)
    }
}

impl<const N: usize, P: FpConfig<N>, E: Pairing<ScalarField = Fp<P, N>>> MalleableTranscript
    for Transcript<E>
{
    fn maul_signature<A: Serialize + Clone>(
        &mut self,
        _ssk: &Self::SigningSecretKey,
        _aux: &A,
        _player: &Player,
    ) {
        // TODO: We're not using this but it probably fails if we don't; but that would probably mean recomputing almost the entire transcript... but then that would require eks and pp
        panic!("Doesn't work for this PVSS, at least for now");
        // self.dealer = *player;

        // let sgn = ssk
        //     .sign(&self.utrs)
        //     .expect("signing of `chunky` PVSS transcript failed");
        // self.sgn = sgn;
    }
}

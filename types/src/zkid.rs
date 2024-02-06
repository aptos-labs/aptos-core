// Copyright © Aptos Foundation

use crate::{
    bn254_circom::{G1Bytes, G2Bytes},
    jwks::rsa::RSA_JWK,
    on_chain_config::CurrentTimeMicroseconds,
    transaction::{
        authenticator::{
            AnyPublicKey, AnySignature, EphemeralPublicKey, EphemeralSignature, MAX_NUM_OF_SIGS,
        },
        SignedTransaction,
    },
};
use anyhow::{bail, ensure, Context, Result};
#[cfg(test)]
use aptos_crypto::ed25519::Ed25519Signature;
use aptos_crypto::{poseidon_bn254, CryptoMaterialError, ValidCryptoMaterial};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use ark_bn254::{self, Bn254, Fr};
use ark_groth16::{Groth16, PreparedVerifyingKey, Proof};
use ark_serialize::CanonicalSerialize;
use base64::URL_SAFE_NO_PAD;
use move_core_types::{ident_str, identifier::IdentStr, move_resource::MoveStructType};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::skip_serializing_none;
use std::{
    collections::BTreeMap,
    str,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// The size of the pepper used to create a _hiding_ identity commitment (IDC) when deriving a zkID
/// address. This value should **NOT* be changed since on-chain addresses are based on it (e.g.,
/// hashing with a larger pepper would lead to a different address).
pub const PEPPER_NUM_BYTES: usize = poseidon_bn254::BYTES_PACKED_PER_SCALAR;

/// The size of the identity commitment (IDC) used to derive a zkID address. This value should **NOT*
/// be changed since on-chain addresses are based on it (e.g., hashing a larger-sized IDC would lead
/// to a different address).
pub const IDC_NUM_BYTES: usize = 32;

// TODO(zkid): add some static asserts here that these don't exceed the MAX poseidon input sizes

// TODO(zkid): The constants here must be moved to Move for uggradeability.
// Everything is a multiple of `poseidon_bn254::BYTES_PACKED_PER_SCALAR`] to maximize the input
// sizes that can be hashed.

/// The size of the "nonce commitment (to the EPK and expiration date)" stored in the JWT's `nonce`
/// field. The commitment is using the Poseidon-BN254 hash function, hence the 254-bit (32 byte) size.
pub const NONCE_COMMITMENT_NUM_BYTES: usize = 32;

/// The max length of an ephemeral public key supported in our circuit (93 bytes)
pub const MAX_COMMITTED_EPK_BYTES: usize = 3 * poseidon_bn254::BYTES_PACKED_PER_SCALAR;

/// The max length of the value of the JWT's `iss` field supported in our circuit
/// TODO(zkid): used to be 5 * poseidon_bn254::BYTES_PACKED_PER_SCALAR
pub const MAX_ISS_BYTES: usize = 115;

/// The max length of the value of the JWT's `aud` field supported in our circuit
/// TODO(zkid): used to be 4 * poseidon_bn254::BYTES_PACKED_PER_SCALAR
pub const MAX_AUD_VAL_BYTES: usize = 115;

/// The max length of the JWT field name that stores the user's ID (e.g., `sub`, `email`) which is
/// supported in our circuit
/// TODO(zkid): used to be 2 * poseidon_bn254::BYTES_PACKED_PER_SCALAR
pub const MAX_UID_KEY_BYTES: usize = 30;

/// The max length of the value of the JWT's UID field (`sub`, `email`) that stores the user's ID
/// which is supported in our circuit
/// TODO(zkid): used to be 4 * poseidon_bn254::BYTES_PACKED_PER_SCALAR
pub const MAX_UID_VAL_BYTES: usize = 330;

/// The max length of the JWT field name and value (e.g., `"max_age":"18"`) supported in our circuit
/// TODO(zkid): used to be 5 * poseidon_bn254::BYTES_PACKED_PER_SCALAR;
pub const MAX_EXTRA_FIELD_BYTES: usize = 350;

/// The max length of the base64url-encoded JWT payload in bytes supported in our circuit
pub const MAX_JWT_PAYLOAD_B64_BYTES: usize = 192 * 8 - 64;

/// The max length of the base64url-encoded JWT header in bytes supported in our circuit
/// TODO(zkid) used to be 8 * poseidon_bn254::BYTES_PACKED_PER_SCALAR
pub const MAX_JWT_HEADER_B64_BYTES: usize = 300;

/// Reflection of aptos_framework::zkid::Configs
#[derive(Serialize, Deserialize, Debug)]
pub struct Configuration {
    pub max_zkid_signatures_per_txn: u16,
    pub max_exp_horizon: u64,
    pub training_wheels_pubkey: Option<Vec<u8>>,
}

impl MoveStructType for Configuration {
    const MODULE_NAME: &'static IdentStr = ident_str!("zkid");
    const STRUCT_NAME: &'static IdentStr = ident_str!("Configuration");
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct JwkId {
    /// The OIDC provider associated with this JWK
    pub iss: String,
    /// The Key ID associated with this JWK (https://datatracker.ietf.org/doc/html/rfc7517#section-4.5)
    pub kid: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash, Serialize)]
pub struct OpenIdSig {
    /// The base64url encoded JWS signature of the OIDC JWT (https://datatracker.ietf.org/doc/html/rfc7515#section-3)
    pub jwt_sig: String,
    /// The base64url encoded JSON payload of the OIDC JWT (https://datatracker.ietf.org/doc/html/rfc7519#section-3)
    pub jwt_payload: String,
    /// The name of the key in the claim that maps to the user identifier; e.g., "sub" or "email"
    pub uid_key: String,
    /// The random value used to obfuscate the EPK from OIDC providers in the nonce field
    pub epk_blinder: Vec<u8>,
    /// The privacy-preserving value used to calculate the identity commitment. It is typically uniquely derived from `(iss, client_id, uid_key, uid_val)`.
    pub pepper: Pepper,
}

impl OpenIdSig {
    /// The size of the blinding factor used to compute the nonce commitment to the EPK and expiration
    /// date. This can be upgraded, if the OAuth nonce reconstruction is upgraded carefully.
    pub const EPK_BLINDER_NUM_BYTES: usize = poseidon_bn254::BYTES_PACKED_PER_SCALAR;

    /// Verifies an `OpenIdSig` by doing the following checks:
    ///  1. Check that the ephemeral public key lifespan is under MAX_EXPIRY_HORIZON_SECS
    ///  2. Check that the iss claim in the ZkIdPublicKey matches the one in the jwt_payload
    ///  3. Check that the identity commitment in the ZkIdPublicKey matches the one constructed from the jwt_payload
    ///  4. Check that the nonce constructed from the ephemeral public key, blinder, and exp_timestamp_secs matches the one in the jwt_payload
    pub fn verify_jwt_claims(
        &self,
        exp_timestamp_secs: u64,
        epk: &EphemeralPublicKey,
        pk: &ZkIdPublicKey,
        max_exp_horizon_secs: u64,
    ) -> Result<()> {
        let jwt_payload_json = base64url_decode_as_str(&self.jwt_payload)?;
        let claims: Claims = serde_json::from_str(&jwt_payload_json)?;

        let max_expiration_date = seconds_from_epoch(claims.oidc_claims.iat + max_exp_horizon_secs);
        let expiration_date: SystemTime = seconds_from_epoch(exp_timestamp_secs);
        ensure!(
            expiration_date < max_expiration_date,
            "The ephemeral public key's expiration date is too far into the future"
        );

        ensure!(
            claims.oidc_claims.iss.eq(&pk.iss),
            "'iss' claim was supposed to match \"{}\"",
            pk.iss
        );

        ensure!(
            self.uid_key.eq("sub") || self.uid_key.eq("email"),
            "uid_key must be either 'sub' or 'email', was \"{}\"",
            self.uid_key
        );
        let uid_val = claims.get_uid_val(&self.uid_key)?;

        ensure!(
            IdCommitment::new_from_preimage(
                &self.pepper,
                &claims.oidc_claims.aud,
                &self.uid_key,
                &uid_val
            )?
            .eq(&pk.idc),
            "Address IDC verification failed"
        );

        ensure!(
            self.reconstruct_oauth_nonce(exp_timestamp_secs, epk)?
                .eq(&claims.oidc_claims.nonce),
            "'nonce' claim did not contain the expected EPK and expiration date commitment"
        );

        Ok(())
    }

    pub fn verify_jwt_signature(&self, rsa_jwk: RSA_JWK, jwt_header: &String) -> Result<()> {
        let jwt_payload = &self.jwt_payload;
        let jwt_sig = &self.jwt_sig;
        let jwt_token = format!("{}.{}.{}", jwt_header, jwt_payload, jwt_sig);
        rsa_jwk.verify_signature(&jwt_token)?;
        Ok(())
    }

    pub fn reconstruct_oauth_nonce(
        &self,
        exp_timestamp_secs: u64,
        epk: &EphemeralPublicKey,
    ) -> Result<String> {
        let mut frs = poseidon_bn254::pad_and_pack_bytes_to_scalars_with_len(
            epk.to_bytes().as_slice(),
            MAX_COMMITTED_EPK_BYTES,
        )?;

        frs.push(Fr::from(exp_timestamp_secs));
        frs.push(poseidon_bn254::pack_bytes_to_one_scalar(
            &self.epk_blinder[..],
        )?);

        let nonce_fr = poseidon_bn254::hash_scalars(frs)?;
        let mut nonce_bytes = [0u8; NONCE_COMMITMENT_NUM_BYTES];
        nonce_fr.serialize_uncompressed(&mut nonce_bytes[..])?;

        Ok(base64url_encode(&nonce_bytes[..]))
    }
}

impl TryFrom<&[u8]> for OpenIdSig {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, CryptoMaterialError> {
        bcs::from_bytes::<OpenIdSig>(bytes).map_err(|_e| CryptoMaterialError::DeserializationError)
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct OidcClaims {
    iss: String,
    aud: String,
    sub: String,
    nonce: String,
    iat: u64,
    email: Option<String>,
    email_verified: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    #[serde(flatten)]
    oidc_claims: OidcClaims,
    #[serde(default)]
    additional_claims: BTreeMap<String, Value>,
}

impl Claims {
    fn get_uid_val(&self, uid_key: &String) -> Result<String> {
        match uid_key.as_str() {
            "email" => {
                let email_verified = self
                    .oidc_claims
                    .email_verified
                    .clone()
                    .context("'email_verified' claim is missing")?;
                // the 'email_verified' claim may be a boolean or a boolean-as-a-string.
                let email_verified_as_bool = email_verified.as_bool().unwrap_or(false);
                let email_verified_as_str = email_verified.as_str().unwrap_or("false");
                ensure!(
                    email_verified_as_bool || email_verified_as_str.eq("true"),
                    "'email_verified' claim was not \"true\""
                );
                self.oidc_claims
                    .email
                    .clone()
                    .context("email claim missing on jwt")
            },
            "sub" => Ok(self.oidc_claims.sub.clone()),
            _ => {
                let uid_val = self
                    .additional_claims
                    .get(uid_key)
                    .context(format!("{} claim missing on jwt", uid_key))?
                    .as_str()
                    .context(format!("{} value is not a string", uid_key))?;
                Ok(uid_val.to_string())
            },
        }
    }
}

#[derive(
    Clone, Debug, Deserialize, PartialEq, Eq, Hash, Serialize, CryptoHasher, BCSCryptoHash,
)]
pub struct Groth16Zkp {
    a: G1Bytes,
    b: G2Bytes,
    c: G1Bytes,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash, Serialize)]
pub struct SignedGroth16Zkp {
    pub proof: Groth16Zkp,
    /// The signature of the proof signed by the private key of the `ephemeral_pubkey`.
    pub non_malleability_signature: EphemeralSignature,
    pub training_wheels_signature: EphemeralSignature,
    pub extra_field: String,
    pub override_aud_val: Option<String>,
}

impl SignedGroth16Zkp {
    /// For testing
    #[cfg(test)]
    fn dummy() -> SignedGroth16Zkp {
        SignedGroth16Zkp {
            proof: Groth16Zkp {
                a: G1Bytes::new_from_vec(vec![0u8; G1_PROJECTIVE_COMPRESSED_NUM_BYTES]).unwrap(),
                b: G2Bytes::new_from_vec(vec![0u8; G2_PROJECTIVE_COMPRESSED_NUM_BYTES]).unwrap(),
                c: G1Bytes::new_from_vec(vec![0u8; G1_PROJECTIVE_COMPRESSED_NUM_BYTES]).unwrap(),
            },
            non_malleability_signature: EphemeralSignature::ed25519(
                Ed25519Signature::dummy_signature(),
            ),
            training_wheels_signature: EphemeralSignature::ed25519(
                Ed25519Signature::dummy_signature(),
            ),
            extra_field: "a".repeat(MAX_EXTRA_FIELD_BYTES),
            override_aud_val: Some("b".repeat(MAX_AUD_VAL_BYTES)),
        }
    }

    pub fn verify_non_malleability_sig(&self, pub_key: &EphemeralPublicKey) -> Result<()> {
        self.non_malleability_signature.verify(&self.proof, pub_key)
    }

    pub fn verify_training_wheels_sig(&self, pub_key: &EphemeralPublicKey) -> Result<()> {
        self.training_wheels_signature.verify(&self.proof, pub_key)
    }

    pub fn verify_proof(
        &self,
        public_inputs_hash: Fr,
        pvk: &PreparedVerifyingKey<Bn254>,
    ) -> Result<()> {
        self.proof.verify_proof(public_inputs_hash, pvk)
    }
}

impl TryFrom<&[u8]> for Groth16Zkp {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, CryptoMaterialError> {
        bcs::from_bytes::<Groth16Zkp>(bytes).map_err(|_e| CryptoMaterialError::DeserializationError)
    }
}

impl Groth16Zkp {
    pub fn new(a: G1Bytes, b: G2Bytes, c: G1Bytes) -> Self {
        Groth16Zkp { a, b, c }
    }

    pub fn verify_proof(
        &self,
        public_inputs_hash: Fr,
        pvk: &PreparedVerifyingKey<Bn254>,
    ) -> Result<()> {
        let proof: Proof<Bn254> = Proof {
            a: self.a.deserialize_into_affine()?,
            b: self.b.to_affine()?,
            c: self.c.deserialize_into_affine()?,
        };
        let result = Groth16::<Bn254>::verify_proof(pvk, &proof, &[public_inputs_hash])?;
        if !result {
            bail!("groth16 proof verification failed")
        }
        Ok(())
    }
}

/// Allows us to support direct verification of OpenID signatures, in the rare case that we would
/// need to turn off ZK proofs due to a bug in the circuit.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash, Serialize)]
pub enum ZkpOrOpenIdSig {
    Groth16Zkp(SignedGroth16Zkp),
    OpenIdSig(OpenIdSig),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash, Serialize)]
pub struct ZkIdSignature {
    /// A \[ZKPoK of an\] OpenID signature over several relevant fields (e.g., `aud`, `sub`, `iss`,
    /// `nonce`) where `nonce` contains a commitment to `ephemeral_pubkey` and an expiration time
    /// `exp_timestamp_secs`.
    pub sig: ZkpOrOpenIdSig,

    /// The base64url-encoded header, which contains two relevant fields:
    ///  1. `kid`, which indicates which of the OIDC provider's JWKs should be used to verify the
    ///     \[ZKPoK of an\] OpenID signature.,
    ///  2. `alg`, which indicates which type of signature scheme was used to sign the JWT
    pub jwt_header: String,

    /// The expiry time of the `ephemeral_pubkey` represented as a UNIX epoch timestamp in seconds.
    pub exp_timestamp_secs: u64,

    /// A short lived public key used to verify the `ephemeral_signature`.
    pub ephemeral_pubkey: EphemeralPublicKey,
    /// The signature of the transaction signed by the private key of the `ephemeral_pubkey`.
    pub ephemeral_signature: EphemeralSignature,
}

impl TryFrom<&[u8]> for ZkIdSignature {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, CryptoMaterialError> {
        bcs::from_bytes::<ZkIdSignature>(bytes)
            .map_err(|_e| CryptoMaterialError::DeserializationError)
    }
}

impl ValidCryptoMaterial for ZkIdSignature {
    fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&self).expect("Only unhandleable errors happen here.")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JWTHeader {
    pub kid: String,
    pub alg: String,
}

impl ZkIdSignature {
    pub fn parse_jwt_header(&self) -> Result<JWTHeader> {
        let jwt_header_json = base64url_decode_as_str(&self.jwt_header)?;
        let header: JWTHeader = serde_json::from_str(&jwt_header_json)?;
        Ok(header)
    }

    pub fn verify_expiry(&self, current_time: &CurrentTimeMicroseconds) -> Result<()> {
        let block_time = UNIX_EPOCH + Duration::from_micros(current_time.microseconds);
        let expiry_time = seconds_from_epoch(self.exp_timestamp_secs);

        if block_time > expiry_time {
            bail!("zkID Signature is expired");
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Pepper(pub(crate) [u8; PEPPER_NUM_BYTES]);

impl Pepper {
    pub fn new(bytes: [u8; PEPPER_NUM_BYTES]) -> Self {
        Self(bytes)
    }

    pub fn to_bytes(&self) -> &[u8; PEPPER_NUM_BYTES] {
        &self.0
    }

    // Used for testing. #[cfg(test)] doesn't seem to allow for use in smoke tests.
    pub fn from_number(num: u128) -> Self {
        let big_int = num_bigint::BigUint::from(num);
        let bytes: Vec<u8> = big_int.to_bytes_le();
        let mut extended_bytes = [0u8; PEPPER_NUM_BYTES];
        extended_bytes[..bytes.len()].copy_from_slice(&bytes);
        Self(extended_bytes)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct IdCommitment(pub(crate) [u8; IDC_NUM_BYTES]);

impl IdCommitment {
    pub fn new_from_preimage(
        pepper: &Pepper,
        aud: &str,
        uid_key: &str,
        uid_val: &str,
    ) -> Result<Self> {
        let aud_val_hash = poseidon_bn254::pad_and_hash_string(aud, MAX_AUD_VAL_BYTES)?;
        let uid_key_hash = poseidon_bn254::pad_and_hash_string(uid_key, MAX_UID_KEY_BYTES)?;
        let uid_val_hash = poseidon_bn254::pad_and_hash_string(uid_val, MAX_UID_VAL_BYTES)?;
        let pepper_scalar = poseidon_bn254::pack_bytes_to_one_scalar(pepper.0.as_slice())?;

        let fr = poseidon_bn254::hash_scalars(vec![
            pepper_scalar,
            aud_val_hash,
            uid_val_hash,
            uid_key_hash,
        ])?;

        let mut idc_bytes = [0u8; IDC_NUM_BYTES];
        fr.serialize_uncompressed(&mut idc_bytes[..])?;
        Ok(IdCommitment(idc_bytes))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&self).expect("Only unhandleable errors happen here.")
    }
}

impl TryFrom<&[u8]> for IdCommitment {
    type Error = CryptoMaterialError;

    fn try_from(_value: &[u8]) -> Result<Self, Self::Error> {
        bcs::from_bytes::<IdCommitment>(_value)
            .map_err(|_e| CryptoMaterialError::DeserializationError)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ZkIdPublicKey {
    /// The OIDC provider.
    pub iss: String,

    /// SNARK-friendly commitment to:
    /// 1. The application's ID; i.e., the `aud` field in the signed OIDC JWT representing the OAuth client ID.
    /// 2. The OIDC provider's internal identifier for the user; e.g., the `sub` field in the signed OIDC JWT
    ///    which is Google's internal user identifier for bob@gmail.com, or the `email` field.
    ///
    /// e.g., H(aud || uid_key || uid_val || pepper), where `pepper` is the commitment's randomness used to hide
    ///  `aud` and `sub`.
    pub idc: IdCommitment,
}

impl ZkIdPublicKey {
    pub fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&self).expect("Only unhandleable errors happen here.")
    }
}

impl TryFrom<&[u8]> for ZkIdPublicKey {
    type Error = CryptoMaterialError;

    fn try_from(_value: &[u8]) -> Result<Self, Self::Error> {
        bcs::from_bytes::<ZkIdPublicKey>(_value)
            .map_err(|_e| CryptoMaterialError::DeserializationError)
    }
}

pub fn get_zkid_authenticators(
    transaction: &SignedTransaction,
) -> Result<Vec<(ZkIdPublicKey, ZkIdSignature)>> {
    // Check all the signers in the TXN
    let single_key_authenticators = transaction
        .authenticator_ref()
        .to_single_key_authenticators()?;
    let mut authenticators = Vec::with_capacity(MAX_NUM_OF_SIGS);
    for authenticator in single_key_authenticators {
        if let (AnyPublicKey::ZkId { public_key }, AnySignature::ZkId { signature }) =
            (authenticator.public_key(), authenticator.signature())
        {
            authenticators.push((public_key.clone(), signature.clone()))
        }
    }
    Ok(authenticators)
}

pub fn base64url_encode(data: &[u8]) -> String {
    base64::encode_config(data, URL_SAFE_NO_PAD)
}

pub fn base64url_encode_str(data: &str) -> String {
    base64::encode_config(data.as_bytes(), URL_SAFE_NO_PAD)
}

pub fn base64url_decode_as_str(b64: &str) -> Result<String> {
    let decoded_bytes = base64::decode_config(b64, URL_SAFE_NO_PAD)?;
    // Convert the decoded bytes to a UTF-8 string
    let str = String::from_utf8(decoded_bytes)?;
    Ok(str)
}

fn seconds_from_epoch(secs: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(secs)
}

#[cfg(test)]
mod test {
    use crate::{
        bn254_circom::{get_public_inputs_hash, DEVNET_VERIFYING_KEY},
        jwks::rsa::RSA_JWK,
        transaction::authenticator::{AuthenticationKey, EphemeralPublicKey, EphemeralSignature},
        zkid::{
            base64url_encode_str, G1Bytes, G2Bytes, Groth16Zkp, IdCommitment, OpenIdSig, Pepper,
            SignedGroth16Zkp, ZkIdPublicKey, ZkIdSignature, ZkpOrOpenIdSig, EPK_BLINDER_NUM_BYTES,
        },
    };
    use aptos_crypto::{
        ed25519::{Ed25519PrivateKey, Ed25519Signature},
        PrivateKey, SigningKey, Uniform,
    };
    use std::ops::Deref;

    // TODO(zkid): This test case must be rewritten to be more modular and updatable.
    //  Right now, there are no instructions on how to produce this test case.
    #[test]
    fn test_groth16_proof_verification() {
        let a = G1Bytes::new_unchecked(
            "19843734071102143602441202443608981862760142725808945198375332557568733182487",
            "7490772921219489322991985736547330118240504032652964776703563444800470517507",
        )
        .unwrap();
        let b = G2Bytes::new_unchecked(
            [
                "799096037534263564394323941982781608031806843599379318443427814019873224162",
                "14026173330568980628011709588549732085308934280497623796136346291913189596064",
            ],
            [
                "18512483370445888670421748202641195280704367913960380279153644128302403162953",
                "11254131899335650800706930224907562847943361881351835752623166468667575239687",
            ],
        )
        .unwrap();
        let c = G1Bytes::new_unchecked(
            "161411929919357135819312594620804205291494587085213166645876168613542945746",
            "20470377953299181976881540108292343474195200393467944112548990712451344598537",
        )
        .unwrap();
        let proof = Groth16Zkp::new(a, b, c);

        let max_exp_horizon = 100_255_944; // old hardcoded value, which is now in Move, that this testcase was generated with
        let sender = Ed25519PrivateKey::generate_for_testing();
        let sender_pub = sender.public_key();
        let sender_auth_key = AuthenticationKey::ed25519(&sender_pub);
        let sender_addr = sender_auth_key.account_address();
        let raw_txn = crate::test_helpers::transaction_test_helpers::get_test_signed_transaction(
            sender_addr,
            0,
            &sender,
            sender.public_key(),
            None,
            0,
            0,
            None,
        )
        .into_raw_transaction();

        let sender_sig = sender.sign(&raw_txn).unwrap();

        let epk = EphemeralPublicKey::ed25519(sender.public_key());
        let es = EphemeralSignature::ed25519(sender_sig);

        let proof_sig = sender.sign(&proof).unwrap();
        let ephem_proof_sig = EphemeralSignature::ed25519(proof_sig);
        let zk_sig = ZkIdSignature {
            sig: ZkpOrOpenIdSig::Groth16Zkp(SignedGroth16Zkp {
                proof: proof.clone(),
                non_malleability_signature: ephem_proof_sig,
                training_wheels_signature: EphemeralSignature::ed25519(
                    Ed25519Signature::dummy_signature(),
                ),
                extra_field: "\"family_name\":\"Straka\",".to_string(),
                override_aud_val: None,
            }),
            jwt_header: "eyJhbGciOiJSUzI1NiIsImtpZCI6InRlc3RfandrIiwidHlwIjoiSldUIn0".to_owned(),
            exp_timestamp_secs: 1900255944,
            ephemeral_pubkey: epk,
            ephemeral_signature: es,
        };

        let pepper = Pepper::from_number(76);
        let addr_seed = IdCommitment::new_from_preimage(
            &pepper,
            "407408718192.apps.googleusercontent.com",
            "sub",
            "113990307082899718775",
        )
        .unwrap();

        let zk_pk = ZkIdPublicKey {
            iss: "https://accounts.google.com".to_owned(),
            idc: addr_seed,
        };
        let jwk = RSA_JWK {
            kid:"1".to_owned(),
            kty:"RSA".to_owned(),
            alg:"RS256".to_owned(),
            e:"AQAB".to_owned(),
            n:"6S7asUuzq5Q_3U9rbs-PkDVIdjgmtgWreG5qWPsC9xXZKiMV1AiV9LXyqQsAYpCqEDM3XbfmZqGb48yLhb_XqZaKgSYaC_h2DjM7lgrIQAp9902Rr8fUmLN2ivr5tnLxUUOnMOc2SQtr9dgzTONYW5Zu3PwyvAWk5D6ueIUhLtYzpcB-etoNdL3Ir2746KIy_VUsDwAM7dhrqSK8U2xFCGlau4ikOTtvzDownAMHMrfE7q1B6WZQDAQlBmxRQsyKln5DIsKv6xauNsHRgBAKctUxZG8M4QJIx3S6Aughd3RZC4Ca5Ae9fd8L8mlNYBCrQhOZ7dS0f4at4arlLcajtw".to_owned(),
        };

        let public_inputs_hash =
            get_public_inputs_hash(&zk_sig, &zk_pk, &jwk, max_exp_horizon).unwrap();

        proof
            .verify_proof(public_inputs_hash, DEVNET_VERIFYING_KEY.deref())
            .unwrap();
    }

    /// Returns frequently-used JSON in our test cases
    fn get_jwt_payload_json(
        iss: &str,
        uid_key: &str,
        uid_val: &str,
        aud: &str,
        nonce: Option<String>,
    ) -> String {
        let nonce_str = match &nonce {
            None => "uxxgjhTml_fhiFwyWCyExJTD3J2YK3MoVDOYdnxieiE",
            Some(s) => s.as_str(),
        };

        format!(
            r#"{{
            "iss": "{}",
            "{}": "{}",
            "aud": "{}",
            "nonce": "{}",
            "exp": 1311281970,
            "iat": 1311280970,
            "name": "Jane Doe",
            "given_name": "Jane",
            "family_name": "Doe",
            "gender": "female",
            "birthdate": "0000-10-31",
            "email": "janedoe@example.com",
            "picture": "http://example.com/janedoe/me.jpg"
           }}"#,
            iss, uid_key, uid_val, aud, nonce_str
        )
    }

    fn get_jwt_default_values() -> (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        u64,
        u64,
        EphemeralPublicKey,
        u128,
        ZkIdPublicKey,
    ) {
        let iss = "https://server.example.com";
        let aud = "s6BhdRkqt3";
        let uid_key = "sub";
        let uid_val = "248289761001";
        let exp_timestamp_secs = 1311281970;
        let max_exp_horizon = 100_255_944; // old hardcoded value, which is now in Move, that this testcase was generated with
        let pepper = 76;

        let zkid_pk = ZkIdPublicKey {
            iss: iss.to_owned(),
            idc: IdCommitment::new_from_preimage(
                &Pepper::from_number(pepper),
                aud,
                uid_key,
                uid_val,
            )
            .unwrap(),
        };

        let epk =
            EphemeralPublicKey::ed25519(Ed25519PrivateKey::generate_for_testing().public_key());

        (
            iss,
            aud,
            uid_key,
            uid_val,
            exp_timestamp_secs,
            max_exp_horizon,
            epk,
            pepper,
            zkid_pk,
        )
    }

    #[test]
    fn test_zkid_oidc_sig_verifies() {
        let (iss, aud, uid_key, uid_val, exp_timestamp_secs, max_exp_horizon, epk, pepper, zkid_pk) =
            get_jwt_default_values();

        let oidc_sig = zkid_simulate_oidc_signature(
            uid_key,
            pepper,
            &get_jwt_payload_json(iss, uid_key, uid_val, aud, None),
        );
        assert!(oidc_sig
            .verify_jwt_claims(exp_timestamp_secs, &epk, &zkid_pk, max_exp_horizon,)
            .is_ok());
    }

    #[test]
    fn test_zkid_oidc_sig_fails_with_different_pepper() {
        let (iss, aud, uid_key, uid_val, exp_timestamp_secs, max_exp_horizon, epk, pepper, zkid_pk) =
            get_jwt_default_values();
        let bad_pepper = pepper + 1;

        let oidc_sig = zkid_simulate_oidc_signature(
            uid_key,
            pepper,
            &get_jwt_payload_json(iss, uid_key, uid_val, aud, None),
        );

        assert!(oidc_sig
            .verify_jwt_claims(exp_timestamp_secs, &epk, &zkid_pk, max_exp_horizon,)
            .is_ok());

        let bad_oidc_sig = zkid_simulate_oidc_signature(
            uid_key,
            bad_pepper, // Pepper does not match
            &get_jwt_payload_json(iss, uid_key, uid_val, aud, None),
        );

        assert!(bad_oidc_sig
            .verify_jwt_claims(exp_timestamp_secs, &epk, &zkid_pk, max_exp_horizon,)
            .is_err());
    }

    #[test]
    fn test_zkid_oidc_sig_fails_with_expiry_past_horizon() {
        let (iss, aud, uid_key, uid_val, exp_timestamp_secs, max_exp_horizon, epk, pepper, zkid_pk) =
            get_jwt_default_values();
        let oidc_sig = zkid_simulate_oidc_signature(
            uid_key,
            pepper,
            &get_jwt_payload_json(iss, uid_key, uid_val, aud, None),
        );

        assert!(oidc_sig
            .verify_jwt_claims(exp_timestamp_secs, &epk, &zkid_pk, max_exp_horizon,)
            .is_ok());

        let bad_exp_timestamp_secs = 1000000000000000000;
        assert!(oidc_sig
            .verify_jwt_claims(bad_exp_timestamp_secs, &epk, &zkid_pk, max_exp_horizon,)
            .is_err());
    }

    #[test]
    fn test_zkid_oidc_sig_fails_with_different_uid_val() {
        let (iss, aud, uid_key, uid_val, exp_timestamp_secs, max_exp_horizon, epk, pepper, zkid_pk) =
            get_jwt_default_values();
        let oidc_sig = zkid_simulate_oidc_signature(
            uid_key,
            pepper,
            &get_jwt_payload_json(iss, uid_key, uid_val, aud, None),
        );

        assert!(oidc_sig
            .verify_jwt_claims(exp_timestamp_secs, &epk, &zkid_pk, max_exp_horizon,)
            .is_ok());

        let bad_uid_val = format!("{}+1", uid_val);
        let bad_oidc_sig = zkid_simulate_oidc_signature(
            uid_key,
            pepper,
            &get_jwt_payload_json(iss, uid_key, bad_uid_val.as_str(), aud, None),
        );

        assert!(bad_oidc_sig
            .verify_jwt_claims(exp_timestamp_secs, &epk, &zkid_pk, max_exp_horizon,)
            .is_err());
    }

    #[test]
    fn test_zkid_oidc_sig_fails_with_bad_nonce() {
        let (iss, aud, uid_key, uid_val, exp_timestamp_secs, max_exp_horizon, epk, pepper, zkid_pk) =
            get_jwt_default_values();
        let oidc_sig = zkid_simulate_oidc_signature(
            uid_key,
            pepper,
            &get_jwt_payload_json(iss, uid_key, uid_val, aud, None),
        );

        assert!(oidc_sig
            .verify_jwt_claims(exp_timestamp_secs, &epk, &zkid_pk, max_exp_horizon,)
            .is_ok());

        let bad_nonce = "bad nonce".to_string();
        let bad_oidc_sig = zkid_simulate_oidc_signature(
            uid_key,
            pepper,
            &get_jwt_payload_json(iss, uid_key, uid_val, aud, Some(bad_nonce)),
        );

        assert!(bad_oidc_sig
            .verify_jwt_claims(exp_timestamp_secs, &epk, &zkid_pk, max_exp_horizon,)
            .is_err());
    }

    #[test]
    fn test_zkid_oidc_sig_with_different_iss() {
        let (iss, aud, uid_key, uid_val, exp_timestamp_secs, max_exp_horizon, epk, pepper, zkid_pk) =
            get_jwt_default_values();
        let oidc_sig = zkid_simulate_oidc_signature(
            uid_key,
            pepper,
            &get_jwt_payload_json(iss, uid_key, uid_val, aud, None),
        );

        assert!(oidc_sig
            .verify_jwt_claims(exp_timestamp_secs, &epk, &zkid_pk, max_exp_horizon,)
            .is_ok());

        let bad_iss = format!("{}+1", iss);
        let bad_oidc_sig = zkid_simulate_oidc_signature(
            uid_key,
            pepper,
            &get_jwt_payload_json(bad_iss.as_str(), uid_key, uid_val, aud, None),
        );

        assert!(bad_oidc_sig
            .verify_jwt_claims(exp_timestamp_secs, &epk, &zkid_pk, max_exp_horizon,)
            .is_err());
    }

    fn zkid_simulate_oidc_signature(
        uid_key: &str,
        pepper: u128,
        jwt_payload_unencoded: &str,
    ) -> OpenIdSig {
        let jwt_payload = base64url_encode_str(jwt_payload_unencoded);

        OpenIdSig {
            jwt_sig: "jwt_sig is verified in the prologue".to_string(),
            jwt_payload,
            uid_key: uid_key.to_owned(),
            epk_blinder: vec![0u8; OpenIdSig::EPK_BLINDER_NUM_BYTES],
            pepper: Pepper::from_number(pepper),
        }
    }
}

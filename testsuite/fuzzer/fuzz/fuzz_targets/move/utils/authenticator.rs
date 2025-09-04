// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]

use velor_types::keyless::{AnyKeylessPublicKey, EphemeralCertificate};
use arbitrary::Arbitrary;
use fuzzer::UserAccount;
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Debug, Arbitrary, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct JwtHeader {
    pub alg: String,
    pub typ: Option<String>,
    pub kid: Option<String>,
    // Add other JWT header fields as needed
}

#[derive(Debug, Arbitrary, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct FuzzingKeylessSignature {
    exp_date_secs: u64,
    jwt_header: JwtHeader,
    cert: EphemeralCertificate,
    //ephemeral_pubkey: EphemeralPublicKey,
    //ephemeral_signature: EphemeralSignature,
}

impl FuzzingKeylessSignature {
    pub fn exp_date_secs(&self) -> u64 {
        self.exp_date_secs
    }

    pub fn jwt_header(&self) -> &JwtHeader {
        &self.jwt_header
    }

    pub fn cert(&self) -> &EphemeralCertificate {
        &self.cert
    }

    /*
    pub fn ephemeral_pubkey(&self) -> &EphemeralPublicKey {
        &self.ephemeral_pubkey
    }

    pub fn ephemeral_signature(&self) -> &EphemeralSignature {
        &self.ephemeral_signature
    }
    */
}

#[derive(Debug, Arbitrary, Eq, PartialEq, Clone)]
pub enum Style {
    Break,
    //MatchJWT,
    //MatchKeys,
}

//TODO: reorganize this type excluding not usefull fields. Do it after implementing JWK and Federated Keyless.
// Used to fuzz the transaction authenticator
#[derive(Debug, Arbitrary, Eq, PartialEq, Clone)]
pub enum FuzzerTransactionAuthenticator {
    Ed25519 {
        sender: UserAccount,
    },
    Keyless {
        sender: UserAccount,
        style: Style,
        any_keyless_public_key: AnyKeylessPublicKey,
        keyless_signature: FuzzingKeylessSignature,
    },
    MultiAgent {
        sender: UserAccount,
        secondary_signers: Vec<UserAccount>,
    },
    FeePayer {
        sender: UserAccount,
        secondary_signers: Vec<UserAccount>,
        fee_payer: UserAccount,
    },
}

impl FuzzerTransactionAuthenticator {
    pub fn sender(&self) -> UserAccount {
        match self {
            FuzzerTransactionAuthenticator::Ed25519 { sender } => *sender,
            FuzzerTransactionAuthenticator::Keyless {
                sender,
                style: _,
                any_keyless_public_key: _,
                keyless_signature: _,
            } => *sender,
            FuzzerTransactionAuthenticator::MultiAgent {
                sender,
                secondary_signers: _,
            } => *sender,
            FuzzerTransactionAuthenticator::FeePayer {
                sender,
                secondary_signers: _,
                fee_payer: _,
            } => *sender,
        }
    }

    pub fn get_jwt_header_json(&self) -> Option<String> {
        if let FuzzerTransactionAuthenticator::Keyless {
            keyless_signature, ..
        } = self
        {
            serde_json::to_string(&keyless_signature.jwt_header).ok()
        } else {
            None
        }
    }
}

#[derive(Debug, Arbitrary, Eq, PartialEq, Clone)]
pub struct TransactionState {
    pub tx_auth_type: FuzzerTransactionAuthenticator,
}

pub(crate) mod miscellaneous {
    use velor_crypto::{ed25519::ed25519_keys::Ed25519PrivateKey, PrivateKey, Uniform};
    use velor_types::{
        jwks::rsa::INSECURE_TEST_RSA_KEY_PAIR,
        keyless::{Configuration, OpenIdSig, Pepper},
        transaction::authenticator::EphemeralPublicKey,
    };
    use once_cell::sync::Lazy;
    use ring::signature::RsaKeyPair;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    pub(crate) const SAMPLE_UID_KEY: &str = "sub";

    /// This is the SK from https://token.dev/.
    /// To convert it into a JSON, you can use https://irrte.ch/jwt-js-decode/pem2jwk.html
    pub(crate) static SAMPLE_JWK_SK: Lazy<&RsaKeyPair> = Lazy::new(|| &*INSECURE_TEST_RSA_KEY_PAIR);

    pub(crate) static SAMPLE_PEPPER: Lazy<Pepper> = Lazy::new(|| Pepper::from_number(76));

    pub(crate) static SAMPLE_ESK: Lazy<Ed25519PrivateKey> =
        Lazy::new(Ed25519PrivateKey::generate_for_testing);

    pub(crate) static SAMPLE_EPK: Lazy<EphemeralPublicKey> =
        Lazy::new(|| EphemeralPublicKey::ed25519(SAMPLE_ESK.public_key()));

    pub(crate) static SAMPLE_EPK_BLINDER: Lazy<Vec<u8>> = Lazy::new(|| {
        let mut byte_vector = vec![0; 31];
        byte_vector[0] = 42;
        byte_vector
    });

    /// The nonce-committed expiration date (not the JWT `exp`), 12/21/5490
    pub(crate) const SAMPLE_EXP_DATE: u64 = 111_111_111_111;

    /// Consistent with what is in `SAMPLE_JWT_PAYLOAD_JSON`
    pub(crate) const SAMPLE_JWT_EXTRA_FIELD_KEY: &str = "family_name";

    /// Consistent with what is in `SAMPLE_JWT_PAYLOAD_JSON`
    pub(crate) static SAMPLE_JWT_EXTRA_FIELD: Lazy<String> =
        Lazy::new(|| format!(r#""{}":"Straka","#, SAMPLE_JWT_EXTRA_FIELD_KEY));

    static SAMPLE_NONCE: Lazy<String> = Lazy::new(|| {
        let config = Configuration::new_for_testing();
        OpenIdSig::reconstruct_oauth_nonce(
            SAMPLE_EPK_BLINDER.as_slice(),
            SAMPLE_EXP_DATE,
            &SAMPLE_EPK,
            &config,
        )
        .unwrap()
    });

    pub(crate) const SAMPLE_TEST_ISS_VALUE: &str = "test.oidc.provider";

    #[derive(Serialize, Deserialize, Debug)]
    pub struct SampleJwtPayload {
        pub iss: String,
        pub azp: String,
        pub aud: String,
        pub sub: String,
        pub hd: String,
        pub email: String,
        pub email_verified: bool,
        pub at_hash: String,
        pub name: String,
        pub picture: String,
        pub given_name: String,
        #[serde(flatten)]
        pub extra_fields: HashMap<String, serde_json::Value>,
        pub locale: String,
        pub iat: u64,
        pub nonce: String,
        pub exp: u64,
    }

    impl Default for SampleJwtPayload {
        fn default() -> Self {
            // Parse the extra fields from the SAMPLE_JWT_EXTRA_FIELD string
            let extra_fields_json = format!("{{{}}}", SAMPLE_JWT_EXTRA_FIELD.as_str());
            let extra_fields: HashMap<String, serde_json::Value> =
                serde_json::from_str(&extra_fields_json).unwrap_or_default();

            SampleJwtPayload {
                iss: SAMPLE_TEST_ISS_VALUE.to_string(),
                azp: "407408718192.apps.googleusercontent.com".to_string(),
                aud: "407408718192.apps.googleusercontent.com".to_string(),
                sub: "113990307082899718775".to_string(),
                hd: "velorlabs.com".to_string(),
                email: "michael@velorlabs.com".to_string(),
                email_verified: true,
                at_hash: "bxIESuI59IoZb5alCASqBg".to_string(),
                name: "Michael Straka".to_string(),
                picture: "https://lh3.googleusercontent.com/a/ACg8ocJvY4kVUBRtLxe1IqKWL5i7tBDJzFp9YuWVXMzwPpbs=s96-c".to_string(),
                given_name: "Michael".to_string(),
                extra_fields,
                locale: "en".to_string(),
                iat: 1700255944,
                nonce: SAMPLE_NONCE.as_str().to_string(),
                exp: 2700259544,
            }
        }
    }
}

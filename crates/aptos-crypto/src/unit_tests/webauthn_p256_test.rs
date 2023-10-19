// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::p256_ecdsa::P256PublicKey;
use crate::test_utils::KeyPair;
use crate::webauthn::webauthn_p256_keys::WebAuthnP256PrivateKey;
use crate::webauthn::{webauthn_p256_keys, WebAuthnP256PublicKey, WebAuthnP256Signature};
use crate::{HashValue, Signature, Uniform};
use p256::elliptic_curve::generic_array;
use rand_core::{OsRng, RngCore};
use webauthn_authenticator_rs::prelude::{RequestChallengeResponse, Url};
use webauthn_authenticator_rs::softpasskey::SoftPasskey;
use webauthn_authenticator_rs::WebauthnAuthenticator;
use webauthn_rs_core::assertion::generate_bcs_encoded_paarr_vector;
use webauthn_rs_core::encoding::SerializableCollectedClientData;
use webauthn_rs_core::interface::{AuthenticationState, COSEEC2Key, COSEKeyType, Credential};
use webauthn_rs_core::internals::Challenge;
use webauthn_rs_core::proto::{
    AttestationConveyancePreference, Base64UrlSafeData, COSEAlgorithm, CollectedClientData,
    PublicKeyCredential, RegisterPublicKeyCredential, UserVerificationPolicy,
};
use webauthn_rs_core::WebauthnCore;

type Base64UrlChallenge = Base64UrlSafeData;
type RawTxnByteVector = Vec<u8>;

/// This helper function generates random bytes to simulate a raw transaction and challenge. Returns:
/// 1. Fake, fixed byte raw txn, `t`
/// 2. The challenge -> SHA3_256(t)
///
/// For context, during a WebAuthn assertion, the device authenticator signs over the binary
/// concatenation of `authenticatorData` and SHA256(`clientDataJSON`).
///
/// `ClientDataJSON` contains a `challenge` field which is used to store the SHA3_256 of the `RawTransaction`.
fn generate_random_challenge_data(raw_txn_size: usize) -> (RawTxnByteVector, Base64UrlSafeData) {
    let mut raw_txn_bytes: Vec<u8> = vec![0; raw_txn_size];
    let mut rng = OsRng;
    rng.fill_bytes(&mut raw_txn_bytes);
    let challenge = Base64UrlSafeData::from(HashValue::sha3_256_of(&raw_txn_bytes).to_vec());
    (raw_txn_bytes, challenge)
}

/// Helper function that creates boilerplate for a WebAuthn registration (creation of a new passkey)
/// Uses [`SoftPasskey`](webauthn_authenticator_rs::SoftPasskey) to simulate the creation of a
/// [`PublicKeyCredential`](webauthn_rs_core::proto::PublicKeyCredential) (Passkey credential).
fn registration_helper(
    challenge_data: Option<(RawTxnByteVector, Base64UrlChallenge)>,
) -> (
    WebauthnCore,
    WebauthnAuthenticator<SoftPasskey>,
    Credential,
    RequestChallengeResponse,
    AuthenticationState,
    RawTxnByteVector,
) {
    let wan = WebauthnCore::new_unsafe_experts_only(
        "https://localhost:8080/auth",
        "localhost",
        vec![Url::parse("https://localhost:8080").unwrap()],
        None,
        None,
        None,
    );

    let unique_id = [
        158, 170, 228, 89, 68, 28, 73, 194, 134, 19, 227, 153, 107, 220, 150, 238,
    ];
    let name = "andrew";

    // These registration options create
    let (chal, reg_state) = wan
        .generate_challenge_register_options(
            &unique_id,
            name,
            name,
            AttestationConveyancePreference::Direct,
            Some(UserVerificationPolicy::Preferred),
            None,
            None,
            vec![COSEAlgorithm::ES256],
            false,
            None,
            false,
        )
        .unwrap();

    let mut wa = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let reg_credential = wa
        .do_registration(Url::parse("https://localhost:8080").unwrap(), chal)
        .expect("Failed to register");

    let credential = wan
        .register_credential(&reg_credential, &reg_state, None)
        .unwrap();

    let (mut req_challenge_resp, mut auth_state) = wan
        .generate_challenge_authenticate(vec![credential.clone()], None)
        .unwrap();

    let (raw_txn, challenge) = match challenge_data {
        None => generate_random_challenge_data(512),
        Some((raw_txn_bytes, challenge_base64)) => (raw_txn_bytes, challenge_base64),
    };

    // We don't want to use the default challenge from `generate_challenge_authenticate`
    // we are replacing the default challenge in the `RequestChallengeResponse` and the
    // `AuthenticationState` with the fake challenge created above
    req_challenge_resp.public_key.challenge = challenge.clone();
    auth_state.set_challenge(challenge.clone());

    (wan, wa, credential, req_challenge_resp, auth_state, raw_txn)
}

/// Generates a `WebAuthnP256PublicKey` from the `key` of a WebAuthn [`Credential`](webauthn_rs_core::interface::Credential)
fn generate_webauthn_pubkey(key: COSEEC2Key) -> WebAuthnP256PublicKey {
    let x_bytes = &key.x.0;
    let y_bytes = &key.y.0;

    let generic_array_x = generic_array::GenericArray::clone_from_slice(x_bytes.as_slice());
    let generic_array_y = generic_array::GenericArray::clone_from_slice(y_bytes.as_slice());
    let encoded_point =
        p256::EncodedPoint::from_affine_coordinates(&generic_array_x, &generic_array_y, false);
    let verifying_key = p256::ecdsa::VerifyingKey::from_encoded_point(&encoded_point);

    let webauthn_pubkey = WebAuthnP256PublicKey(P256PublicKey(verifying_key.unwrap()));
    webauthn_pubkey
}

/// Test to ensure an error occurs when the bytes used for signature is malformed due to incorrect bcs encoding of
/// [`PartialAuthenticatorAssertionResponseRaw`](webauthn_rs_core::assertion::PartialAuthenticatorAssertionResponseRaw)
/// Test to ensure it errors
#[test]
fn signature_serialization_failure_test() {
    let (_wan, mut wa, _credential, req_challenge_resp, _auth_state, ..) =
        registration_helper(None);

    let assertion_credential = wa
        .do_authentication(
            Url::parse("https://localhost:8080").unwrap(),
            req_challenge_resp,
        )
        .expect("Failed to auth");

    // Wrongly encoded bcs_paarr -> client_data and authenticator_data are switched
    let wrong_bcs_paarr = generate_bcs_encoded_paarr_vector(
        assertion_credential.response.signature.0.as_slice(),
        assertion_credential.response.client_data_json.0.as_slice(),
        assertion_credential
            .response
            .authenticator_data
            .0
            .as_slice(),
    );

    assert!(wrong_bcs_paarr.is_err());
}

/// According to the WebAuthn specification [§5.8.1](https://www.w3.org/TR/webauthn-3/#dictionary-client-data),
/// the [`CollectedClientData`](CollectedClientData) struct can be
/// extended in the future. This tests that signature verification still holds even in the
/// event that the struct is extended.
///
/// In this test we use the Secure Payment Confirmation (SPC) specification as an example where
/// `CollectedClientData` could be extended. SPC assertion responses include a
/// [`CollectedClientPaymentData`](https://www.w3.org/TR/secure-payment-confirmation/#sctn-collectedclientpaymentdata-dictionary)
/// struct that extends `CollectedClientData`.
#[test]
fn extended_client_data_verification_test() {
    let wan = WebauthnCore::new_unsafe_experts_only(
        "http://localhost:4000",
        "localhost",
        vec![Url::parse("http://localhost:4000").unwrap()],
        None,
        None,
        None,
    );

    // for reference, the plaintext, utf-8 encoded raw_txn is "hello world"
    let fake_raw_txn = Base64UrlSafeData::try_from("aGVsbG8gd29ybGQ").unwrap();
    let actual_sha3_256_fake_raw_txn =
        Base64UrlSafeData::try_from("ZEvMflZDcwQJmarInnYi88px-6HZcv2Uoxw7-_JOOTg").unwrap();
    let expected_sha3_256_fake_raw_txn = HashValue::sha3_256_of(fake_raw_txn.0.as_slice()).to_vec();

    // confirm that the sha3_256 is correctly computed
    assert_eq!(
        actual_sha3_256_fake_raw_txn.clone().0.to_vec(),
        expected_sha3_256_fake_raw_txn
    );

    let registration_client_data_json = format!(
        r#"{{
        "type": "webauthn.create",
        "challenge": "{}",
        "origin": "http://localhost:4000",
        "crossOrigin": false
    }}"#,
        actual_sha3_256_fake_raw_txn.clone().to_string()
    );

    let registration_collected_client_data: CollectedClientData =
        serde_json::from_str(registration_client_data_json.as_str()).unwrap();
    let registration_client_data_base_64_url =
        serde_json::to_vec(&registration_collected_client_data)
            .map(Base64UrlSafeData)
            .unwrap();
    let registration_client_data_base_64_url_string =
        registration_client_data_base_64_url.to_string();

    let registration_rsp: RegisterPublicKeyCredential = serde_json::from_str(format!(r#"{{
            "id": "5XMfH5f3cwTiiQ680WrMK-_dIOlcmcMYpHXK5a6vBjc",
            "rawId": "5XMfH5f3cwTiiQ680WrMK-_dIOlcmcMYpHXK5a6vBjc",
            "response": {{
                "attestationObject": "o2NmbXRkbm9uZWdhdHRTdG10oGhhdXRoRGF0YVikSZYN5YgOjGh0NBcPZHZgW4_krrmihjLHmVzzuoMdl2NFAAAAAK3OAAI1vMYKZIsLJfHwVQMAIOVzHx-X93ME4okOvNFqzCvv3SDpXJnDGKR1yuWurwY3pQECAyYgASFYILb4mrp6cCBeb5Clre8Gw1khoxVM2ni8WM7scgghamReIlggFmKfY_3vaYdim8-8XEQBDwb1u1F3a5wTzmFkxwBgHqA",
                "clientDataJSON": "{}"
            }},
            "type": "public-key"
        }}"#, registration_client_data_base_64_url_string).as_str()).unwrap();

    let credential = wan
        .register_credential_internal(
            &registration_rsp,
            Default::default(),
            &Challenge::new(registration_collected_client_data.challenge.0),
            &[],
            &[COSEAlgorithm::ES256],
            None,
            false,
            &Default::default(),
            true,
        )
        .unwrap();

    // ensure client_data_json for registration_rsp was serialized / deserialized properly
    assert_eq!(
        registration_rsp.response.client_data_json,
        registration_client_data_base_64_url
    );

    // This is a sample Secure Payment Confirmation (SPC) client_data response
    // It will help us test for any issues in extensibility of the CollectedClientData struct
    // More info: https://www.w3.org/TR/secure-payment-confirmation/#sctn-collectedclientpaymentdata-dictionary
    let auth_client_data = r#"{
        "type": "payment.get",
        "challenge": "ZEvMflZDcwQJmarInnYi88px-6HZcv2Uoxw7-_JOOTg",
        "origin": "http://localhost:4000",
        "crossOrigin": false,
        "payment": {
            "rpId": "localhost",
            "topOrigin": "http://localhost:4000",
            "payeeOrigin": "https://localhost:4000",
            "total": {
                "value": "1.01",
                "currency": "APT"
            },
            "instrument": {
                "icon": "https://aptoslabs.com/assets/favicon-2c9e23abc3a3f4c45038e8c784b0a4ecb9051baa.ico",
                "displayName": "Petra test"
            }
        }
    }"#;

    // Using custom SerializableCollectedClientData to ensure bytes are serialized correctly
    let client_data: SerializableCollectedClientData = serde_json::from_str(&auth_client_data).unwrap();
    let auth_client_data_base64_url = Base64UrlSafeData::from(client_data.to_bytes());

    let assertion_credential: PublicKeyCredential = serde_json::from_str(
        format!(
            r#"{{
            "id": "5XMfH5f3cwTiiQ680WrMK-_dIOlcmcMYpHXK5a6vBjc",
            "rawId": "5XMfH5f3cwTiiQ680WrMK-_dIOlcmcMYpHXK5a6vBjc",
            "response": {{
                "authenticatorData": "SZYN5YgOjGh0NBcPZHZgW4_krrmihjLHmVzzuoMdl2MFAAAAAA",
                "clientDataJSON": "{}",
                "signature": "MEUCIQCl5RgiE754do1km7YCjlFZKR65cv7NUbBagrbx-BXzIwIgTVC7iAWwkKFAyNkFR-5DrVrpxiItDU0Lw7JsJyUBJ6M",
                "userHandle": "AQABAgMIAwYGAAYEAQYIAgcB"
            }},
            "type": "public-key"
        }}"#,
            auth_client_data_base64_url.to_string().as_str()
        )
        .as_str(),
    )
    .unwrap();

    // This is the signature the client would include in a SignedTransaction
    // Its a BCS encoded vector representation of PartialAuthenticatorAssertionResponseRaw
    // that contains the following items in order:
    // 1. signature
    // 2. authenticator_data
    // 3. client_data_json
    let bcs_paarr = generate_bcs_encoded_paarr_vector(
        assertion_credential.response.signature.0.as_slice(),
        assertion_credential
            .response
            .authenticator_data
            .0
            .as_slice(),
        assertion_credential.response.client_data_json.0.as_slice(),
    )
    .unwrap();

    let webauthn_sig = WebAuthnP256Signature::try_from(bcs_paarr.as_slice());
    assert!(webauthn_sig.is_ok());

    // Now verify it!
    match credential.cred.key {
        COSEKeyType::EC_EC2(key) => {
            let webauthn_pubkey = generate_webauthn_pubkey(key);

            let verification_result = webauthn_sig
                .unwrap()
                .verify_arbitrary_msg(fake_raw_txn.0.as_slice(), &webauthn_pubkey);

            assert!(verification_result.is_ok());
        },
        _ => {
            panic!("Test failed, credential key is not of type EC_EC2 and COSE Algorithm ES256")
        },
    }
}

#[test]
fn verify_failure_test() {
    let (wan, mut wa, _credential, req_challenge_resp, auth_state, raw_txn) =
        registration_helper(None);

    let assertion_credential = wa
        .do_authentication(
            Url::parse("https://localhost:8080").unwrap(),
            req_challenge_resp,
        )
        .expect("Failed to auth");

    wan.authenticate_credential(&assertion_credential, &auth_state)
        .expect("webauth authentication denied");

    // This is the signature the client would include in a SignedTransaction
    // Its a BCS encoded vector representation of PartialAuthenticatorAssertionResponseRaw
    // that contains the following items in order:
    // 1. signature
    // 2. authenticator_data
    // 3. client_data_json
    let bcs_paarr = generate_bcs_encoded_paarr_vector(
        assertion_credential.response.signature.0.as_slice(),
        assertion_credential
            .response
            .authenticator_data
            .0
            .as_slice(),
        assertion_credential.response.client_data_json.0.as_slice(),
    )
    .unwrap();

    let webauthn_sig = WebAuthnP256Signature::try_from(bcs_paarr.as_slice());
    assert!(webauthn_sig.is_ok());

    // Create a new public key credential
    let (.., credential, _req_challenge_resp, _auth_state, _raw_txn) = registration_helper(None);

    // Now verify it!
    match credential.cred.key {
        COSEKeyType::EC_EC2(key) => {
            let webauthn_pubkey = generate_webauthn_pubkey(key);

            let verification_result = webauthn_sig
                .unwrap()
                .verify_arbitrary_msg(&raw_txn, &webauthn_pubkey);

            // Should error since this is using the wrong public key to verify the signature
            assert!(verification_result.is_err());
        },
        _ => {
            panic!("Test failed, credential key is not of type EC_EC2 and COSE Algorithm ES256")
        },
    }
}

#[test]
fn verify_test() {
    let (wan, mut wa, credential, req_challenge_resp, auth_state, raw_txn) =
        registration_helper(None);

    let assertion_credential = wa
        .do_authentication(
            Url::parse("https://localhost:8080").unwrap(),
            req_challenge_resp,
        )
        .expect("Failed to auth");

    // This is the signature the client would include in a SignedTransaction
    // Its a BCS encoded vector representation of PartialAuthenticatorAssertionResponseRaw
    // that contains the following items in order:
    // 1. signature
    // 2. authenticator_data
    // 3. client_data_json
    let bcs_paarr = generate_bcs_encoded_paarr_vector(
        assertion_credential.response.signature.0.as_slice(),
        assertion_credential
            .response
            .authenticator_data
            .0
            .as_slice(),
        assertion_credential.response.client_data_json.0.as_slice(),
    )
    .unwrap();

    let webauthn_sig = WebAuthnP256Signature::try_from(bcs_paarr.as_slice());
    assert!(webauthn_sig.is_ok());

    // Now verify it!
    match credential.cred.key {
        COSEKeyType::EC_EC2(key) => {
            let webauthn_pubkey = generate_webauthn_pubkey(key);

            let verification_result = webauthn_sig
                .unwrap()
                .verify_arbitrary_msg(&raw_txn, &webauthn_pubkey);

            assert!(verification_result.is_ok());

            // This is an extra check, using the webauthn_rs built-in authenticate_credential method
            // Note: this will not work for non-"webauthn.get" payloads, like Secure Payment Confirmation (SPC)
            wan.authenticate_credential(&assertion_credential, &auth_state)
                .expect("webauth authentication denied");
        },
        _ => {
            panic!("Test failed, credential key is not of type EC_EC2 and COSE Algorithm ES256")
        },
    }
}

/// Tests public key private key (de)serialization
#[test]
fn key_serialization_test() {
    let mut rng = OsRng;
    let key_pair = KeyPair::<WebAuthnP256PrivateKey, WebAuthnP256PublicKey>::generate(&mut rng);

    let private_key_bytes = key_pair.private_key.to_bytes();
    let private_key_deserialized =
        WebAuthnP256PrivateKey::try_from(&private_key_bytes[..]).unwrap();
    assert_eq!(key_pair.private_key, private_key_deserialized);

    let public_key_bytes = key_pair.public_key.to_bytes();
    let public_key_deserialized = WebAuthnP256PublicKey::try_from(&public_key_bytes[..]).unwrap();
    assert_eq!(key_pair.public_key, public_key_deserialized);
}

/// Tests signature (de)serialization
#[test]
fn signature_serialization_test() {
    let (wan, mut wa, _credential, req_challenge_resp, auth_state, ..) = registration_helper(None);

    let assertion_credential = wa
        .do_authentication(
            Url::parse("https://localhost:8080").unwrap(),
            req_challenge_resp,
        )
        .expect("Failed to auth");

    wan.authenticate_credential(&assertion_credential, &auth_state)
        .expect("webauth authentication denied");

    let bcs_paarr = generate_bcs_encoded_paarr_vector(
        assertion_credential.response.signature.0.as_slice(),
        assertion_credential
            .response
            .authenticator_data
            .0
            .as_slice(),
        assertion_credential.response.client_data_json.0.as_slice(),
    )
    .unwrap();

    let signature = WebAuthnP256Signature::try_from(bcs_paarr.as_slice()).unwrap();
    let signature_bytes = signature.to_bytes();
    let signature_deserialized = WebAuthnP256Signature::try_from(&signature_bytes[..]).unwrap();
    assert_eq!(signature, signature_deserialized);
}

/// Test deserialization_failures
#[test]
fn deserialization_failure() {
    let fake = [0u8, 31];
    webauthn_p256_keys::WebAuthnP256PrivateKey::try_from(fake.as_slice()).unwrap_err();
    webauthn_p256_keys::WebAuthnP256PublicKey::try_from(fake.as_slice()).unwrap_err();
}

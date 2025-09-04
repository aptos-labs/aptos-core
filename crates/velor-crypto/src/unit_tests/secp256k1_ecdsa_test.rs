// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    secp256k1_ecdsa::{self, PrivateKey, PublicKey},
    test_utils::KeyPair,
    Signature, SigningKey, Uniform,
};
use rand_core::OsRng;

/// Tests that an individual signature share computed correctly on a message m passes verification on m.
/// Tests that a signature share computed on a different message m' fails verification on m.
/// Tests that a signature share fails verification under the wrong public key.
#[test]
fn basic() {
    let mut rng = OsRng;

    let message = b"Hello world";
    let message_wrong = b"Wello Horld";

    let key_pair = KeyPair::<PrivateKey, PublicKey>::generate(&mut rng);
    let key_pair_wrong = KeyPair::<PrivateKey, PublicKey>::generate(&mut rng);

    let signature = key_pair.private_key.sign_arbitrary_message(message);
    let signature_wrong = key_pair_wrong.private_key.sign_arbitrary_message(message);

    // sig on message under key_pair should verify
    assert!(signature
        .verify_arbitrary_msg(message, &key_pair.public_key)
        .is_ok());

    // sig_wrong on message under key_pair_wrong should verify
    assert!(signature_wrong
        .verify_arbitrary_msg(message, &key_pair_wrong.public_key)
        .is_ok());

    // sig on message under keypair should NOT verify under keypair_wrong
    assert!(signature
        .verify_arbitrary_msg(message, &key_pair_wrong.public_key)
        .is_err());

    // sig on message under keypair should NOT verify on message_wrong under key_pair
    assert!(signature
        .verify_arbitrary_msg(message_wrong, &key_pair.public_key)
        .is_err());

    // sig on message under keypair_wrong should NOT verify under key_pair
    assert!(signature_wrong
        .verify_arbitrary_msg(message, &key_pair.public_key)
        .is_err());
}

/// Tests signature (de)serialization
#[test]
fn serialization() {
    let mut rng = OsRng;
    let message = b"Hello world";
    let key_pair = KeyPair::<PrivateKey, PublicKey>::generate(&mut rng);

    let signature = key_pair.private_key.sign_arbitrary_message(message);
    assert!(signature
        .verify_arbitrary_msg(message, &key_pair.public_key)
        .is_ok());

    let signature_bytes = signature.to_bytes();
    let signature_deserialized =
        secp256k1_ecdsa::Signature::try_from(&signature_bytes[..]).unwrap();
    assert_eq!(signature, signature_deserialized);

    let private_key_bytes = key_pair.private_key.to_bytes();
    let private_key_deserialized =
        secp256k1_ecdsa::PrivateKey::try_from(&private_key_bytes[..]).unwrap();
    assert_eq!(key_pair.private_key, private_key_deserialized);

    let public_key_bytes = key_pair.public_key.to_bytes();
    let public_key_deserialized =
        secp256k1_ecdsa::PublicKey::try_from(&public_key_bytes[..]).unwrap();
    assert_eq!(key_pair.public_key, public_key_deserialized);
}

/// Tests malleability
#[test]
fn malleability() {
    let mut rng = OsRng;
    let message = b"Hello world";
    let key_pair = KeyPair::<PrivateKey, PublicKey>::generate(&mut rng);

    let signature = key_pair.private_key.sign_arbitrary_message(message);
    assert!(signature
        .verify_arbitrary_msg(message, &key_pair.public_key)
        .is_ok());

    let signature_bytes = signature.to_bytes();
    let signature_deserialized =
        secp256k1_ecdsa::Signature::try_from(&signature_bytes[..]).unwrap();
    assert_eq!(signature, signature_deserialized);

    let mut high_signature = signature.clone();
    high_signature.0.s = -high_signature.0.s;
    let high_signature_bytes = high_signature.to_bytes();

    // We can load
    secp256k1_ecdsa::Signature::try_from(&high_signature_bytes[..]).unwrap();

    // Ensure this is now high.
    assert!(!signature.0.s.is_high());
    assert!(high_signature.0.s.is_high());
    assert!(high_signature.0.s != signature.0.s);
    high_signature
        .verify_arbitrary_msg(message, &key_pair.public_key)
        .unwrap_err();
}

/// Test deserialization_failures
#[test]
fn deserialization_failure() {
    let fake = [0u8, 31];
    secp256k1_ecdsa::Signature::try_from(fake.as_slice()).unwrap_err();
    secp256k1_ecdsa::PrivateKey::try_from(fake.as_slice()).unwrap_err();
    secp256k1_ecdsa::PublicKey::try_from(fake.as_slice()).unwrap_err();
}

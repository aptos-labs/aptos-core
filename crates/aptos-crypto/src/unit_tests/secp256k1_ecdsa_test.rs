// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    secp256k1_ecdsa::{self, PrivateKey, PublicKey, Signature as ECDSASignature},
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

fn from_u32_be(val: u32) -> Vec<u8> {
    let res_0 = (val >> 24) as u8;
    let res_1 = (val >> 16) as u8;
    let res_2 = (val >> 8) as u8;
    let res_3 = val as u8; 
    vec![res_0, res_1, res_2, res_3]
}

/// Tests malleability
#[test]
fn malleability() {
    println!("running mal test");
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

    const SECP256K1_HALF_ORDER_FLOOR: [u32; 8] = [0x681B20A0, 0xDFE92F46, 0x57A4501D, 0x5D576E73, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0x7FFFFFFF];
    let mut vec_0 = from_u32_be(SECP256K1_HALF_ORDER_FLOOR[0]);
    let mut vec_1 = from_u32_be(SECP256K1_HALF_ORDER_FLOOR[1]); 
    let mut vec_2 = from_u32_be(SECP256K1_HALF_ORDER_FLOOR[2]); 
    let mut vec_3 = from_u32_be(SECP256K1_HALF_ORDER_FLOOR[3]); 
    let mut vec_4 = from_u32_be(SECP256K1_HALF_ORDER_FLOOR[4]); 
    let mut vec_5 = from_u32_be(SECP256K1_HALF_ORDER_FLOOR[5]); 
    let mut vec_6 = from_u32_be(SECP256K1_HALF_ORDER_FLOOR[6]); 
    let mut vec_7 = from_u32_be(SECP256K1_HALF_ORDER_FLOOR[7]); 
    let mut vector = vec!();
    vector.append(&mut vec_0);
    vector.append(&mut vec_1); 
    vector.append(&mut vec_2);
    vector.append(&mut vec_3);
    vector.append(&mut vec_4);
    vector.append(&mut vec_5);
    vector.append(&mut vec_6);
    vector.append(&mut vec_7);
    let mut dummy_vec: Vec<u8> = vec![0; 32];
    vector.append(&mut dummy_vec);

    //let sig: Signature = vector[..].try_from().unwrap();
    let sig = ECDSASignature::try_from(&vector[..]);
}

/// Test deserialization_failures
#[test]
fn deserialization_failure() {
    let fake = [0u8, 31];
    secp256k1_ecdsa::Signature::try_from(fake.as_slice()).unwrap_err();
    secp256k1_ecdsa::PrivateKey::try_from(fake.as_slice()).unwrap_err();
    secp256k1_ecdsa::PublicKey::try_from(fake.as_slice()).unwrap_err();
}

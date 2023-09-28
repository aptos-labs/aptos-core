// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::redundant_clone)] // Required to work around prop_assert_eq! limitations

use crate as aptos_crypto;
use crate::{
    ecdsa_p256::{P256_SIGNATURE_LENGTH, P256PublicKey, P256_PUBLIC_KEY_LENGTH, P256_PRIVATE_KEY_LENGTH, P256PrivateKey, P256Signature, ORDER_HALF}, 
    test_utils::{
        random_serializable_struct,
        uniform_keypair_strategy,
    },
    traits::*,
};
use signature::Verifier;
use p256::{EncodedPoint, NonZeroScalar};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use core::convert::TryFrom;
use proptest::{collection::vec, prelude::*};
use serde::{Deserialize, Serialize};

#[derive(CryptoHasher, BCSCryptoHash, Serialize, Deserialize)]
struct CryptoHashable(pub usize);
proptest! {
    #[test]
    fn test_pub_key_deserialization(bits in any::<[u8; 32]>()){
        let pt_deser = EncodedPoint::from_bytes(&bits[..]);
        let pub_key = P256PublicKey::try_from(&bits[..]);
        let check = matches!((pt_deser, pub_key),
            (Ok(_), Ok(_)) // we agree with RustCrypto's sec1 implementation,
            | (Err(_), Err(_)) // we agree on point decompression failures,
        );
        prop_assert!(check);
    }

    #[test]
    fn test_keys_encode(keypair in uniform_keypair_strategy::<P256PrivateKey, P256PublicKey>()) {
        {
            let encoded = keypair.private_key.to_encoded_string().unwrap();
            // Hex encoding of a 64-bytes key is 128 (2 x 64) characters.
            prop_assert_eq!(2 + 2 * P256_PRIVATE_KEY_LENGTH, encoded.len());
            let decoded = P256PrivateKey::from_encoded_string(&encoded);
            prop_assert_eq!(Some(keypair.private_key), decoded.ok());
        }
        {
            let encoded = keypair.public_key.to_encoded_string().unwrap();
            // Hex encoding of a 65-bytes key is 130 (2 x 65) characters.
            prop_assert_eq!(2 + 2 * P256_PUBLIC_KEY_LENGTH, encoded.len());
            let decoded = P256PublicKey::from_encoded_string(&encoded);
            prop_assert_eq!(Some(keypair.public_key), decoded.ok());
        }
    }

    #[test]
    fn test_batch_verify(
        message in random_serializable_struct(),
        keypairs in proptest::array::uniform10(uniform_keypair_strategy::<P256PrivateKey, P256PublicKey>())
    ) {
        let mut pks_and_sigs: Vec<(P256PublicKey, P256Signature)> = keypairs.iter().map(|keypair| {
            (keypair.public_key.clone(), keypair.private_key.sign(&message).unwrap())
        }).collect();
        prop_assert!(P256Signature::batch_verify(&message, pks_and_sigs.clone()).is_ok());
        // We swap message and signature for the last element,
        // resulting in an incorrect signature
        let (pk, _sig) = pks_and_sigs.pop().unwrap();
        let other_sig = pks_and_sigs.last().unwrap().clone().1;
        pks_and_sigs.push((pk, other_sig));
        prop_assert!(P256Signature::batch_verify(&message, pks_and_sigs).is_err());
    }

    #[test]
    fn test_keys_custom_serialisation(
        keypair in uniform_keypair_strategy::<P256PrivateKey, P256PublicKey>()
    ) {
        {
            let serialized: &[u8] = &(keypair.private_key.to_bytes());
            prop_assert_eq!(P256_PRIVATE_KEY_LENGTH, serialized.len());
            let deserialized = P256PrivateKey::try_from(serialized);
            prop_assert_eq!(Some(keypair.private_key), deserialized.ok());
        }
        {
            let serialized: &[u8] = &(keypair.public_key.to_bytes());
            prop_assert_eq!(P256_PUBLIC_KEY_LENGTH, serialized.len());
            let deserialized = P256PublicKey::try_from(serialized);
            prop_assert_eq!(Some(keypair.public_key), deserialized.ok());
        }
    }

    #[test]
    fn test_signature_verification_custom_serialisation(
        message in random_serializable_struct(),
        keypair in uniform_keypair_strategy::<P256PrivateKey, P256PublicKey>()
    ) {
        let signature = keypair.private_key.sign(&message).unwrap();
        let serialized: &[u8] = &(signature.to_bytes());
        prop_assert_eq!(P256_SIGNATURE_LENGTH, serialized.len());
        let deserialized = P256Signature::try_from(serialized).unwrap();
        prop_assert!(deserialized.verify(&message, &keypair.public_key).is_ok());
    }

    #[test]
    fn test_signature_verification_from_arbitrary(
        // this should be > 64 bits to go over the length of a default hash
        msg in vec(proptest::num::u8::ANY, 1..128),
        keypair in uniform_keypair_strategy::<P256PrivateKey, P256PublicKey>()
    ) {
        let signature = keypair.private_key.sign_arbitrary_message(&msg);
        let serialized: &[u8] = &(signature.to_bytes());
        prop_assert_eq!(P256_SIGNATURE_LENGTH, serialized.len());
        let deserialized = P256Signature::try_from(serialized).unwrap();
        prop_assert!(deserialized.verify_arbitrary_msg(&msg, &keypair.public_key).is_ok());
    }

    #[test]
    fn test_signature_verification_from_struct(
        x in any::<usize>(),
        keypair in uniform_keypair_strategy::<P256PrivateKey, P256PublicKey>()
    ) {
        let hashable = CryptoHashable(x);
        let signature = keypair.private_key.sign(&hashable).unwrap();
        let serialized: &[u8] = &(signature.to_bytes());
        prop_assert_eq!(P256_SIGNATURE_LENGTH, serialized.len());
        let deserialized = P256Signature::try_from(serialized).unwrap();
        prop_assert!(deserialized.verify(&hashable, &keypair.public_key).is_ok());
    }


    // Check for canonical S.
    #[test]
    fn test_signature_malleability(
        message in random_serializable_struct(),
        keypair in uniform_keypair_strategy::<P256PrivateKey, P256PublicKey>()
    ) {
        let signature = keypair.private_key.sign(&message).unwrap();
        let mut serialized = signature.to_bytes();
        let serialized_old = serialized; // implements Copy trait
        prop_assert_eq!(serialized_old, serialized);

        let mut r_bytes: [u8; 32] = [0u8; 32];
        r_bytes.copy_from_slice(&serialized[..32]);

        let mut s_bytes: [u8; 32] = [0u8; 32];
        s_bytes.copy_from_slice(&serialized[32..]);

        // NIST-P256 signing ensures a canonical S value.
        let s = NonZeroScalar::try_from(&s_bytes[..]).unwrap();

        // computing s' = n - s to obtain the non-canonical valid signature over `message`
        let malleable_s = NonZeroScalar::new(-*s).unwrap();
        let malleable_s_bytes = malleable_s.to_bytes();
        // Update the signature (the S part).
        serialized[32..].copy_from_slice(&malleable_s_bytes);

        prop_assert_ne!(serialized_old, serialized);

        // Check that valid non-canonical signatures will pass verification and deserialization in the RustCrypto
        // p256 crate.
        // Construct the corresponding RustCrypto p256 public key.
        let rustcrypto_public_key = p256::ecdsa::VerifyingKey::from_sec1_bytes(
            &keypair.public_key.to_bytes()
        ).unwrap();

        // Construct the corresponding RustCrypto p256 Signature. This signature is valid but
        // non-canonical.
        let rustcrypto_sig = p256::ecdsa::Signature::try_from(&serialized[..]);

        // RustCrypto p256 will deserialize the non-canonical
        // signature. It does not detect it.
        prop_assert!(rustcrypto_sig.is_ok());

        let msg_bytes = signing_message(&message);
        prop_assert!(msg_bytes.is_ok());

        let rustcrypto_sig = rustcrypto_sig.unwrap();
        // RustCrypto p256 verify WILL accept the mauled signature
        prop_assert!(rustcrypto_public_key.verify(msg_bytes.as_ref().unwrap(), &rustcrypto_sig).is_ok());
        // ...however, our own P256Signature::verify will not
        let sig = P256Signature::from_bytes_unchecked(&serialized).unwrap();
        prop_assert!(sig.verify(&message, &keypair.public_key).is_err());

        let serialized_malleable: &[u8] = &serialized;
        // try_from will fail on non-canonical signatures. We detect non-canonical signatures
        // early during deserialization.
        prop_assert_eq!(
            P256Signature::try_from(serialized_malleable),
            Err(CryptoMaterialError::CanonicalRepresentationError)
        );

        // We expect from_bytes_unchecked deserialization to succeed, as RustCrypto p256
        // does not check for non-canonical signatures. This method is pub(crate)
        // and only used for test purposes.
        let sig_unchecked = P256Signature::from_bytes_unchecked(&serialized);
        prop_assert!(sig_unchecked.is_ok());

        // Update the signature by setting S = L to make it invalid.
        serialized[32..].copy_from_slice(&ORDER_HALF);
        let serialized_malleable_l: &[u8] = &serialized;
        // try_from will fail with CanonicalRepresentationError.
        prop_assert_eq!(
            P256Signature::try_from(serialized_malleable_l),
            Err(CryptoMaterialError::CanonicalRepresentationError)
        );
    }
}

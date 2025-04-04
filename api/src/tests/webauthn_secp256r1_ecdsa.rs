// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use crate::tests::new_test_context_with_orderless_flags;
    use aptos_api_test_context::current_function_name;
    use aptos_crypto::{
        ed25519::Ed25519PrivateKey,
        secp256r1_ecdsa::{
            PrivateKey as Secp256r1EcdsaPrivateKey, PublicKey as Secp256r1EcdsaPublicKey,
        },
        signing_message, HashValue, SigningKey,
    };
    use aptos_sdk::types::LocalAccount;
    use aptos_types::transaction::{
        authenticator::{
            AccountAuthenticator, AnyPublicKey, AnySignature, AuthenticationKey,
            SingleKeyAuthenticator, TransactionAuthenticator,
        },
        webauthn::{AssertionSignature, PartialAuthenticatorAssertionResponse},
        RawTransaction, SignedTransaction,
    };
    use passkey_types::{
        crypto::sha256,
        webauthn::{ClientDataType, CollectedClientData},
        Bytes,
    };
    use rand::{prelude::StdRng, SeedableRng};
    use rstest::rstest;

    /// Sample `AuthenticatorData`
    static AUTHENTICATOR_DATA: &[u8] = &[
        73, 150, 13, 229, 136, 14, 140, 104, 116, 52, 23, 15, 100, 118, 96, 91, 143, 228, 174, 185,
        162, 134, 50, 199, 153, 92, 243, 186, 131, 29, 151, 99, 29, 0, 0, 0, 0,
    ];

    /// Given a `RawTransaction`, returns a test `CollectedClientData` struct
    fn get_collected_client_data(raw_transaction: &RawTransaction) -> CollectedClientData {
        let signing_message = signing_message(raw_transaction).unwrap();
        let sha3_256_raw_txn = HashValue::sha3_256_of(signing_message.as_slice());
        let sha3_256_raw_txn_bytes = Bytes::from(sha3_256_raw_txn.to_vec());

        CollectedClientData {
            ty: ClientDataType::Get,
            challenge: String::from(sha3_256_raw_txn_bytes),
            origin: "http://localhost:5173".to_string(),
            cross_origin: None,
            unknown_keys: Default::default(),
        }
    }

    fn sign_webauthn_transaction(
        raw_txn: &RawTransaction,
        collected_client_data: CollectedClientData,
        authenticator_data: &[u8],
        private_key: &Secp256r1EcdsaPrivateKey,
    ) -> SignedTransaction {
        let public_key = Secp256r1EcdsaPublicKey::from(private_key);

        let client_data_json = serde_json::to_vec(&collected_client_data).unwrap();
        let client_data_hash = sha256(client_data_json.as_slice());

        let signature_material = [authenticator_data, &client_data_hash].concat();
        let signature = private_key.sign_arbitrary_message(signature_material.as_slice());
        let assertion_signature = AssertionSignature::Secp256r1Ecdsa { signature };

        let partial_authenticator_assertion_response = PartialAuthenticatorAssertionResponse::new(
            assertion_signature,
            authenticator_data.to_vec(),
            client_data_json,
        );
        let public_key = AnyPublicKey::Secp256r1Ecdsa { public_key };
        let signature = AnySignature::WebAuthn {
            signature: partial_authenticator_assertion_response,
        };
        let authenticator = SingleKeyAuthenticator::new(public_key, signature);
        let account_authenticator = AccountAuthenticator::SingleKey { authenticator };
        let txn_authenticator = TransactionAuthenticator::SingleSender {
            sender: account_authenticator,
        };
        SignedTransaction::new_signed_transaction(raw_txn.clone(), txn_authenticator)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[rstest(
        use_txn_payload_v2_format,
        use_orderless_transactions,
        case(false, false),
        case(true, false),
        case(true, true)
    )]
    async fn test_webauthn_secp256r1_ecdsa(
        use_txn_payload_v2_format: bool,
        use_orderless_transactions: bool,
    ) {
        let mut context = new_test_context_with_orderless_flags(
            current_function_name!(),
            use_txn_payload_v2_format,
            use_orderless_transactions,
        );
        let other = context.create_account().await;

        let mut rng: StdRng = SeedableRng::from_seed([0; 32]);
        let private_key: Secp256r1EcdsaPrivateKey = aptos_crypto::Uniform::generate(&mut rng);
        let public_key = aptos_crypto::PrivateKey::public_key(&private_key);
        let address = AuthenticationKey::any_key(AnyPublicKey::secp256r1_ecdsa(public_key.clone()))
            .account_address();

        // Set a dummy key
        let key_bytes =
            hex::decode("a38ba78b1a0fbfc55e2c5dfdedf48d1172283d0f7c59fd64c02d811130a2f4b2")
                .unwrap();
        let ed25519_private_key: Ed25519PrivateKey = (&key_bytes[..]).try_into().unwrap();
        let mut account = LocalAccount::new(address, ed25519_private_key, 0);

        let txn0 = context.create_user_account(&account).await;
        context.commit_block(&vec![txn0]).await;
        let txn1 = context.mint_user_account(&account).await;
        context.commit_block(&vec![txn1]).await;
        let txn2 = context.create_user_account(&other).await;
        context.commit_block(&vec![txn2]).await;

        let ed22519_txn = context.account_transfer(&mut account, &other, 5);
        let raw_txn = ed22519_txn.into_raw_transaction();
        let collected_client_data = get_collected_client_data(&raw_txn);
        let signed_txn = sign_webauthn_transaction(
            &raw_txn,
            collected_client_data,
            AUTHENTICATOR_DATA,
            &private_key,
        );

        // verifies transaction authenticator
        let webauthn_secp256r1_ecdsa_txn = signed_txn.check_signature().unwrap();
        let balance_start = context.get_apt_balance(other.address()).await;
        let bcs_txn = bcs::to_bytes(&webauthn_secp256r1_ecdsa_txn.into_inner()).unwrap();

        context
            .expect_status_code(202)
            .post_bcs_txn("/transactions", bcs_txn)
            .await;
        context.commit_mempool_txns(1).await;
        assert_eq!(
            balance_start + 5,
            context.get_apt_balance(other.address()).await
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[rstest(
        use_txn_payload_v2_format,
        use_orderless_transactions,
        case(false, false),
        case(true, false),
        case(true, true)
    )]
    async fn test_webauthn_secp256r1_ecdsa_failure(
        use_txn_payload_v2_format: bool,
        use_orderless_transactions: bool,
    ) {
        let mut context = new_test_context_with_orderless_flags(
            current_function_name!(),
            use_txn_payload_v2_format,
            use_orderless_transactions,
        );
        let other = context.create_account().await;

        let mut rng: StdRng = SeedableRng::from_seed([0; 32]);
        let private_key: Secp256r1EcdsaPrivateKey = aptos_crypto::Uniform::generate(&mut rng);
        let public_key = aptos_crypto::PrivateKey::public_key(&private_key);
        let address = AuthenticationKey::any_key(AnyPublicKey::secp256r1_ecdsa(public_key.clone()))
            .account_address();

        // Set a dummy key
        let key_bytes =
            hex::decode("a38ba78b1a0fbfc55e2c5dfdedf48d1172283d0f7c59fd64c02d811130a2f4b2")
                .unwrap();
        let ed25519_private_key: Ed25519PrivateKey = (&key_bytes[..]).try_into().unwrap();
        let mut account = LocalAccount::new(address, ed25519_private_key, 0);

        let txn0 = context.create_user_account(&account).await;
        context.commit_block(&vec![txn0]).await;
        let txn1 = context.mint_user_account(&account).await;
        context.commit_block(&vec![txn1]).await;
        let txn2 = context.create_user_account(&other).await;
        context.commit_block(&vec![txn2]).await;

        let ed22519_txn = context.account_transfer(&mut account, &other, 5);
        let raw_txn = ed22519_txn.into_raw_transaction();

        let wrong_ed25519_txn = context.account_transfer(&mut account, &other, 0);
        let wrong_raw_txn = wrong_ed25519_txn.into_raw_transaction();

        let collected_client_data = get_collected_client_data(&wrong_raw_txn);
        let signed_txn = sign_webauthn_transaction(
            &raw_txn,
            collected_client_data,
            AUTHENTICATOR_DATA,
            &private_key,
        );

        let bcs_txn = bcs::to_bytes(&signed_txn).unwrap();

        // Should fail since the bcs_txn is using collected_client_data with the wrong_raw_txn
        context
            .expect_status_code(400)
            .post_bcs_txn("/transactions", bcs_txn)
            .await;
    }
}

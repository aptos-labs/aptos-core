// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Signed, valid encrypted transactions and state mutations for admission tests.

use crate::{
    account_address::AccountAddress,
    chain_id::ChainId,
    secret_sharing::{EncryptionKey, EvalProof},
    transaction::{
        authenticator::{AccountAuthenticator, AuthenticationKey},
        encrypted_payload::{
            DecryptedPlaintext, DecryptionFailureReason, EncryptedInner, EncryptedPayload,
            PayloadAssociatedData,
        },
        EntryFunction, RawTransaction, RawTransactionWithData, SignedTransaction,
        TransactionExecutable, TransactionExtraConfig, TransactionPayload,
    },
};
use aptos_batch_encryption::{
    schemes::fptx_weighted::FPTXWeighted, traits::BatchThresholdEncryption,
};
use aptos_crypto::{
    ed25519::Ed25519PrivateKey, hash::CryptoHash, weighted_config::WeightedConfigArkworks,
    PrivateKey, SigningKey, Uniform,
};
use ark_std::rand::{rngs::StdRng, SeedableRng};
use move_core_types::{ident_str, language_storage::ModuleId};
use once_cell::sync::Lazy;

pub fn encrypted_transaction(sequence_number: u64, fee_payer: bool) -> SignedTransaction {
    encrypted_transaction_with_gas(sequence_number, fee_payer, 100)
}

pub fn encrypted_transaction_with_gas(
    sequence_number: u64,
    fee_payer: bool,
    gas_unit_price: u64,
) -> SignedTransaction {
    static KEY: Lazy<EncryptionKey> = Lazy::new(|| {
        let config = WeightedConfigArkworks::new(1, vec![1]).unwrap();
        FPTXWeighted::setup_for_testing(7, 8, 1, &config).unwrap().0
    });
    let sender_key = Ed25519PrivateKey::generate_for_testing();
    let sender_pubkey = sender_key.public_key();
    let sender_auth_key = AuthenticationKey::ed25519(&sender_pubkey);
    let sender = sender_auth_key.account_address();
    let payer_key = Ed25519PrivateKey::generate(&mut rand::thread_rng());
    let payer_pubkey = payer_key.public_key();
    let payer_auth_key = AuthenticationKey::ed25519(&payer_pubkey);
    let payer = payer_auth_key.account_address();
    let mut auth_keys = vec![(sender, sender_auth_key)];
    if fee_payer {
        auth_keys.push((payer, payer_auth_key));
    }
    let plaintext = transfer_plaintext(1);
    let ciphertext = FPTXWeighted::encrypt(
        &KEY,
        &mut StdRng::from_entropy(),
        &plaintext,
        &PayloadAssociatedData::new(sender, auth_keys),
    )
    .unwrap();
    let raw = RawTransaction::new(
        sender,
        sequence_number,
        TransactionPayload::EncryptedPayload(EncryptedPayload::Encrypted(EncryptedInner {
            ciphertext,
            extra_config: TransactionExtraConfig::V1 {
                multisig_address: None,
                replay_protection_nonce: None,
            },
            payload_hash: plaintext.hash(),
            encryption_epoch: 1,
            claimed_entry_fun: None,
        })),
        1_000_000,
        gas_unit_price,
        u64::MAX,
        ChainId::test(),
    );
    if fee_payer {
        let message = RawTransactionWithData::new_fee_payer(raw.clone(), vec![], payer);
        SignedTransaction::new_fee_payer(
            raw,
            AccountAuthenticator::ed25519(sender_pubkey, sender_key.sign(&message).unwrap()),
            vec![],
            vec![],
            payer,
            AccountAuthenticator::ed25519(payer_pubkey, payer_key.sign(&message).unwrap()),
        )
    } else {
        let signature = sender_key.sign(&raw).unwrap();
        SignedTransaction::new(raw, sender_pubkey, signature)
    }
}

fn transfer_plaintext(amount: u64) -> DecryptedPlaintext {
    DecryptedPlaintext::new(
        TransactionExecutable::EntryFunction(EntryFunction::new(
            ModuleId::new(AccountAddress::ONE, ident_str!("aptos_account").to_owned()),
            ident_str!("transfer").to_owned(),
            vec![],
            vec![
                bcs::to_bytes(&AccountAddress::TWO).unwrap(),
                bcs::to_bytes(&amount).unwrap(),
            ],
        )),
        [0; 16],
    )
}

/// Mutate a transaction after signing, keeping the ciphertext and signatures.
/// Return FailedDecryption and two Decrypted cases: the original plaintext and
/// an altered transfer the fee payer never authorized.
pub fn non_valid_encrypted_transactions() -> Vec<SignedTransaction> {
    let plaintext = transfer_plaintext(1);
    let modified_plaintext = transfer_plaintext(1_000_000);
    let signed_payload_hash = plaintext.hash();
    assert_ne!(signed_payload_hash, modified_plaintext.hash());

    [None, Some(plaintext), Some(modified_plaintext)]
        .into_iter()
        .map(|plaintext| {
            let mut txn = encrypted_transaction(0, plaintext.is_some());
            let TransactionPayload::EncryptedPayload(EncryptedPayload::Encrypted(original)) =
                txn.payload().clone()
            else {
                unreachable!()
            };
            assert_eq!(original.payload_hash, signed_payload_hash);
            *txn.payload_mut() = TransactionPayload::EncryptedPayload(match plaintext {
                Some(decrypted) => EncryptedPayload::Decrypted {
                    original,
                    eval_proof: EvalProof::random(),
                    decrypted,
                },
                None => EncryptedPayload::FailedDecryption {
                    original,
                    eval_proof: None,
                    reason: DecryptionFailureReason::CryptoFailure,
                },
            });
            txn.verify_signature().unwrap();
            let payload = txn.payload().as_encrypted_payload().unwrap();
            payload
                .verify(
                    txn.sender(),
                    txn.authenticator()
                        .all_signer_auth_keys(txn.sender())
                        .unwrap(),
                )
                .unwrap();
            txn
        })
        .collect()
}

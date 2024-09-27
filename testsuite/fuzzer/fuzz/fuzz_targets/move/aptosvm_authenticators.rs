#![no_main]

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    PrivateKey, SigningKey, Uniform,
};
use aptos_language_e2e_tests::{
    account::Account, data_store::GENESIS_CHANGE_SET_HEAD, executor::FakeExecutor,
};
use aptos_types::{
    chain_id::ChainId,
    keyless::{
        EphemeralCertificate, IdCommitment, KeylessPublicKey, KeylessSignature, OpenIdSig, Pepper,
        TransactionAndProof,
    },
    transaction::{
        authenticator::{
            AccountAuthenticator, AnyPublicKey, AnySignature, EphemeralPublicKey,
            EphemeralSignature, SingleKeyAuthenticator, TransactionAuthenticator,
        },
        ExecutionStatus, SignedTransaction, TransactionStatus,
    },
    write_set::WriteSet,
};
use aptos_vm::AptosVM;
use libfuzzer_sys::{fuzz_target, Corpus};
use move_core_types::vm_status::{StatusCode, StatusType};
use once_cell::sync::Lazy;
use std::sync::Arc;
mod utils;
use utils::{check_for_invariant_violation, FuzzerTransactionAuthenticator, TransactionState};

// genesis write set generated once for each fuzzing session
static VM: Lazy<WriteSet> = Lazy::new(|| GENESIS_CHANGE_SET_HEAD.write_set().clone());

const FUZZER_CONCURRENCY_LEVEL: usize = 1;
static TP: Lazy<Arc<rayon::ThreadPool>> = Lazy::new(|| {
    Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(FUZZER_CONCURRENCY_LEVEL)
            .build()
            .unwrap(),
    )
});

fn run_case(input: TransactionState) -> Result<(), Corpus> {
    tdbg!(&input);

    AptosVM::set_concurrency_level_once(FUZZER_CONCURRENCY_LEVEL);
    let mut vm = FakeExecutor::from_genesis_with_existing_thread_pool(
        &VM,
        ChainId::mainnet(),
        Arc::clone(&TP),
    )
    .set_not_parallel();

    let sender_acc = if true {
        // create sender pub/priv key. initialize and fund account
        vm.create_accounts(1, input.tx_auth_type.sender().fund_amount(), 0)
            .remove(0)
    } else {
        // only create sender pub/priv key. do not initialize
        Account::new()
    };

    let receiver = Account::new();

    // build tx
    let tx = sender_acc
        .transaction()
        .payload(aptos_stdlib::aptos_coin_transfer(*receiver.address(), 1))
        .sequence_number(0)
        .gas_unit_price(100)
        .max_gas_amount(1000);

    let tx_auth_type = input.tx_auth_type.clone();

    let raw_tx = tx.raw();
    let tx = match tx_auth_type {
        FuzzerTransactionAuthenticator::Ed25519 { sender: _ } => raw_tx
            .sign(&sender_acc.privkey, sender_acc.pubkey.as_ed25519().unwrap())
            .map_err(|_| Corpus::Keep)?
            .into_inner(),
        FuzzerTransactionAuthenticator::Keyless {
            sender: _,
            exp_date_secs: expiration_timestamp_secs,
            jwt_header: _,
            cert: cert_c,
        } => {
            // Generate a keypair for ephemeral keys
            let private_key = Ed25519PrivateKey::generate_for_testing();
            let public_key: Ed25519PublicKey = private_key.public_key();

            // Create a TransactionAndProof to be signed
            let txn_and_proof = TransactionAndProof {
                message: raw_tx.clone(),
                proof: None,
            };

            // Sign the transaction
            let signature = private_key.sign(&txn_and_proof).map_err(|_| Corpus::Keep)?;

            // Build AnyPublicKey::Keyless
            let any_public_key = AnyPublicKey::Keyless {
                public_key: KeylessPublicKey {
                    iss_val: "test.oidc.provider".to_string(),
                    idc: IdCommitment::new_from_preimage(
                        &Pepper::from_number(0x5678),
                        "aud",
                        "uid_key",
                        "uid_val",
                    )
                    .map_err(|_| Corpus::Keep)?,
                },
            };

            /*
            EphemeralCertificate::OpenIdSig(OpenIdSig {
                        jwt_sig: vec![],
                        jwt_payload_json: "jwt_payload_json".to_string(),
                        uid_key: "uid_key".to_string(),
                        epk_blinder: b"epk_blinder".to_vec(),
                        pepper: Pepper::from_number(0x1234),
                        idc_aud_val: None,
                    })
            */

            // Build AnySignature::Keyless
            let any_signature = AnySignature::Keyless {
                signature: KeylessSignature {
                    cert: cert_c,
                    jwt_header_json: input.tx_auth_type.get_jwt_header_json().unwrap(),
                    exp_date_secs: expiration_timestamp_secs,
                    ephemeral_pubkey: EphemeralPublicKey::ed25519(public_key),
                    ephemeral_signature: EphemeralSignature::ed25519(signature),
                },
            };

            // Build an authenticator
            let authenticator = TransactionAuthenticator::SingleSender {
                sender: AccountAuthenticator::SingleKey {
                    authenticator: SingleKeyAuthenticator::new(any_public_key, any_signature),
                },
            };

            // Construct the SignedTransaction
            SignedTransaction::new_signed_transaction(raw_tx, authenticator)
        },
        FuzzerTransactionAuthenticator::MultiAgent {
            sender: _,
            secondary_signers,
        } => {
            // higher number here slows down fuzzer significatly due to slow signing process.
            if secondary_signers.len() > 10 {
                return Err(Corpus::Keep);
            }
            let secondary_accs: Vec<_> = secondary_signers
                .iter()
                .map(|acc| acc.convert_account(&mut vm))
                .collect();
            let secondary_signers = secondary_accs.iter().map(|acc| *acc.address()).collect();
            let secondary_private_keys = secondary_accs.iter().map(|acc| &acc.privkey).collect();
            raw_tx
                .sign_multi_agent(
                    &sender_acc.privkey,
                    secondary_signers,
                    secondary_private_keys,
                )
                .map_err(|_| Corpus::Keep)?
                .into_inner()
        },
        FuzzerTransactionAuthenticator::FeePayer {
            sender: _,
            secondary_signers,
            fee_payer,
        } => {
            // higher number here slows down fuzzer significatly due to slow signing process.
            if secondary_signers.len() > 10 {
                return Err(Corpus::Keep);
            }
            let secondary_accs: Vec<_> = secondary_signers
                .iter()
                .map(|acc| acc.convert_account(&mut vm))
                .collect();

            let secondary_signers = secondary_accs.iter().map(|acc| *acc.address()).collect();
            let secondary_private_keys = secondary_accs.iter().map(|acc| &acc.privkey).collect();
            let fee_payer_acc = fee_payer.convert_account(&mut vm);
            raw_tx
                .sign_fee_payer(
                    &sender_acc.privkey,
                    secondary_signers,
                    secondary_private_keys,
                    *fee_payer_acc.address(),
                    &fee_payer_acc.privkey,
                )
                .map_err(|_| Corpus::Keep)?
                .into_inner()
        },
    };

    // exec tx
    tdbg!("exec start");

    let res = vm.execute_block(vec![tx.clone()]);

    let res = res
        .map_err(|e| {
            check_for_invariant_violation(e);
            Corpus::Keep
        })?
        .pop()
        .expect("expect 1 output");
    tdbg!("exec end");

    // if error exit gracefully
    let status = match tdbg!(res.status()) {
        TransactionStatus::Keep(status) => status,
        TransactionStatus::Discard(e) => {
            if e.status_type() == StatusType::InvariantViolation {
                panic!("invariant violation {:?}", e);
            }
            return Err(Corpus::Keep);
        },
        _ => return Err(Corpus::Keep),
    };
    match tdbg!(status) {
        ExecutionStatus::Success => (),
        ExecutionStatus::MiscellaneousError(e) => {
            if let Some(e) = e {
                if e.status_type() == StatusType::InvariantViolation
                    && *e != StatusCode::TYPE_RESOLUTION_FAILURE
                    && *e != StatusCode::STORAGE_ERROR
                {
                    panic!("invariant violation {:?}", e);
                }
            }
            return Err(Corpus::Keep);
        },
        _ => return Err(Corpus::Keep),
    };

    Ok(())
}

fuzz_target!(|fuzz_data: TransactionState| -> Corpus {
    run_case(fuzz_data).err().unwrap_or(Corpus::Keep)
});

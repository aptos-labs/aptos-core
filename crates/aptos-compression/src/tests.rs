// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{CompressedData, CompressionClient, Error};
use aptos_crypto::{ed25519::Ed25519PrivateKey, hash::HashValue, PrivateKey, SigningKey, Uniform};
use aptos_types::{
    account_address::AccountAddress,
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    chain_id::ChainId,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    transaction::{
        ExecutionStatus, RawTransaction, Script, SignedTransaction, Transaction,
        TransactionListWithProof, TransactionOutput, TransactionOutputListWithProof,
        TransactionPayload, TransactionStatus,
    },
    write_set::WriteSet,
};
use rand::Rng;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

// Useful test constants
const MAX_COMPRESSION_SIZE: usize = 64 * 1024 * 1024; // 64 MiBi
const MIB: usize = 1024 * 1024;

#[test]
fn test_basic_compression() {
    // Test both regular and variable compression
    for variable_compression in [true, false] {
        // Test compress random bytes
        let raw_bytes: Vec<_> = (0..MIB).map(|_| rand::thread_rng().gen::<u8>()).collect();
        test_compress_and_decompress(raw_bytes, variable_compression);

        // Test compress epoch ending ledger infos
        let epoch_ending_ledger_infos = create_epoch_ending_ledger_infos(0, 999);
        test_compress_and_decompress(epoch_ending_ledger_infos, variable_compression);

        // Test compress transaction outputs with proof
        let outputs_with_proof = create_output_list_with_proof(13434, 17000, 19000);
        test_compress_and_decompress(outputs_with_proof, variable_compression);

        // Test compress transactions with proof
        let transactions_with_proof = create_transaction_list_with_proof(1000, 1999, 1999, true);
        test_compress_and_decompress(transactions_with_proof, variable_compression);
    }
}

#[test]
fn test_compression_limits() {
    // Create test data
    let too_small_bytes = 10;
    let transactions_with_proof = create_transaction_list_with_proof(1000, 1999, 1999, true);

    // Test both regular and variable compression
    for variable_compression in [true, false] {
        // Test the compression limit
        let maybe_compressed_bytes = test_compress(
            transactions_with_proof.clone(),
            variable_compression,
            too_small_bytes,
        );
        assert!(maybe_compressed_bytes.is_err());

        // Test the decompression limit
        let compressed_bytes = test_compress(
            transactions_with_proof.clone(),
            variable_compression,
            MAX_COMPRESSION_SIZE,
        )
        .unwrap();
        let maybe_decompressed_bytes = crate::decompress(
            &compressed_bytes,
            CompressionClient::StateSync,
            too_small_bytes,
        );
        assert!(maybe_decompressed_bytes.is_err());
    }
}

/// Ensures that the given object can be compressed and
/// decompressed successfully when BCS encoded.
fn test_compress_and_decompress<T: Clone + Debug + DeserializeOwned + PartialEq + Serialize>(
    object: T,
    use_variable_compression: bool,
) {
    // Compress the object
    let compressed_bytes = test_compress(
        object.clone(),
        use_variable_compression,
        MAX_COMPRESSION_SIZE,
    )
    .unwrap();

    // Decompress the bytes and then BCS decode the object
    let decompressed_bytes = crate::decompress(
        &compressed_bytes,
        CompressionClient::StateSync,
        MAX_COMPRESSION_SIZE,
    )
    .unwrap();
    let decoded_object = bcs::from_bytes::<T>(&decompressed_bytes).unwrap();

    // Verify that the objects are equal
    assert_eq!(object, decoded_object);
}

/// Compresses the given object depending on if variable compression is enabled
fn test_compress<T: Debug + DeserializeOwned + PartialEq + Serialize>(
    object: T,
    use_variable_compression: bool,
    max_bytes: usize,
) -> Result<CompressedData, Error> {
    // BCS encode the object
    let bcs_encoded_bytes = bcs::to_bytes(&object).unwrap();

    // Compress the bytes
    if use_variable_compression {
        crate::compress_with_variable_compression(
            bcs_encoded_bytes,
            CompressionClient::StateSync,
            max_bytes,
        )
    } else {
        crate::compress(bcs_encoded_bytes, CompressionClient::StateSync, max_bytes)
    }
}

/// Creates a test epoch change proof
fn create_epoch_ending_ledger_infos(
    start_epoch: u64,
    end_epoch: u64,
) -> Vec<LedgerInfoWithSignatures> {
    let mut ledger_info_with_sigs = vec![];
    for epoch in start_epoch..end_epoch {
        ledger_info_with_sigs.push(create_test_ledger_info_with_sigs(epoch, 0));
    }
    ledger_info_with_sigs
}

/// Creates a test transaction output list with proof
fn create_output_list_with_proof(
    start_version: u64,
    end_version: u64,
    proof_version: u64,
) -> TransactionOutputListWithProof {
    let transaction_list_with_proof =
        create_transaction_list_with_proof(start_version, end_version, proof_version, false);
    let transactions_and_outputs = transaction_list_with_proof
        .transactions
        .iter()
        .map(|txn| (txn.clone(), create_test_transaction_output()))
        .collect();

    TransactionOutputListWithProof::new(
        transactions_and_outputs,
        Some(start_version),
        transaction_list_with_proof.proof,
    )
}

/// Creates a test ledger info with signatures
fn create_test_ledger_info_with_sigs(epoch: u64, version: u64) -> LedgerInfoWithSignatures {
    // Create a mock ledger info with signatures
    let ledger_info = LedgerInfo::new(
        BlockInfo::new(
            epoch,
            0,
            HashValue::zero(),
            HashValue::zero(),
            version,
            0,
            None,
        ),
        HashValue::zero(),
    );
    LedgerInfoWithSignatures::new(ledger_info, AggregateSignature::empty())
}

/// Creates a test transaction output
fn create_test_transaction_output() -> TransactionOutput {
    TransactionOutput::new(
        WriteSet::default(),
        vec![],
        0,
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(None)),
    )
}

/// Creates a test user transaction
fn create_test_transaction(sequence_number: u64) -> Transaction {
    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = private_key.public_key();

    let transaction_payload = TransactionPayload::Script(Script::new(vec![], vec![], vec![]));
    let raw_transaction = RawTransaction::new(
        AccountAddress::random(),
        sequence_number,
        transaction_payload,
        0,
        0,
        0,
        ChainId::new(10),
    );
    let signed_transaction = SignedTransaction::new(
        raw_transaction.clone(),
        public_key,
        private_key.sign(&raw_transaction).unwrap(),
    );

    Transaction::UserTransaction(signed_transaction)
}

/// Creates a test transaction output list with proof
fn create_transaction_list_with_proof(
    start_version: u64,
    end_version: u64,
    _proof_version: u64,
    include_events: bool,
) -> TransactionListWithProof {
    // Include events if required
    let events = if include_events { Some(vec![]) } else { None };

    // Create the requested transactions
    let mut transactions = vec![];
    for sequence_number in start_version..=end_version {
        transactions.push(create_test_transaction(sequence_number));
    }

    // Create a transaction list with an empty proof
    let mut transaction_list_with_proof = TransactionListWithProof::new_empty();
    transaction_list_with_proof.first_transaction_version = Some(start_version);
    transaction_list_with_proof.events = events;
    transaction_list_with_proof.transactions = transactions;

    transaction_list_with_proof
}

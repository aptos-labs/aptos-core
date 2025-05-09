// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_address::AccountAddress,
    block_info::BlockInfo,
    block_metadata::BlockMetadata,
    contract_event::ContractEvent,
    event::EventKey,
    ledger_info::LedgerInfo,
    proof::{
        definition::MAX_ACCUMULATOR_PROOF_DEPTH, AccumulatorExtensionProof, AccumulatorRangeProof,
        SparseMerkleInternalNode, SparseMerkleLeafNode, TestAccumulatorInternalNode,
        TestAccumulatorProof, TransactionAccumulatorInternalNode, TransactionAccumulatorProof,
        TransactionInfoListWithProof, TransactionInfoWithProof,
    },
    state_store::state_value::StateValue,
    transaction::{
        ExecutionStatus, Transaction, TransactionAuxiliaryData, TransactionInfo,
        TransactionListWithProof, TransactionOutput, TransactionOutputListWithProof,
        TransactionStatus,
    },
    write_set::WriteSet,
};
use aptos_crypto::{
    hash::{
        CryptoHash, TestOnlyHash, TestOnlyHasher, ACCUMULATOR_PLACEHOLDER_HASH, GENESIS_BLOCK_ID,
        SPARSE_MERKLE_PLACEHOLDER_HASH,
    },
    HashValue,
};
use move_core_types::language_storage::TypeTag;

type SparseMerkleProof = crate::proof::SparseMerkleProof;

#[test]
fn test_verify_empty_accumulator() {
    let element_hash = b"hello".test_only_hash();
    let root_hash = *ACCUMULATOR_PLACEHOLDER_HASH;
    let proof = TestAccumulatorProof::new(vec![]);
    assert!(proof.verify(root_hash, element_hash, 0).is_err());
}

#[test]
fn test_verify_single_element_accumulator() {
    let element_hash = b"hello".test_only_hash();
    let root_hash = element_hash;
    let proof = TestAccumulatorProof::new(vec![]);
    assert!(proof.verify(root_hash, element_hash, 0).is_ok());
}

#[test]
fn test_verify_two_element_accumulator() {
    let element0_hash = b"hello".test_only_hash();
    let element1_hash = b"world".test_only_hash();
    let root_hash = TestAccumulatorInternalNode::new(element0_hash, element1_hash).hash();

    assert!(TestAccumulatorProof::new(vec![element1_hash])
        .verify(root_hash, element0_hash, 0)
        .is_ok());
    assert!(TestAccumulatorProof::new(vec![element0_hash])
        .verify(root_hash, element1_hash, 1)
        .is_ok());
}

#[test]
fn test_verify_three_element_accumulator() {
    let element0_hash = b"hello".test_only_hash();
    let element1_hash = b"world".test_only_hash();
    let element2_hash = b"!".test_only_hash();
    let internal0_hash = TestAccumulatorInternalNode::new(element0_hash, element1_hash).hash();
    let internal1_hash =
        TestAccumulatorInternalNode::new(element2_hash, *ACCUMULATOR_PLACEHOLDER_HASH).hash();
    let root_hash = TestAccumulatorInternalNode::new(internal0_hash, internal1_hash).hash();

    assert!(
        TestAccumulatorProof::new(vec![element1_hash, internal1_hash])
            .verify(root_hash, element0_hash, 0)
            .is_ok()
    );
    assert!(
        TestAccumulatorProof::new(vec![element0_hash, internal1_hash])
            .verify(root_hash, element1_hash, 1)
            .is_ok()
    );
    assert!(
        TestAccumulatorProof::new(vec![*ACCUMULATOR_PLACEHOLDER_HASH, internal0_hash])
            .verify(root_hash, element2_hash, 2)
            .is_ok()
    );
}

#[test]
fn test_accumulator_proof_max_siblings_leftmost() {
    let element_hash = b"hello".test_only_hash();
    let mut siblings = vec![];
    for i in 0..MAX_ACCUMULATOR_PROOF_DEPTH as u8 {
        siblings.push(HashValue::new([i; 32]));
    }
    let root_hash = siblings.iter().fold(element_hash, |hash, sibling_hash| {
        TestAccumulatorInternalNode::new(hash, *sibling_hash).hash()
    });
    let proof = TestAccumulatorProof::new(siblings);

    assert!(proof.verify(root_hash, element_hash, 0).is_ok());
}

#[test]
fn test_accumulator_proof_max_siblings_rightmost() {
    let element_hash = b"hello".test_only_hash();
    let mut siblings = vec![];
    for i in 0..MAX_ACCUMULATOR_PROOF_DEPTH as u8 {
        siblings.push(HashValue::new([i; 32]));
    }
    let root_hash = siblings.iter().fold(element_hash, |hash, sibling_hash| {
        TestAccumulatorInternalNode::new(*sibling_hash, hash).hash()
    });
    let leaf_index = (std::u64::MAX - 1) / 2;
    let proof = TestAccumulatorProof::new(siblings);

    assert!(proof.verify(root_hash, element_hash, leaf_index).is_ok());
}

#[test]
#[allow(clippy::range_plus_one)]
fn test_accumulator_proof_sibling_overflow() {
    let element_hash = b"hello".test_only_hash();
    let mut siblings = vec![];
    for i in 0..MAX_ACCUMULATOR_PROOF_DEPTH as u8 + 1 {
        siblings.push(HashValue::new([i; 32]));
    }
    let root_hash = siblings
        .iter()
        .rev()
        .fold(element_hash, |hash, sibling_hash| {
            TestAccumulatorInternalNode::new(hash, *sibling_hash).hash()
        });
    let proof = TestAccumulatorProof::new(siblings);

    assert!(proof.verify(root_hash, element_hash, 0).is_err());
}

#[test]
fn test_verify_empty_sparse_merkle() {
    let key = b"hello".test_only_hash();
    let blob = b"world".to_vec().into();
    let root_hash = *SPARSE_MERKLE_PLACEHOLDER_HASH;
    let proof = SparseMerkleProof::new(None, vec![]);

    // Trying to show that this key doesn't exist.
    assert!(proof.verify::<StateValue>(root_hash, key, None).is_ok());
    // Trying to show that this key exists.
    assert!(proof
        .verify::<StateValue>(root_hash, key, Some(&blob))
        .is_err());
}

#[test]
fn test_verify_single_element_sparse_merkle() {
    let key = b"hello".test_only_hash();
    let blob: StateValue = b"world".to_vec().into();
    let blob_hash = blob.hash();
    let non_existing_blob: StateValue = b"world?".to_vec().into();
    let root_node = SparseMerkleLeafNode::new(key, blob_hash);
    let root_hash = root_node.hash();
    let proof = SparseMerkleProof::new(Some(root_node), vec![]);

    // Trying to show this exact key exists with its value.
    assert!(proof
        .verify::<StateValue>(root_hash, key, Some(&blob))
        .is_ok());
    // Trying to show this exact key exists with another value.
    assert!(proof
        .verify::<StateValue>(root_hash, key, Some(&non_existing_blob))
        .is_err());
    // Trying to show this key doesn't exist.
    assert!(proof.verify::<StateValue>(root_hash, key, None).is_err());

    let non_existing_key = b"HELLO".test_only_hash();

    // The proof can be used to show non_existing_key doesn't exist.
    assert!(proof
        .verify::<StateValue>(root_hash, non_existing_key, None)
        .is_ok());
    // The proof can't be used to non_existing_key exists.
    assert!(proof
        .verify::<StateValue>(root_hash, non_existing_key, Some(&blob))
        .is_err());
}

#[test]
fn test_verify_three_element_sparse_merkle() {
    //            root
    //           /    \
    //          a      default
    //         / \
    //     key1   b
    //           / \
    //       key2   key3
    let key1 = b"hello".test_only_hash();
    let key2 = b"world".test_only_hash();
    let key3 = b"!".test_only_hash();
    assert_eq!(key1[0], 0b0011_0011);
    assert_eq!(key2[0], 0b0100_0010);
    assert_eq!(key3[0], 0b0110_1001);

    let blob1 = StateValue::from(b"1".to_vec());
    let blob2 = StateValue::from(b"2".to_vec());
    let blob3 = StateValue::from(b"3".to_vec());

    let leaf1 = SparseMerkleLeafNode::new(key1, blob1.hash());
    let leaf1_hash = leaf1.hash();
    let leaf2_hash = SparseMerkleLeafNode::new(key2, blob2.hash()).hash();
    let leaf3_hash = SparseMerkleLeafNode::new(key3, blob3.hash()).hash();
    let internal_b_hash = SparseMerkleInternalNode::new(leaf2_hash, leaf3_hash).hash();
    let internal_a_hash = SparseMerkleInternalNode::new(leaf1_hash, internal_b_hash).hash();
    let root_hash =
        SparseMerkleInternalNode::new(internal_a_hash, *SPARSE_MERKLE_PLACEHOLDER_HASH).hash();

    let non_existing_key1 = b"abc".test_only_hash();
    let non_existing_key2 = b"def".test_only_hash();
    assert_eq!(non_existing_key1[0], 0b0011_1010);
    assert_eq!(non_existing_key2[0], 0b1000_1110);

    {
        // Construct a proof of key1.
        let proof = SparseMerkleProof::new(Some(leaf1), vec![
            *SPARSE_MERKLE_PLACEHOLDER_HASH,
            internal_b_hash,
        ]);

        // The exact key value exists.
        assert!(proof.verify(root_hash, key1, Some(&blob1)).is_ok());
        // Trying to show that this key has another value.
        assert!(proof.verify(root_hash, key1, Some(&blob2)).is_err());
        // Trying to show that this key doesn't exist.
        assert!(proof.verify::<StateValue>(root_hash, key1, None).is_err());
        // This proof can't be used to show anything about key2.
        assert!(proof.verify::<StateValue>(root_hash, key2, None).is_err());
        assert!(proof.verify(root_hash, key2, Some(&blob1)).is_err());
        assert!(proof.verify(root_hash, key2, Some(&blob2)).is_err());

        // This proof can be used to show that non_existing_key1 indeed doesn't exist.
        assert!(proof
            .verify::<StateValue>(root_hash, non_existing_key1, None)
            .is_ok());
        // This proof can't be used to show that non_existing_key2 doesn't exist because it lives
        // in a different subtree.
        assert!(proof
            .verify::<StateValue>(root_hash, non_existing_key2, None)
            .is_err());
    }

    {
        // Construct a proof of the default node.
        let proof = SparseMerkleProof::new(None, vec![internal_a_hash]);

        // This proof can't be used to show that a key starting with 0 doesn't exist.
        assert!(proof
            .verify::<StateValue>(root_hash, non_existing_key1, None)
            .is_err());
        // This proof can be used to show that a key starting with 1 doesn't exist.
        assert!(proof
            .verify::<StateValue>(root_hash, non_existing_key2, None)
            .is_ok());
    }
}

#[test]
fn test_verify_transaction() {
    //            root
    //           /     \
    //         /         \
    //       a             b
    //      / \           / \
    //  txn0   txn1   txn2   default
    let txn_info0_hash = b"hello".test_only_hash();
    let txn_info2_hash = b"!".test_only_hash();

    let txn1_hash = HashValue::random();
    let state_root1_hash = b"a".test_only_hash();
    let event_root1_hash = b"b".test_only_hash();
    let txn_info1 = TransactionInfo::new(
        txn1_hash,
        HashValue::zero(),
        event_root1_hash,
        Some(state_root1_hash),
        /* gas_used = */ 0,
        /* major_status = */ ExecutionStatus::Success,
    );
    let txn_info1_hash = txn_info1.hash();

    let internal_a_hash =
        TransactionAccumulatorInternalNode::new(txn_info0_hash, txn_info1_hash).hash();
    let internal_b_hash =
        TransactionAccumulatorInternalNode::new(txn_info2_hash, *ACCUMULATOR_PLACEHOLDER_HASH)
            .hash();
    let root_hash =
        TransactionAccumulatorInternalNode::new(internal_a_hash, internal_b_hash).hash();
    let consensus_data_hash = b"c".test_only_hash();
    let ledger_info = LedgerInfo::new(
        BlockInfo::new(0, 0, *GENESIS_BLOCK_ID, root_hash, 2, 10000, None),
        consensus_data_hash,
    );

    let ledger_info_to_transaction_info_proof =
        TransactionAccumulatorProof::new(vec![txn_info0_hash, internal_b_hash]);
    let proof =
        TransactionInfoWithProof::new(ledger_info_to_transaction_info_proof.clone(), txn_info1);

    // The proof can be used to verify txn1.
    assert!(proof.verify(&ledger_info, 1).is_ok());
    // Trying to show that txn1 is at version 2.
    assert!(proof.verify(&ledger_info, 2).is_err());
    // Replacing txn1 with some other txn should cause the verification to fail.
    let fake_txn_info = TransactionInfo::new(
        HashValue::random(),
        HashValue::zero(),
        event_root1_hash,
        Some(state_root1_hash),
        /* gas_used = */ 0,
        /* major_status = */ ExecutionStatus::Success,
    );
    let proof = TransactionInfoWithProof::new(ledger_info_to_transaction_info_proof, fake_txn_info);
    assert!(proof.verify(&ledger_info, 1).is_err());
}

// This test does the following:
// 1) Test that empty has a well defined definition
// 2) Test a single value
// 3) Test multiple values
// 4) Random nonsense returns an error
#[test]
fn test_accumulator_extension_proof() {
    // Test empty
    let empty = AccumulatorExtensionProof::<TestOnlyHasher>::new(vec![], 0, vec![]);

    let derived_tree = empty.verify(*ACCUMULATOR_PLACEHOLDER_HASH).unwrap();
    assert_eq!(*ACCUMULATOR_PLACEHOLDER_HASH, derived_tree.root_hash());
    assert_eq!(derived_tree.version(), 0);

    // Test a single value
    HashValue::zero();
    let one_tree =
        AccumulatorExtensionProof::<TestOnlyHasher>::new(vec![], 0, vec![HashValue::zero()]);

    let derived_tree = one_tree.verify(*ACCUMULATOR_PLACEHOLDER_HASH).unwrap();
    assert_eq!(HashValue::zero(), derived_tree.root_hash());
    assert_eq!(derived_tree.version(), 0);

    // Test multiple values
    let two_tree =
        AccumulatorExtensionProof::<TestOnlyHasher>::new(vec![HashValue::zero()], 1, vec![
            HashValue::zero(),
        ]);

    let derived_tree = two_tree.verify(HashValue::zero()).unwrap();
    let two_hash = TestAccumulatorInternalNode::new(HashValue::zero(), HashValue::zero()).hash();
    assert_eq!(two_hash, derived_tree.root_hash());
    assert_eq!(derived_tree.version(), 1);

    // Test nonsense breaks
    let derived_tree_err = two_tree.verify(*ACCUMULATOR_PLACEHOLDER_HASH);
    assert!(derived_tree_err.is_err());
}

#[test]
fn test_transaction_info_list_with_proof() {
    // Create transaction info list proof
    let transaction_info_list_proof = create_single_transaction_info_proof(None, None, None);

    // Verify first transaction version must match the proof
    let empty_ledger_info = LedgerInfo::new(BlockInfo::empty(), HashValue::zero());
    transaction_info_list_proof
        .verify(&empty_ledger_info, None)
        .unwrap_err();

    // Verify info hash mismatch (the empty ledger info expected an info hash of zero)
    transaction_info_list_proof
        .verify(&empty_ledger_info, Some(1))
        .unwrap_err();

    // Calculate the expected transaction info hash
    let expected_info_hash = transaction_info_list_proof.transaction_infos[0].hash();

    // Verify correct info hash according to the expected hash in the block
    let block_info = BlockInfo::new(0, 0, HashValue::random(), expected_info_hash, 0, 0, None);
    let ledger_info = LedgerInfo::new(block_info, HashValue::zero());
    transaction_info_list_proof
        .verify(&ledger_info, Some(1))
        .unwrap();
}

#[test]
fn test_transaction_list_with_proof() {
    // Create test event and transaction
    let event = create_event();
    let transactions = vec![Transaction::BlockMetadata(BlockMetadata::new(
        HashValue::random(),
        0,
        0,
        AccountAddress::random(),
        vec![0],
        vec![],
        0,
    ))];

    // Create transaction list with proof
    let transaction_list_with_proof = TransactionListWithProof::new(
        transactions.clone(),
        Some(vec![vec![event.clone()]]),
        Some(1),
        create_single_transaction_info_proof(None, None, None),
    );

    // Verify first transaction version must match the proof
    let empty_ledger_info = LedgerInfo::new(BlockInfo::empty(), HashValue::zero());
    transaction_list_with_proof
        .verify(&empty_ledger_info, None)
        .unwrap_err();

    // Verify mismatch between hash of transaction and hash stored in transaction info
    transaction_list_with_proof
        .verify(&empty_ledger_info, Some(1))
        .unwrap_err();

    // Verify transaction hashes match but info root hash verification fails (ledger info expected zero root hash)
    let transaction_list_proof =
        create_single_transaction_info_proof(Some(transactions[0].hash()), None, None);
    let transaction_list_with_proof = TransactionListWithProof::new(
        transactions.clone(),
        Some(vec![vec![event.clone()]]),
        Some(1),
        transaction_list_proof.clone(),
    );
    transaction_list_with_proof
        .verify(&empty_ledger_info, Some(1))
        .unwrap_err();

    // Verify correct info hash but event verification fails (event hash mismatch)
    let expected_info_hash = transaction_list_proof.transaction_infos[0].hash();
    let block_info = BlockInfo::new(0, 0, HashValue::random(), expected_info_hash, 0, 0, None);
    let ledger_info = LedgerInfo::new(block_info, HashValue::zero());
    transaction_list_with_proof
        .verify(&ledger_info, Some(1))
        .unwrap_err();

    // Construct a new transaction list with proof where the transaction info and event hashes match
    let transaction_list_proof = create_single_transaction_info_proof(
        Some(transactions[0].hash()),
        Some(event.hash()),
        None,
    );
    let transaction_list_with_proof = TransactionListWithProof::new(
        transactions,
        Some(vec![vec![event]]),
        Some(1),
        transaction_list_proof.clone(),
    );

    // Ensure ledger verification now passes
    let expected_info_hash = transaction_list_proof.transaction_infos[0].hash();
    let block_info = BlockInfo::new(0, 0, HashValue::random(), expected_info_hash, 0, 0, None);
    let ledger_info = LedgerInfo::new(block_info, HashValue::zero());
    transaction_list_with_proof
        .verify(&ledger_info, Some(1))
        .unwrap();
}

#[test]
fn test_transaction_and_output_list_with_proof() {
    // Create test transaction, event and transaction output
    let transaction = Transaction::BlockMetadata(BlockMetadata::new(
        HashValue::random(),
        0,
        0,
        AccountAddress::random(),
        vec![0],
        vec![],
        0,
    ));
    let txn_hash = transaction.hash();
    let event = create_event();
    let event_root_hash = event.hash();
    let write_set = WriteSet::default();
    let write_set_hash = CryptoHash::hash(&write_set);
    let transaction_output = TransactionOutput::new(
        write_set,
        vec![event],
        0,
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(None)),
        TransactionAuxiliaryData::default(),
    );

    // Create transaction output list with proof
    let (_root_hash, transaction_output_list_proof) = create_txn_output_list_with_proof(
        &transaction,
        &transaction_output,
        Some(txn_hash),
        Some(event_root_hash),
        Some(write_set_hash),
    );

    // Verify first transaction version must match the proof
    let empty_ledger_info = LedgerInfo::new(BlockInfo::empty(), HashValue::zero());
    transaction_output_list_proof
        .verify(&empty_ledger_info, None)
        .unwrap_err();

    // Verify correct info hash but event verification now fails (event hash mismatch)
    let (root_hash, transaction_output_list_proof) = create_txn_output_list_with_proof(
        &transaction,
        &transaction_output,
        Some(txn_hash),
        None,
        Some(write_set_hash),
    );
    let ledger_info = create_ledger_info_at_version0(root_hash);
    transaction_output_list_proof
        .verify(&ledger_info, Some(1))
        .unwrap_err();

    // Verify failure on state change hash mismatch
    let (root_hash, transaction_output_list_proof) = create_txn_output_list_with_proof(
        &transaction,
        &transaction_output,
        Some(txn_hash),
        Some(event_root_hash),
        None,
    );
    let ledger_info = create_ledger_info_at_version0(root_hash);
    transaction_output_list_proof
        .verify(&ledger_info, Some(1))
        .unwrap_err();

    // Construct a new transaction output list proof where the transaction info and event hashes match
    let (root_hash, transaction_output_list_proof) = create_txn_output_list_with_proof(
        &transaction,
        &transaction_output,
        Some(txn_hash),
        Some(event_root_hash),
        Some(write_set_hash),
    );
    let ledger_info = create_ledger_info_at_version0(root_hash);
    transaction_output_list_proof
        .verify(&ledger_info, Some(1))
        .unwrap();
}

fn create_ledger_info_at_version0(root_hash: HashValue) -> LedgerInfo {
    let block_info = BlockInfo::new(0, 0, HashValue::random(), root_hash, 0, 0, None);
    LedgerInfo::new(block_info, HashValue::zero())
}

fn create_txn_output_list_with_proof(
    transaction: &Transaction,
    transaction_output: &TransactionOutput,
    transaction_hash: Option<HashValue>,
    event_root_hash: Option<HashValue>,
    state_change_hash: Option<HashValue>,
) -> (HashValue, TransactionOutputListWithProof) {
    let transaction_info_list_proof =
        create_single_transaction_info_proof(transaction_hash, event_root_hash, state_change_hash);
    let root_hash = transaction_info_list_proof.transaction_infos[0].hash();
    let transaction_output_list_proof = TransactionOutputListWithProof::new(
        vec![(transaction.clone(), transaction_output.clone())],
        Some(1),
        transaction_info_list_proof,
    );

    (root_hash, transaction_output_list_proof)
}

fn create_single_transaction_info_proof(
    transaction_hash: Option<HashValue>,
    event_root_hash: Option<HashValue>,
    state_change_hash: Option<HashValue>,
) -> TransactionInfoListWithProof {
    let transaction_infos = vec![create_transaction_info(
        transaction_hash,
        event_root_hash,
        state_change_hash,
    )];
    TransactionInfoListWithProof::new(AccumulatorRangeProof::new_empty(), transaction_infos)
}

fn create_transaction_info(
    transaction_hash: Option<HashValue>,
    event_root_hash: Option<HashValue>,
    state_change_hash: Option<HashValue>,
) -> TransactionInfo {
    TransactionInfo::new(
        transaction_hash.unwrap_or_else(HashValue::random),
        state_change_hash.unwrap_or_else(HashValue::random),
        event_root_hash.unwrap_or_else(HashValue::random),
        Some(HashValue::random()),
        0,
        ExecutionStatus::MiscellaneousError(None),
    )
}

fn create_event() -> ContractEvent {
    let event_key = EventKey::new(0, AccountAddress::random());
    ContractEvent::new_v1(event_key, 0, TypeTag::Bool, bcs::to_bytes(&0).unwrap()).unwrap()
}

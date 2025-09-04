// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_shuffler::use_case_aware::{
    iterator::ShuffledTransactionIterator,
    tests,
    tests::{Account, Contract},
    Config,
};
use velor_crypto::{ed25519::Ed25519PrivateKey, PrivateKey, SigningKey, Uniform};
use velor_types::{
    account_address::AccountAddress,
    chain_id::ChainId,
    transaction::{
        EntryFunction, RawTransaction, Script, SignedTransaction, TransactionExecutable,
        TransactionExtraConfig, TransactionPayload, TransactionPayloadInner,
    },
};
use itertools::Itertools;
use move_core_types::{identifier::Identifier, language_storage::ModuleId};
use std::str::FromStr;

const PP: Contract = Contract::Platform;
const OO: Contract = Contract::Others;
const C1: Contract = Contract::User(0xF1);
const C2: Contract = Contract::User(0xF2);
const C3: Contract = Contract::User(0xF3);
const A1: Account = Account(1);
const A2: Account = Account(2);
const A3: Account = Account(3);
const A4: Account = Account(4);

fn assert_shuffle_result(
    config: Config,
    txns: impl IntoIterator<Item = (Contract, Account)>,
    expected_order: impl IntoIterator<Item = usize>,
) {
    let txns = tests::into_txns(txns);
    let actual_order = ShuffledTransactionIterator::new(config)
        .extended_with(txns)
        .map(|txn| txn.original_idx)
        .collect_vec();
    let expected_order = expected_order.into_iter().collect_vec();
    assert_eq!(actual_order, expected_order, "actual != expected");
}

fn three_senders_txns() -> [(Contract, Account); 10] {
    [
        // 5 txns from A1
        (PP, A1),
        (OO, A1),
        (C1, A1),
        (C2, A1),
        (C3, A1),
        // 3 txns from A2
        (PP, A2),
        (PP, A2),
        (PP, A2),
        // 2 txns from A3
        (C1, A3),
        (C1, A3),
    ]
}

#[test]
fn test_no_spreading() {
    let config = Config {
        sender_spread_factor: 0,
        platform_use_case_spread_factor: 0,
        user_use_case_spread_factor: 0,
    };
    let txns = three_senders_txns();

    assert_shuffle_result(config, txns, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[test]
fn test_spread_by_sender_1() {
    let config = Config {
        sender_spread_factor: 1,
        // ignore use case conflicts
        platform_use_case_spread_factor: 0,
        // ignore use case conflicts
        user_use_case_spread_factor: 0,
    };
    let txns = three_senders_txns();

    assert_shuffle_result(config, txns, [0, 5, 1, 6, 2, 7, 3, 8, 4, 9]);
}

#[test]
fn test_spread_by_sender_by_large_factor() {
    for sender_spread_factor in [2, 3, 4] {
        let config = Config {
            sender_spread_factor,
            platform_use_case_spread_factor: 0,
            user_use_case_spread_factor: 0,
        };
        let txns = three_senders_txns();

        assert_shuffle_result(config, txns, [0, 5, 8, 1, 6, 9, 2, 7, 3, 4]);
    }
}

fn three_contracts_txns() -> [(Contract, Account); 10] {
    [
        // 5 txns from C1
        (C1, A1),
        (C1, A1),
        (C1, A1),
        (C1, A1),
        (C1, A1),
        // 3 txns from C2
        (C2, A2),
        (C2, A2),
        (C2, A2),
        // 2 txns from C3
        (C3, A3),
        (C3, A3),
    ]
}

#[test]
fn test_spread_by_use_case_1() {
    let config = Config {
        sender_spread_factor: 0,
        platform_use_case_spread_factor: 0,
        user_use_case_spread_factor: 1,
    };
    let txns = three_contracts_txns();

    assert_shuffle_result(config, txns, [0, 5, 1, 6, 2, 7, 3, 8, 4, 9]);
}

#[test]
fn test_spread_by_use_case_by_large_factor() {
    for user_use_case_spread_factor in [2, 3, 4] {
        let config = Config {
            sender_spread_factor: 0,
            platform_use_case_spread_factor: 0,
            user_use_case_spread_factor,
        };
        let txns = three_contracts_txns();

        assert_shuffle_result(config, txns, [0, 5, 8, 1, 6, 9, 2, 7, 3, 4]);
    }
}

fn user_and_platform_use_cases() -> [(Contract, Account); 10] {
    [
        // 5 txns from C1
        (C1, A1),
        (C1, A1),
        (C1, A1),
        (C1, A1),
        (C1, A1),
        // 3 txns from C2
        (PP, A2),
        (PP, A2),
        (PP, A2),
        // 2 txns from C3
        (PP, A3),
        (PP, A3),
    ]
}

#[test]
fn test_platform_txn_priority_0() {
    let config = Config {
        sender_spread_factor: 0,
        platform_use_case_spread_factor: 0,
        user_use_case_spread_factor: 3,
    };
    let txns = user_and_platform_use_cases();

    assert_shuffle_result(config, txns, [0, 5, 6, 7, 1, 8, 9, 2, 3, 4]);
}

#[test]
fn test_platform_txn_priority_1() {
    let config = Config {
        sender_spread_factor: 0,
        platform_use_case_spread_factor: 1,
        user_use_case_spread_factor: 3,
    };
    let txns = user_and_platform_use_cases();

    assert_shuffle_result(config, txns, [0, 5, 6, 1, 7, 8, 2, 9, 3, 4]);
}

#[test]
fn test_spread_sender_within_use_case() {
    let config = Config {
        sender_spread_factor: 2,
        platform_use_case_spread_factor: 0,
        user_use_case_spread_factor: 1,
    };
    let txns = [
        // 5 txns from C1
        (C1, A1),
        (C1, A1),
        (C1, A2),
        (C1, A2),
        (C1, A2),
        // 3 txns from C2
        (C2, A3),
        (C2, A3),
        (C2, A3),
        (C2, A4),
        (C2, A4),
    ];

    assert_shuffle_result(config, txns, [0, 5, 2, 8, 1, 6, 3, 9, 4, 7]);
}

#[test]
fn test_different_transaction_types() {
    // Create test accounts with private keys
    let sender1_private_key = Ed25519PrivateKey::generate_for_testing();
    let sender1_public_key = sender1_private_key.public_key();
    let sender1 = AccountAddress::from_str("0x1").unwrap();

    let sender2_private_key = Ed25519PrivateKey::generate_for_testing();
    let sender2_public_key = sender2_private_key.public_key();
    let sender2 = AccountAddress::from_str("0x2").unwrap();

    let sender3_private_key = Ed25519PrivateKey::generate_for_testing();
    let sender3_public_key = sender3_private_key.public_key();
    let sender3 = AccountAddress::from_str("0x3").unwrap();

    let sender4_private_key = Ed25519PrivateKey::generate_for_testing();
    let sender4_public_key = sender4_private_key.public_key();
    let sender4 = AccountAddress::from_str("0x4").unwrap();

    // Create different types of transactions
    let mut transactions = Vec::new();

    // Sender 1: Mix of platform and contract transactions
    // Platform entry function
    let platform_entry = EntryFunction::new(
        ModuleId::new(
            AccountAddress::from_str("0x1").unwrap(),
            Identifier::new("test").unwrap(),
        ),
        Identifier::new("platform_function").unwrap(),
        vec![],
        vec![],
    );
    let raw_txn = RawTransaction::new(
        sender1,
        1,
        TransactionPayload::EntryFunction(platform_entry),
        1000,
        0,
        u64::MAX,
        ChainId::test(),
    );
    let signature = sender1_private_key.sign(&raw_txn).unwrap();
    let signed_txn = SignedTransaction::new(raw_txn, sender1_public_key.clone(), signature);
    transactions.push(signed_txn);

    // Contract entry function with Payload
    let contract_entry = EntryFunction::new(
        ModuleId::new(
            AccountAddress::from_str("0x123").unwrap(),
            Identifier::new("test").unwrap(),
        ),
        Identifier::new("contract_function").unwrap(),
        vec![],
        vec![],
    );
    let raw_txn = RawTransaction::new(
        sender1,
        2,
        TransactionPayload::Payload(TransactionPayloadInner::V1 {
            executable: TransactionExecutable::EntryFunction(contract_entry),
            extra_config: TransactionExtraConfig::V1 {
                replay_protection_nonce: Some(2),
                multisig_address: None,
            },
        }),
        1100,
        0,
        u64::MAX,
        ChainId::test(),
    );
    let signature = sender1_private_key.sign(&raw_txn).unwrap();
    let signed_txn = SignedTransaction::new(raw_txn, sender1_public_key.clone(), signature);
    transactions.push(signed_txn);

    // Sender 2: Mix of script and multisig transactions
    // Script transaction
    let script = Script::new(vec![1, 2, 3], vec![], vec![]);
    let raw_txn = RawTransaction::new(
        sender2,
        1,
        TransactionPayload::Script(script),
        2000,
        0,
        u64::MAX,
        ChainId::test(),
    );
    let signature = sender2_private_key.sign(&raw_txn).unwrap();
    let signed_txn = SignedTransaction::new(raw_txn, sender2_public_key.clone(), signature);
    transactions.push(signed_txn);

    // Multisig transaction with Payload
    let multisig_payload = TransactionPayload::Multisig(velor_types::transaction::Multisig {
        multisig_address: AccountAddress::from_str("0x4").unwrap(),
        transaction_payload: Some(
            velor_types::transaction::MultisigTransactionPayload::EntryFunction(
                EntryFunction::new(
                    ModuleId::new(
                        AccountAddress::from_str("0x1").unwrap(),
                        Identifier::new("test").unwrap(),
                    ),
                    Identifier::new("multisig_function").unwrap(),
                    vec![],
                    vec![],
                ),
            ),
        ),
    });
    let raw_txn = RawTransaction::new(
        sender2,
        2,
        multisig_payload,
        2100,
        0,
        u64::MAX,
        ChainId::test(),
    );
    let signature = sender2_private_key.sign(&raw_txn).unwrap();
    let signed_txn = SignedTransaction::new(raw_txn, sender2_public_key.clone(), signature);
    transactions.push(signed_txn);

    let multisig_payload = TransactionPayload::Payload(TransactionPayloadInner::V1 {
        executable: TransactionExecutable::Empty,
        extra_config: TransactionExtraConfig::V1 {
            replay_protection_nonce: Some(2),
            multisig_address: Some(AccountAddress::from_str("0x4").unwrap()),
        },
    });
    let raw_txn = RawTransaction::new(
        sender2,
        3,
        multisig_payload,
        2200,
        0,
        u64::MAX,
        ChainId::test(),
    );
    let signature = sender2_private_key.sign(&raw_txn).unwrap();
    let signed_txn = SignedTransaction::new(raw_txn, sender2_public_key.clone(), signature);
    transactions.push(signed_txn);

    // Sender 3: Mix of platform and script transactions
    // Script transaction with Payload
    let script = Script::new(vec![4, 5, 6], vec![], vec![]);
    let raw_txn = RawTransaction::new(
        sender3,
        2,
        TransactionPayload::Payload(TransactionPayloadInner::V1 {
            executable: TransactionExecutable::Script(script),
            extra_config: TransactionExtraConfig::V1 {
                replay_protection_nonce: Some(2),
                multisig_address: None,
            },
        }),
        3100,
        0,
        u64::MAX,
        ChainId::test(),
    );
    let signature = sender3_private_key.sign(&raw_txn).unwrap();
    let signed_txn = SignedTransaction::new(raw_txn, sender3_public_key.clone(), signature);
    transactions.push(signed_txn);

    let multisig_payload = TransactionPayload::Payload(TransactionPayloadInner::V1 {
        executable: TransactionExecutable::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::from_str("0x23").unwrap(),
                Identifier::new("test").unwrap(),
            ),
            Identifier::new("multisig_function_2").unwrap(),
            vec![],
            vec![],
        )),
        extra_config: TransactionExtraConfig::V1 {
            replay_protection_nonce: Some(2),
            multisig_address: Some(AccountAddress::from_str("0x4").unwrap()),
        },
    });
    let raw_txn = RawTransaction::new(
        sender3,
        3,
        multisig_payload,
        3200,
        0,
        u64::MAX,
        ChainId::test(),
    );
    let signature = sender3_private_key.sign(&raw_txn).unwrap();
    let signed_txn = SignedTransaction::new(raw_txn, sender3_public_key.clone(), signature);
    transactions.push(signed_txn);

    // Platform entry function with Payload
    let platform_entry = EntryFunction::new(
        ModuleId::new(
            AccountAddress::from_str("0x1").unwrap(),
            Identifier::new("test").unwrap(),
        ),
        Identifier::new("platform_function_2").unwrap(),
        vec![],
        vec![],
    );
    let raw_txn = RawTransaction::new(
        sender3,
        1,
        TransactionPayload::Payload(TransactionPayloadInner::V1 {
            executable: TransactionExecutable::EntryFunction(platform_entry),
            extra_config: TransactionExtraConfig::V1 {
                replay_protection_nonce: Some(1),
                multisig_address: None,
            },
        }),
        3000,
        0,
        u64::MAX,
        ChainId::test(),
    );
    let signature = sender3_private_key.sign(&raw_txn).unwrap();
    let signed_txn = SignedTransaction::new(raw_txn, sender3_public_key.clone(), signature);
    transactions.push(signed_txn);

    // Sender 4: Mix of contract and multisig transactions
    // Contract entry function
    let contract_entry = EntryFunction::new(
        ModuleId::new(
            AccountAddress::from_str("0x456").unwrap(),
            Identifier::new("test").unwrap(),
        ),
        Identifier::new("contract_function_2").unwrap(),
        vec![],
        vec![],
    );
    let raw_txn = RawTransaction::new(
        sender4,
        1,
        TransactionPayload::EntryFunction(contract_entry),
        4000,
        0,
        u64::MAX,
        ChainId::test(),
    );
    let signature = sender4_private_key.sign(&raw_txn).unwrap();
    let signed_txn = SignedTransaction::new(raw_txn, sender4_public_key.clone(), signature);
    transactions.push(signed_txn);

    // Multisig transaction
    let multisig_payload = TransactionPayload::Multisig(velor_types::transaction::Multisig {
        multisig_address: AccountAddress::from_str("0x4").unwrap(),
        transaction_payload: Some(
            velor_types::transaction::MultisigTransactionPayload::EntryFunction(
                EntryFunction::new(
                    ModuleId::new(
                        AccountAddress::from_str("0x1").unwrap(),
                        Identifier::new("test").unwrap(),
                    ),
                    Identifier::new("multisig_function_2").unwrap(),
                    vec![],
                    vec![],
                ),
            ),
        ),
    });
    let raw_txn = RawTransaction::new(
        sender4,
        2,
        multisig_payload,
        4100,
        0,
        u64::MAX,
        ChainId::test(),
    );
    let signature = sender4_private_key.sign(&raw_txn).unwrap();
    let signed_txn = SignedTransaction::new(raw_txn, sender4_public_key.clone(), signature);
    transactions.push(signed_txn);

    // Add another transaction for sender 4 with Payload multisig
    let multisig_payload = TransactionPayload::Multisig(velor_types::transaction::Multisig {
        multisig_address: AccountAddress::from_str("0x4").unwrap(),
        transaction_payload: Some(
            velor_types::transaction::MultisigTransactionPayload::EntryFunction(
                EntryFunction::new(
                    ModuleId::new(
                        AccountAddress::from_str("0x1").unwrap(),
                        Identifier::new("test").unwrap(),
                    ),
                    Identifier::new("multisig_function_3").unwrap(),
                    vec![],
                    vec![],
                ),
            ),
        ),
    });
    let raw_txn = RawTransaction::new(
        sender4,
        3,
        multisig_payload,
        4200,
        0,
        u64::MAX,
        ChainId::test(),
    );
    let signature = sender4_private_key.sign(&raw_txn).unwrap();
    let signed_txn = SignedTransaction::new(raw_txn, sender4_public_key.clone(), signature);
    transactions.push(signed_txn);

    // Create config with different spread factors
    let config = Config {
        sender_spread_factor: 2,
        platform_use_case_spread_factor: 1,
        user_use_case_spread_factor: 3,
    };

    let shuffled_txns = ShuffledTransactionIterator::new(config.clone())
        .extended_with(transactions.clone())
        .collect_vec();
    assert_eq!(shuffled_txns.len(), 11);

    // Verify the order of shuffled transactions matches expected order
    let expected_order = [0, 2, 8, 1, 5, 6, 3, 7, 9, 4, 10];
    for (i, &expected_idx) in expected_order.iter().enumerate() {
        assert_eq!(
            shuffled_txns[i], transactions[expected_idx],
            "Transaction at position {} has wrong sender",
            i
        );
    }
}

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction::{
    signature_verified_transaction::SignatureVerifiedTransaction, SignedTransaction, Transaction,
    TransactionExecutableRef, TransactionPayload,
};
use move_core_types::account_address::AccountAddress;

#[derive(Clone, Eq, Hash, PartialEq)]
pub enum UseCaseKey {
    Platform,
    ContractAddress(AccountAddress),
    // ModuleBundle (deprecated anyway), scripts, Multisig.
    Others,
}

impl std::fmt::Debug for UseCaseKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use UseCaseKey::*;

        match self {
            Platform => write!(f, "PP"),
            ContractAddress(addr) => write!(f, "c{}", hex::encode_upper(&addr[29..])),
            Others => write!(f, "OO"),
        }
    }
}

fn parse_use_case(payload: &TransactionPayload) -> UseCaseKey {
    use TransactionPayload::*;
    use UseCaseKey::*;

    let maybe_entry_func = match payload {
        Script(_) | ModuleBundle(_) | Multisig(_) => None,
        EntryFunction(entry_fun) => Some(entry_fun),
        v2 @ Payload(_) => {
            if let Ok(TransactionExecutableRef::EntryFunction(entry_fun)) = v2.executable_ref() {
                Some(entry_fun)
            } else {
                None
            }
        },
    };

    match maybe_entry_func {
        Some(entry_func) => {
            let module_id = entry_func.module();
            if module_id.address().is_special() {
                Platform
            } else {
                ContractAddress(*module_id.address())
            }
        },
        None => Others,
    }
}

pub trait UseCaseAwareTransaction {
    fn parse_sender(&self) -> AccountAddress;

    fn parse_use_case(&self) -> UseCaseKey;
}

impl UseCaseAwareTransaction for SignedTransaction {
    fn parse_sender(&self) -> AccountAddress {
        self.sender()
    }

    fn parse_use_case(&self) -> UseCaseKey {
        parse_use_case(self.payload())
    }
}

impl UseCaseAwareTransaction for SignatureVerifiedTransaction {
    fn parse_sender(&self) -> AccountAddress {
        self.sender()
            .expect("Expected a sender on SignatureVerifiedTransaction but received None")
    }

    fn parse_use_case(&self) -> UseCaseKey {
        let payload: Option<&TransactionPayload> = match self {
            SignatureVerifiedTransaction::Valid(txn) => match txn {
                Transaction::UserTransaction(signed_txn) => Some(signed_txn.payload()),
                Transaction::GenesisTransaction(_)
                | Transaction::BlockMetadata(_)
                | Transaction::StateCheckpoint(_)
                | Transaction::ValidatorTransaction(_)
                | Transaction::ScheduledTransaction(_)
                | Transaction::BlockMetadataExt(_)
                | Transaction::BlockEpilogue(_) => None,
            },
            // TODO I don't think we want invalid transactions during shuffling, but double check this logic...
            SignatureVerifiedTransaction::Invalid(_) => None,
        };

        let payload =
            payload.expect("No payload found for SignatureVerifiedTransaction in parse_use_case");

        parse_use_case(payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        block_metadata::BlockMetadata,
        transaction::{
            ChainId, EntryFunction, Multisig, MultisigTransactionPayload, RawTransaction, Script,
            TransactionExecutable, TransactionExtraConfig, TransactionPayloadInner,
        },
    };
    use aptos_crypto::{
        ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
        HashValue, PrivateKey, SigningKey, Uniform,
    };
    use move_core_types::{identifier::Identifier, language_storage::ModuleId};
    use std::str::FromStr;

    fn create_test_keys() -> (Ed25519PrivateKey, Ed25519PublicKey) {
        let private_key = Ed25519PrivateKey::generate_for_testing();
        let public_key = private_key.public_key();
        (private_key, public_key)
    }

    #[test]
    fn test_entry_function_use_case() {
        let (private_key, public_key) = create_test_keys();
        let sender = AccountAddress::from_str("0x1").unwrap();

        // Test platform entry function
        let platform_module = ModuleId::new(
            AccountAddress::from_str("0x1").unwrap(),
            Identifier::new("test").unwrap(),
        );
        let entry_function = EntryFunction::new(
            platform_module,
            Identifier::new("test_function").unwrap(),
            vec![],
            vec![],
        );
        let raw_txn = RawTransaction::new(
            sender,
            1,
            TransactionPayload::EntryFunction(entry_function),
            1000,
            0,
            u64::MAX,
            ChainId::test(),
        );
        let signature = private_key.sign(&raw_txn).unwrap();
        let signed_txn = SignedTransaction::new(raw_txn, public_key.clone(), signature);
        assert!(matches!(signed_txn.parse_use_case(), UseCaseKey::Platform));

        // Test contract entry function
        let contract_module = ModuleId::new(
            AccountAddress::from_str("0x123").unwrap(),
            Identifier::new("test").unwrap(),
        );
        let entry_function = EntryFunction::new(
            contract_module,
            Identifier::new("test_function").unwrap(),
            vec![],
            vec![],
        );
        let raw_txn = RawTransaction::new(
            sender,
            2,
            TransactionPayload::Payload(TransactionPayloadInner::V1 {
                executable: TransactionExecutable::EntryFunction(entry_function),
                extra_config: TransactionExtraConfig::V1 {
                    replay_protection_nonce: Some(2),
                    multisig_address: None,
                },
            }),
            1000,
            0,
            u64::MAX,
            ChainId::test(),
        );
        let signature = private_key.sign(&raw_txn).unwrap();
        let signed_txn = SignedTransaction::new(raw_txn, public_key, signature);
        match signed_txn.parse_use_case() {
            UseCaseKey::ContractAddress(addr) => {
                assert_eq!(addr, AccountAddress::from_str("0x123").unwrap())
            },
            _ => panic!("Expected ContractAddress use case"),
        }
    }

    #[test]
    fn test_script_use_case() {
        let (private_key, public_key) = create_test_keys();
        let sender = AccountAddress::from_str("0x1").unwrap();

        let script = Script::new(vec![1, 2, 3], vec![], vec![]);
        let raw_txn = RawTransaction::new(
            sender,
            1,
            TransactionPayload::Script(script),
            1000,
            0,
            u64::MAX,
            ChainId::test(),
        );
        let signature = private_key.sign(&raw_txn).unwrap();
        let signed_txn = SignedTransaction::new(raw_txn, public_key, signature);
        assert!(matches!(signed_txn.parse_use_case(), UseCaseKey::Others));
    }

    #[test]
    fn test_multisig_use_case() {
        let (private_key, public_key) = create_test_keys();
        let sender = AccountAddress::from_str("0x1").unwrap();

        let multisig_payload = TransactionPayload::Multisig(Multisig {
            multisig_address: AccountAddress::from_str("0x4").unwrap(),
            transaction_payload: Some(MultisigTransactionPayload::EntryFunction(
                EntryFunction::new(
                    ModuleId::new(
                        AccountAddress::from_str("0x1").unwrap(),
                        Identifier::new("test").unwrap(),
                    ),
                    Identifier::new("multisig_function").unwrap(),
                    vec![],
                    vec![],
                ),
            )),
        });
        let raw_txn = RawTransaction::new(
            sender,
            1,
            multisig_payload,
            1000,
            0,
            u64::MAX,
            ChainId::test(),
        );
        let signature = private_key.sign(&raw_txn).unwrap();
        let signed_txn = SignedTransaction::new(raw_txn, public_key.clone(), signature);
        assert!(matches!(signed_txn.parse_use_case(), UseCaseKey::Others));

        // Test contract entry function
        let contract_module = ModuleId::new(
            AccountAddress::from_str("0x123").unwrap(),
            Identifier::new("test").unwrap(),
        );
        let entry_function = EntryFunction::new(
            contract_module,
            Identifier::new("test_function").unwrap(),
            vec![],
            vec![],
        );
        let raw_txn = RawTransaction::new(
            sender,
            2,
            TransactionPayload::Payload(TransactionPayloadInner::V1 {
                executable: TransactionExecutable::EntryFunction(entry_function),
                extra_config: TransactionExtraConfig::V1 {
                    replay_protection_nonce: Some(2),
                    multisig_address: Some(AccountAddress::from_str("0x4").unwrap()),
                },
            }),
            1000,
            0,
            u64::MAX,
            ChainId::test(),
        );
        let signature = private_key.sign(&raw_txn).unwrap();
        let signed_txn = SignedTransaction::new(raw_txn, public_key, signature);
        match signed_txn.parse_use_case() {
            UseCaseKey::ContractAddress(addr) => {
                assert_eq!(addr, AccountAddress::from_str("0x123").unwrap())
            },
            _ => panic!("Expected ContractAddress use case"),
        }
    }

    #[test]
    fn test_signature_verified_transaction_use_case() {
        let (private_key, public_key) = create_test_keys();
        let sender = AccountAddress::from_str("0x1").unwrap();

        // Test platform entry function
        let platform_module = ModuleId::new(
            AccountAddress::from_str("0x1").unwrap(),
            Identifier::new("test").unwrap(),
        );
        let entry_function = EntryFunction::new(
            platform_module,
            Identifier::new("test_function").unwrap(),
            vec![],
            vec![],
        );
        let raw_txn = RawTransaction::new(
            sender,
            1,
            TransactionPayload::EntryFunction(entry_function),
            1000,
            0,
            u64::MAX,
            ChainId::test(),
        );
        let signature = private_key.sign(&raw_txn).unwrap();
        let signed_txn = SignedTransaction::new(raw_txn, public_key.clone(), signature);
        let verified_txn =
            SignatureVerifiedTransaction::Valid(Transaction::UserTransaction(signed_txn));
        assert!(matches!(
            verified_txn.parse_use_case(),
            UseCaseKey::Platform
        ));

        // Test script transaction
        let script = Script::new(vec![1, 2, 3], vec![], vec![]);
        let raw_txn = RawTransaction::new(
            sender,
            2,
            TransactionPayload::Script(script),
            1000,
            0,
            u64::MAX,
            ChainId::test(),
        );
        let signature = private_key.sign(&raw_txn).unwrap();
        let signed_txn = SignedTransaction::new(raw_txn.clone(), public_key.clone(), signature);
        let verified_txn =
            SignatureVerifiedTransaction::Valid(Transaction::UserTransaction(signed_txn));
        assert!(matches!(verified_txn.parse_use_case(), UseCaseKey::Others));

        // Test multisig transaction
        let multisig_payload = TransactionPayload::Multisig(Multisig {
            multisig_address: AccountAddress::from_str("0x4").unwrap(),
            transaction_payload: Some(MultisigTransactionPayload::EntryFunction(
                EntryFunction::new(
                    ModuleId::new(
                        AccountAddress::from_str("0x1").unwrap(),
                        Identifier::new("test").unwrap(),
                    ),
                    Identifier::new("multisig_function").unwrap(),
                    vec![],
                    vec![],
                ),
            )),
        });
        let raw_txn = RawTransaction::new(
            sender,
            3,
            multisig_payload,
            1000,
            0,
            u64::MAX,
            ChainId::test(),
        );
        let signature = private_key.sign(&raw_txn).unwrap();
        let signed_txn = SignedTransaction::new(raw_txn, public_key, signature);
        let verified_txn =
            SignatureVerifiedTransaction::Valid(Transaction::UserTransaction(signed_txn));
        assert!(matches!(verified_txn.parse_use_case(), UseCaseKey::Others));
    }

    #[test]
    #[should_panic(
        expected = "No payload found for SignatureVerifiedTransaction in parse_use_case"
    )]
    fn test_invalid_signature_verified_transaction() {
        let verified_txn =
            SignatureVerifiedTransaction::Invalid(Transaction::BlockMetadata(BlockMetadata::new(
                HashValue::zero(),
                1,
                1,
                AccountAddress::from_str("0x1").unwrap(),
                vec![],
                vec![],
                u64::MAX,
            )));
        verified_txn.parse_use_case();
    }
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_matcher::Filter;
use aptos_crypto::HashValue;
use aptos_types::transaction::SignedTransaction;

pub struct TransactionFilter {
    filter: Filter,
}

impl TransactionFilter {
    pub fn new(filter: Filter) -> Self {
        Self { filter }
    }

    pub fn filter(
        &self,
        block_id: HashValue,
        block_epoch: u64,
        block_timestamp: u64,
        txns: Vec<SignedTransaction>,
    ) -> Vec<SignedTransaction> {
        // Special case for no filter to avoid unnecessary iteration through all transactions in the default case
        if self.filter.is_empty() {
            return txns;
        }

        txns.into_iter()
            .filter(|txn| {
                self.filter
                    .allows(block_id, block_epoch, block_timestamp, txn)
            })
            .collect()
    }
}

#[cfg(test)]
mod test {
    use crate::{
        transaction_filter::TransactionFilter,
        transaction_matcher::{Filter, Matcher},
    };
    use aptos_crypto::{
        ed25519::{Ed25519PrivateKey, Ed25519PublicKey, Ed25519Signature},
        HashValue, PrivateKey, SigningKey, Uniform,
    };
    use aptos_types::{
        chain_id::ChainId,
        move_utils::MemberId,
        transaction::{
            authenticator::{AccountAuthenticator, AnyPublicKey, TransactionAuthenticator},
            EntryFunction, Multisig, MultisigTransactionPayload, RawTransaction, Script,
            SignedTransaction, TransactionExecutable, TransactionExecutableRef,
            TransactionExtraConfig, TransactionPayload, TransactionPayloadInner,
        },
    };
    use move_core_types::{
        account_address::AccountAddress, transaction_argument::TransactionArgument,
    };
    use rand::thread_rng;

    fn create_account_authenticator(public_key: Ed25519PublicKey) -> AccountAuthenticator {
        AccountAuthenticator::Ed25519 {
            public_key,
            signature: Ed25519Signature::dummy_signature(),
        }
    }

    fn create_entry_function(function: MemberId) -> EntryFunction {
        let MemberId {
            module_id,
            member_id: function_id,
        } = function;
        EntryFunction::new(module_id, function_id, vec![], vec![])
    }

    fn create_entry_function_transaction(
        function: MemberId,
        use_new_txn_payload_format: bool,
    ) -> SignedTransaction {
        let entry_function = create_entry_function(function);
        let transaction_payload = if use_new_txn_payload_format {
            // Use the new payload format
            let executable = TransactionExecutable::EntryFunction(entry_function);
            let extra_config = TransactionExtraConfig::V1 {
                multisig_address: None,
                replay_protection_nonce: None,
            };
            TransactionPayload::Payload(TransactionPayloadInner::V1 {
                executable,
                extra_config,
            })
        } else {
            // Use the old payload format
            TransactionPayload::EntryFunction(entry_function)
        };

        create_signed_transaction(transaction_payload, false)
    }

    fn create_fee_payer_transaction() -> SignedTransaction {
        let entry_function = create_entry_function(str::parse("0x0::fee_payer::pay").unwrap());
        let transaction_payload = TransactionPayload::EntryFunction(entry_function);

        create_signed_transaction(transaction_payload, true)
    }

    fn create_multisig_transaction(
        multisig_address: AccountAddress,
        function: MemberId,
        use_new_txn_payload_format: bool,
    ) -> SignedTransaction {
        let transaction_payload = if use_new_txn_payload_format {
            // Use the new payload format
            let executable = TransactionExecutable::EntryFunction(create_entry_function(function));
            let extra_config = TransactionExtraConfig::V1 {
                multisig_address: Some(multisig_address),
                replay_protection_nonce: None,
            };
            TransactionPayload::Payload(TransactionPayloadInner::V1 {
                executable,
                extra_config,
            })
        } else {
            // Use the old payload format
            TransactionPayload::Multisig(Multisig {
                multisig_address,
                transaction_payload: Some(MultisigTransactionPayload::EntryFunction(
                    create_entry_function(function),
                )),
            })
        };

        create_signed_transaction(transaction_payload, false)
    }

    fn create_script_transaction(use_new_txn_payload_format: bool) -> SignedTransaction {
        let script_arguments = vec![
            TransactionArgument::U64(0),
            TransactionArgument::U128(0),
            TransactionArgument::Address(AccountAddress::random()),
            TransactionArgument::Bool(true),
        ];
        let script = Script::new(vec![], vec![], script_arguments);

        let transaction_payload = if use_new_txn_payload_format {
            // Use the new payload format
            let executable = TransactionExecutable::Script(script);
            let extra_config = TransactionExtraConfig::V1 {
                multisig_address: None,
                replay_protection_nonce: None,
            };
            TransactionPayload::Payload(TransactionPayloadInner::V1 {
                executable,
                extra_config,
            })
        } else {
            // Use the old payload format
            TransactionPayload::Script(script)
        };

        create_signed_transaction(transaction_payload, false)
    }

    fn create_signed_transaction(
        transaction_payload: TransactionPayload,
        fee_payer: bool,
    ) -> SignedTransaction {
        let sender = AccountAddress::random();
        let sequence_number = 0;
        let raw_transaction = RawTransaction::new(
            sender,
            sequence_number,
            transaction_payload,
            0,
            0,
            0,
            ChainId::new(10),
        );

        let private_key = Ed25519PrivateKey::generate(&mut thread_rng());
        let public_key = private_key.public_key();

        if fee_payer {
            SignedTransaction::new_fee_payer(
                raw_transaction.clone(),
                create_account_authenticator(public_key.clone()),
                vec![],
                vec![],
                AccountAddress::random(),
                create_account_authenticator(public_key.clone()),
            )
        } else {
            SignedTransaction::new(
                raw_transaction.clone(),
                public_key.clone(),
                private_key.sign(&raw_transaction).unwrap(),
            )
        }
    }

    fn get_fee_payer_address(signed_transaction: &SignedTransaction) -> AccountAddress {
        match signed_transaction.authenticator() {
            TransactionAuthenticator::FeePayer {
                fee_payer_address, ..
            } => fee_payer_address,
            payload => panic!("Unexpected transaction payload: {:?}", payload),
        }
    }

    fn get_address_argument(script: &Script) -> AccountAddress {
        for arg in script.args() {
            if let TransactionArgument::Address(address) = arg {
                return *address;
            }
        }
        panic!("No address argument found in script transaction");
    }

    fn get_auth_public_key(signed_transaction: &SignedTransaction) -> AnyPublicKey {
        match signed_transaction.authenticator() {
            TransactionAuthenticator::Ed25519 { public_key, .. } => {
                AnyPublicKey::ed25519(public_key)
            },
            authenticator => panic!("Unexpected transaction authenticator: {:?}", authenticator),
        }
    }

    fn get_block_id_and_entry_function_transactions(
        use_new_txn_payload_format: bool,
    ) -> (HashValue, Vec<SignedTransaction>) {
        let block_id = HashValue::random();
        let mut entry_function_txns = vec![];
        for (i, function_name) in [
            "add", "check", "new", "sub", "mul", "div", "mod", "pow", "exp", "sqrt",
        ]
        .iter()
        .enumerate()
        {
            let transaction = create_entry_function_transaction(
                str::parse(&format!("0x{}::entry::{}", i, function_name)).unwrap(),
                use_new_txn_payload_format,
            );
            entry_function_txns.push(transaction);
        }

        (block_id, entry_function_txns)
    }

    fn get_block_id_and_fee_payer_transactions() -> (HashValue, Vec<SignedTransaction>) {
        let block_id = HashValue::random();
        let mut fee_payer_transactions = vec![];
        for _ in 0..10 {
            let transaction = create_fee_payer_transaction();
            fee_payer_transactions.push(transaction)
        }

        (block_id, fee_payer_transactions)
    }

    fn get_block_id_and_multisig_transactions(
        use_new_txn_payload_format: bool,
    ) -> (HashValue, Vec<SignedTransaction>) {
        let block_id = HashValue::random();
        let mut multisig_transactions = vec![];
        for i in 0..10 {
            let transaction = create_multisig_transaction(
                AccountAddress::random(),
                str::parse(&format!("0x{}::multisig::sign", i)).unwrap(),
                use_new_txn_payload_format,
            );
            multisig_transactions.push(transaction);
        }

        (block_id, multisig_transactions)
    }

    fn get_block_id_and_script_transactions(
        use_new_txn_payload_format: bool,
    ) -> (HashValue, Vec<SignedTransaction>) {
        let block_id = HashValue::random();
        let mut script_transactions = vec![];
        for _ in 0..10 {
            let transaction = create_script_transaction(use_new_txn_payload_format);
            script_transactions.push(transaction);
        }

        (block_id, script_transactions)
    }

    fn get_ed25519_public_key(signed_transaction: &SignedTransaction) -> Ed25519PublicKey {
        match signed_transaction.authenticator() {
            TransactionAuthenticator::Ed25519 { public_key, .. } => public_key.clone(),
            authenticator => panic!("Unexpected transaction authenticator: {:?}", authenticator),
        }
    }

    fn get_function_name(txn: &SignedTransaction) -> String {
        match txn.payload().executable_ref() {
            Ok(TransactionExecutableRef::EntryFunction(entry_func)) => {
                entry_func.function().to_string()
            },
            payload => panic!("Unexpected transaction payload: {:?}", payload),
        }
    }

    fn get_module_address(txn: &SignedTransaction) -> AccountAddress {
        match txn.payload().executable_ref() {
            Ok(TransactionExecutableRef::EntryFunction(entry_func)) => {
                *entry_func.module().address()
            },
            payload => panic!("Unexpected transaction payload: {:?}", payload),
        }
    }

    fn get_module_name(txn: &SignedTransaction) -> String {
        match txn.payload().executable_ref() {
            Ok(TransactionExecutableRef::EntryFunction(entry_func)) => {
                entry_func.module().name().to_string()
            },
            payload => panic!("Unexpected transaction payload: {:?}", payload),
        }
    }

    fn get_multisig_address(txn: &SignedTransaction) -> AccountAddress {
        match txn.payload() {
            TransactionPayload::Multisig(multisig) => multisig.multisig_address,
            TransactionPayload::Payload(TransactionPayloadInner::V1 {
                extra_config:
                    TransactionExtraConfig::V1 {
                        multisig_address, ..
                    },
                ..
            }) => multisig_address.expect("Expected multisig address!"),
            payload => panic!("Unexpected transaction payload: {:?}", payload),
        }
    }

    fn get_script_argument_address(txn: &SignedTransaction) -> AccountAddress {
        match txn.payload() {
            TransactionPayload::Script(script) => get_address_argument(script),
            TransactionPayload::Payload(TransactionPayloadInner::V1 {
                executable: TransactionExecutable::Script(script),
                ..
            }) => get_address_argument(script),
            payload => panic!("Unexpected transaction payload: {:?}", payload),
        }
    }

    #[test]
    fn test_empty_filter() {
        for use_new_txn_payload_format in [false, true] {
            // Create an empty filter
            let filter = TransactionFilter::new(Filter::empty());

            // Verify that it returns all transactions
            let (block_id, txns) =
                get_block_id_and_entry_function_transactions(use_new_txn_payload_format);
            let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, txns);
        }
    }

    #[test]
    fn test_all_filter() {
        for use_new_txn_payload_format in [false, true] {
            // Create a filter that allows all transactions
            let filter = TransactionFilter::new(Filter::empty().add_all_filter(true));

            // Verify that it returns all transactions
            let (block_id, txns) =
                get_block_id_and_entry_function_transactions(use_new_txn_payload_format);
            let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, txns);

            // Create a filter that denies all transactions
            let filter = TransactionFilter::new(Filter::empty().add_all_filter(false));

            // Verify that it returns no transactions
            let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, vec![]);
        }
    }

    #[test]
    fn test_block_id_filter() {
        for use_new_txn_payload_format in [false, true] {
            // Create a filter that only allows transactions with a specific block ID
            let (block_id, txns) =
                get_block_id_and_entry_function_transactions(use_new_txn_payload_format);
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_block_id_filter(true, block_id)
                    .add_all_filter(false),
            );

            // Verify that it returns all transactions with the specified block ID
            let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, txns);

            // Verify that it returns no transactions with a different block ID
            let different_block_id = HashValue::random();
            let filtered_txns = filter.filter(different_block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, vec![]);

            // Create a filter that denies transactions with a specific block ID
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_block_id_filter(false, block_id)
                    .add_all_filter(true),
            );

            // Verify that it returns all transactions except those with the specified block ID
            let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, vec![]);

            // Verify that it returns all transactions with a different block ID
            let different_block_id = HashValue::random();
            let filtered_txns = filter.filter(different_block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, txns);
        }
    }

    #[test]
    fn test_block_timestamp_greater_than_filter() {
        for use_new_txn_payload_format in [false, true] {
            // Create a filter that only allows transactions with a block timestamp greater than a specific value
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_block_timestamp_greater_than_filter(true, 1000)
                    .add_all_filter(false),
            );

            // Verify that it returns no transactions with a block timestamp less than or equal to 1000
            let (block_id, txns) =
                get_block_id_and_entry_function_transactions(use_new_txn_payload_format);
            for block_timestamp in [0, 999, 1000] {
                let filtered_txns = filter.filter(block_id, 0, block_timestamp, txns.clone());
                assert_eq!(filtered_txns, vec![]);
            }

            // Verify that it returns all transactions with a block timestamp greater than 1000
            let filtered_txns = filter.filter(block_id, 0, 1001, txns.clone());
            assert_eq!(filtered_txns, txns);

            // Create a filter that denies transactions with a block timestamp greater than a specific value
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_block_timestamp_greater_than_filter(false, 1000)
                    .add_all_filter(true),
            );

            // Verify that it returns all transactions with a block timestamp less than or equal to 1000
            for block_timestamp in [0, 999, 1000] {
                let filtered_txns = filter.filter(block_id, 0, block_timestamp, txns.clone());
                assert_eq!(filtered_txns, txns);
            }

            // Verify that it returns no transactions with a block timestamp greater than 1000
            let filtered_txns = filter.filter(block_id, 0, 1001, txns.clone());
            assert_eq!(filtered_txns, vec![]);
        }
    }

    #[test]
    fn test_block_timestamp_less_than_filter() {
        for use_new_txn_payload_format in [false, true] {
            // Create a filter that only allows transactions with a block timestamp less than a specific value
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_block_timestamp_less_than_filter(true, 1000)
                    .add_all_filter(false),
            );

            // Verify that it returns all transactions with a block timestamp less than 1000
            let (block_id, txns) =
                get_block_id_and_entry_function_transactions(use_new_txn_payload_format);
            let filtered_txns = filter.filter(block_id, 0, 999, txns.clone());
            assert_eq!(filtered_txns, txns);

            // Verify that it returns no transactions with a block timestamp greater than or equal to 1000
            for block_timestamp in [1000, 1001] {
                let filtered_txns = filter.filter(block_id, 0, block_timestamp, txns.clone());
                assert_eq!(filtered_txns, vec![]);
            }

            // Create a filter that denies transactions with a block timestamp less than a specific value
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_block_timestamp_less_than_filter(false, 1000)
                    .add_all_filter(true),
            );

            // Verify that it returns no transactions with a block timestamp less than 1000
            let filtered_txns = filter.filter(block_id, 0, 999, txns.clone());
            assert_eq!(filtered_txns, vec![]);

            // Verify that it returns all transactions with a block timestamp greater than or equal to 1000
            for block_timestamp in [1000, 1001] {
                let filtered_txns = filter.filter(block_id, 0, block_timestamp, txns.clone());
                assert_eq!(filtered_txns, txns);
            }
        }
    }

    #[test]
    fn test_transaction_id_filter() {
        for use_new_txn_payload_format in [false, true] {
            // Create a filter that only allows transactions with a specific transaction ID (txn 0)
            let (block_id, txns) =
                get_block_id_and_entry_function_transactions(use_new_txn_payload_format);
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_transaction_id_filter(true, txns[0].committed_hash())
                    .add_all_filter(false),
            );

            // Verify that it returns the transaction with the specified ID
            let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, vec![txns[0].clone()]);

            // Create a filter that denies transactions with a specific transaction ID (txn 0)
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_transaction_id_filter(false, txns[0].committed_hash())
                    .add_all_filter(true),
            );

            // Verify that it returns all transactions except the one with the specified ID
            let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, txns[1..].to_vec());
        }
    }

    #[test]
    fn test_sender_filter() {
        for use_new_txn_payload_format in [false, true] {
            // Create a filter that only allows transactions from a specific sender (txn 0 and txn 1)
            let (block_id, txns) =
                get_block_id_and_entry_function_transactions(use_new_txn_payload_format);
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_sender_filter(true, txns[0].sender())
                    .add_sender_filter(true, txns[1].sender())
                    .add_all_filter(false),
            );

            // Verify that it returns transactions from the specified senders
            let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, txns[0..2].to_vec());

            // Create a filter that denies transactions from a specific sender (txn 0, txn 1 and txn 2)
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_sender_filter(false, txns[0].sender())
                    .add_sender_filter(false, txns[1].sender())
                    .add_sender_filter(false, txns[2].sender()),
            );

            // Verify that it returns transactions from other senders
            let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, txns[3..].to_vec());
        }
    }

    #[test]
    fn test_module_address_filter() {
        for use_new_txn_payload_format in [false, true] {
            // Create a filter that only allows transactions from a specific module address (txn 0 and txn 1)
            let (block_id, txns) =
                get_block_id_and_entry_function_transactions(use_new_txn_payload_format);
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_module_address_filter(true, get_module_address(&txns[0]))
                    .add_module_address_filter(true, get_module_address(&txns[1]))
                    .add_all_filter(false),
            );

            // Verify that it returns transactions from the specified module addresses
            let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, txns[0..2].to_vec());

            // Create a filter that denies transactions from a specific module address (txn 0)
            let filter = TransactionFilter::new(
                Filter::empty().add_module_address_filter(false, get_module_address(&txns[0])),
            );

            // Verify that it returns transactions from other module addresses
            let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, txns[1..].to_vec());
        }
    }

    #[test]
    fn test_entry_function_filter() {
        for use_new_txn_payload_format in [false, true] {
            // Create a filter that only allows transactions with specific entry functions (txn 0 and txn 1)
            let (block_id, txns) =
                get_block_id_and_entry_function_transactions(use_new_txn_payload_format);
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_entry_function_filter(
                        true,
                        get_module_address(&txns[0]),
                        get_module_name(&txns[0]),
                        get_function_name(&txns[0]),
                    )
                    .add_entry_function_filter(
                        true,
                        get_module_address(&txns[1]),
                        get_module_name(&txns[1]),
                        get_function_name(&txns[1]),
                    )
                    .add_all_filter(false),
            );

            // Verify that it returns transactions with the specified entry functions
            let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, txns[0..2].to_vec());

            // Create a filter that denies transactions with specific entry functions (txn 0)
            let filter = TransactionFilter::new(Filter::empty().add_entry_function_filter(
                false,
                get_module_address(&txns[0]),
                get_module_name(&txns[0]),
                get_function_name(&txns[0]),
            ));

            // Verify that it returns transactions with other entry functions
            let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, txns[1..].to_vec());
        }
    }

    #[test]
    fn test_block_epoch_greater_than_filter() {
        for use_new_txn_payload_format in [false, true] {
            // Create a filter that only allows transactions with a block epoch greater than a specific value
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_block_epoch_greater_than_filter(true, 1000)
                    .add_all_filter(false),
            );

            // Verify that it returns no transactions with a block epoch less than or equal to 1000
            let (block_id, txns) =
                get_block_id_and_entry_function_transactions(use_new_txn_payload_format);
            for block_epoch in [0, 999, 1000] {
                let filtered_txns = filter.filter(block_id, block_epoch, 0, txns.clone());
                assert_eq!(filtered_txns, vec![]);
            }

            // Verify that it returns all transactions with a block epoch greater than 1000
            let filtered_txns = filter.filter(block_id, 1001, 0, txns.clone());
            assert_eq!(filtered_txns, txns);

            // Create a filter that denies transactions with a block epoch greater than a specific value
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_block_epoch_greater_than_filter(false, 1000)
                    .add_all_filter(true),
            );

            // Verify that it returns all transactions with a block epoch less than or equal to 1000
            for block_epoch in [0, 999, 1000] {
                let filtered_txns = filter.filter(block_id, block_epoch, 0, txns.clone());
                assert_eq!(filtered_txns, txns);
            }

            // Verify that it returns no transactions with a block epoch greater than 1000
            let filtered_txns = filter.filter(block_id, 1001, 0, txns.clone());
            assert_eq!(filtered_txns, vec![]);
        }
    }

    #[test]
    fn test_block_epoch_less_than_filter() {
        for use_new_txn_payload_format in [false, true] {
            // Create a filter that only allows transactions with a block epoch less than a specific value
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_block_epoch_less_than_filter(true, 1000)
                    .add_all_filter(false),
            );

            // Verify that it returns all transactions with a block epoch less than 1000
            let (block_id, txns) =
                get_block_id_and_entry_function_transactions(use_new_txn_payload_format);
            let filtered_txns = filter.filter(block_id, 999, 0, txns.clone());
            assert_eq!(filtered_txns, txns);

            // Verify that it returns no transactions with a block epoch greater than or equal to 1000
            for block_epoch in [1000, 1001] {
                let filtered_txns = filter.filter(block_id, block_epoch, 0, txns.clone());
                assert_eq!(filtered_txns, vec![]);
            }

            // Create a filter that denies transactions with a block epoch less than a specific value
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_block_epoch_less_than_filter(false, 1000)
                    .add_all_filter(true),
            );

            // Verify that it returns no transactions with a block epoch less than 1000
            let filtered_txns = filter.filter(block_id, 999, 0, txns.clone());
            assert_eq!(filtered_txns, vec![]);

            // Verify that it returns all transactions with a block epoch greater than or equal to 1000
            for block_epoch in [1000, 1001] {
                let filtered_txns = filter.filter(block_id, block_epoch, 0, txns.clone());
                assert_eq!(filtered_txns, txns);
            }
        }
    }

    #[test]
    fn test_matches_all_of_filter() {
        for use_new_txn_payload_format in [false, true] {
            // Create a filter that only matches transactions with epoch greater than 1000 and a specific sender (only txn 0)
            let (block_id, txns) =
                get_block_id_and_entry_function_transactions(use_new_txn_payload_format);
            let matchers = vec![
                Matcher::BlockEpochGreaterThan(1000),
                Matcher::Sender(txns[0].sender()),
            ];
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_matches_all_of_filter(true, matchers)
                    .add_all_filter(false),
            );

            // Verify that it returns no transactions with block epoch less than or equal to 1000
            for block_epoch in [0, 999, 1000] {
                let filtered_txns = filter.filter(block_id, block_epoch, 0, txns.clone());
                assert_eq!(filtered_txns, vec![]);
            }

            // Verify that it returns transactions with block epoch greater than 1000 and the specified sender
            let filtered_txns = filter.filter(block_id, 1001, 0, txns.clone());
            assert_eq!(filtered_txns, txns[0..1].to_vec());

            // Create a filter that denies transactions with timestamp greater than 1000 and a specific sender (only txn 0)
            let matchers = vec![
                Matcher::BlockTimeStampGreaterThan(1000),
                Matcher::Sender(txns[0].sender()),
            ];
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_matches_all_of_filter(false, matchers)
                    .add_all_filter(true),
            );

            // Verify that it returns all transactions with block timestamp less than or equal to 1000
            for block_timestamp in [0, 999, 1000] {
                let filtered_txns = filter.filter(block_id, 0, block_timestamp, txns.clone());
                assert_eq!(filtered_txns, txns);
            }

            // Verify that it returns no transactions with block timestamp greater than 1000 and the specified sender
            let filtered_txns = filter.filter(block_id, 0, 1001, txns.clone());
            assert_eq!(filtered_txns, txns[1..].to_vec());
        }
    }

    #[test]
    fn test_account_address_filter_simple() {
        for use_new_txn_payload_format in [false, true] {
            // Create a filter that only allows transactions from specific account addresses.
            // These are: (i) txn 0 sender; (ii) txn 1 sender; and (iii) txn 2 entry function address.
            let (block_id, txns) =
                get_block_id_and_entry_function_transactions(use_new_txn_payload_format);
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_account_address_filter(true, txns[0].sender())
                    .add_account_address_filter(true, txns[1].sender())
                    .add_account_address_filter(true, get_module_address(&txns[2]))
                    .add_all_filter(false),
            );

            // Verify that it returns transactions from the specified account address
            let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, txns[0..3].to_vec());

            // Create a filter that denies transactions from the specified account addresses (as above)
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_account_address_filter(false, txns[0].sender())
                    .add_account_address_filter(false, txns[1].sender())
                    .add_account_address_filter(false, get_module_address(&txns[2]))
                    .add_all_filter(true),
            );

            // Verify that it returns transactions from other account addresses
            let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, txns[3..].to_vec());
        }
    }

    #[test]
    fn test_account_address_filter_multisig() {
        for use_new_txn_payload_format in [false, true] {
            // Create a filter that only allows transactions from specific account addresses.
            // These are: (i) txn 0 multisig address; (ii) txn 1 sender; and (iii) txn 2 multisig address.
            let (block_id, txns) =
                get_block_id_and_multisig_transactions(use_new_txn_payload_format);
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_account_address_filter(true, get_multisig_address(&txns[0]))
                    .add_account_address_filter(true, txns[1].sender())
                    .add_account_address_filter(true, get_multisig_address(&txns[2]))
                    .add_all_filter(false),
            );

            // Verify that it returns transactions from the specified account address
            let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, txns[0..3].to_vec());

            // Create a filter that denies transactions from the specified account addresses (as above)
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_account_address_filter(false, get_multisig_address(&txns[0]))
                    .add_account_address_filter(false, txns[1].sender())
                    .add_account_address_filter(false, get_multisig_address(&txns[2]))
                    .add_all_filter(true),
            );

            // Verify that it returns transactions from other account addresses
            let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, txns[3..].to_vec());
        }
    }

    #[test]
    fn test_account_address_filter_script_argument() {
        for use_new_txn_payload_format in [false, true] {
            // Create a filter that only allows transactions from specific account addresses.
            // These are: (i) txn 0 script arg address; (ii) txn 1 sender; and (iii) txn 2 script arg address.
            let (block_id, txns) = get_block_id_and_script_transactions(use_new_txn_payload_format);
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_account_address_filter(true, get_script_argument_address(&txns[0]))
                    .add_account_address_filter(true, txns[1].sender())
                    .add_account_address_filter(true, get_script_argument_address(&txns[2]))
                    .add_all_filter(false),
            );

            // Verify that it returns transactions from the specified account address
            let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, txns[0..3].to_vec());

            // Create a filter that denies transactions from the specified account addresses (as above)
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_account_address_filter(false, get_script_argument_address(&txns[0]))
                    .add_account_address_filter(false, txns[1].sender())
                    .add_account_address_filter(false, get_script_argument_address(&txns[2]))
                    .add_all_filter(true),
            );

            // Verify that it returns transactions from other account addresses
            let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, txns[3..].to_vec());
        }
    }

    #[test]
    fn test_account_address_filter_transaction_authenticator() {
        // Create a filter that only allows transactions from specific account addresses.
        // These are: (i) txn 0 account authenticator; (ii) txn 1 account authenticator; and (iii) txn 2 sender.
        let (block_id, txns) = get_block_id_and_fee_payer_transactions();
        let filter = TransactionFilter::new(
            Filter::empty()
                .add_account_address_filter(true, get_fee_payer_address(&txns[0]))
                .add_account_address_filter(true, get_fee_payer_address(&txns[1]))
                .add_account_address_filter(true, txns[2].sender())
                .add_all_filter(false),
        );

        // Verify that it returns transactions from the specified account address
        let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[0..3].to_vec());

        // Create a filter that denies transactions from the specified account addresses (as above)
        let filter = TransactionFilter::new(
            Filter::empty()
                .add_account_address_filter(false, get_fee_payer_address(&txns[0]))
                .add_account_address_filter(false, get_fee_payer_address(&txns[1]))
                .add_account_address_filter(false, txns[2].sender())
                .add_all_filter(true),
        );

        // Verify that it returns transactions from other account addresses
        let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[3..].to_vec());
    }

    #[test]
    fn test_public_key_filter() {
        for use_new_txn_payload_format in [false, true] {
            // Create a filter that only allows transactions from specific public keys.
            // These are: (i) txn 0 authenticator public key; and (ii) txn 1 authenticator public key.
            let (block_id, txns) =
                get_block_id_and_entry_function_transactions(use_new_txn_payload_format);
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_public_key_filter(true, get_auth_public_key(&txns[0]))
                    .add_public_key_filter(true, get_auth_public_key(&txns[1]))
                    .add_all_filter(false),
            );

            // Verify that it returns transactions from the specified account address
            let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, txns[0..2].to_vec());

            // Create a filter that denies transactions from the specified account addresses (as above)
            let filter = TransactionFilter::new(
                Filter::empty()
                    .add_public_key_filter(false, get_auth_public_key(&txns[0]))
                    .add_public_key_filter(false, get_auth_public_key(&txns[1]))
                    .add_all_filter(true),
            );

            // Verify that it returns transactions from other account addresses
            let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, txns[2..].to_vec());
        }
    }

    #[test]
    fn test_composite_allow_list_filter() {
        for use_new_txn_payload_format in [false, true] {
            // Create a filter that only allows transactions based on multiple criteria
            let (block_id, txns) =
                get_block_id_and_entry_function_transactions(use_new_txn_payload_format);
            let filter_string = format!(
                r#"
            rules:
                - Allow:
                    Sender: "{}"
                - Allow:
                    ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000001"
                - Allow:
                    PublicKey:
                        Ed25519:
                            - "{}"
                - Allow:
                    EntryFunction:
                        - "0000000000000000000000000000000000000000000000000000000000000003"
                        - entry
                        - sub
                - Allow:
                    AccountAddress: "{}"
                - Deny: All
          "#,
                txns[0].sender().to_standard_string(),
                get_ed25519_public_key(&txns[2]),
                get_module_address(&txns[4]).to_standard_string(),
            );
            let filter = serde_yaml::from_str::<Filter>(&filter_string).unwrap();
            let allow_list_filter = TransactionFilter::new(filter);

            // Verify that only the first five transactions are allowed
            let filtered_txns = allow_list_filter.filter(block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, txns[0..5].to_vec());
        }
    }

    #[test]
    fn test_composite_block_list_filter() {
        for use_new_txn_payload_format in [false, true] {
            // Create a filter that denies transactions based on multiple criteria
            let (block_id, txns) =
                get_block_id_and_entry_function_transactions(use_new_txn_payload_format);
            let filter_string = format!(
                r#"
            rules:
                - Deny:
                    ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000000"
                - Deny:
                    Sender: "{}"
                - Deny:
                    EntryFunction:
                        - "0000000000000000000000000000000000000000000000000000000000000002"
                        - entry
                        - new
                - Deny:
                    ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000003"
                - Deny:
                    AccountAddress: "{}"
                - Allow: All
          "#,
                txns[1].sender().to_standard_string(),
                get_module_address(&txns[4]).to_standard_string(),
            );
            let filter = serde_yaml::from_str::<Filter>(&filter_string).unwrap();
            let block_list_filter = TransactionFilter::new(filter);

            // Verify that the first five transactions are denied
            let filtered_txns = block_list_filter.filter(block_id, 0, 0, txns.clone());
            assert_eq!(filtered_txns, txns[5..].to_vec());
        }
    }

    #[test]
    fn test_composite_matches_all_of_filter() {
        for use_new_txn_payload_format in [false, true] {
            // Create a filter that denies transactions based on the matches all of rule
            let (block_id, txns) =
                get_block_id_and_entry_function_transactions(use_new_txn_payload_format);
            let filter_string = format!(
                r#"
            rules:
                - Deny:
                    MatchesAllOf:
                        - AccountAddress: "{}"
                        - ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000000"
                        - BlockEpochGreaterThan: 10
                - Deny:
                    MatchesAllOf:
                        - Sender: "{}"
                        - ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000001"
                        - BlockEpochGreaterThan: 10
                        - BlockTimeStampGreaterThan: 1000
                - Deny:
                    MatchesAllOf:
                        - Sender: "{}"
                        - ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000002"
                        - BlockEpochGreaterThan: 10
                        - BlockTimeStampGreaterThan: 1000
                        - BlockId: "{}"
                - Allow: All
          "#,
                txns[0].sender().to_standard_string(),
                txns[1].sender().to_standard_string(),
                txns[2].sender().to_standard_string(),
                block_id.to_hex()
            );
            let filter = serde_yaml::from_str::<Filter>(&filter_string).unwrap();
            let block_list_filter = TransactionFilter::new(filter);

            // Filter transactions with a block epoch of 11, timestamp of 1001, and the expected block ID
            let filtered_txns = block_list_filter.filter(block_id, 11, 1001, txns.clone());

            // Verify that only the first three transactions are denied
            assert_eq!(filtered_txns, txns[3..].to_vec());

            // Filter transactions with a block epoch of 11, timestamp of 1001, and a random block ID
            let random_block_id = HashValue::random();
            let filtered_txns = block_list_filter.filter(random_block_id, 11, 1001, txns.clone());

            // Verify that only the first two transactions are denied
            assert_eq!(filtered_txns, txns[2..].to_vec());

            // Filter transactions with a block epoch of 11, timestamp of 999, and the expected block ID
            let filtered_txns = block_list_filter.filter(block_id, 11, 999, txns.clone());

            // Verify that only the first transaction is denied
            assert_eq!(filtered_txns, txns[1..].to_vec());
        }
    }
}

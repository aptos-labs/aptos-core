// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::tests::utils;
use velor_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey, Ed25519Signature},
    HashValue, PrivateKey, SigningKey, Uniform,
};
use velor_types::{
    chain_id::ChainId,
    move_utils::MemberId,
    quorum_store::BatchId,
    transaction::{
        authenticator::{AccountAuthenticator, AnyPublicKey, TransactionAuthenticator},
        EntryFunction, Multisig, MultisigTransactionPayload, RawTransaction, Script,
        SignedTransaction, TransactionExecutable, TransactionExecutableRef, TransactionExtraConfig,
        TransactionPayload, TransactionPayloadInner,
    },
    PeerId,
};
use move_core_types::{account_address::AccountAddress, transaction_argument::TransactionArgument};
use rand::{rngs::OsRng, thread_rng, Rng};

/// Creates and returns an account authenticator with the given public key
pub fn create_account_authenticator(public_key: Ed25519PublicKey) -> AccountAuthenticator {
    AccountAuthenticator::Ed25519 {
        public_key,
        signature: Ed25519Signature::dummy_signature(),
    }
}

/// Creates and returns an entry function with the given member ID
pub fn create_entry_function(function: MemberId) -> EntryFunction {
    let MemberId {
        module_id,
        member_id: function_id,
    } = function;
    EntryFunction::new(module_id, function_id, vec![], vec![])
}

/// Creates and returns a signed transaction with an entry function payload
pub fn create_entry_function_transaction(
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

/// Creates and returns a list of signed entry function transactions
pub fn create_entry_function_transactions(
    use_new_txn_payload_format: bool,
) -> Vec<SignedTransaction> {
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

    entry_function_txns
}

/// Creates and returns a signed fee payer transaction
pub fn create_fee_payer_transaction() -> SignedTransaction {
    let entry_function = create_entry_function(str::parse("0x0::fee_payer::pay").unwrap());
    let transaction_payload = TransactionPayload::EntryFunction(entry_function);

    create_signed_transaction(transaction_payload, true)
}

/// Creates and returns a list of signed fee payer transactions
pub fn create_fee_payer_transactions() -> Vec<SignedTransaction> {
    let mut fee_payer_transactions = vec![];

    for _ in 0..10 {
        let transaction = create_fee_payer_transaction();
        fee_payer_transactions.push(transaction)
    }

    fee_payer_transactions
}

/// Creates and returns a multisig transaction with the given multisig address and function
pub fn create_multisig_transaction(
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

/// Creates and returns a list of signed multisig transactions
pub fn create_multisig_transactions(use_new_txn_payload_format: bool) -> Vec<SignedTransaction> {
    let mut multisig_transactions = vec![];

    for i in 0..10 {
        let transaction = create_multisig_transaction(
            AccountAddress::random(),
            str::parse(&format!("0x{}::multisig::sign", i)).unwrap(),
            use_new_txn_payload_format,
        );
        multisig_transactions.push(transaction);
    }

    multisig_transactions
}

/// Creates and returns a signed transaction with the given payload and fee payer
pub fn create_signed_transaction(
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

/// Creates and returns a script transaction with the given payload
pub fn create_script_transaction(use_new_txn_payload_format: bool) -> SignedTransaction {
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

/// Creates and returns a list of signed script transactions
pub fn create_script_transactions(use_new_txn_payload_format: bool) -> Vec<SignedTransaction> {
    let mut script_transactions = vec![];

    for _ in 0..10 {
        let transaction = create_script_transaction(use_new_txn_payload_format);
        script_transactions.push(transaction);
    }

    script_transactions
}

/// Returns the first address argument of the given script
pub fn get_address_argument(script: &Script) -> AccountAddress {
    for arg in script.args() {
        if let TransactionArgument::Address(address) = arg {
            return *address;
        }
    }
    panic!("No address argument found in script transaction");
}

/// Returns the public key of the authenticator of the given transaction
pub fn get_auth_public_key(signed_transaction: &SignedTransaction) -> AnyPublicKey {
    match signed_transaction.authenticator() {
        TransactionAuthenticator::Ed25519 { public_key, .. } => AnyPublicKey::ed25519(public_key),
        authenticator => panic!("Unexpected transaction authenticator: {:?}", authenticator),
    }
}

/// Returns the Ed25519 public key of the authenticator of the given transaction
pub fn get_ed25519_public_key(signed_transaction: &SignedTransaction) -> Ed25519PublicKey {
    match signed_transaction.authenticator() {
        TransactionAuthenticator::Ed25519 { public_key, .. } => public_key.clone(),
        authenticator => panic!("Unexpected transaction authenticator: {:?}", authenticator),
    }
}

/// Returns the fee payer address of the given transaction
pub fn get_fee_payer_address(signed_transaction: &SignedTransaction) -> AccountAddress {
    match signed_transaction.authenticator() {
        TransactionAuthenticator::FeePayer {
            fee_payer_address, ..
        } => fee_payer_address,
        payload => panic!("Unexpected transaction payload: {:?}", payload),
    }
}

/// Returns the function name of the given transaction
pub fn get_function_name(txn: &SignedTransaction) -> String {
    match txn.payload().executable_ref() {
        Ok(TransactionExecutableRef::EntryFunction(entry_func)) => {
            entry_func.function().to_string()
        },
        payload => panic!("Unexpected transaction payload: {:?}", payload),
    }
}

/// Returns the module address of the given transaction
pub fn get_module_address(txn: &SignedTransaction) -> AccountAddress {
    match txn.payload().executable_ref() {
        Ok(TransactionExecutableRef::EntryFunction(entry_func)) => *entry_func.module().address(),
        payload => panic!("Unexpected transaction payload: {:?}", payload),
    }
}

/// Returns the module name of the given transaction
pub fn get_module_name(txn: &SignedTransaction) -> String {
    match txn.payload().executable_ref() {
        Ok(TransactionExecutableRef::EntryFunction(entry_func)) => {
            entry_func.module().name().to_string()
        },
        payload => panic!("Unexpected transaction payload: {:?}", payload),
    }
}

/// Returns the multisig address of the given transaction
pub fn get_multisig_address(txn: &SignedTransaction) -> AccountAddress {
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

/// Creates and returns a random batch ID, author, and digest.
pub fn get_random_batch_info() -> (BatchId, PeerId, HashValue) {
    let batch_id = BatchId::new_for_test(get_random_u64());
    let batch_author = PeerId::random();
    let batch_digest = HashValue::random();
    (batch_id, batch_author, batch_digest)
}

/// Creates and returns a random block ID, author, epoch, and timestamp.
pub fn get_random_block_info() -> (HashValue, AccountAddress, u64, u64) {
    let block_id = HashValue::random();
    let block_author = AccountAddress::random();
    let block_epoch = utils::get_random_u64();
    let block_timestamp = utils::get_random_u64();
    (block_id, block_author, block_epoch, block_timestamp)
}

/// Generates and returns a random number (u64)
pub fn get_random_u64() -> u64 {
    OsRng.gen()
}

/// Returns the script argument address of the given transaction
pub fn get_script_argument_address(txn: &SignedTransaction) -> AccountAddress {
    match txn.payload() {
        TransactionPayload::Script(script) => get_address_argument(script),
        TransactionPayload::Payload(TransactionPayloadInner::V1 {
            executable: TransactionExecutable::Script(script),
            ..
        }) => get_address_argument(script),
        payload => panic!("Unexpected transaction payload: {:?}", payload),
    }
}

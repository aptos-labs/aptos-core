// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_types::transaction::{
    authenticator::AnySignature,
    user_transaction_context::{TransactionIndexKind, UserTransactionContext},
    AuxiliaryInfo, EntryFunction, Multisig, MultisigTransactionPayload, ReplayProtector, SessionId,
    SignedTransaction, TransactionExecutable, TransactionExecutableRef, TransactionPayload,
    TransactionPayloadInner,
};
use move_core_types::account_address::AccountAddress;

/// The slice of transaction metadata need by the executor.
pub(crate) struct TxnMetadata {
    pub sender: AccountAddress,
    pub fee_payer: Option<AccountAddress>,
    pub secondary_signers: Vec<AccountAddress>,
    pub sender_auth_key: Option<Vec<u8>>,
    pub fee_payer_auth_key: Option<Vec<u8>>,
    pub secondary_auth_keys: Vec<Option<Vec<u8>>>,
    pub gas_unit_price: u64,
    pub max_gas_amount: u64,
    /// Size of the full signed transaction.
    pub transaction_size: u64,
    /// Whether any signer authenticates with a keyless signature (a gas
    /// surcharge applies).
    pub is_keyless: bool,
    /// Whether any signer authenticates with an SLH-DSA signature (a gas
    /// surcharge applies).
    pub is_slh_dsa: bool,
    pub expiration_timestamp_secs: u64,
    pub chain_id: u8,
    /// Whether the transaction carries an encrypted payload.
    pub is_encrypted_txn: bool,
    pub replay_protector: ReplayProtector,
    /// Seeds unique-address generation. Derived like the legacy VM's, from the
    /// payload session's id, so generated addresses match.
    pub txn_hash: [u8; 32],
    /// Hash of the executed script, or empty when the payload is not a script.
    pub script_hash: Vec<u8>,
    /// The payload session's counter, one term of
    /// `monotonically_increasing_number`; matches the legacy VM's.
    pub session_counter: u8,
    /// The transaction's index within its block and whether it comes from block
    /// execution or validation/simulation.
    pub transaction_index_kind: TransactionIndexKind,
    /// The entry function the transaction calls directly, for the natives that
    /// inspect it.
    pub entry_function_payload: Option<EntryFunction>,
    /// The multisig account and the payload provided for it, for the natives
    /// that inspect them.
    pub multisig_payload: Option<Multisig>,
}

impl TxnMetadata {
    pub fn new(txn: &SignedTransaction, aux_info: &AuxiliaryInfo) -> Self {
        let transaction_index_kind = aux_info.transaction_index_kind();
        let script_hash = txn.payload().script_hash();
        let entry_function_payload = if txn.payload().is_multisig() {
            None
        } else if let Ok(TransactionExecutableRef::EntryFunction(entry)) =
            txn.payload().executable_ref()
        {
            Some(entry.clone())
        } else {
            None
        };
        let session_id = SessionId::txn(
            txn.sender(),
            txn.replay_protector(),
            script_hash.clone(),
            txn.expiration_timestamp_secs(),
        );
        let authenticator = txn.authenticator_ref();
        Self {
            sender: txn.sender(),
            fee_payer: authenticator.fee_payer_address(),
            secondary_signers: authenticator.secondary_signer_addresses(),
            sender_auth_key: authenticator
                .sender()
                .authentication_proof()
                .optional_auth_key(),
            fee_payer_auth_key: authenticator
                .fee_payer_signer()
                .and_then(|signer| signer.authentication_proof().optional_auth_key()),
            secondary_auth_keys: authenticator
                .secondary_signers()
                .iter()
                .map(|account_auth| account_auth.authentication_proof().optional_auth_key())
                .collect(),
            gas_unit_price: txn.gas_unit_price(),
            max_gas_amount: txn.max_gas_amount(),
            transaction_size: txn.txn_bytes_len() as u64,
            is_keyless: aptos_types::keyless::get_authenticators(txn)
                .map(|auths| !auths.is_empty())
                .unwrap_or(false),
            is_slh_dsa: authenticator
                .to_single_key_authenticators()
                .map(|auths| {
                    auths.iter().any(|auth| {
                        matches!(auth.signature(), AnySignature::SlhDsa_Sha2_128s { .. })
                    })
                })
                .unwrap_or(false),
            expiration_timestamp_secs: txn.expiration_timestamp_secs(),
            chain_id: txn.chain_id().id(),
            is_encrypted_txn: txn.is_encrypted_txn(),
            replay_protector: txn.replay_protector(),
            txn_hash: session_id.txn_hash(),
            script_hash,
            session_counter: session_id.session_counter(),
            transaction_index_kind,
            entry_function_payload,
            multisig_payload: multisig_payload(txn.payload()),
        }
    }

    pub fn is_orderless(&self) -> bool {
        matches!(self.replay_protector, ReplayProtector::Nonce(_))
    }

    /// Builds the user transaction context used by some native functions.
    pub(crate) fn as_user_transaction_context(&self) -> UserTransactionContext {
        UserTransactionContext::new(
            self.sender,
            self.secondary_signers.clone(),
            self.fee_payer.unwrap_or(self.sender),
            self.max_gas_amount,
            self.gas_unit_price,
            self.chain_id,
            self.entry_function_payload
                .as_ref()
                .map(EntryFunction::as_entry_function_payload),
            self.multisig_payload
                .as_ref()
                .map(Multisig::as_multisig_payload),
            self.transaction_index_kind,
            self.is_encrypted_txn,
            self.is_orderless(),
        )
    }
}

/// The multisig account and the payload provided for it, if the transaction
/// is a multisig transaction. Only an entry function counts as provided.
fn multisig_payload(payload: &TransactionPayload) -> Option<Multisig> {
    match payload {
        TransactionPayload::Multisig(multisig) => Some(multisig.clone()),
        TransactionPayload::Payload(TransactionPayloadInner::V1 {
            executable,
            extra_config,
        }) => extra_config
            .multisig_address()
            .map(|multisig_address| Multisig {
                multisig_address,
                transaction_payload: match executable {
                    TransactionExecutable::EntryFunction(entry) => {
                        Some(MultisigTransactionPayload::EntryFunction(entry.clone()))
                    },
                    TransactionExecutable::Script(_)
                    | TransactionExecutable::Empty
                    | TransactionExecutable::Encrypted => None,
                },
            }),
        TransactionPayload::EncryptedPayload(encrypted) => encrypted
            .extra_config()
            .multisig_address()
            .map(|multisig_address| Multisig {
                multisig_address,
                transaction_payload: match encrypted.executable_ref() {
                    Ok(TransactionExecutableRef::EntryFunction(entry)) => {
                        Some(MultisigTransactionPayload::EntryFunction(entry.clone()))
                    },
                    Ok(
                        TransactionExecutableRef::Script(_)
                        | TransactionExecutableRef::Empty
                        | TransactionExecutableRef::Encrypted,
                    )
                    | Err(_) => None,
                },
            }),
        TransactionPayload::Script(_)
        | TransactionPayload::ModuleBundle(_)
        | TransactionPayload::EntryFunction(_) => None,
    }
}

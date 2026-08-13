// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_types::transaction::{
    PersistedAuxiliaryInfo, ReplayProtector, SessionId, SignedTransaction,
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
    pub expiration_timestamp_secs: u64,
    pub chain_id: u8,
    pub replay_protector: ReplayProtector,
    /// Seeds unique-address generation. Derived like the legacy VM's, from the
    /// payload session's id, so generated addresses match.
    pub txn_hash: Vec<u8>,
    /// The payload session's counter, one term of
    /// `monotonically_increasing_number`; matches the legacy VM's.
    pub session_counter: u8,
    /// The transaction's index in its block, another term of
    /// `monotonically_increasing_number`.
    pub transaction_index: u32,
    /// Distinguishes block execution (0) from validation/simulation (1) in
    /// `monotonically_increasing_number`.
    pub reserved_byte: u8,
}

impl TxnMetadata {
    pub fn new(txn: &SignedTransaction, aux_info: PersistedAuxiliaryInfo) -> Self {
        let (transaction_index, reserved_byte) = match aux_info {
            PersistedAuxiliaryInfo::V1 { transaction_index } => (transaction_index, 0),
            PersistedAuxiliaryInfo::TimestampNotYetAssignedV1 { transaction_index } => {
                (transaction_index, 1)
            },
            PersistedAuxiliaryInfo::None => (0, 0),
        };
        let session_id = SessionId::txn(
            txn.sender(),
            txn.replay_protector(),
            txn.payload().script_hash(),
            txn.expiration_timestamp_secs(),
        );
        Self {
            sender: txn.sender(),
            fee_payer: txn.authenticator_ref().fee_payer_address(),
            secondary_signers: txn.authenticator().secondary_signer_addresses(),
            sender_auth_key: txn
                .authenticator()
                .sender()
                .authentication_proof()
                .optional_auth_key(),
            fee_payer_auth_key: txn
                .authenticator()
                .fee_payer_signer()
                .and_then(|signer| signer.authentication_proof().optional_auth_key()),
            secondary_auth_keys: txn
                .authenticator()
                .secondary_signers()
                .iter()
                .map(|account_auth| account_auth.authentication_proof().optional_auth_key())
                .collect(),
            gas_unit_price: txn.gas_unit_price(),
            max_gas_amount: txn.max_gas_amount(),
            expiration_timestamp_secs: txn.expiration_timestamp_secs(),
            chain_id: txn.chain_id().id(),
            replay_protector: txn.replay_protector(),
            txn_hash: session_id.txn_hash().to_vec(),
            session_counter: session_id.session_counter(),
            transaction_index,
            reserved_byte,
        }
    }

    pub fn is_orderless(&self) -> bool {
        matches!(self.replay_protector, ReplayProtector::Nonce(_))
    }
}

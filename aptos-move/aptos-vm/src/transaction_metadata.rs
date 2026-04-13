// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::AptosVM;
use aptos_crypto::HashValue;
use aptos_gas_algebra::{FeePerGasUnit, Gas, NumBytes};
use aptos_types::{
    account_address::AccountAddress,
    chain_id::ChainId,
    on_chain_config::{ApprovedExecutionHashes, ConfigStorage, OnChainConfig, TimedFeatureFlag},
    transaction::{
        authenticator::AuthenticationProof,
        user_transaction_context::{TransactionIndexKind, UserTransactionContext},
        AuxiliaryInfo, EntryFunction, Multisig, MultisigTransactionPayload, ReplayProtector,
        SignedTransaction, TransactionExecutable, TransactionExecutableRef, TransactionExtraConfig,
        TransactionPayload, TransactionPayloadInner, TxnLimitsRequest,
    },
};
use move_core_types::vm_status::{StatusCode, VMStatus};

pub struct TransactionMetadata {
    pub sender: AccountAddress,
    pub authentication_proof: AuthenticationProof,
    pub secondary_signers: Vec<AccountAddress>,
    pub secondary_authentication_proofs: Vec<AuthenticationProof>,
    pub replay_protector: ReplayProtector,
    pub fee_payer: Option<AccountAddress>,
    /// `None` if the [TransactionAuthenticator] lacks an authenticator for the fee payer.
    /// `Some([])` if the authenticator for the fee payer is a [NoAccountAuthenticator].
    pub fee_payer_authentication_proof: Option<AuthenticationProof>,
    pub max_gas_amount: Gas,
    pub gas_unit_price: FeePerGasUnit,
    pub transaction_size: NumBytes,
    pub expiration_timestamp_secs: u64,
    pub chain_id: ChainId,
    pub script_hash: Vec<u8>,
    pub script_size: NumBytes,
    pub is_keyless: bool,
    pub is_slh_dsa_sha2_128s: bool,
    pub entry_function_payload: Option<EntryFunction>,
    pub multisig_payload: Option<Multisig>,
    /// The transaction index context for the monotonically increasing counter.
    pub transaction_index_kind: TransactionIndexKind,
    /// Transaction limits request (requested by user or set by the system).
    pub txn_limits: Option<TxnLimitsRequest>,
}

impl TransactionMetadata {
    pub(crate) fn new(
        vm: &AptosVM,
        resolver: &impl ConfigStorage,
        txn: &SignedTransaction,
        auxiliary_info: &AuxiliaryInfo,
    ) -> Result<Self, VMStatus> {
        // Use full transaction size (including authenticator) when the feature is enabled,
        // otherwise use the raw transaction size for backwards compatibility.
        let transaction_size = if vm
            .timed_features()
            .is_enabled(TimedFeatureFlag::UseFullTransactionSizeForGasCheck)
        {
            txn.txn_bytes_len()
        } else {
            txn.raw_txn_bytes_len()
        };

        let (script_hash, is_approved_gov_script) =
            if let Ok(TransactionExecutableRef::Script(s)) = txn.payload().executable_ref() {
                let script_hash = HashValue::sha3_256_of(s.code()).to_vec();
                let is_approved_gov_script = ApprovedExecutionHashes::fetch_config(resolver)
                    .is_some_and(|approved| {
                        approved
                            .entries
                            .iter()
                            .any(|(_, hash)| hash == &script_hash)
                    });
                (script_hash, is_approved_gov_script)
            } else {
                (vec![], false)
            };

        let txn_limits = if is_approved_gov_script {
            Some(TxnLimitsRequest::ApprovedGovernanceScript)
        } else if let Some(request) = txn.extra_config().txn_limits_request() {
            if !vm.features().is_transaction_limits_enabled() {
                return Err(VMStatus::error(
                    StatusCode::FEATURE_UNDER_GATING,
                    Some("Transaction limits feature is not yet supported".to_string()),
                ));
            }

            // This runs before the prologue & gas meter creation, preventing the
            // meter from operating with a 0, nonsensical, or overflowing limit. The
            // Move prologue additionally validates that the multiplier exists in the
            // on-chain config.
            //
            // INVARIANT: these bounds must match Move constants defined in
            // transaction_limits.move.
            const MIN_MULTIPLIER_BPS: u64 = 100;
            const MAX_MULTIPLIER_BPS: u64 = 10000;

            let m = request.multipliers();
            if m.execution_bps() <= MIN_MULTIPLIER_BPS
                || MAX_MULTIPLIER_BPS < m.execution_bps()
                || m.io_bps() <= MIN_MULTIPLIER_BPS
                || MAX_MULTIPLIER_BPS < m.io_bps()
            {
                return Err(VMStatus::error(
                    StatusCode::INVALID_HIGH_TXN_LIMITS_MULTIPLIER,
                    Some("Multipliers must be in (1x, 100x] range".to_string()),
                ));
            }
            Some(TxnLimitsRequest::Staking(request.clone()))
        } else {
            None
        };

        Ok(Self {
            sender: txn.sender(),
            authentication_proof: txn.authenticator().sender().authentication_proof(),
            secondary_signers: txn.authenticator().secondary_signer_addresses(),
            secondary_authentication_proofs: txn
                .authenticator()
                .secondary_signers()
                .iter()
                .map(|account_auth| account_auth.authentication_proof())
                .collect(),
            replay_protector: txn.replay_protector(),
            fee_payer: txn.authenticator_ref().fee_payer_address(),
            fee_payer_authentication_proof: txn
                .authenticator()
                .fee_payer_signer()
                .map(|signer| signer.authentication_proof()),
            max_gas_amount: txn.max_gas_amount().into(),
            gas_unit_price: txn.gas_unit_price().into(),
            transaction_size: (transaction_size as u64).into(),
            expiration_timestamp_secs: txn.expiration_timestamp_secs(),
            chain_id: txn.chain_id(),
            script_hash,
            script_size: if let Ok(TransactionExecutableRef::Script(s)) =
                txn.payload().executable_ref()
            {
                (s.code().len() as u64).into()
            } else {
                NumBytes::zero()
            },
            is_keyless: aptos_types::keyless::get_authenticators(txn)
                .map(|res| !res.is_empty())
                .unwrap_or(false),
            is_slh_dsa_sha2_128s: txn
                .authenticator_ref()
                .to_single_key_authenticators()
                .map(|authenticators| {
                    authenticators
                        .iter()
                        .any(|auth| matches!(auth.signature(), aptos_types::transaction::authenticator::AnySignature::SlhDsa_Sha2_128s { .. }))
                })
                .unwrap_or(false),
            entry_function_payload: if txn.payload().is_multisig() {
                None
            } else if let Ok(TransactionExecutableRef::EntryFunction(e)) =
                txn.payload().executable_ref()
            {
                Some(e.clone())
            } else {
                None
            },
            multisig_payload: match txn.payload() {
                TransactionPayload::Multisig(m) => Some(m.clone()),
                TransactionPayload::Payload(TransactionPayloadInner::V1 {
                    executable,
                    extra_config:
                        TransactionExtraConfig::V1 {
                            multisig_address: Some(multisig_address),
                            ..
                        } | TransactionExtraConfig::V2 {
                            multisig_address: Some(multisig_address),
                            ..
                        }
                }) => Some(Multisig {
                    multisig_address: *multisig_address,
                    transaction_payload: match executable {
                        TransactionExecutable::EntryFunction(e) => {
                            // TODO[Orderless]: How to avoid the clone operation here.
                            Some(MultisigTransactionPayload::EntryFunction(e.clone()))
                        },
                        _ => None,
                    },
                }),
                TransactionPayload::EncryptedPayload(ep) => {
                    ep.extra_config().multisig_address().map(|multisig_address| Multisig {
                        multisig_address,
                        transaction_payload: match ep.executable_ref() {
                            Ok(TransactionExecutableRef::EntryFunction(e)) => {
                                Some(MultisigTransactionPayload::EntryFunction(e.clone()))
                            },
                            _ => None,
                        },
                    })
                },
                TransactionPayload::Payload(TransactionPayloadInner::V1 { extra_config: TransactionExtraConfig::V1 { multisig_address: None, ..} | TransactionExtraConfig::V2 { multisig_address: None, ..}, .. }) |
                TransactionPayload::Script(_) |
                TransactionPayload::ModuleBundle(_) | TransactionPayload::EntryFunction(_) => None,
            },
            transaction_index_kind: auxiliary_info.transaction_index_kind(),
            txn_limits,
        })
    }

    pub fn max_gas_amount(&self) -> Gas {
        self.max_gas_amount
    }

    pub fn gas_unit_price(&self) -> FeePerGasUnit {
        self.gas_unit_price
    }

    pub fn fee_payer(&self) -> Option<AccountAddress> {
        self.fee_payer.to_owned()
    }

    pub fn script_size(&self) -> NumBytes {
        self.script_size
    }

    pub fn sender(&self) -> AccountAddress {
        self.sender.to_owned()
    }

    pub fn secondary_signers(&self) -> Vec<AccountAddress> {
        self.secondary_signers.to_owned()
    }

    pub fn senders(&self) -> Vec<AccountAddress> {
        let mut senders = vec![self.sender()];
        senders.extend(self.secondary_signers());
        senders
    }

    pub fn authentication_proofs(&self) -> Vec<&AuthenticationProof> {
        let mut proofs = vec![self.authentication_proof()];
        proofs.extend(self.secondary_authentication_proofs.iter());
        proofs
    }

    pub fn authentication_proof(&self) -> &AuthenticationProof {
        &self.authentication_proof
    }

    pub fn replay_protector(&self) -> ReplayProtector {
        self.replay_protector
    }

    pub fn is_orderless(&self) -> bool {
        match self.replay_protector {
            ReplayProtector::SequenceNumber(_) => false,
            ReplayProtector::Nonce(_) => true,
        }
    }

    pub fn transaction_size(&self) -> NumBytes {
        self.transaction_size
    }

    pub fn expiration_timestamp_secs(&self) -> u64 {
        self.expiration_timestamp_secs
    }

    pub fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    pub fn is_multi_agent(&self) -> bool {
        !self.secondary_signers.is_empty() || self.fee_payer.is_some()
    }

    pub fn is_keyless(&self) -> bool {
        self.is_keyless
    }

    pub fn is_slh_dsa_sha2_128s(&self) -> bool {
        self.is_slh_dsa_sha2_128s
    }

    pub fn entry_function_payload(&self) -> Option<EntryFunction> {
        self.entry_function_payload.clone()
    }

    pub fn multisig_payload(&self) -> Option<Multisig> {
        self.multisig_payload.clone()
    }

    pub fn as_user_transaction_context(&self) -> UserTransactionContext {
        UserTransactionContext::new(
            self.sender,
            self.secondary_signers.clone(),
            self.fee_payer.unwrap_or(self.sender),
            self.max_gas_amount.into(),
            self.gas_unit_price.into(),
            self.chain_id.id(),
            self.entry_function_payload()
                .map(|entry_func| entry_func.as_entry_function_payload()),
            self.multisig_payload()
                .map(|multisig| multisig.as_multisig_payload()),
            self.transaction_index_kind,
        )
    }

    pub fn transaction_index_kind(&self) -> TransactionIndexKind {
        self.transaction_index_kind
    }

    /// Returns the transaction index if available, regardless of execution mode.
    /// Returns `Some(index)` for both `BlockExecution` and `ValidationOrSimulation`,
    /// Returns `None` for `NotAvailable`.
    pub fn transaction_index(&self) -> Option<u32> {
        match self.transaction_index_kind {
            TransactionIndexKind::BlockExecution { transaction_index }
            | TransactionIndexKind::ValidationOrSimulation { transaction_index } => {
                Some(transaction_index)
            },
            TransactionIndexKind::NotAvailable => None,
        }
    }

    /// Returns true if this is a governance proposal script that was approved.
    pub fn is_approved_gov_script(&self) -> bool {
        self.txn_limits
            .as_ref()
            .is_some_and(|r| matches!(r, TxnLimitsRequest::ApprovedGovernanceScript))
    }
}

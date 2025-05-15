// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use aptos_gas_algebra::{FeePerGasUnit, Gas, NumBytes};
use aptos_types::{
    account_address::AccountAddress,
    chain_id::ChainId,
    transaction::{
        authenticator::AuthenticationProof, user_transaction_context::UserTransactionContext,
        EntryFunction, Multisig, MultisigTransactionPayload, ReplayProtector, SignedTransaction,
        TransactionExecutable, TransactionExecutableRef, TransactionExtraConfig,
        TransactionPayload, TransactionPayloadInner,
    },
};

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
    pub entry_function_payload: Option<EntryFunction>,
    pub multisig_payload: Option<Multisig>,
}

impl TransactionMetadata {
    pub fn new(txn: &SignedTransaction) -> Self {
        Self {
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
            transaction_size: (txn.raw_txn_bytes_len() as u64).into(),
            expiration_timestamp_secs: txn.expiration_timestamp_secs(),
            chain_id: txn.chain_id(),
            script_hash: if let Ok(TransactionExecutableRef::Script(s)) =
                txn.payload().executable_ref()
            {
                HashValue::sha3_256_of(s.code()).to_vec()
            } else {
                vec![]
            },
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
                        },
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
                _ => None,
            },
        }
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
        )
    }
}

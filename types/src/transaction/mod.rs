// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::arc_with_non_send_sync)]

use crate::{
    account_address::AccountAddress,
    block_metadata::BlockMetadata,
    chain_id::ChainId,
    contract_event::{ContractEvent, FEE_STATEMENT_EVENT_TYPE},
    keyless::{KeylessPublicKey, KeylessSignature},
    ledger_info::LedgerInfo,
    proof::{TransactionInfoListWithProof, TransactionInfoWithProof},
    transaction::authenticator::{
        AccountAuthenticator, AnyPublicKey, AnySignature, SingleKeyAuthenticator,
        TransactionAuthenticator,
    },
    vm_status::{DiscardedVMStatus, KeptVMStatus, StatusCode, StatusType, VMStatus},
    write_set::WriteSet,
};
use anyhow::{ensure, format_err, Context, Error, Result};
use aptos_crypto::{
    ed25519::*,
    hash::CryptoHash,
    multi_ed25519::{MultiEd25519PublicKey, MultiEd25519Signature},
    secp256k1_ecdsa,
    traits::{signing_message, SigningKey},
    CryptoMaterialError, HashValue,
};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use rand::Rng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    convert::TryFrom,
    fmt::{self, Debug, Display, Formatter},
};

pub mod analyzed_transaction;
pub mod authenticator;
pub mod block_epilogue;
mod block_output;
mod change_set;
mod module;
mod multisig;
mod script;
pub mod signature_verified_transaction;
pub mod use_case;
pub mod user_transaction_context;
pub mod webauthn;

pub use self::block_epilogue::{
    BlockEndInfo, BlockEndInfoExt, BlockEpiloguePayload, FeeDistribution,
};
use crate::{
    block_metadata_ext::BlockMetadataExt,
    contract_event::TransactionEvent,
    executable::ModulePath,
    fee_statement::FeeStatement,
    function_info::FunctionInfo,
    keyless::FederatedKeylessPublicKey,
    on_chain_config::{FeatureFlag, Features},
    proof::accumulator::InMemoryEventAccumulator,
    state_store::{state_key::StateKey, state_value::StateValue},
    validator_txn::ValidatorTransaction,
    write_set::TransactionWrite,
};
pub use block_output::BlockOutput;
pub use change_set::ChangeSet;
pub use module::{Module, ModuleBundle};
pub use move_core_types::transaction_argument::TransactionArgument;
use move_core_types::{
    value::{MoveStruct, MoveValue},
    vm_status::AbortLocation,
};
pub use multisig::{ExecutionError, Multisig, MultisigTransactionPayload};
use once_cell::sync::OnceCell;
pub use script::{
    ArgumentABI, EntryABI, EntryFunction, EntryFunctionABI, Script, TransactionScriptABI,
    TypeArgumentABI,
};
use serde::de::DeserializeOwned;
use std::{
    collections::BTreeSet,
    hash::Hash,
    ops::Deref,
    sync::{atomic::AtomicU64, Arc},
};

pub type Version = u64; // Height - also used for MVCC in StateDB
pub type AtomicVersion = AtomicU64;

#[derive(Clone)]
pub enum Auth<'a> {
    Ed25519(&'a Ed25519PrivateKey),
    Abstraction(FunctionInfo, Arc<dyn Fn(&[u8]) -> Vec<u8>>),
    DerivableAbstraction {
        function_info: FunctionInfo,
        account_identity: Vec<u8>,
        sign_function: Arc<dyn Fn(&[u8]) -> Vec<u8>>,
    },
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ReplayProtector {
    Nonce(u64),
    SequenceNumber(u64),
}

impl Display for ReplayProtector {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ReplayProtector::Nonce(nonce) => write!(f, "Nonce({})", nonce),
            ReplayProtector::SequenceNumber(sequence_number) => {
                write!(f, "SequenceNumber({})", sequence_number)
            },
        }
    }
}

impl ReplayProtector {
    pub fn get_nonce(&self) -> Option<u64> {
        match self {
            ReplayProtector::Nonce(nonce) => Some(*nonce),
            ReplayProtector::SequenceNumber(_) => None,
        }
    }

    pub fn get_sequence_number(&self) -> Option<u64> {
        match self {
            ReplayProtector::Nonce(_) => None,
            ReplayProtector::SequenceNumber(sequence_number) => Some(*sequence_number),
        }
    }

    pub fn to_move_value(&self) -> MoveValue {
        match self {
            ReplayProtector::Nonce(nonce) => {
                MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![MoveValue::U64(*nonce)]))
            },
            ReplayProtector::SequenceNumber(sequence_number) => MoveValue::Struct(
                MoveStruct::RuntimeVariant(1, vec![MoveValue::U64(*sequence_number)]),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_replay_protector_order() {
        let nonce = ReplayProtector::Nonce(1);
        let sequence_number = ReplayProtector::SequenceNumber(1);
        assert!(nonce < sequence_number);

        let nonce = ReplayProtector::Nonce(2);
        let sequence_number = ReplayProtector::SequenceNumber(1);
        assert!(nonce < sequence_number);

        let sequence_number1 = ReplayProtector::SequenceNumber(3);
        let sequence_number2 = ReplayProtector::SequenceNumber(4);
        assert!(sequence_number1 < sequence_number2);
    }
}

/// RawTransaction is the portion of a transaction that a client signs.
#[derive(
    Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash,
)]
pub struct RawTransaction {
    /// Sender's address.
    sender: AccountAddress,

    /// Sequence number of this transaction. This must match the sequence number
    /// stored in the sender's account at the time the transaction executes.
    sequence_number: u64,

    /// The transaction payload, e.g., a script to execute.
    payload: TransactionPayload,

    /// Maximal total gas to spend for this transaction.
    max_gas_amount: u64,

    /// Price to be paid per gas unit.
    gas_unit_price: u64,

    /// Expiration timestamp for this transaction, represented
    /// as seconds from the Unix Epoch. If the current blockchain timestamp
    /// is greater than or equal to this time, then the transaction has
    /// expired and will be discarded. This can be set to a large value far
    /// in the future to indicate that a transaction does not expire.
    expiration_timestamp_secs: u64,

    /// Chain ID of the Aptos network this transaction is intended for.
    chain_id: ChainId,
}

impl RawTransaction {
    /// Create a new `RawTransaction` with a payload.
    ///
    /// It can be either to publish a module, to execute a script, or to issue a writeset
    /// transaction.
    pub fn new(
        sender: AccountAddress,
        sequence_number: u64,
        payload: TransactionPayload,
        max_gas_amount: u64,
        gas_unit_price: u64,
        expiration_timestamp_secs: u64,
        chain_id: ChainId,
    ) -> Self {
        RawTransaction {
            sender,
            sequence_number,
            payload,
            max_gas_amount,
            gas_unit_price,
            expiration_timestamp_secs,
            chain_id,
        }
    }

    // TODO[Orderless]: Deprecate this
    /// Create a new `RawTransaction` with a script.
    ///
    /// A script transaction contains only code to execute. No publishing is allowed in scripts.
    pub fn new_script(
        sender: AccountAddress,
        sequence_number: u64,
        script: Script,
        max_gas_amount: u64,
        gas_unit_price: u64,
        expiration_timestamp_secs: u64,
        chain_id: ChainId,
    ) -> Self {
        RawTransaction {
            sender,
            sequence_number,
            payload: TransactionPayload::Script(script),
            max_gas_amount,
            gas_unit_price,
            expiration_timestamp_secs,
            chain_id,
        }
    }

    // TODO[Orderless]: Deprecate this
    /// Create a new `RawTransaction` with an entry function.
    pub fn new_entry_function(
        sender: AccountAddress,
        sequence_number: u64,
        entry_function: EntryFunction,
        max_gas_amount: u64,
        gas_unit_price: u64,
        expiration_timestamp_secs: u64,
        chain_id: ChainId,
    ) -> Self {
        RawTransaction {
            sender,
            sequence_number,
            payload: TransactionPayload::EntryFunction(entry_function),
            max_gas_amount,
            gas_unit_price,
            expiration_timestamp_secs,
            chain_id,
        }
    }

    // TODO[Orderless]: Deprecate this
    /// Create a new `RawTransaction` of multisig type.
    pub fn new_multisig(
        sender: AccountAddress,
        sequence_number: u64,
        multisig: Multisig,
        max_gas_amount: u64,
        gas_unit_price: u64,
        expiration_timestamp_secs: u64,
        chain_id: ChainId,
    ) -> Self {
        RawTransaction {
            sender,
            sequence_number,
            payload: TransactionPayload::Multisig(multisig),
            max_gas_amount,
            gas_unit_price,
            expiration_timestamp_secs,
            chain_id,
        }
    }

    // TODO[Orderless]: After the new transaction format is fully adopted, make `new_txn` as the default
    // function to create new RawTransaction, and remove other `new_..` variants.
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_txn(
        sender: AccountAddress,
        replay_protector: ReplayProtector,
        executable: TransactionExecutable,
        multisig_address: Option<AccountAddress>,
        max_gas_amount: u64,
        gas_unit_price: u64,
        expiration_timestamp_secs: u64,
        chain_id: ChainId,
    ) -> Self {
        match replay_protector {
            ReplayProtector::SequenceNumber(sequence_number) => RawTransaction {
                sender,
                sequence_number,
                payload: TransactionPayload::Payload(TransactionPayloadInner::V1 {
                    executable,
                    extra_config: TransactionExtraConfig::V1 {
                        multisig_address,
                        replay_protection_nonce: None,
                    },
                }),
                max_gas_amount,
                gas_unit_price,
                expiration_timestamp_secs,
                chain_id,
            },
            ReplayProtector::Nonce(nonce) => RawTransaction {
                sender,
                sequence_number: u64::MAX,
                payload: TransactionPayload::Payload(TransactionPayloadInner::V1 {
                    executable,
                    extra_config: TransactionExtraConfig::V1 {
                        multisig_address,
                        replay_protection_nonce: Some(nonce),
                    },
                }),
                max_gas_amount,
                gas_unit_price,
                expiration_timestamp_secs,
                chain_id,
            },
        }
    }

    /// Signs the given `RawTransaction`. Note that this consumes the `RawTransaction` and turns it
    /// into a `SignatureCheckedTransaction`.
    ///
    /// For a transaction that has just been signed, its signature is expected to be valid.
    pub fn sign(
        self,
        private_key: &Ed25519PrivateKey,
        public_key: Ed25519PublicKey,
    ) -> Result<SignatureCheckedTransaction> {
        let signature = private_key.sign(&self)?;
        Ok(SignatureCheckedTransaction(SignedTransaction::new(
            self, public_key, signature,
        )))
    }

    /// Signs the given multi-agent `RawTransaction`, which is a transaction with secondary
    /// signers in addition to a sender. The private keys of the sender and the
    /// secondary signers are used to sign the transaction.
    ///
    /// The order and length of the secondary keys provided here have to match the order and
    /// length of the `secondary_signers`.
    pub fn sign_multi_agent(
        self,
        sender_private_key: &Ed25519PrivateKey,
        secondary_signers: Vec<AccountAddress>,
        secondary_private_keys: Vec<&Ed25519PrivateKey>,
    ) -> Result<SignatureCheckedTransaction> {
        let message =
            RawTransactionWithData::new_multi_agent(self.clone(), secondary_signers.clone());
        let sender_signature = sender_private_key.sign(&message)?;
        let sender_authenticator = AccountAuthenticator::ed25519(
            Ed25519PublicKey::from(sender_private_key),
            sender_signature,
        );

        if secondary_private_keys.len() != secondary_signers.len() {
            return Err(format_err!(
                "number of secondary private keys and number of secondary signers don't match"
            ));
        }
        let mut secondary_authenticators = vec![];
        for priv_key in secondary_private_keys {
            let signature = priv_key.sign(&message)?;
            secondary_authenticators.push(AccountAuthenticator::ed25519(
                Ed25519PublicKey::from(priv_key),
                signature,
            ));
        }

        Ok(SignatureCheckedTransaction(
            SignedTransaction::new_multi_agent(
                self,
                sender_authenticator,
                secondary_signers,
                secondary_authenticators,
            ),
        ))
    }

    /// Signs the given fee-payer `RawTransaction`, which is a transaction with secondary
    /// signers and a gas payer in addition to a sender. The private keys of the sender, the
    /// secondary signers, and gas payer signer are used to sign the transaction.
    ///
    /// The order and length of the secondary keys provided here have to match the order and
    /// length of the `secondary_signers`.
    pub fn sign_fee_payer(
        self,
        sender_private_key: &Ed25519PrivateKey,
        secondary_signers: Vec<AccountAddress>,
        secondary_private_keys: Vec<&Ed25519PrivateKey>,
        fee_payer_address: AccountAddress,
        fee_payer_private_key: &Ed25519PrivateKey,
    ) -> Result<SignatureCheckedTransaction> {
        let message = RawTransactionWithData::new_fee_payer(
            self.clone(),
            secondary_signers.clone(),
            fee_payer_address,
        );
        let sender_signature = sender_private_key.sign(&message)?;
        let sender_authenticator = AccountAuthenticator::ed25519(
            Ed25519PublicKey::from(sender_private_key),
            sender_signature,
        );

        if secondary_private_keys.len() != secondary_signers.len() {
            return Err(format_err!(
                "number of secondary private keys and number of secondary signers don't match"
            ));
        }
        let mut secondary_authenticators = vec![];
        for priv_key in secondary_private_keys {
            let signature = priv_key.sign(&message)?;
            secondary_authenticators.push(AccountAuthenticator::ed25519(
                Ed25519PublicKey::from(priv_key),
                signature,
            ));
        }

        let fee_payer_signature = fee_payer_private_key.sign(&message)?;
        let fee_payer_authenticator = AccountAuthenticator::ed25519(
            Ed25519PublicKey::from(fee_payer_private_key),
            fee_payer_signature,
        );

        Ok(SignatureCheckedTransaction(
            SignedTransaction::new_fee_payer(
                self,
                sender_authenticator,
                secondary_signers,
                secondary_authenticators,
                fee_payer_address,
                fee_payer_authenticator,
            ),
        ))
    }

    pub fn sign_aa_transaction(
        self,
        sender_auth: Auth,
        secondary_signers: Vec<AccountAddress>,
        secondary_auths: Vec<Auth>,
        fee_payer: Option<(AccountAddress, Auth)>,
    ) -> Result<SignatureCheckedTransaction> {
        let user_signed_message = if fee_payer.is_some() {
            RawTransactionWithData::new_fee_payer(
                self.clone(),
                secondary_signers.clone(),
                AccountAddress::ZERO,
            )
        } else {
            RawTransactionWithData::new_multi_agent(self.clone(), secondary_signers.clone())
        };
        let sender_authenticator = gen_auth(sender_auth, &user_signed_message)?;

        if secondary_auths.len() != secondary_signers.len() {
            return Err(format_err!(
                "number of secondary private keys and number of secondary signers don't match"
            ));
        }
        let mut secondary_authenticators = vec![];
        for auth in secondary_auths {
            let secondary_authenticator = gen_auth(auth, &user_signed_message)?;
            secondary_authenticators.push(secondary_authenticator);
        }

        if let Some((fee_payer_address, fee_payer_auth)) = fee_payer {
            let user_signed_message = RawTransactionWithData::new_fee_payer(
                self.clone(),
                secondary_signers.clone(),
                fee_payer_address,
            );
            let fee_payer_authenticator = gen_auth(fee_payer_auth, &user_signed_message)?;
            Ok(SignatureCheckedTransaction(
                SignedTransaction::new_fee_payer(
                    self,
                    sender_authenticator,
                    secondary_signers,
                    secondary_authenticators,
                    fee_payer_address,
                    fee_payer_authenticator,
                ),
            ))
        } else {
            Ok(SignatureCheckedTransaction(
                SignedTransaction::new_multi_agent(
                    self,
                    sender_authenticator,
                    secondary_signers,
                    secondary_authenticators,
                ),
            ))
        }
    }

    /// Signs the given `RawTransaction`. Note that this consumes the `RawTransaction` and turns it
    /// into a `SignatureCheckedTransaction`.
    ///
    /// For a transaction that has just been signed, its signature is expected to be valid.
    pub fn sign_secp256k1_ecdsa(
        self,
        private_key: &secp256k1_ecdsa::PrivateKey,
        public_key: secp256k1_ecdsa::PublicKey,
    ) -> Result<SignatureCheckedTransaction> {
        let signature = private_key.sign(&self)?;
        Ok(SignatureCheckedTransaction(
            SignedTransaction::new_secp256k1_ecdsa(self, public_key, signature),
        ))
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn multi_sign_for_testing(
        self,
        private_key: &Ed25519PrivateKey,
        public_key: Ed25519PublicKey,
    ) -> Result<SignatureCheckedTransaction> {
        let signature = private_key.sign(&self)?;
        Ok(SignatureCheckedTransaction(
            SignedTransaction::new_multisig(self, public_key.into(), signature.into()),
        ))
    }

    pub fn into_payload(self) -> TransactionPayload {
        self.payload
    }

    pub fn executable_ref(&self) -> Result<TransactionExecutableRef> {
        self.payload.executable_ref()
    }

    pub fn extra_config(&self) -> TransactionExtraConfig {
        self.payload.extra_config()
    }

    pub fn replay_protector(&self) -> ReplayProtector {
        if let Some(nonce) = self.payload.replay_protection_nonce() {
            ReplayProtector::Nonce(nonce)
        } else {
            ReplayProtector::SequenceNumber(self.sequence_number)
        }
    }

    pub fn set_gas_unit_price(&mut self, gas_unit_price: u64) {
        self.gas_unit_price = gas_unit_price;
    }

    /// Return the sender of this transaction.
    pub fn sender(&self) -> AccountAddress {
        self.sender
    }

    /// Return the signing message for creating transaction signature.
    pub fn signing_message(&self) -> Result<Vec<u8>, CryptoMaterialError> {
        signing_message(self)
    }
}

fn gen_auth(
    auth: Auth,
    user_signed_message: &RawTransactionWithData,
) -> Result<AccountAuthenticator> {
    Ok(match auth {
        Auth::Ed25519(private_key) => {
            let sender_signature = private_key.sign(user_signed_message)?;
            AccountAuthenticator::ed25519(Ed25519PublicKey::from(private_key), sender_signature)
        },
        Auth::Abstraction(function_info, sign_function) => {
            let digest =
                HashValue::sha3_256_of(signing_message(user_signed_message)?.as_slice()).to_vec();
            AccountAuthenticator::abstraction(
                function_info.clone(),
                digest.clone(),
                sign_function(digest.as_ref()),
            )
        },
        Auth::DerivableAbstraction {
            function_info,
            account_identity,
            sign_function,
        } => {
            let digest =
                HashValue::sha3_256_of(signing_message(user_signed_message)?.as_slice()).to_vec();
            AccountAuthenticator::derivable_abstraction(
                function_info.clone(),
                digest.clone(),
                sign_function(digest.as_ref()),
                account_identity.clone(),
            )
        },
    })
}

#[derive(
    Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash,
)]
pub enum RawTransactionWithData {
    MultiAgent {
        raw_txn: RawTransaction,
        secondary_signer_addresses: Vec<AccountAddress>,
    },
    MultiAgentWithFeePayer {
        raw_txn: RawTransaction,
        secondary_signer_addresses: Vec<AccountAddress>,
        fee_payer_address: AccountAddress,
    },
}

impl RawTransactionWithData {
    pub fn new_fee_payer(
        raw_txn: RawTransaction,
        secondary_signer_addresses: Vec<AccountAddress>,
        fee_payer_address: AccountAddress,
    ) -> Self {
        Self::MultiAgentWithFeePayer {
            raw_txn,
            secondary_signer_addresses,
            fee_payer_address,
        }
    }

    pub fn new_multi_agent(
        raw_txn: RawTransaction,
        secondary_signer_addresses: Vec<AccountAddress>,
    ) -> Self {
        Self::MultiAgent {
            raw_txn,
            secondary_signer_addresses,
        }
    }
}

/// Marks payload as deprecated. We need to use it to ensure serialization or
/// deserialization is not broken.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct DeprecatedPayload {
    // Used because 'analyze_serde_formats' complains with "Please avoid 0-sized containers".
    dummy_value: u64,
}

/// Different kinds of transactions.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum TransactionPayload {
    /// A transaction that executes code.
    Script(Script),
    /// Deprecated.
    ModuleBundle(DeprecatedPayload),
    /// A transaction that executes an existing entry function published on-chain.
    EntryFunction(EntryFunction),
    /// A multisig transaction that allows an owner of a multisig account to execute a pre-approved
    /// transaction as the multisig account.
    Multisig(Multisig),
    /// A new transaction payload format with support for versioning.
    /// Contains an executable (script/entry function) along with extra configuration.
    /// Once this new format is fully rolled out, above payload variants will be deprecated.
    Payload(TransactionPayloadInner),
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum TransactionPayloadInner {
    V1 {
        executable: TransactionExecutable,
        extra_config: TransactionExtraConfig,
    },
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum TransactionExecutable {
    Script(Script),
    EntryFunction(EntryFunction),
    Empty,
}

impl TransactionExecutable {
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    pub fn is_script(&self) -> bool {
        matches!(self, Self::Script(_))
    }

    pub fn is_entry_function(&self) -> bool {
        matches!(self, Self::EntryFunction(_))
    }

    pub fn as_ref(&self) -> TransactionExecutableRef {
        match self {
            TransactionExecutable::EntryFunction(entry_function) => {
                TransactionExecutableRef::EntryFunction(entry_function)
            },
            TransactionExecutable::Script(script) => TransactionExecutableRef::Script(script),
            TransactionExecutable::Empty => TransactionExecutableRef::Empty,
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum TransactionExecutableRef<'a> {
    Script(&'a Script),
    EntryFunction(&'a EntryFunction),
    Empty,
}

impl TransactionExecutableRef<'_> {
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    pub fn is_script(&self) -> bool {
        matches!(self, Self::Script(_))
    }

    pub fn is_entry_function(&self) -> bool {
        matches!(self, Self::EntryFunction(_))
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum TransactionExtraConfig {
    V1 {
        multisig_address: Option<AccountAddress>,
        // None for regular transactions
        // Some(nonce) for orderless transactions
        replay_protection_nonce: Option<u64>,
    },
}

impl TransactionPayload {
    pub fn is_multisig(&self) -> bool {
        match self {
            TransactionPayload::EntryFunction(_) => false,
            TransactionPayload::Script(_) => false,
            TransactionPayload::ModuleBundle(_) => false,
            TransactionPayload::Multisig(_) => true,
            TransactionPayload::Payload(TransactionPayloadInner::V1 { extra_config, .. }) => {
                extra_config.is_multisig()
            },
        }
    }

    pub fn into_entry_function(self) -> EntryFunction {
        match self {
            Self::EntryFunction(f) => f,
            payload => panic!("Expected EntryFunction(_) payload, found: {:#?}", payload),
        }
    }

    pub fn replay_protection_nonce(&self) -> Option<u64> {
        match self {
            Self::Payload(TransactionPayloadInner::V1 { extra_config, .. }) => {
                extra_config.replay_protection_nonce()
            },
            _ => None,
        }
    }

    pub fn executable(&self) -> Result<TransactionExecutable> {
        match self {
            TransactionPayload::EntryFunction(entry_function) => {
                Ok(TransactionExecutable::EntryFunction(entry_function.clone()))
            },
            TransactionPayload::Script(script) => Ok(TransactionExecutable::Script(script.clone())),
            TransactionPayload::Multisig(multisig) => Ok(multisig.as_transaction_executable()),
            TransactionPayload::Payload(TransactionPayloadInner::V1 { executable, .. }) => {
                Ok(executable.clone())
            },
            TransactionPayload::ModuleBundle(_) => {
                Err(format_err!("ModuleBundle variant is deprecated"))
            },
        }
    }

    pub fn executable_ref(&self) -> Result<TransactionExecutableRef> {
        match self {
            TransactionPayload::EntryFunction(entry_function) => {
                Ok(TransactionExecutableRef::EntryFunction(entry_function))
            },
            TransactionPayload::Script(script) => Ok(TransactionExecutableRef::Script(script)),
            TransactionPayload::Multisig(multisig) => Ok(multisig.as_transaction_executable_ref()),
            TransactionPayload::Payload(TransactionPayloadInner::V1 { executable, .. }) => {
                Ok(executable.as_ref())
            },
            TransactionPayload::ModuleBundle(_) => {
                Err(format_err!("ModuleBundle variant is deprecated"))
            },
        }
    }

    pub fn extra_config(&self) -> TransactionExtraConfig {
        match self {
            TransactionPayload::Script(_)
            | TransactionPayload::EntryFunction(_)
            | TransactionPayload::ModuleBundle(_) => TransactionExtraConfig::V1 {
                multisig_address: None,
                replay_protection_nonce: None,
            },
            TransactionPayload::Multisig(multisig) => TransactionExtraConfig::V1 {
                multisig_address: Some(multisig.multisig_address),
                replay_protection_nonce: None,
            },
            TransactionPayload::Payload(TransactionPayloadInner::V1 { extra_config, .. }) => {
                extra_config.clone()
            },
        }
    }

    // Used in transaction emitter when printing errors.
    pub fn payload_type(&self) -> Cow<'static, str> {
        if self.is_multisig() {
            return "multisig".into();
        }
        match self.executable_ref() {
            Ok(TransactionExecutableRef::EntryFunction(entry_function)) => format!(
                "entry {}::{}",
                entry_function.module(),
                entry_function.function()
            )
            .into(),
            Ok(TransactionExecutableRef::Script(_)) => "script".into(),
            Ok(TransactionExecutableRef::Empty) => "empty".into(),
            Err(_) => "deprecated_payload".into(),
        }
    }

    // Used in sdk and a lot of tests when upgrading current payload format to the new format.
    pub fn upgrade_payload(
        self,
        use_txn_payload_v2_format: bool,
        use_orderless_transactions: bool,
    ) -> Self {
        if use_txn_payload_v2_format {
            let executable = self
                .executable()
                .expect("ModuleBundle variant is deprecated");
            let mut extra_config = self.extra_config();
            if use_orderless_transactions {
                extra_config = match extra_config {
                    TransactionExtraConfig::V1 {
                        multisig_address,
                        replay_protection_nonce,
                    } => TransactionExtraConfig::V1 {
                        multisig_address,
                        replay_protection_nonce: replay_protection_nonce.or_else(|| {
                            let mut rng = rand::thread_rng();
                            Some(rng.gen())
                        }),
                    },
                }
            }
            TransactionPayload::Payload(TransactionPayloadInner::V1 {
                executable,
                extra_config,
            })
        } else {
            self
        }
    }
}

impl TransactionExtraConfig {
    pub fn is_multisig(&self) -> bool {
        self.multisig_address().is_some()
    }

    pub fn replay_protection_nonce(&self) -> Option<u64> {
        match self {
            Self::V1 {
                replay_protection_nonce,
                ..
            } => *replay_protection_nonce,
        }
    }

    pub fn is_orderless(&self) -> bool {
        self.replay_protection_nonce().is_some()
    }

    pub fn multisig_address(&self) -> Option<AccountAddress> {
        match self {
            Self::V1 {
                multisig_address,
                replay_protection_nonce: _,
            } => *multisig_address,
        }
    }
}

/// Two different kinds of WriteSet transactions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WriteSetPayload {
    /// Directly passing in the WriteSet.
    Direct(ChangeSet),
    /// Generate the WriteSet by running a script.
    Script {
        /// Execute the script as the designated signer.
        execute_as: AccountAddress,
        /// Script body that gets executed.
        script: Script,
    },
}

impl WriteSetPayload {
    pub fn should_trigger_reconfiguration_by_default(&self) -> bool {
        match self {
            Self::Direct(_) => true,
            Self::Script { .. } => false,
        }
    }
}

/// A transaction that has been signed.
///
/// A `SignedTransaction` is a single transaction that can be atomically executed. Clients submit
/// these to validator nodes, and the validator and executor submits these to the VM.
///
/// **IMPORTANT:** The signature of a `SignedTransaction` is not guaranteed to be verified. For a
/// transaction whose signature is statically guaranteed to be verified, see
/// [`SignatureCheckedTransaction`].
#[derive(Clone, Eq, Serialize, Deserialize)]
pub struct SignedTransaction {
    /// The raw transaction
    raw_txn: RawTransaction,

    /// Public key and signature to authenticate
    authenticator: TransactionAuthenticator,

    /// A cached size of the raw transaction bytes.
    /// Prevents serializing the same transaction multiple times to determine size.
    #[serde(skip)]
    raw_txn_size: OnceCell<usize>,

    /// A cached size of the authenticator.
    /// Prevents serializing the same authenticator multiple times to determine size.
    #[serde(skip)]
    authenticator_size: OnceCell<usize>,

    /// A cached hash of the transaction.
    #[serde(skip)]
    committed_hash: OnceCell<HashValue>,
}

/// PartialEq ignores the cached OnceCell fields that may or may not be initialized.
impl PartialEq for SignedTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.raw_txn == other.raw_txn && self.authenticator == other.authenticator
    }
}

/// A transaction for which the signature has been verified. Created by
/// [`SignedTransaction::check_signature`] and [`RawTransaction::sign`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignatureCheckedTransaction(SignedTransaction);

impl SignatureCheckedTransaction {
    /// Returns the `SignedTransaction` within.
    pub fn into_inner(self) -> SignedTransaction {
        self.0
    }

    /// Returns the `RawTransaction` within.
    pub fn into_raw_transaction(self) -> RawTransaction {
        self.0.into_raw_transaction()
    }
}

impl Deref for SignatureCheckedTransaction {
    type Target = SignedTransaction;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Debug for SignedTransaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SignedTransaction {{ \n \
             {{ raw_txn: {:#?}, \n \
             authenticator: {:#?}, \n \
             }} \n \
             }}",
            self.raw_txn, self.authenticator
        )
    }
}

impl SignedTransaction {
    pub fn new_signed_transaction(
        raw_txn: RawTransaction,
        authenticator: TransactionAuthenticator,
    ) -> SignedTransaction {
        SignedTransaction {
            raw_txn,
            authenticator,
            raw_txn_size: OnceCell::new(),
            authenticator_size: OnceCell::new(),
            committed_hash: OnceCell::new(),
        }
    }

    pub fn new(
        raw_txn: RawTransaction,
        public_key: Ed25519PublicKey,
        signature: Ed25519Signature,
    ) -> SignedTransaction {
        let authenticator = TransactionAuthenticator::ed25519(public_key, signature);
        SignedTransaction {
            raw_txn,
            authenticator,
            raw_txn_size: OnceCell::new(),
            authenticator_size: OnceCell::new(),
            committed_hash: OnceCell::new(),
        }
    }

    pub fn new_fee_payer(
        raw_txn: RawTransaction,
        sender: AccountAuthenticator,
        secondary_signer_addresses: Vec<AccountAddress>,
        secondary_signers: Vec<AccountAuthenticator>,
        fee_payer_address: AccountAddress,
        fee_payer_signer: AccountAuthenticator,
    ) -> Self {
        let authenticator = TransactionAuthenticator::fee_payer(
            sender,
            secondary_signer_addresses,
            secondary_signers,
            fee_payer_address,
            fee_payer_signer,
        );
        Self::new_signed_transaction(raw_txn, authenticator)
    }

    pub fn new_multisig(
        raw_txn: RawTransaction,
        public_key: MultiEd25519PublicKey,
        signature: MultiEd25519Signature,
    ) -> SignedTransaction {
        let authenticator = TransactionAuthenticator::multi_ed25519(public_key, signature);
        Self::new_signed_transaction(raw_txn, authenticator)
    }

    pub fn new_multi_agent(
        raw_txn: RawTransaction,
        sender: AccountAuthenticator,
        secondary_signer_addresses: Vec<AccountAddress>,
        secondary_signers: Vec<AccountAuthenticator>,
    ) -> Self {
        let authenticator = TransactionAuthenticator::multi_agent(
            sender,
            secondary_signer_addresses,
            secondary_signers,
        );
        Self::new_signed_transaction(raw_txn, authenticator)
    }

    pub fn new_secp256k1_ecdsa(
        raw_txn: RawTransaction,
        public_key: secp256k1_ecdsa::PublicKey,
        signature: secp256k1_ecdsa::Signature,
    ) -> SignedTransaction {
        let authenticator = AccountAuthenticator::single_key(SingleKeyAuthenticator::new(
            AnyPublicKey::secp256k1_ecdsa(public_key),
            AnySignature::secp256k1_ecdsa(signature),
        ));
        Self::new_single_sender(raw_txn, authenticator)
    }

    pub fn new_keyless(
        raw_txn: RawTransaction,
        public_key: KeylessPublicKey,
        signature: KeylessSignature,
    ) -> SignedTransaction {
        let authenticator = AccountAuthenticator::single_key(SingleKeyAuthenticator::new(
            AnyPublicKey::keyless(public_key),
            AnySignature::keyless(signature),
        ));
        Self::new_single_sender(raw_txn, authenticator)
    }

    pub fn new_federated_keyless(
        raw_txn: RawTransaction,
        public_key: FederatedKeylessPublicKey,
        signature: KeylessSignature,
    ) -> SignedTransaction {
        let authenticator = AccountAuthenticator::single_key(SingleKeyAuthenticator::new(
            AnyPublicKey::federated_keyless(public_key),
            AnySignature::keyless(signature),
        ));
        Self::new_single_sender(raw_txn, authenticator)
    }

    pub fn new_single_sender(
        raw_txn: RawTransaction,
        authenticator: AccountAuthenticator,
    ) -> SignedTransaction {
        Self::new_signed_transaction(
            raw_txn,
            TransactionAuthenticator::single_sender(authenticator),
        )
    }

    pub fn authenticator(&self) -> TransactionAuthenticator {
        self.authenticator.clone()
    }

    pub fn authenticator_ref(&self) -> &TransactionAuthenticator {
        &self.authenticator
    }

    pub fn sender(&self) -> AccountAddress {
        self.raw_txn.sender
    }

    pub fn into_raw_transaction(self) -> RawTransaction {
        self.raw_txn
    }

    pub fn raw_transaction_ref(&self) -> &RawTransaction {
        &self.raw_txn
    }

    pub fn sequence_number(&self) -> u64 {
        self.raw_txn.sequence_number
    }

    pub fn chain_id(&self) -> ChainId {
        self.raw_txn.chain_id
    }

    pub fn payload(&self) -> &TransactionPayload {
        &self.raw_txn.payload
    }

    pub fn executable_ref(&self) -> Result<TransactionExecutableRef> {
        self.raw_txn.executable_ref()
    }

    pub fn extra_config(&self) -> TransactionExtraConfig {
        self.raw_txn.extra_config()
    }

    pub fn multisig_address(&self) -> Option<AccountAddress> {
        self.raw_txn.extra_config().multisig_address()
    }

    pub fn is_module_bundle(&self) -> bool {
        matches!(self.raw_txn.payload, TransactionPayload::ModuleBundle(_))
    }

    pub fn max_gas_amount(&self) -> u64 {
        self.raw_txn.max_gas_amount
    }

    pub fn gas_unit_price(&self) -> u64 {
        self.raw_txn.gas_unit_price
    }

    pub fn expiration_timestamp_secs(&self) -> u64 {
        self.raw_txn.expiration_timestamp_secs
    }

    pub fn raw_txn_bytes_len(&self) -> usize {
        *self.raw_txn_size.get_or_init(|| {
            bcs::serialized_size(&self.raw_txn).expect("Unable to serialize RawTransaction")
        })
    }

    pub fn txn_bytes_len(&self) -> usize {
        let authenticator_size = *self.authenticator_size.get_or_init(|| {
            bcs::serialized_size(&self.authenticator)
                .expect("Unable to serialize TransactionAuthenticator")
        });
        self.raw_txn_bytes_len() + authenticator_size
    }

    /// Checks that the signature of given transaction. Returns `Ok(SignatureCheckedTransaction)` if
    /// the signature is valid.
    pub fn check_signature(self) -> Result<SignatureCheckedTransaction> {
        self.authenticator.verify(&self.raw_txn)?;
        Ok(SignatureCheckedTransaction(self))
    }

    pub fn verify_signature(&self) -> Result<()> {
        self.authenticator.verify(&self.raw_txn)?;
        Ok(())
    }

    pub fn contains_duplicate_signers(&self) -> bool {
        let mut all_signer_addresses = self.authenticator.secondary_signer_addresses();
        all_signer_addresses.push(self.sender());
        let mut s = BTreeSet::new();
        all_signer_addresses.iter().any(|a| !s.insert(*a))
    }

    pub fn is_multi_agent(&self) -> bool {
        matches!(
            self.authenticator,
            TransactionAuthenticator::MultiAgent { .. }
        )
    }

    /// Returns the hash when the transaction is committed onchain.
    pub fn committed_hash(&self) -> HashValue {
        *self
            .committed_hash
            .get_or_init(|| Transaction::UserTransaction(self.clone()).hash())
    }

    pub fn replay_protector(&self) -> ReplayProtector {
        self.raw_txn.replay_protector()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexedTransactionSummary {
    V1 {
        sender: AccountAddress,
        version: Version,
        transaction_hash: HashValue,
        replay_protector: ReplayProtector,
    },
}

impl IndexedTransactionSummary {
    pub fn version(&self) -> Version {
        match self {
            IndexedTransactionSummary::V1 { version, .. } => *version,
        }
    }

    pub fn transaction_hash(&self) -> HashValue {
        match self {
            IndexedTransactionSummary::V1 {
                transaction_hash, ..
            } => *transaction_hash,
        }
    }

    pub fn replay_protector(&self) -> ReplayProtector {
        match self {
            IndexedTransactionSummary::V1 {
                replay_protector, ..
            } => *replay_protector,
        }
    }

    pub fn sender(&self) -> AccountAddress {
        match self {
            IndexedTransactionSummary::V1 { sender, .. } => *sender,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct TransactionWithProof {
    pub version: Version,
    pub transaction: Transaction,
    pub events: Option<Vec<ContractEvent>>,
    pub proof: TransactionInfoWithProof,
}

impl TransactionWithProof {
    pub fn new(
        version: Version,
        transaction: Transaction,
        events: Option<Vec<ContractEvent>>,
        proof: TransactionInfoWithProof,
    ) -> Self {
        Self {
            version,
            transaction,
            events,
            proof,
        }
    }

    pub fn verify(&self, ledger_info: &LedgerInfo) -> Result<()> {
        let txn_hash = self.transaction.hash();
        ensure!(
            txn_hash == self.proof.transaction_info().transaction_hash(),
            "Transaction hash ({}) not expected ({}).",
            txn_hash,
            self.proof.transaction_info().transaction_hash(),
        );

        if let Some(events) = &self.events {
            let event_hashes: Vec<_> = events.iter().map(CryptoHash::hash).collect();
            let event_root_hash =
                InMemoryEventAccumulator::from_leaves(&event_hashes[..]).root_hash();
            ensure!(
                event_root_hash == self.proof.transaction_info().event_root_hash(),
                "Event root hash ({}) not expected ({}).",
                event_root_hash,
                self.proof.transaction_info().event_root_hash(),
            );
        }

        self.proof.verify(ledger_info, self.version)
    }

    /// Verifies the transaction with the proof, both carried by `self`.
    ///
    /// A few things are ensured if no error is raised:
    ///   1. This transaction exists in the ledger represented by `ledger_info`.
    ///   2. This transaction is a `UserTransaction`.
    ///   3. And this user transaction has the same `version`, `sender`, and `sequence_number` as
    ///      indicated by the parameter list. If any of these parameter is unknown to the call site
    ///      that is supposed to be informed via this struct, get it from the struct itself, such
    ///      as version and sender.
    pub fn verify_user_txn(
        &self,
        ledger_info: &LedgerInfo,
        version: Version,
        sender: AccountAddress,
        replay_protector: ReplayProtector,
    ) -> Result<()> {
        let signed_transaction = self
            .transaction
            .try_as_signed_user_txn()
            .context("not user transaction")?;

        ensure!(
            self.version == version,
            "Version ({}) is not expected ({}).",
            self.version,
            version,
        );
        ensure!(
            signed_transaction.sender() == sender,
            "Sender ({}) not expected ({}).",
            signed_transaction.sender(),
            sender,
        );
        ensure!(
            signed_transaction.replay_protector() == replay_protector,
            "Replay protector ({}) not expected ({}).",
            signed_transaction.replay_protector(),
            replay_protector,
        );

        self.verify(ledger_info)
    }
}

/// The status of VM execution, which contains more detailed failure info
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
pub enum ExecutionStatus {
    Success,
    OutOfGas,
    MoveAbort {
        location: AbortLocation,
        code: u64,
        info: Option<AbortInfo>,
    },
    ExecutionFailure {
        location: AbortLocation,
        function: u16,
        code_offset: u16,
    },
    MiscellaneousError(Option<StatusCode>),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
pub struct AbortInfo {
    pub reason_name: String,
    pub description: String,
}

impl From<KeptVMStatus> for ExecutionStatus {
    fn from(kept_status: KeptVMStatus) -> Self {
        match kept_status {
            KeptVMStatus::Executed => ExecutionStatus::Success,
            KeptVMStatus::OutOfGas => ExecutionStatus::OutOfGas,
            KeptVMStatus::MoveAbort(location, code) => ExecutionStatus::MoveAbort {
                location,
                code,
                info: None,
            },
            KeptVMStatus::ExecutionFailure {
                location: loc,
                function: func,
                code_offset: offset,
                message: _,
            } => ExecutionStatus::ExecutionFailure {
                location: loc,
                function: func,
                code_offset: offset,
            },
            KeptVMStatus::MiscellaneousError => ExecutionStatus::MiscellaneousError(None),
        }
    }
}

impl ExecutionStatus {
    pub fn is_success(&self) -> bool {
        matches!(self, ExecutionStatus::Success)
    }

    pub fn aug_with_aux_data(self, aux_data: &TransactionAuxiliaryData) -> Self {
        if let Some(aux_error) = aux_data.get_detail_error_message() {
            if let ExecutionStatus::MiscellaneousError(status_code) = self {
                if status_code.is_none() {
                    return ExecutionStatus::MiscellaneousError(Some(aux_error.status_code()));
                }
            }
        }
        self
    }

    // Used by simulation API for showing detail error message. Should not be used by production code.
    pub fn conmbine_vm_status_for_simulation(
        aux_data: &TransactionAuxiliaryData,
        partial_status: TransactionStatus,
    ) -> Self {
        match partial_status {
            TransactionStatus::Keep(exec_status) => exec_status.aug_with_aux_data(aux_data),
            TransactionStatus::Discard(status) => ExecutionStatus::MiscellaneousError(Some(status)),
            _ => ExecutionStatus::MiscellaneousError(None),
        }
    }
}

/// The status of executing a transaction. The VM decides whether or not we should `Keep` the
/// transaction output or `Discard` it based upon the execution of the transaction. We wrap these
/// decisions around a `VMStatus` that provides more detail on the final execution state of the VM.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TransactionStatus {
    /// Discard the transaction output
    Discard(DiscardedVMStatus),

    /// Keep the transaction output
    Keep(ExecutionStatus),

    /// Retry the transaction, e.g., after a reconfiguration
    Retry,
}

impl TransactionStatus {
    pub fn status(&self) -> Result<ExecutionStatus, StatusCode> {
        match self {
            TransactionStatus::Keep(status) => Ok(status.clone()),
            TransactionStatus::Discard(code) => Err(*code),
            TransactionStatus::Retry => Err(StatusCode::UNKNOWN_VALIDATION_STATUS),
        }
    }

    pub fn is_discarded(&self) -> bool {
        match self {
            TransactionStatus::Discard(_) => true,
            TransactionStatus::Keep(_) => false,
            TransactionStatus::Retry => true,
        }
    }

    pub fn is_retry(&self) -> bool {
        match self {
            TransactionStatus::Discard(_) => false,
            TransactionStatus::Keep(_) => false,
            TransactionStatus::Retry => true,
        }
    }

    pub fn as_kept_status(&self) -> Result<ExecutionStatus> {
        match self {
            TransactionStatus::Keep(s) => Ok(s.clone()),
            _ => Err(format_err!("Not Keep.")),
        }
    }

    pub fn from_vm_status(vm_status: VMStatus, features: &Features) -> Self {
        let status_code = vm_status.status_code();
        // TODO: keep_or_discard logic should be deprecated from Move repo and refactored into here.
        match vm_status.keep_or_discard(features.is_enabled(FeatureFlag::ENABLE_FUNCTION_VALUES)) {
            Ok(recorded) => match recorded {
                // TODO(bowu):status code should be removed from transaction status
                KeptVMStatus::MiscellaneousError => {
                    TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(status_code)))
                },
                _ => TransactionStatus::Keep(recorded.into()),
            },
            Err(code) => {
                if code.status_type() == StatusType::InvariantViolation
                    && features.is_enabled(FeatureFlag::CHARGE_INVARIANT_VIOLATION)
                {
                    TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(code)))
                } else {
                    TransactionStatus::Discard(code)
                }
            },
        }
    }

    pub fn from_executed_vm_status(vm_status: VMStatus) -> Self {
        if vm_status == VMStatus::Executed {
            TransactionStatus::Keep(ExecutionStatus::Success)
        } else {
            panic!("Auto-conversion should not be called with non-executed status.")
        }
    }
}

impl From<ExecutionStatus> for TransactionStatus {
    fn from(txn_execution_status: ExecutionStatus) -> Self {
        TransactionStatus::Keep(txn_execution_status)
    }
}

/// The result of running the transaction through the VM validator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VMValidatorResult {
    /// Result of the validation: `None` if the transaction was successfully validated
    /// or `Some(DiscardedVMStatus)` if the transaction should be discarded.
    status: Option<DiscardedVMStatus>,

    /// Score for ranking the transaction priority (e.g., based on the gas price).
    /// Only used when the status is `None`. Higher values indicate a higher priority.
    score: u64,
}

impl VMValidatorResult {
    pub fn new(vm_status: Option<DiscardedVMStatus>, score: u64) -> Self {
        debug_assert!(
            match vm_status {
                None => true,
                Some(status) =>
                    status.status_type() == StatusType::Unknown
                        || status.status_type() == StatusType::Validation
                        || status.status_type() == StatusType::InvariantViolation
                        || status.status_type() == StatusType::Execution,
            },
            "Unexpected discarded status: {:?}",
            vm_status
        );
        Self {
            status: vm_status,
            score,
        }
    }

    pub fn error(vm_status: DiscardedVMStatus) -> Self {
        Self {
            status: Some(vm_status),
            score: 0,
        }
    }

    pub fn status(&self) -> Option<DiscardedVMStatus> {
        self.status
    }

    pub fn score(&self) -> u64 {
        self.score
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct VMErrorDetail {
    pub status_code: StatusCode,
    pub message: Option<String>,
}

impl VMErrorDetail {
    pub fn new(status_code: StatusCode, message: Option<String>) -> Self {
        Self {
            status_code,
            message,
        }
    }

    pub fn status_code(&self) -> StatusCode {
        self.status_code
    }

    pub fn message(&self) -> &Option<String> {
        &self.message
    }
}
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct TransactionAuxiliaryDataV1 {
    pub detail_error_message: Option<VMErrorDetail>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub enum TransactionAuxiliaryData {
    None,
    V1(TransactionAuxiliaryDataV1),
}

impl Default for TransactionAuxiliaryData {
    fn default() -> Self {
        Self::None
    }
}

impl TransactionAuxiliaryData {
    pub fn get_detail_error_message(&self) -> Option<&VMErrorDetail> {
        match self {
            Self::V1(data) => data.detail_error_message.as_ref(),
            _ => None,
        }
    }
}

/// The output of executing a transaction.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransactionOutput {
    /// The list of writes this transaction intends to do.
    write_set: WriteSet,

    /// The list of events emitted during this transaction.
    events: Vec<ContractEvent>,

    /// The amount of gas used during execution.
    gas_used: u64,

    /// The execution status. The detailed error info will not be stored here instead will be stored in the auxiliary data.
    status: TransactionStatus,

    /// The transaction auxiliary data that includes detail error info that is not used for calculating the hash
    #[serde(skip)]
    auxiliary_data: TransactionAuxiliaryData,
}

impl TransactionOutput {
    pub fn new(
        write_set: WriteSet,
        events: Vec<ContractEvent>,
        gas_used: u64,
        status: TransactionStatus,
        auxiliary_data: TransactionAuxiliaryData,
    ) -> Self {
        // TODO: add feature flag to enable
        TransactionOutput {
            write_set,
            events,
            gas_used,
            status,
            auxiliary_data,
        }
    }

    pub fn new_success_with_write_set(write_set: WriteSet) -> Self {
        Self {
            write_set,
            events: vec![],
            gas_used: 0,
            status: TransactionStatus::Keep(ExecutionStatus::Success),
            auxiliary_data: TransactionAuxiliaryData::None,
        }
    }

    pub fn new_empty_success() -> Self {
        Self::new_success_with_write_set(WriteSet::default())
    }

    pub fn into(self) -> (WriteSet, Vec<ContractEvent>) {
        (self.write_set, self.events)
    }

    pub fn write_set(&self) -> &WriteSet {
        &self.write_set
    }

    // This is a special function to update the total supply in the write set. 'TransactionOutput'
    // already has materialized write set, but in case of sharding support for total_supply, we
    // want to update the total supply in the write set by aggregating the total supply deltas from
    // each shard. However, is costly to materialize the entire write set again, hence we have this
    // inplace update hack.
    pub fn update_total_supply(&mut self, value: u128) {
        self.write_set.update_total_supply(value);
    }

    pub fn events(&self) -> &[ContractEvent] {
        &self.events
    }

    pub fn gas_used(&self) -> u64 {
        self.gas_used
    }

    pub fn status(&self) -> &TransactionStatus {
        &self.status
    }

    pub fn auxiliary_data(&self) -> &TransactionAuxiliaryData {
        &self.auxiliary_data
    }

    pub fn unpack(
        self,
    ) -> (
        WriteSet,
        Vec<ContractEvent>,
        u64,
        TransactionStatus,
        TransactionAuxiliaryData,
    ) {
        let Self {
            write_set,
            events,
            gas_used,
            status,
            auxiliary_data,
        } = self;
        (write_set, events, gas_used, status, auxiliary_data)
    }

    pub fn ensure_match_transaction_info(
        &self,
        version: Version,
        txn_info: &TransactionInfo,
        expected_write_set: Option<&WriteSet>,
        expected_events: Option<&[ContractEvent]>,
    ) -> Result<()> {
        const ERR_MSG: &str = "TransactionOutput does not match TransactionInfo";

        let expected_txn_status: TransactionStatus = txn_info.status().clone().into();
        ensure!(
            self.status() == &expected_txn_status,
            "{}: version:{}, status:{:?}, auxiliary data:{:?}, expected:{:?}",
            ERR_MSG,
            version,
            self.status(),
            self.auxiliary_data(),
            expected_txn_status,
        );

        ensure!(
            self.gas_used() == txn_info.gas_used(),
            "{}: version:{}, gas_used:{:?}, expected:{:?}",
            ERR_MSG,
            version,
            self.gas_used(),
            txn_info.gas_used(),
        );

        let write_set_hash = CryptoHash::hash(self.write_set());
        ensure!(
            write_set_hash == txn_info.state_change_hash(),
            "{}: version:{}, write_set_hash:{:?}, expected:{:?}, write_set: {:?}, expected(if known): {:?}",
            ERR_MSG,
            version,
            write_set_hash,
            txn_info.state_change_hash(),
            self.write_set,
            expected_write_set,
        );

        let event_hashes = self
            .events()
            .iter()
            .map(CryptoHash::hash)
            .collect::<Vec<_>>();
        let event_root_hash = InMemoryEventAccumulator::from_leaves(&event_hashes).root_hash;
        ensure!(
            event_root_hash == txn_info.event_root_hash(),
            "{}: version:{}, event_root_hash:{:?}, expected:{:?}, events: {:?}, expected(if known): {:?}",
            ERR_MSG,
            version,
            event_root_hash,
            txn_info.event_root_hash(),
            self.events(),
            expected_events,
        );

        Ok(())
    }

    pub fn try_extract_fee_statement(&self) -> Result<Option<FeeStatement>> {
        // Look backwards since the fee statement is expected to be the last event.
        for event in self.events.iter().rev() {
            if let Some(fee_statement) = event.try_v2_typed(&FEE_STATEMENT_EVENT_TYPE)? {
                return Ok(Some(fee_statement));
            }
        }
        Ok(None)
    }

    pub fn has_new_epoch_event(&self) -> bool {
        self.events.iter().any(ContractEvent::is_new_epoch_event)
    }

    pub fn state_update_refs(&self) -> impl Iterator<Item = (&StateKey, Option<&StateValue>)> + '_ {
        self.write_set.state_update_refs()
    }
}

/// `TransactionInfo` is the object we store in the transaction accumulator. It consists of the
/// transaction as well as the execution result of this transaction.
#[derive(Clone, CryptoHasher, BCSCryptoHash, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub enum TransactionInfo {
    V0(TransactionInfoV0),
}

impl TransactionInfo {
    pub fn new(
        transaction_hash: HashValue,
        state_change_hash: HashValue,
        event_root_hash: HashValue,
        state_checkpoint_hash: Option<HashValue>,
        gas_used: u64,
        status: ExecutionStatus,
        auxiliary_info_hash: Option<HashValue>,
    ) -> Self {
        Self::V0(TransactionInfoV0::new(
            transaction_hash,
            state_change_hash,
            event_root_hash,
            state_checkpoint_hash,
            gas_used,
            status,
            auxiliary_info_hash,
        ))
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_placeholder(
        gas_used: u64,
        state_checkpoint_hash: Option<HashValue>,
        status: ExecutionStatus,
    ) -> Self {
        Self::new(
            HashValue::default(),
            HashValue::default(),
            HashValue::default(),
            state_checkpoint_hash,
            gas_used,
            status,
            None,
        )
    }

    #[cfg(any(test, feature = "fuzzing"))]
    fn dummy() -> Self {
        Self::new(
            HashValue::default(),
            HashValue::default(),
            HashValue::default(),
            None,
            0,
            ExecutionStatus::Success,
            None,
        )
    }
}

impl Deref for TransactionInfo {
    type Target = TransactionInfoV0;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::V0(txn_info) => txn_info,
        }
    }
}

#[derive(Clone, CryptoHasher, BCSCryptoHash, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct TransactionInfoV0 {
    /// The amount of gas used.
    gas_used: u64,

    /// The vm status. If it is not `Executed`, this will provide the general error class. Execution
    /// failures and Move abort's receive more detailed information. But other errors are generally
    /// categorized with no status code or other information
    status: ExecutionStatus,

    /// The hash of this transaction.
    transaction_hash: HashValue,

    /// The root hash of Merkle Accumulator storing all events emitted during this transaction.
    event_root_hash: HashValue,

    /// The hash value summarizing all changes caused to the world state by this transaction.
    /// i.e. hash of the output write set.
    state_change_hash: HashValue,

    /// The root hash of the Sparse Merkle Tree describing the world state at the end of this
    /// transaction. Depending on the protocol configuration, this can be generated periodical
    /// only, like per block.
    state_checkpoint_hash: Option<HashValue>,

    /// The hash value summarizing PersistedAuxiliaryInfo.
    auxiliary_info_hash: Option<HashValue>,
}

impl TransactionInfoV0 {
    pub fn new(
        transaction_hash: HashValue,
        state_change_hash: HashValue,
        event_root_hash: HashValue,
        state_checkpoint_hash: Option<HashValue>,
        gas_used: u64,
        status: ExecutionStatus,
        auxiliary_info_hash: Option<HashValue>,
    ) -> Self {
        Self {
            gas_used,
            status,
            transaction_hash,
            event_root_hash,
            state_change_hash,
            state_checkpoint_hash,
            auxiliary_info_hash,
        }
    }

    pub fn transaction_hash(&self) -> HashValue {
        self.transaction_hash
    }

    pub fn state_change_hash(&self) -> HashValue {
        self.state_change_hash
    }

    pub fn has_state_checkpoint_hash(&self) -> bool {
        self.state_checkpoint_hash().is_some()
    }

    pub fn state_checkpoint_hash(&self) -> Option<HashValue> {
        self.state_checkpoint_hash
    }

    pub fn auxiliary_info_hash(&self) -> Option<HashValue> {
        self.auxiliary_info_hash
    }

    pub fn ensure_state_checkpoint_hash(&self) -> Result<HashValue> {
        self.state_checkpoint_hash
            .ok_or_else(|| format_err!("State checkpoint hash not present in TransactionInfo"))
    }

    pub fn event_root_hash(&self) -> HashValue {
        self.event_root_hash
    }

    pub fn gas_used(&self) -> u64 {
        self.gas_used
    }

    pub fn status(&self) -> &ExecutionStatus {
        &self.status
    }
}

impl Display for TransactionInfo {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "TransactionInfo: [txn_hash: {}, state_change_hash: {}, event_root_hash: {}, state_checkpoint_hash: {:?}, gas_used: {}, recorded_status: {:?}]",
            self.transaction_hash(), self.state_change_hash(), self.event_root_hash(), self.state_checkpoint_hash(), self.gas_used(), self.status(),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct TransactionToCommit {
    pub transaction: Transaction,
    pub transaction_info: TransactionInfo,
    pub write_set: WriteSet,
    pub events: Vec<ContractEvent>,
    pub is_reconfig: bool,
    pub transaction_auxiliary_data: TransactionAuxiliaryData,
}

impl TransactionToCommit {
    pub fn new(
        transaction: Transaction,
        transaction_info: TransactionInfo,
        write_set: WriteSet,
        events: Vec<ContractEvent>,
        is_reconfig: bool,
        transaction_auxiliary_data: TransactionAuxiliaryData,
    ) -> Self {
        TransactionToCommit {
            transaction,
            transaction_info,
            write_set,
            events,
            is_reconfig,
            transaction_auxiliary_data,
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy() -> Self {
        Self {
            transaction: Transaction::StateCheckpoint(HashValue::zero()),
            transaction_info: TransactionInfo::dummy(),
            write_set: Default::default(),
            events: vec![],
            is_reconfig: false,
            transaction_auxiliary_data: TransactionAuxiliaryData::default(),
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy_with_events(events: Vec<ContractEvent>) -> Self {
        Self {
            events,
            ..Self::dummy()
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy_with_transaction_info(transaction_info: TransactionInfo) -> Self {
        Self {
            transaction_info,
            ..Self::dummy()
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy_with_transaction(transaction: Transaction) -> Self {
        Self {
            transaction,
            transaction_info: TransactionInfo::dummy(),
            ..Self::dummy()
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy_with_transaction_auxiliary_data(
        transaction_auxiliary_data: TransactionAuxiliaryData,
    ) -> Self {
        Self {
            transaction_auxiliary_data,
            ..Self::dummy()
        }
    }

    pub fn transaction(&self) -> &Transaction {
        &self.transaction
    }

    pub fn transaction_info(&self) -> &TransactionInfo {
        &self.transaction_info
    }

    pub fn has_state_checkpoint_hash(&self) -> bool {
        self.transaction_info().has_state_checkpoint_hash()
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn set_transaction_info(&mut self, txn_info: TransactionInfo) {
        self.transaction_info = txn_info
    }

    pub fn write_set(&self) -> &WriteSet {
        &self.write_set
    }

    pub fn state_update_refs(&self) -> impl Iterator<Item = (&StateKey, Option<&StateValue>)> + '_ {
        self.write_set.state_update_refs()
    }

    pub fn events(&self) -> &[ContractEvent] {
        &self.events
    }

    pub fn gas_used(&self) -> u64 {
        self.transaction_info.gas_used
    }

    pub fn status(&self) -> &ExecutionStatus {
        &self.transaction_info.status
    }

    pub fn is_reconfig(&self) -> bool {
        self.is_reconfig
    }

    pub fn transaction_auxiliary_data(&self) -> &TransactionAuxiliaryData {
        &self.transaction_auxiliary_data
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct TransactionListWithProof {
    pub transactions: Vec<Transaction>,
    pub events: Option<Vec<Vec<ContractEvent>>>,
    pub first_transaction_version: Option<Version>,
    pub proof: TransactionInfoListWithProof,
}

impl TransactionListWithProof {
    /// Constructor.
    pub fn new(
        transactions: Vec<Transaction>,
        events: Option<Vec<Vec<ContractEvent>>>,
        first_transaction_version: Option<Version>,
        proof: TransactionInfoListWithProof,
    ) -> Self {
        Self {
            transactions,
            events,
            first_transaction_version,
            proof,
        }
    }

    /// A convenience function to create an empty proof. Mostly used for tests.
    pub fn new_empty() -> Self {
        Self::new(
            vec![],
            None,
            None,
            TransactionInfoListWithProof::new_empty(),
        )
    }

    /// Verifies the transaction list with proof using the given `ledger_info`.
    /// This method will ensure:
    /// 1. All transactions exist on the given `ledger_info`.
    /// 2. All transactions in the list have consecutive versions.
    /// 3. If `first_transaction_version` is None, the transaction list is empty.
    ///    Otherwise, the transaction list starts at `first_transaction_version`.
    /// 4. If events exist, they match the expected event root hashes in the proof.
    pub fn verify(
        &self,
        ledger_info: &LedgerInfo,
        first_transaction_version: Option<Version>,
    ) -> Result<()> {
        // Verify the first transaction versions match
        ensure!(
            self.first_transaction_version == first_transaction_version,
            "First transaction version ({:?}) doesn't match given version ({:?}).",
            self.first_transaction_version,
            first_transaction_version,
        );

        // Verify the lengths of the transactions and transaction infos match
        ensure!(
            self.proof.transaction_infos.len() == self.transactions.len(),
            "The number of TransactionInfo objects ({}) does not match the number of \
             transactions ({}).",
            self.proof.transaction_infos.len(),
            self.transactions.len(),
        );

        // Verify the transaction hashes match those of the transaction infos
        self.transactions
            .par_iter()
            .zip_eq(self.proof.transaction_infos.par_iter())
            .map(|(txn, txn_info)| {
                let txn_hash = CryptoHash::hash(txn);
                ensure!(
                    txn_hash == txn_info.transaction_hash(),
                    "The hash of transaction does not match the transaction info in proof. \
                     Transaction hash: {:x}. Transaction hash in txn_info: {:x}.",
                    txn_hash,
                    txn_info.transaction_hash(),
                );
                Ok(())
            })
            .collect::<Result<Vec<_>>>()?;

        // Verify the transaction infos are proven by the ledger info.
        self.proof
            .verify(ledger_info, self.first_transaction_version)?;

        // Verify the events if they exist.
        if let Some(event_lists) = &self.events {
            ensure!(
                event_lists.len() == self.transactions.len(),
                "The length of event_lists ({}) does not match the number of transactions ({}).",
                event_lists.len(),
                self.transactions.len(),
            );
            event_lists
                .into_par_iter()
                .zip_eq(self.proof.transaction_infos.par_iter())
                .map(|(events, txn_info)| verify_events_against_root_hash(events, txn_info))
                .collect::<Result<Vec<_>>>()?;
        }

        Ok(())
    }
}

// Note: this type must remain an enum to accommodate future changes/extensions
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum TransactionListWithProofV2 {
    TransactionListWithAuxiliaryInfos(TransactionListWithAuxiliaryInfos),
}

impl TransactionListWithProofV2 {
    pub fn new(transaction_list_with_auxiliary_infos: TransactionListWithAuxiliaryInfos) -> Self {
        Self::TransactionListWithAuxiliaryInfos(transaction_list_with_auxiliary_infos)
    }

    /// A convenience function to create an empty proof. Mostly used for tests.
    pub fn new_empty() -> Self {
        Self::TransactionListWithAuxiliaryInfos(TransactionListWithAuxiliaryInfos::new_empty())
    }

    /// A convenience function to create a v2 list from a v1 list.
    pub fn new_from_v1(transaction_list_with_proof: TransactionListWithProof) -> Self {
        Self::TransactionListWithAuxiliaryInfos(TransactionListWithAuxiliaryInfos::new_from_v1(
            transaction_list_with_proof,
        ))
    }

    pub fn verify(
        &self,
        ledger_info: &LedgerInfo,
        first_transaction_output_version: Option<Version>,
    ) -> Result<()> {
        match self {
            Self::TransactionListWithAuxiliaryInfos(inner) => {
                inner.verify(ledger_info, first_transaction_output_version)
            },
        }
    }

    /// Returns a reference to the inner transaction list with proof
    pub fn get_transaction_list_with_proof(&self) -> &TransactionListWithProof {
        match self {
            Self::TransactionListWithAuxiliaryInfos(transaction_list_with_auxiliary_infos) => {
                &transaction_list_with_auxiliary_infos.transaction_list_with_proof
            },
        }
    }

    /// Returns a reference to the inner persisted auxiliary infos
    pub fn get_persisted_auxiliary_infos(&self) -> &Vec<PersistedAuxiliaryInfo> {
        match self {
            Self::TransactionListWithAuxiliaryInfos(transaction_list_with_auxiliary_infos) => {
                &transaction_list_with_auxiliary_infos.persisted_auxiliary_infos
            },
        }
    }

    pub fn into_parts(self) -> (TransactionListWithProof, Vec<PersistedAuxiliaryInfo>) {
        match self {
            Self::TransactionListWithAuxiliaryInfos(txn_list_with_auxiliary_infos) => (
                txn_list_with_auxiliary_infos.transaction_list_with_proof,
                txn_list_with_auxiliary_infos.persisted_auxiliary_infos,
            ),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct TransactionListWithAuxiliaryInfos {
    pub transaction_list_with_proof: TransactionListWithProof,
    pub persisted_auxiliary_infos: Vec<PersistedAuxiliaryInfo>,
}

impl TransactionListWithAuxiliaryInfos {
    pub fn new(
        transaction_list_with_proof: TransactionListWithProof,
        persisted_auxiliary_infos: Vec<PersistedAuxiliaryInfo>,
    ) -> Self {
        Self {
            transaction_list_with_proof,
            persisted_auxiliary_infos,
        }
    }

    /// A convenience function to create an empty list. Mostly used for tests.
    pub fn new_empty() -> Self {
        Self::new(TransactionListWithProof::new_empty(), vec![])
    }

    /// A convenience function to create a v2 transaction list from
    /// a v1 list. In this case, all auxiliary infos are set to None.
    pub fn new_from_v1(transaction_list_with_proof: TransactionListWithProof) -> Self {
        let num_transaction_infos = transaction_list_with_proof.proof.transaction_infos.len();
        let persisted_auxiliary_infos = vec![PersistedAuxiliaryInfo::None; num_transaction_infos];

        Self {
            transaction_list_with_proof,
            persisted_auxiliary_infos,
        }
    }

    /// Verifies the transaction list with auxiliary infos using the given ledger
    /// info. This method verifies the inner transaction list first, before ensuring
    /// that the auxiliary infos are valid using the transaction infos in the proof.
    pub fn verify(
        &self,
        ledger_info: &LedgerInfo,
        first_transaction_version: Option<Version>,
    ) -> Result<()> {
        // Verify the inner transaction list with proof
        self.transaction_list_with_proof
            .verify(ledger_info, first_transaction_version)?;

        // Verify the auxiliary infos against the transaction infos
        verify_auxiliary_infos_against_transaction_infos(
            &self.persisted_auxiliary_infos,
            &self.transaction_list_with_proof.proof.transaction_infos,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct TransactionOutputListWithProof {
    pub transactions_and_outputs: Vec<(Transaction, TransactionOutput)>,
    pub first_transaction_output_version: Option<Version>,
    pub proof: TransactionInfoListWithProof,
}

impl TransactionOutputListWithProof {
    pub fn new(
        transactions_and_outputs: Vec<(Transaction, TransactionOutput)>,
        first_transaction_output_version: Option<Version>,
        proof: TransactionInfoListWithProof,
    ) -> Self {
        Self {
            transactions_and_outputs,
            first_transaction_output_version,
            proof,
        }
    }

    /// A convenience function to create an empty proof. Mostly used for tests.
    pub fn new_empty() -> Self {
        Self::new(vec![], None, TransactionInfoListWithProof::new_empty())
    }

    /// Verifies the transaction output list with proof using the given `ledger_info`.
    /// This method will ensure:
    /// 1. All transaction infos exist on the given `ledger_info`.
    /// 2. If `first_transaction_output_version` is None, the transaction output list is empty.
    ///    Otherwise, the list starts at `first_transaction_output_version`.
    /// 3. Events, gas, write set, status in each transaction output match the expected event root hashes,
    ///    the gas used and the transaction execution status in the proof, respectively.
    /// 4. The transaction hashes match those of the transaction infos.
    pub fn verify(
        &self,
        ledger_info: &LedgerInfo,
        first_transaction_output_version: Option<Version>,
    ) -> Result<()> {
        // Verify the first transaction output versions match
        ensure!(
            self.first_transaction_output_version == first_transaction_output_version,
            "First transaction and output version ({:?}) doesn't match given version ({:?}).",
            self.first_transaction_output_version,
            first_transaction_output_version,
        );

        // Verify the lengths of the transactions and outputs match the transaction infos
        ensure!(
            self.proof.transaction_infos.len() == self.transactions_and_outputs.len(),
            "The number of TransactionInfo objects ({}) does not match the number of \
             transactions and outputs ({}).",
            self.proof.transaction_infos.len(),
            self.transactions_and_outputs.len(),
        );

        // Verify the events, write set, status, gas used and transaction hashes.
        self.transactions_and_outputs.par_iter().zip_eq(self.proof.transaction_infos.par_iter())
        .map(|((txn, txn_output), txn_info)| {
            // Check the events against the expected events root hash
            verify_events_against_root_hash(&txn_output.events, txn_info)?;

            // Verify the write set matches for both the transaction info and output
            let write_set_hash = CryptoHash::hash(&txn_output.write_set);
            ensure!(
                txn_info.state_change_hash == write_set_hash,
                "The write set in transaction output does not match the transaction info \
                     in proof. Hash of write set in transaction output: {}. Write set hash in txn_info: {}.",
                write_set_hash,
                txn_info.state_change_hash,
            );

            // Verify the gas matches for both the transaction info and output
            ensure!(
                txn_output.gas_used() == txn_info.gas_used(),
                "The gas used in transaction output does not match the transaction info \
                     in proof. Gas used in transaction output: {}. Gas used in txn_info: {}.",
                txn_output.gas_used(),
                txn_info.gas_used(),
            );

            // Verify the execution status matches for both the transaction info and output.
            ensure!(
                *txn_output.status() == TransactionStatus::Keep(txn_info.status().clone()),
                "The execution status of transaction output does not match the transaction \
                     info in proof. Status in transaction output: {:?}. Status in txn_info: {:?}.",
                txn_output.status(),
                txn_info.status(),
            );

            // Verify the transaction hashes match those of the transaction infos
            let txn_hash = txn.hash();
            ensure!(
                txn_hash == txn_info.transaction_hash(),
                "The transaction hash does not match the hash in transaction info. \
                     Transaction hash: {:x}. Transaction hash in txn_info: {:x}.",
                txn_hash,
                txn_info.transaction_hash(),
            );
            Ok(())
        })
        .collect::<Result<Vec<_>>>()?;

        // Verify the transaction infos are proven by the ledger info.
        self.proof
            .verify(ledger_info, self.first_transaction_output_version)?;

        Ok(())
    }
}

/// Verifies a list of events against an expected event root hash. This is done
/// by calculating the hash of the events using an event accumulator hasher.
fn verify_events_against_root_hash(
    events: &[ContractEvent],
    transaction_info: &TransactionInfo,
) -> Result<()> {
    let event_hashes: Vec<_> = events.iter().map(CryptoHash::hash).collect();
    let event_root_hash = InMemoryEventAccumulator::from_leaves(&event_hashes).root_hash();
    ensure!(
        event_root_hash == transaction_info.event_root_hash(),
        "The event root hash calculated doesn't match that carried on the \
                         transaction info! Calculated hash {:?}, transaction info hash {:?}",
        event_root_hash,
        transaction_info.event_root_hash()
    );
    Ok(())
}

// Note: this type must remain an enum to accommodate future changes/extensions
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum TransactionOutputListWithProofV2 {
    TransactionOutputListWithAuxiliaryInfos(TransactionOutputListWithAuxiliaryInfos),
}

impl TransactionOutputListWithProofV2 {
    pub fn new(
        transaction_output_list_with_auxiliary_infos: TransactionOutputListWithAuxiliaryInfos,
    ) -> Self {
        Self::TransactionOutputListWithAuxiliaryInfos(transaction_output_list_with_auxiliary_infos)
    }

    /// A convenience function to create an empty proof. Mostly used for tests.
    pub fn new_empty() -> Self {
        Self::TransactionOutputListWithAuxiliaryInfos(
            TransactionOutputListWithAuxiliaryInfos::new_empty(),
        )
    }

    /// A convenience function to create a v2 output list from a v1 list.
    pub fn new_from_v1(transaction_output_list_with_proof: TransactionOutputListWithProof) -> Self {
        Self::TransactionOutputListWithAuxiliaryInfos(
            TransactionOutputListWithAuxiliaryInfos::new_from_v1(
                transaction_output_list_with_proof,
            ),
        )
    }

    pub fn verify(
        &self,
        ledger_info: &LedgerInfo,
        first_transaction_output_version: Option<Version>,
    ) -> Result<()> {
        match self {
            Self::TransactionOutputListWithAuxiliaryInfos(inner) => {
                inner.verify(ledger_info, first_transaction_output_version)
            },
        }
    }

    /// Returns a reference to the inner transaction output list with proof
    pub fn get_output_list_with_proof(&self) -> &TransactionOutputListWithProof {
        match self {
            Self::TransactionOutputListWithAuxiliaryInfos(output_list_with_auxiliary_infos) => {
                &output_list_with_auxiliary_infos.transaction_output_list_with_proof
            },
        }
    }

    /// Returns a reference to the inner persisted auxiliary infos
    pub fn get_persisted_auxiliary_infos(&self) -> &Vec<PersistedAuxiliaryInfo> {
        match self {
            Self::TransactionOutputListWithAuxiliaryInfos(output_list_with_auxiliary_infos) => {
                &output_list_with_auxiliary_infos.persisted_auxiliary_infos
            },
        }
    }

    pub fn into_parts(self) -> (TransactionOutputListWithProof, Vec<PersistedAuxiliaryInfo>) {
        match self {
            Self::TransactionOutputListWithAuxiliaryInfos(output_list_with_auxiliary_infos) => (
                output_list_with_auxiliary_infos.transaction_output_list_with_proof,
                output_list_with_auxiliary_infos.persisted_auxiliary_infos,
            ),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct TransactionOutputListWithAuxiliaryInfos {
    pub transaction_output_list_with_proof: TransactionOutputListWithProof,
    pub persisted_auxiliary_infos: Vec<PersistedAuxiliaryInfo>,
}

impl TransactionOutputListWithAuxiliaryInfos {
    pub fn new(
        transaction_output_list_with_proof: TransactionOutputListWithProof,
        persisted_auxiliary_infos: Vec<PersistedAuxiliaryInfo>,
    ) -> Self {
        Self {
            transaction_output_list_with_proof,
            persisted_auxiliary_infos,
        }
    }

    /// A convenience function to create an empty list. Mostly used for tests.
    pub fn new_empty() -> Self {
        Self::new(TransactionOutputListWithProof::new_empty(), vec![])
    }

    /// A convenience function to create a v2 output list from a
    /// v1 list. In this case, all auxiliary infos are set to None.
    pub fn new_from_v1(transaction_output_list_with_proof: TransactionOutputListWithProof) -> Self {
        let num_transaction_infos = transaction_output_list_with_proof
            .proof
            .transaction_infos
            .len();
        let persisted_auxiliary_infos = vec![PersistedAuxiliaryInfo::None; num_transaction_infos];

        Self {
            transaction_output_list_with_proof,
            persisted_auxiliary_infos,
        }
    }

    /// Verifies the transaction output list with auxiliary infos using the given ledger
    /// info. This method verifies the inner transaction output list first, before ensuring
    /// that the auxiliary infos are valid using the transaction infos in the proof.
    pub fn verify(
        &self,
        ledger_info: &LedgerInfo,
        first_transaction_output_version: Option<Version>,
    ) -> Result<()> {
        // Verify the inner transaction output list with proof
        self.transaction_output_list_with_proof
            .verify(ledger_info, first_transaction_output_version)?;

        // Verify the auxiliary infos against the transaction infos
        verify_auxiliary_infos_against_transaction_infos(
            &self.persisted_auxiliary_infos,
            &self
                .transaction_output_list_with_proof
                .proof
                .transaction_infos,
        )
    }
}

/// Verifies the auxiliary infos against the given transaction infos.
///
/// Note: this function assumes that the transaction infos have already
/// been verified against a ledger info proof, so it only checks the
/// consistency between auxiliary infos and transaction infos.
fn verify_auxiliary_infos_against_transaction_infos(
    auxiliary_infos: &[PersistedAuxiliaryInfo],
    transaction_infos: &[TransactionInfo],
) -> Result<()> {
    // Verify the lengths of the auxiliary infos and transaction infos match
    ensure!(
        auxiliary_infos.len() == transaction_infos.len(),
        "The number of auxiliary infos ({}) does not match the number of transaction infos ({})",
        auxiliary_infos.len(),
        transaction_infos.len(),
    );

    // Verify the auxiliary info hashes match those of the transaction infos
    auxiliary_infos
        .par_iter()
        .zip_eq(transaction_infos.par_iter())
        .map(|(aux_info, txn_info)| {
            match aux_info {
                PersistedAuxiliaryInfo::None => {
                    ensure!(
                        txn_info.auxiliary_info_hash().is_none(),
                        "The transaction info has an auxiliary info hash: {:?}, \
                             but the persisted auxiliary info is None!",
                        txn_info.auxiliary_info_hash()
                    );
                },
                PersistedAuxiliaryInfo::V1 { .. } => {
                    let aux_info_hash = CryptoHash::hash(aux_info);
                    ensure!(
                        txn_info.auxiliary_info_hash() == Some(aux_info_hash),
                        "The auxiliary info hash does not match the transaction info! \
                             Auxiliary info hash: {:?}. Auxiliary info hash in txn_info: {:?}.",
                        aux_info_hash,
                        txn_info.auxiliary_info_hash()
                    );
                },
            }
            Ok(())
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(())
}

/// A list of ordered transactions (seq number based transactions) under an account
/// that are contiguous by sequence number and include proofs.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct AccountOrderedTransactionsWithProof(pub Vec<TransactionWithProof>);

impl AccountOrderedTransactionsWithProof {
    pub fn new(txns_with_proofs: Vec<TransactionWithProof>) -> Self {
        Self(txns_with_proofs)
    }

    pub fn new_empty() -> Self {
        Self::new(Vec::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn inner(&self) -> &[TransactionWithProof] {
        &self.0
    }

    pub fn into_inner(self) -> Vec<TransactionWithProof> {
        self.0
    }

    /// 1. Verify all transactions are consistent with the given ledger info.
    /// 2. All transactions were sent by `account`.
    /// 3. The transactions are contiguous by sequence number, starting at `start_seq_num`.
    /// 4. No more transactions than limit.
    /// 5. Events are present when requested (and not present when not requested).
    /// 6. Transactions are not newer than requested ledger version.
    pub fn verify(
        &self,
        ledger_info: &LedgerInfo,
        account: AccountAddress,
        start_seq_num: u64,
        limit: u64,
        include_events: bool,
        ledger_version: Version,
    ) -> Result<()> {
        ensure!(
            self.len() as u64 <= limit,
            "number of account transactions ({}) exceeded limit ({})",
            self.len(),
            limit,
        );

        self.0
            .iter()
            .enumerate()
            .try_for_each(|(seq_num_offset, txn_with_proof)| {
                let expected_seq_num = start_seq_num.saturating_add(seq_num_offset as u64);
                let txn_version = txn_with_proof.version;

                ensure!(
                    include_events == txn_with_proof.events.is_some(),
                    "unexpected events or missing events"
                );
                ensure!(
                    txn_version <= ledger_version,
                    "transaction with version ({}) greater than requested ledger version ({})",
                    txn_version,
                    ledger_version,
                );

                txn_with_proof.verify_user_txn(
                    ledger_info,
                    txn_version,
                    account,
                    ReplayProtector::SequenceNumber(expected_seq_num),
                )
            })
    }
}

/// `Transaction` will be the transaction type used internally in the aptos node to represent the
/// transaction to be processed and persisted.
///
/// We suppress the clippy warning here as we would expect most of the transaction to be user
/// transaction.
#[allow(clippy::large_enum_variant)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub enum Transaction {
    /// Transaction submitted by the user. e.g: P2P payment transaction, publishing module
    /// transaction, etc.
    /// TODO: We need to rename SignedTransaction to SignedUserTransaction, as well as all the other
    ///       transaction types we had in our codebase.
    UserTransaction(SignedTransaction),

    /// Transaction that applies a WriteSet to the current storage, it's applied manually via aptos-db-bootstrapper.
    GenesisTransaction(WriteSetPayload),

    /// Transaction to update the block metadata resource at the beginning of a block,
    /// when on-chain randomness is disabled.
    BlockMetadata(BlockMetadata),

    /// Transaction to let the executor update the global state tree and record the root hash
    /// in the TransactionInfo
    /// The hash value inside is unique block id which can generate unique hash of state checkpoint transaction
    StateCheckpoint(HashValue),

    /// Transaction that only proposed by a validator mainly to update on-chain configs.
    ValidatorTransaction(ValidatorTransaction),

    /// Transaction to update the block metadata resource at the beginning of a block,
    /// when on-chain randomness is enabled.
    BlockMetadataExt(BlockMetadataExt),

    /// Transaction to let the executor update the global state tree and record the root hash
    /// in the TransactionInfo
    /// The hash value inside is unique block id which can generate unique hash of state checkpoint transaction
    /// Replaces StateCheckpoint, with optionally having more data.
    BlockEpilogue(BlockEpiloguePayload),
}

impl From<BlockMetadataExt> for Transaction {
    fn from(metadata: BlockMetadataExt) -> Self {
        match metadata {
            BlockMetadataExt::V0(v0) => Transaction::BlockMetadata(v0),
            vx => Transaction::BlockMetadataExt(vx),
        }
    }
}

impl Transaction {
    pub fn block_epilogue_v0(block_id: HashValue, block_end_info: BlockEndInfo) -> Self {
        Self::BlockEpilogue(BlockEpiloguePayload::V0 {
            block_id,
            block_end_info,
        })
    }

    pub fn block_epilogue_v1(
        block_id: HashValue,
        block_end_info: BlockEndInfoExt,
        fee_distribution: FeeDistribution,
    ) -> Self {
        Self::BlockEpilogue(BlockEpiloguePayload::V1 {
            block_id,
            block_end_info,
            fee_distribution,
        })
    }

    pub fn try_as_signed_user_txn(&self) -> Option<&SignedTransaction> {
        match self {
            Transaction::UserTransaction(txn) => Some(txn),
            _ => None,
        }
    }

    pub fn try_as_block_metadata(&self) -> Option<&BlockMetadata> {
        match self {
            Transaction::BlockMetadata(bm) => Some(bm),
            _ => None,
        }
    }

    pub fn try_as_block_metadata_ext(&self) -> Option<&BlockMetadataExt> {
        match self {
            Transaction::BlockMetadataExt(bme) => Some(bme),
            _ => None,
        }
    }

    pub fn try_as_validator_txn(&self) -> Option<&ValidatorTransaction> {
        match self {
            Transaction::ValidatorTransaction(t) => Some(t),
            _ => None,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Transaction::UserTransaction(_) => "user_transaction",
            Transaction::GenesisTransaction(_) => "genesis_transaction",
            Transaction::BlockMetadata(_) => "block_metadata",
            Transaction::StateCheckpoint(_) => "state_checkpoint",
            Transaction::BlockEpilogue(_) => "block_epilogue",
            Transaction::ValidatorTransaction(vt) => vt.type_name(),
            Transaction::BlockMetadataExt(bmet) => bmet.type_name(),
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy() -> Self {
        Transaction::StateCheckpoint(HashValue::zero())
    }

    pub fn is_non_reconfig_block_ending(&self) -> bool {
        match self {
            Transaction::StateCheckpoint(_) | Transaction::BlockEpilogue(_) => true,
            Transaction::UserTransaction(_)
            | Transaction::GenesisTransaction(_)
            | Transaction::BlockMetadata(_)
            | Transaction::BlockMetadataExt(_)
            | Transaction::ValidatorTransaction(_) => false,
        }
    }

    pub fn is_block_start(&self) -> bool {
        match self {
            Transaction::BlockMetadata(_) | Transaction::BlockMetadataExt(_) => true,
            Transaction::StateCheckpoint(_)
            | Transaction::BlockEpilogue(_)
            | Transaction::UserTransaction(_)
            | Transaction::GenesisTransaction(_)
            | Transaction::ValidatorTransaction(_) => false,
        }
    }
}

impl TryFrom<Transaction> for SignedTransaction {
    type Error = Error;

    fn try_from(txn: Transaction) -> Result<Self> {
        match txn {
            Transaction::UserTransaction(txn) => Ok(txn),
            _ => Err(format_err!("Not a user transaction.")),
        }
    }
}

/// Trait that defines a transaction type that can be executed by the block executor. A transaction
/// transaction will write to a key value storage as their side effect.
pub trait BlockExecutableTransaction: Sync + Send + Clone + 'static {
    type Key: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + Debug;
    /// Some keys contain multiple "resources" distinguished by a tag. Reading these keys requires
    /// specifying a tag, and output requires merging all resources together (Note: this may change
    /// in the future if write-set format changes to be per-resource, could be more performant).
    /// Is generic primarily to provide easy plug-in replacement for mock tests and be extensible.
    type Tag: PartialOrd
        + Ord
        + Send
        + Sync
        + Clone
        + Hash
        + Eq
        + Debug
        + DeserializeOwned
        + Serialize;
    type Value: Send + Sync + Debug + Clone + Eq + PartialEq + TransactionWrite;
    type Event: Send + Sync + Debug + Clone + TransactionEvent;

    /// Size of the user transaction in bytes, 0 otherwise
    fn user_txn_bytes_len(&self) -> usize;

    /// None if it is not a user transaction.
    fn try_as_signed_user_txn(&self) -> Option<&SignedTransaction> {
        None
    }

    fn from_txn(_txn: Transaction) -> Self {
        unimplemented!()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ViewFunctionError {
    // This is to represent errors are from a MoveAbort and has error info from the module metadata to display.
    // The ExecutionStatus is used to construct the error message in the same way as MoveAborts for entry functions.
    MoveAbort(ExecutionStatus, Option<StatusCode>),
    // This is a generic error message that takes in a string and display it in the error response.
    ErrorMessage(String, Option<StatusCode>),
}

impl std::fmt::Display for ViewFunctionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ViewFunctionError::MoveAbort(status, vm_status) => {
                write!(
                    f,
                    "Execution status: {:?}, VM status: {:?}",
                    status, vm_status
                )
            },
            ViewFunctionError::ErrorMessage(msg, vm_status) => {
                write!(f, "Error: {}, VM status: {:?}", msg, vm_status)
            },
        }
    }
}

pub struct ViewFunctionOutput {
    pub values: Result<Vec<Vec<u8>>, ViewFunctionError>,
    pub gas_used: u64,
}

impl ViewFunctionOutput {
    pub fn new(values: Result<Vec<Vec<u8>>, ViewFunctionError>, gas_used: u64) -> Self {
        Self { values, gas_used }
    }

    pub fn new_ok(values: Vec<Vec<u8>>, gas_used: u64) -> Self {
        Self {
            values: Ok(values),
            gas_used,
        }
    }

    pub fn new_error_message(
        message: String,
        vm_status: Option<StatusCode>,
        gas_used: u64,
    ) -> Self {
        Self {
            values: Err(ViewFunctionError::ErrorMessage(message, vm_status)),
            gas_used,
        }
    }

    pub fn new_move_abort_error(
        status: ExecutionStatus,
        vm_status: Option<StatusCode>,
        gas_used: u64,
    ) -> Self {
        Self {
            values: Err(ViewFunctionError::MoveAbort(status, vm_status)),
            gas_used,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AuxiliaryInfo {
    persisted_info: PersistedAuxiliaryInfo,
    ephemeral_info: Option<EphemeralAuxiliaryInfo>,
}

impl AuxiliaryInfo {
    pub fn new(
        persisted_info: PersistedAuxiliaryInfo,
        ephemeral_info: Option<EphemeralAuxiliaryInfo>,
    ) -> Self {
        Self {
            persisted_info,
            ephemeral_info,
        }
    }

    pub fn new_empty() -> Self {
        Self {
            persisted_info: PersistedAuxiliaryInfo::None,
            ephemeral_info: None,
        }
    }

    pub fn into_persisted_info(self) -> PersistedAuxiliaryInfo {
        self.persisted_info
    }

    pub fn persisted_info(&self) -> &PersistedAuxiliaryInfo {
        &self.persisted_info
    }

    pub fn ephemeral_info(&self) -> &Option<EphemeralAuxiliaryInfo> {
        &self.ephemeral_info
    }
}

#[derive(
    BCSCryptoHash, Clone, Copy, CryptoHasher, Debug, Eq, Serialize, Deserialize, PartialEq,
)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub enum PersistedAuxiliaryInfo {
    None,
    // The index of the transaction in a block (after shuffler, before execution).
    // Note that this would be slightly different from the index of transactions that get committed
    // onchain, as this considers transactions that may get discarded.
    V1 { transaction_index: u32 },
}

#[derive(Debug, Clone, Copy)]
pub struct EphemeralAuxiliaryInfo {
    // TODO(grao): After execution pool is implemented we might want this information be persisted
    // onchain?
    pub proposer_index: u64,
}

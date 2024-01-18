// Copyright © Aptos Foundation

use crate::{
    chain_id::ChainId,
    transaction::{
        authenticator::{
            AccountAuthenticator, AnyPublicKey, AnySignature, SingleKeyAuthenticator,
            TransactionAuthenticator,
        },
        EntryFunction, Module, ModuleBundle, Multisig, Script, SignatureCheckedTransaction,
        Transaction, TransactionPayload,
    },
};
use anyhow::{anyhow as format_err, Error};
use aptos_crypto::{
    ed25519::{Ed25519PublicKey, Ed25519Signature},
    hash::CryptoHash,
    multi_ed25519::{MultiEd25519PublicKey, MultiEd25519Signature},
    secp256k1_ecdsa, signing_message, CryptoMaterialError, HashValue,
};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use move_core_types::{account_address::AccountAddress, transaction_argument::convert_txn_args};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fmt, fmt::Debug, format, matches, vec, write};

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

impl From<RawTransaction> for crate::transaction::RawTransaction {
    fn from(t: RawTransaction) -> Self {
        Self::new(
            t.sender,
            t.sequence_number,
            t.payload,
            t.max_gas_amount,
            t.gas_unit_price,
            t.expiration_timestamp_secs,
            t.chain_id,
        )
    }
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

    /// Create a new `RawTransaction` with a module to publish.
    pub fn new_module(
        sender: AccountAddress,
        sequence_number: u64,
        module: Module,
        max_gas_amount: u64,
        gas_unit_price: u64,
        expiration_timestamp_secs: u64,
        chain_id: ChainId,
    ) -> Self {
        RawTransaction {
            sender,
            sequence_number,
            payload: TransactionPayload::ModuleBundle(ModuleBundle::from(module)),
            max_gas_amount,
            gas_unit_price,
            expiration_timestamp_secs,
            chain_id,
        }
    }

    /// Create a new `RawTransaction` with a list of modules to publish.
    ///
    /// Multiple modules per transaction can be published.
    pub fn new_module_bundle(
        sender: AccountAddress,
        sequence_number: u64,
        modules: ModuleBundle,
        max_gas_amount: u64,
        gas_unit_price: u64,
        expiration_timestamp_secs: u64,
        chain_id: ChainId,
    ) -> Self {
        RawTransaction {
            sender,
            sequence_number,
            payload: TransactionPayload::ModuleBundle(modules),
            max_gas_amount,
            gas_unit_price,
            expiration_timestamp_secs,
            chain_id,
        }
    }

    pub fn into_payload(self) -> TransactionPayload {
        self.payload
    }

    pub fn format_for_client(&self, get_transaction_name: impl Fn(&[u8]) -> String) -> String {
        let (code, args) = match &self.payload {
            TransactionPayload::Script(script) => (
                get_transaction_name(script.code()),
                convert_txn_args(script.args()),
            ),
            TransactionPayload::EntryFunction(script_fn) => (
                format!("{}::{}", script_fn.module(), script_fn.function()),
                script_fn.args().to_vec(),
            ),
            TransactionPayload::Multisig(multisig) => (
                format!(
                    "Executing next transaction for multisig account {}",
                    multisig.multisig_address,
                ),
                vec![],
            ),
            TransactionPayload::ModuleBundle(_) => ("module publishing".to_string(), vec![]),
        };
        let mut f_args: String = "".to_string();
        for arg in args {
            f_args = format!("{}\n\t\t\t{:02X?},", f_args, arg);
        }
        format!(
            "RawTransaction {{ \n\
             \tsender: {}, \n\
             \tsequence_number: {}, \n\
             \tpayload: {{, \n\
             \t\ttransaction: {}, \n\
             \t\targs: [ {} \n\
             \t\t]\n\
             \t}}, \n\
             \tmax_gas_amount: {}, \n\
             \tgas_unit_price: {}, \n\
             \texpiration_timestamp_secs: {:#?}, \n\
             \tchain_id: {},
             }}",
            self.sender,
            self.sequence_number,
            code,
            f_args,
            self.max_gas_amount,
            self.gas_unit_price,
            self.expiration_timestamp_secs,
            self.chain_id,
        )
    }

    /// Return the sender of this transaction.
    pub fn sender(&self) -> AccountAddress {
        self.sender
    }

    /// Return the signing message for creating transaction signature.
    pub fn signing_message(&self) -> anyhow::Result<Vec<u8>, CryptoMaterialError> {
        signing_message(self)
    }
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
}

impl From<SignedTransaction> for crate::transaction::SignedTransaction {
    fn from(t: SignedTransaction) -> Self {
        Self::V0(t)
    }
}

/// PartialEq ignores the "bytes" field as this is a OnceCell that may or
/// may not be initialized during runtime comparison.
impl PartialEq for SignedTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.raw_txn == other.raw_txn && self.authenticator == other.authenticator
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
        SignedTransaction {
            raw_txn,
            authenticator: TransactionAuthenticator::fee_payer(
                sender,
                secondary_signer_addresses,
                secondary_signers,
                fee_payer_address,
                fee_payer_signer,
            ),
            raw_txn_size: OnceCell::new(),
            authenticator_size: OnceCell::new(),
        }
    }

    pub fn new_multisig(
        raw_txn: RawTransaction,
        public_key: MultiEd25519PublicKey,
        signature: MultiEd25519Signature,
    ) -> SignedTransaction {
        let authenticator = TransactionAuthenticator::multi_ed25519(public_key, signature);
        SignedTransaction {
            raw_txn,
            authenticator,
            raw_txn_size: OnceCell::new(),
            authenticator_size: OnceCell::new(),
        }
    }

    pub fn new_multi_agent(
        raw_txn: RawTransaction,
        sender: AccountAuthenticator,
        secondary_signer_addresses: Vec<AccountAddress>,
        secondary_signers: Vec<AccountAuthenticator>,
    ) -> Self {
        SignedTransaction {
            raw_txn,
            authenticator: TransactionAuthenticator::multi_agent(
                sender,
                secondary_signer_addresses,
                secondary_signers,
            ),
            raw_txn_size: OnceCell::new(),
            authenticator_size: OnceCell::new(),
        }
    }

    pub fn new_secp256k1_ecdsa(
        raw_txn: RawTransaction,
        public_key: secp256k1_ecdsa::PublicKey,
        signature: secp256k1_ecdsa::Signature,
    ) -> SignedTransaction {
        let authenticator = TransactionAuthenticator::single_sender(
            AccountAuthenticator::single_key(SingleKeyAuthenticator::new(
                AnyPublicKey::secp256k1_ecdsa(public_key),
                AnySignature::secp256k1_ecdsa(signature),
            )),
        );
        SignedTransaction {
            raw_txn,
            authenticator,
            raw_txn_size: OnceCell::new(),
            authenticator_size: OnceCell::new(),
        }
    }

    pub fn new_single_sender(
        raw_txn: RawTransaction,
        authenticator: AccountAuthenticator,
    ) -> SignedTransaction {
        SignedTransaction {
            raw_txn,
            authenticator: TransactionAuthenticator::single_sender(authenticator),
            raw_txn_size: OnceCell::new(),
            authenticator_size: OnceCell::new(),
        }
    }

    pub fn new_with_authenticator(
        raw_txn: RawTransaction,
        authenticator: TransactionAuthenticator,
    ) -> Self {
        Self {
            raw_txn,
            authenticator,
            raw_txn_size: OnceCell::new(),
            authenticator_size: OnceCell::new(),
        }
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
    pub fn check_signature(self) -> anyhow::Result<SignatureCheckedTransaction> {
        self.authenticator.verify(&self.raw_txn)?;
        Ok(SignatureCheckedTransaction(self.into()))
    }

    pub fn verify_signature(&self) -> anyhow::Result<()> {
        self.authenticator.verify(&self.raw_txn)?;
        Ok(())
    }

    pub fn contains_duplicate_signers(&self) -> bool {
        let mut all_signer_addresses = self.authenticator.secondary_signer_addresses();
        all_signer_addresses.push(self.sender());
        let mut s = BTreeSet::new();
        all_signer_addresses.iter().any(|a| !s.insert(*a))
    }

    pub fn format_for_client(&self, get_transaction_name: impl Fn(&[u8]) -> String) -> String {
        format!(
            "DeprecatedSignedTransaction {{ \n \
             raw_txn: {}, \n \
             authenticator: {:#?}, \n \
             }}",
            self.raw_txn.format_for_client(get_transaction_name),
            self.authenticator
        )
    }

    pub fn is_multi_agent(&self) -> bool {
        matches!(
            self.authenticator,
            TransactionAuthenticator::MultiAgent { .. }
        )
    }

    /// Returns the hash when the transaction is commited onchain.
    pub fn committed_hash(self) -> HashValue {
        Transaction::UserTransaction(self).hash()
    }
}

impl TryFrom<Transaction> for SignedTransaction {
    type Error = Error;

    fn try_from(txn: Transaction) -> anyhow::Result<Self> {
        match txn {
            Transaction::UserTransaction(txn) => Ok(txn),
            _ => Err(format_err!("Not a user transaction.")),
        }
    }
}

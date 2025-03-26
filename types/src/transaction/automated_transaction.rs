// Copyright (c) 2024 Supra.
// SPDX-License-Identifier: Apache-2.0

use crate::chain_id::ChainId;
use crate::transaction::automation::AutomationTaskMetaData;
use crate::transaction::{EntryFunction, RawTransaction, Transaction, TransactionPayload};
use anyhow::anyhow;
use aptos_crypto::HashValue;
use move_core_types::account_address::AccountAddress;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Debug;

/// A transaction that has been created based on the automation-task in automation registry.
///
/// A `AutomatedTransaction` is a single transaction that can be atomically executed.
/// `AutomatedTransaction`s are considered as internal transactions and submitted by the application
/// layer based on the registered tasks.
///
#[derive(Clone, Eq, Serialize, Deserialize)]
pub struct AutomatedTransaction {
    /// The raw transaction
    raw_txn: RawTransaction,

    /// Hash of the transaction which registered this automated transaction.
    authenticator: HashValue,

    /// Height of the block for which this transaction has been scheduled for execution.
    block_height: u64,

    /// A cached size of the raw transaction bytes.
    /// Prevents serializing the same transaction multiple times to determine size.
    #[serde(skip)]
    raw_txn_size: OnceCell<usize>,

    /// A cached hash of the transaction.
    #[serde(skip)]
    hash: OnceCell<HashValue>,
}

/// PartialEq ignores the cached OnceCell fields that may or may not be initialized.
impl PartialEq for AutomatedTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.raw_txn == other.raw_txn
            && self.authenticator == other.authenticator
            && self.block_height == other.block_height
    }
}

impl Debug for AutomatedTransaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "AutomatedTransaction {{ raw_txn: {:?}, authenticator: {:?} }}",
            self.raw_txn, self.authenticator
        )
    }
}

impl AutomatedTransaction {
    pub fn new(raw_txn: RawTransaction, authenticator: HashValue, block_height: u64) -> Self {
        Self {
            raw_txn,
            authenticator,
            block_height,
            raw_txn_size: Default::default(),
            hash: Default::default(),
        }
    }

    pub fn authenticator(&self) -> HashValue {
        self.authenticator
    }

    pub fn authenticator_ref(&self) -> &HashValue {
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

    pub fn block_height(&self) -> u64 {
        self.block_height
    }

    pub fn raw_txn_bytes_len(&self) -> usize {
        *self.raw_txn_size.get_or_init(|| {
            bcs::serialized_size(&self.raw_txn).expect("Unable to serialize RawTransaction")
        })
    }

    pub fn txn_bytes_len(&self) -> usize {
        let authenticator_size = HashValue::LENGTH;
        self.raw_txn_bytes_len() + authenticator_size
    }

    /// Returns the hash of the transaction.
    pub fn hash(&self) -> HashValue {
        *self.hash.get_or_init(|| {
            HashValue::keccak_256_of(
                &bcs::to_bytes(&self).expect("Unable to serialize AutomatedTransaction"),
            )
        })
    }

    /// Returns transaction TTL since base_timestamp if the transaction expiry time is in the future,
    /// otherwise None.
    pub fn duration_since(&self, base_timestamp: u64) -> Option<u64> {
        self.expiration_timestamp_secs().checked_sub(base_timestamp)
    }
}

impl From<AutomatedTransaction> for Transaction {
    fn from(value: AutomatedTransaction) -> Self {
        Transaction::AutomatedTransaction(value)
    }
}

macro_rules! value_or_missing {
    ($value: ident , $message: literal) => {
        match $value {
            Some(v) => v,
            None => return BuilderResult::missing_value($message),
        }
    };
}
#[derive(Clone, Debug)]
pub enum BuilderResult {
    Success(AutomatedTransaction),
    GasPriceThresholdExceeded {
        task_index: u64,
        threshold: u64,
        value: u64,
    },
    MissingValue(&'static str),
}

impl BuilderResult {
    pub fn success(txn: AutomatedTransaction) -> BuilderResult {
        Self::Success(txn)
    }

    pub fn gas_price_threshold_exceeded(
        task_index: u64,
        threshold: u64,
        value: u64,
    ) -> BuilderResult {
        Self::GasPriceThresholdExceeded {
            task_index,
            threshold,
            value,
        }
    }

    pub fn missing_value(missing: &'static str) -> BuilderResult {
        Self::MissingValue(missing)
    }
}

/// Builder interface for [AutomatedTransaction]
#[derive(Clone, Debug, Default)]
pub struct AutomatedTransactionBuilder {
    /// Gas unit price threshold. Default to 0.
    pub(crate) gas_price_cap: u64,

    /// Sender's address.
    pub(crate) sender: Option<AccountAddress>,

    /// Sequence number of the automated transaction which corresponds to the task index in registry
    /// based on which this automated transaction is going to be created.
    pub(crate) sequence_number: Option<u64>,

    /// The transaction payload to execute.
    pub(crate) payload: Option<TransactionPayload>,

    /// Maximal total gas to spend for this transaction.
    pub(crate) max_gas_amount: Option<u64>,

    /// Price to be paid per gas unit.
    pub(crate) gas_unit_price: Option<u64>,

    /// Expiration timestamp for this transaction, represented
    /// as seconds from the Unix Epoch. If the current blockchain timestamp
    /// is greater than or equal to this time, then the transaction has
    /// expired and will be discarded. This can be set to a large value far
    /// in the future to indicate that a transaction does not expire.
    pub(crate) expiration_timestamp_secs: Option<u64>,

    /// Chain ID of the Supra network this transaction is intended for.
    pub(crate) chain_id: Option<ChainId>,

    /// Hash of the transaction which registered this automated transaction.
    pub(crate) authenticator: Option<HashValue>,

    /// Height of the block for which this transaction has should be scheduled for execution.
    pub(crate) block_height: Option<u64>,
}

/// Getter interfaces of the builder
impl AutomatedTransactionBuilder {
    pub fn gas_price_cap(&self) -> &u64 {
        &self.gas_price_cap
    }

    pub fn sender(&self) -> &Option<AccountAddress> {
        &self.sender
    }

    pub fn sequence_number(&self) -> &Option<u64> {
        &self.sequence_number
    }

    pub fn payload(&self) -> &Option<TransactionPayload> {
        &self.payload
    }

    pub fn max_gas_amount(&self) -> &Option<u64> {
        &self.max_gas_amount
    }

    pub fn gas_unit_price(&self) -> &Option<u64> {
        &self.gas_unit_price
    }
    pub fn expiration_timestamp_secs(&self) -> &Option<u64> {
        &self.expiration_timestamp_secs
    }

    pub fn chain_id(&self) -> &Option<ChainId> {
        &self.chain_id
    }
    pub fn authenticator(&self) -> &Option<HashValue> {
        &self.authenticator
    }

    pub fn block_height(&self) -> &Option<u64> {
        &self.block_height
    }
}

impl AutomatedTransactionBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_gas_price_cap(mut self, cap: u64) -> Self {
        self.gas_price_cap = cap;
        self
    }

    pub fn with_sender(mut self, sender: AccountAddress) -> Self {
        self.sender = Some(sender);
        self
    }
    pub fn with_sequence_number(mut self, seq: u64) -> Self {
        self.sequence_number = Some(seq);
        self
    }
    pub fn with_payload(mut self, payload: TransactionPayload) -> Self {
        self.payload = Some(payload);
        self
    }

    pub fn with_entry_function(mut self, entry_fn: EntryFunction) -> Self {
        self.payload = Some(TransactionPayload::EntryFunction(entry_fn));
        self
    }
    pub fn with_max_gas_amount(mut self, max_gas_amount: u64) -> Self {
        self.max_gas_amount = Some(max_gas_amount);
        self
    }
    pub fn with_gas_unit_price(mut self, gas_unit_price: u64) -> Self {
        self.gas_unit_price = Some(gas_unit_price);
        self
    }
    pub fn with_expiration_timestamp_secs(mut self, secs: u64) -> Self {
        self.expiration_timestamp_secs = Some(secs);
        self
    }
    pub fn with_chain_id(mut self, chain_id: ChainId) -> Self {
        self.chain_id = Some(chain_id);
        self
    }
    pub fn with_authenticator(mut self, authenticator: HashValue) -> Self {
        self.authenticator = Some(authenticator);
        self
    }
    pub fn with_block_height(mut self, block_height: u64) -> Self {
        self.block_height = Some(block_height);
        self
    }

    /// Build an [AutomatedTransaction] instance.
    /// Fails if
    ///    - any of the mandatory fields is missing
    ///    - if specified gas price threshold is crossed by gas unit price value
    pub fn build(self) -> BuilderResult {
        let AutomatedTransactionBuilder {
            gas_price_cap,
            sender,
            sequence_number,
            payload,
            max_gas_amount,
            gas_unit_price,
            expiration_timestamp_secs,
            chain_id,
            authenticator,
            block_height,
        } = self;
        let sender = value_or_missing!(sender, "sender");
        let sequence_number = value_or_missing!(sequence_number, "sequence_number");
        let payload = value_or_missing!(payload, "payload");
        let max_gas_amount = value_or_missing!(max_gas_amount, "max_gas_amount");
        let gas_unit_price = value_or_missing!(gas_unit_price, "gas_unit_price");
        let chain_id = value_or_missing!(chain_id, "chain_id");
        let authenticator = value_or_missing!(authenticator, "authenticator");
        let block_height = value_or_missing!(block_height, "block_height");
        let expiration_timestamp_secs =
            value_or_missing!(expiration_timestamp_secs, "expiration_timestamp_secs");
        if gas_price_cap < gas_unit_price {
            return BuilderResult::gas_price_threshold_exceeded(
                sequence_number,
                gas_price_cap,
                gas_unit_price,
            );
        }
        let raw_transaction = RawTransaction::new(
            sender,
            sequence_number,
            payload,
            max_gas_amount,
            gas_unit_price,
            expiration_timestamp_secs,
            chain_id,
        );
        BuilderResult::Success(AutomatedTransaction::new(
            raw_transaction,
            authenticator,
            block_height,
        ))
    }
}

/// Creates [AutomatedTransaction] builder from [AutomationTaskMetaData]
/// Fails if:
///   - payload is not successfully converted to entry function
///   - txn_hash can not be converted to [HashValue]
impl TryFrom<AutomationTaskMetaData> for AutomatedTransactionBuilder {
    type Error = anyhow::Error;

    fn try_from(value: AutomationTaskMetaData) -> Result<Self, Self::Error> {
        let AutomationTaskMetaData {
            id,
            owner,
            payload_tx,
            expiry_time,
            tx_hash,
            max_gas_amount,
            gas_price_cap,
            ..
        } = value;
        let entry_function =
            bcs::from_bytes::<EntryFunction>(payload_tx.as_slice()).map_err(|err| {
                anyhow!("Failed to extract entry function from Automation meta data{err:?}",)
            })?;
        let authenticator = HashValue::from_slice(&tx_hash)
            .map_err(|err| anyhow!("Invalid authenticator value {err:?}"))?;
        Ok(AutomatedTransactionBuilder::default()
            .with_sender(owner)
            .with_sequence_number(id)
            .with_max_gas_amount(max_gas_amount)
            .with_gas_price_cap(gas_price_cap)
            .with_expiration_timestamp_secs(expiry_time)
            .with_entry_function(entry_function)
            .with_authenticator(authenticator))
    }
}

// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::match_result_ok)] // Required to overcome the limitations of deriving Union

mod account;
mod address;
mod block;
mod bytecode;
mod convert;
mod derives;
mod error;
mod hash;
mod headers;
mod index;
mod ledger_info;
pub mod mime_types;
mod move_types;
mod state;
mod table;
pub mod transaction;
mod view;
mod wrappers;

pub use account::{AccountData, AssetType};
pub use address::Address;
pub use block::{BcsBlock, Block};
pub use bytecode::Bytecode;
pub use convert::{new_vm_utf8_string, AsConverter, MoveConverter};
pub use error::{AptosError, AptosErrorCode};
pub use hash::HashValue;
pub use headers::*;
pub use index::{IndexResponse, IndexResponseBcs};
pub use ledger_info::LedgerInfo;
pub use move_types::{
    verify_field_identifier, verify_function_identifier, verify_module_identifier, EntryFunctionId,
    HexEncodedBytes, MoveAbility, MoveFunction, MoveFunctionGenericTypeParam,
    MoveFunctionVisibility, MoveModule, MoveModuleBytecode, MoveModuleId, MoveResource,
    MoveScriptBytecode, MoveStruct, MoveStructField, MoveStructTag, MoveType, MoveValue,
    ResourceGroup, MAX_RECURSIVE_TYPES_ALLOWED, U128, U256, U64,
};
use serde::{Deserialize, Deserializer};
pub use state::RawStateValueRequest;
use std::str::FromStr;
pub use table::{RawTableItemRequest, TableItemRequest};
pub use transaction::{
    AbstractSignature, AccountSignature, BlockMetadataTransaction, DeleteModule, DeleteResource,
    DeleteTableItem, DirectWriteSet, Ed25519Signature, EncodeSubmissionRequest,
    EntryFunctionPayload, Event, FeePayerSignature, GasEstimation, GasEstimationBcs,
    GenesisPayload, GenesisTransaction, MultiAgentSignature, MultiEd25519Signature,
    MultiKeySignature, MultisigPayload, MultisigTransactionPayload, NoAccountSignature,
    PendingTransaction, PublicKey, ScriptPayload, ScriptWriteSet, Signature, SingleKeySignature,
    SubmitTransactionRequest, Transaction, TransactionData, TransactionId, TransactionInfo,
    TransactionOnChainData, TransactionPayload, TransactionSignature, TransactionSigningMessage,
    TransactionSummary, TransactionsBatchSingleSubmissionFailure,
    TransactionsBatchSubmissionResult, UserCreateSigningMessageRequest, UserTransaction,
    UserTransactionRequest, VersionedEvent, WriteModule, WriteResource, WriteSet, WriteSetChange,
    WriteSetPayload, WriteTableItem,
};
pub use view::{ViewFunction, ViewRequest};
pub use wrappers::{EventGuid, IdentifierWrapper, StateKeyWrapper};

pub fn deserialize_from_string<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    use serde::de::Error;

    let s = <String>::deserialize(deserializer)?;
    s.parse::<T>().map_err(D::Error::custom)
}

/// For verifying a given struct
pub trait VerifyInput {
    fn verify(&self) -> anyhow::Result<()>;
}

/// For verifying a given struct that needs to limit recursion
pub trait VerifyInputWithRecursion {
    fn verify(&self, recursion_count: u8) -> anyhow::Result<()>;
}

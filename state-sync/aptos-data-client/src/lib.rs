// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_storage_service_types::{
    Epoch,
    responses::{CompleteDataRange, TransactionOrOutputListWithProof},
};
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    state_store::state_value::StateValueChunkWithProof,
    transaction::{TransactionListWithProof, TransactionOutputListWithProof, Version},
};
use async_trait::async_trait;
use error::Error;
use global_summary::GlobalDataSummary;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{fmt, fmt::Display};
use thiserror::Error;

pub type ResponseId = u64;

pub mod aptosnet;
mod error;
mod global_summary;
mod interface;

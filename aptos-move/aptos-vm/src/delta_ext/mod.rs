// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::account_config::CORE_CODE_ADDRESS;
use move_deps::move_core_types::{ident_str, identifier::IdentStr, language_storage::ModuleId};
use once_cell::sync::Lazy;

mod delta_change_set;
mod transaction;

pub(crate) const AGGREGATOR_MODULE_IDENTIFIER: &IdentStr = ident_str!("aggregator");
pub(crate) static AGGREGATOR_MODULE: Lazy<ModuleId> =
    Lazy::new(|| ModuleId::new(CORE_CODE_ADDRESS, AGGREGATOR_MODULE_IDENTIFIER.to_owned()));

pub use crate::delta_ext::{
    delta_change_set::{addition, deserialize, serialize, subtraction, DeltaChangeSet, DeltaOp},
    transaction::{ChangeSetExt, TransactionOutputExt},
};

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::traits::{FromOnChainGasSchedule, InitialGasSchedule, ToOnChainGasSchedule};
use std::collections::BTreeMap;

mod instr;
mod macros;
mod misc;
mod storage;
mod transaction;

pub use instr::{gas_params as instr_gas_params, InstructionGasParameters};
pub use misc::{gas_params as misc_gas_params, AbstractValueSizeGasParameters, MiscGasParameters};
pub use storage::*;
pub use transaction::{gas_params as txn_gas_params, TransactionGasParameters};

/// Gas parameters for everything that is needed to run the Aptos blockchain, including
/// instructions, transactions and native functions from various packages.
#[derive(Debug, Clone)]
pub struct AptosGasParameters {
    pub misc: MiscGasParameters,
    pub instr: InstructionGasParameters,
    pub txn: TransactionGasParameters,
}

impl FromOnChainGasSchedule for AptosGasParameters {
    fn from_on_chain_gas_schedule(
        gas_schedule: &BTreeMap<String, u64>,
        feature_version: u64,
    ) -> Result<Self, String> {
        Ok(Self {
            misc: FromOnChainGasSchedule::from_on_chain_gas_schedule(
                gas_schedule,
                feature_version,
            )?,
            instr: FromOnChainGasSchedule::from_on_chain_gas_schedule(
                gas_schedule,
                feature_version,
            )?,
            txn: FromOnChainGasSchedule::from_on_chain_gas_schedule(gas_schedule, feature_version)?,
        })
    }
}

impl ToOnChainGasSchedule for AptosGasParameters {
    fn to_on_chain_gas_schedule(&self, feature_version: u64) -> Vec<(String, u64)> {
        let mut entries = self.instr.to_on_chain_gas_schedule(feature_version);
        entries.extend(self.txn.to_on_chain_gas_schedule(feature_version));
        entries.extend(self.misc.to_on_chain_gas_schedule(feature_version));
        entries
    }
}

impl AptosGasParameters {
    pub fn zeros() -> Self {
        Self {
            misc: MiscGasParameters::zeros(),
            instr: InstructionGasParameters::zeros(),
            txn: TransactionGasParameters::zeros(),
        }
    }
}

impl InitialGasSchedule for AptosGasParameters {
    fn initial() -> Self {
        Self {
            misc: InitialGasSchedule::initial(),
            instr: InitialGasSchedule::initial(),
            txn: InitialGasSchedule::initial(),
        }
    }
}

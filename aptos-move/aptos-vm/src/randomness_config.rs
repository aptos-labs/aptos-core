// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::AptosMoveResolver;
use aptos_types::on_chain_config::{
    randomness_api_v0_config::{AllowCustomMaxGasFlag, RequiredGasDeposit},
    OnChainConfig,
};

/// A collection of on-chain randomness API configs that VM needs to be aware of.
pub(crate) struct AptosVMRandomnessConfig {
    pub(crate) randomness_api_v0_required_deposit: Option<u64>,
    pub(crate) allow_rand_contract_custom_max_gas: bool,
}

impl AptosVMRandomnessConfig {
    pub(crate) fn fetch(resolver: &impl AptosMoveResolver) -> Self {
        let randomness_api_v0_required_deposit = RequiredGasDeposit::fetch_config(resolver)
            .unwrap_or_else(RequiredGasDeposit::default_if_missing)
            .gas_amount;
        let allow_rand_contract_custom_max_gas = AllowCustomMaxGasFlag::fetch_config(resolver)
            .unwrap_or_else(AllowCustomMaxGasFlag::default_if_missing)
            .value;
        Self {
            randomness_api_v0_required_deposit,
            allow_rand_contract_custom_max_gas,
        }
    }
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils::*;
use anyhow::Result;
use aptos_crypto::HashValue;
use move_model::{code_writer::CodeWriter, emitln, model::Loc};

pub fn generate_fee_distribution_proposal(
    function_name: String,
    burn_percentage: u8,
    is_testnet: bool,
    next_execution_hash: Option<HashValue>,
    is_multi_step: bool,
) -> Result<Vec<(String, String)>> {
    let mut result = vec![];

    let writer = CodeWriter::new(Loc::default());

    let proposal = generate_governance_proposal(
        &writer,
        is_testnet,
        next_execution_hash,
        is_multi_step,
        &["aptos_framework::transaction_fee"],
        |writer| {
            emitln!(
                writer,
                "transaction_fee::{}(framework_signer, {});",
                function_name,
                burn_percentage,
            );
        },
    );

    result.push(("transaction_fee".to_string(), proposal));
    Ok(result)
}

pub fn generate_proposal_to_initialize_fee_collection_and_distribution(
    burn_percentage: u8,
    is_testnet: bool,
    next_execution_hash: Option<HashValue>,
    is_multi_step: bool,
) -> Result<Vec<(String, String)>> {
    generate_fee_distribution_proposal(
        "initialize_fee_collection_and_distribution".to_string(),
        burn_percentage,
        is_testnet,
        next_execution_hash,
        is_multi_step,
    )
}

pub fn generate_proposal_to_upgrade_burn_percentage(
    burn_percentage: u8,
    is_testnet: bool,
    next_execution_hash: Option<HashValue>,
    is_multi_step: bool,
) -> Result<Vec<(String, String)>> {
    generate_fee_distribution_proposal(
        "upgrade_burn_percentage".to_string(),
        burn_percentage,
        is_testnet,
        next_execution_hash,
        is_multi_step,
    )
}

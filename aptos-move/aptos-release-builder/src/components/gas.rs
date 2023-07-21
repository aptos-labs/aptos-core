// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils::*;
use anyhow::Result;
use aptos_types::on_chain_config::GasScheduleV2;
use move_model::{code_writer::CodeWriter, emit, emitln, model::Loc};

pub fn generate_gas_upgrade_proposal(
    gas_schedule: &GasScheduleV2,
    is_testnet: bool,
    next_execution_hash: Vec<u8>,
) -> Result<Vec<(String, String)>> {
    let mut result = vec![];

    let writer = CodeWriter::new(Loc::default());

    emitln!(
        writer,
        "// source commit hash: {}\n",
        aptos_build_info::get_git_hash()
    );

    emitln!(writer, "// Gas schedule upgrade proposal\n");

    emitln!(
        writer,
        "// Feature version: {}",
        gas_schedule.feature_version
    );
    emitln!(writer, "//");
    emitln!(writer, "// Entries:");
    let max_len = gas_schedule
        .entries
        .iter()
        .fold(0, |acc, (name, _)| usize::max(acc, name.len()));
    for (name, val) in &gas_schedule.entries {
        let name_with_spaces = format!("{}{}", name, " ".repeat(max_len - name.len()));
        emitln!(writer, "//     {} : {}", name_with_spaces, val);
    }
    emitln!(writer);

    let proposal = generate_governance_proposal(
        &writer,
        is_testnet,
        next_execution_hash.clone(),
        &["aptos_framework::gas_schedule"],
        |writer| {
            let gas_schedule_blob = bcs::to_bytes(gas_schedule).unwrap();
            assert!(gas_schedule_blob.len() < 65536);
            emit!(writer, "let gas_schedule_blob: vector<u8> = ");
            generate_blob(writer, &gas_schedule_blob);
            emitln!(writer, ";\n");
            // The else statement has & before the framework_signer.
            // The testnet single-step generation had something like let framework_signer = &core_signer;
            // so that their framework_signer is of type &signer, but for mainnet single-step and multi-step,
            // the framework_signer is of type signer.
            if is_testnet && next_execution_hash.is_empty() {
                emitln!(
                    writer,
                    "gas_schedule::set_gas_schedule(framework_signer, gas_schedule_blob);"
                );
            } else {
                emitln!(
                    writer,
                    "gas_schedule::set_gas_schedule(&framework_signer, gas_schedule_blob);"
                );
            }
        },
    );

    result.push(("gas-schedule".to_string(), proposal));
    Ok(result)
}

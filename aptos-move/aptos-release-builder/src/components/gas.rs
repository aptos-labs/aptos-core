// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{components::get_signer_arg, utils::*};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_framework::generate_blob_as_hex_string;
use aptos_types::on_chain_config::{DiffItem, GasScheduleV2};
use move_model::{code_writer::CodeWriter, emit, emitln, model::Loc};
use sha3::{Digest, Sha3_512};

fn emit_gas_schedule_diff(
    writer: &CodeWriter,
    old_gas_schedule: &GasScheduleV2,
    new_gas_schedule: &GasScheduleV2,
) -> Result<()> {
    emitln!(writer, "// Changes");
    if old_gas_schedule.feature_version != new_gas_schedule.feature_version {
        emitln!(
            writer,
            "//   Feature version: {} -> {}",
            old_gas_schedule.feature_version,
            new_gas_schedule.feature_version
        );
    }
    let changes = GasScheduleV2::diff(old_gas_schedule, new_gas_schedule);
    if !changes.is_empty() {
        let max_len = changes
            .iter()
            .fold(0, |acc, (name, _)| usize::max(acc, name.len()));

        emitln!(writer, "//   Parameters");
        for (param_name, delta) in &changes {
            let name_with_spaces =
                format!("{}{}", param_name, " ".repeat(max_len - param_name.len()));
            match delta {
                DiffItem::Add { new_val } => {
                    emitln!(writer, "//      +  {} :  {}", name_with_spaces, new_val);
                },
                DiffItem::Delete { old_val } => {
                    emitln!(writer, "//      -  {} :  {}", name_with_spaces, old_val);
                },
                DiffItem::Modify { old_val, new_val } => {
                    emitln!(
                        writer,
                        "//         {} :  {} -> {}",
                        name_with_spaces,
                        old_val,
                        new_val
                    );
                },
            }
        }
    }

    Ok(())
}

fn emit_full_gas_schedule(writer: &CodeWriter, gas_schedule: &GasScheduleV2) -> Result<()> {
    emitln!(writer, "// Full gas schedule");
    emitln!(
        writer,
        "//   Feature version: {}",
        gas_schedule.feature_version
    );
    emitln!(writer, "//   Parameters:");
    let max_len = gas_schedule
        .entries
        .iter()
        .fold(0, |acc, (name, _)| usize::max(acc, name.len()));
    for (name, val) in &gas_schedule.entries {
        let name_with_spaces = format!("{}{}", name, " ".repeat(max_len - name.len()));
        emitln!(writer, "//     {} : {}", name_with_spaces, val);
    }
    emitln!(writer);

    Ok(())
}

pub fn generate_gas_upgrade_proposal(
    old_gas_schedule: Option<&GasScheduleV2>,
    new_gas_schedule: &GasScheduleV2,
    is_testnet: bool,
    next_execution_hash: Option<HashValue>,
    is_multi_step: bool,
) -> Result<Vec<(String, String)>> {
    let signer_arg = get_signer_arg(is_testnet, &next_execution_hash);
    let mut result = vec![];

    let writer = CodeWriter::new(Loc::default());

    emitln!(
        writer,
        "// Source commit hash: {}",
        aptos_build_info::get_git_hash()
    );
    emitln!(writer);

    emitln!(writer, "// Gas schedule upgrade proposal");

    let old_hash = match old_gas_schedule {
        Some(old_gas_schedule) => {
            let old_bytes = bcs::to_bytes(old_gas_schedule)?;
            let old_hash = hex::encode(Sha3_512::digest(old_bytes.as_slice()));
            emitln!(writer, "//");
            emitln!(writer, "// Old Gas Schedule Hash (Sha3-512): {}", old_hash);

            emit_gas_schedule_diff(&writer, old_gas_schedule, new_gas_schedule)?;

            Some(old_hash)
        },
        None => None,
    };
    emitln!(writer, "//");
    emit_full_gas_schedule(&writer, new_gas_schedule)?;

    let proposal = generate_governance_proposal(
        &writer,
        is_testnet,
        next_execution_hash,
        is_multi_step,
        &["aptos_framework::gas_schedule"],
        |writer| {
            let gas_schedule_blob = bcs::to_bytes(new_gas_schedule).unwrap();
            assert!(gas_schedule_blob.len() < 65536);

            emit!(writer, "let gas_schedule_blob: vector<u8> = ");
            generate_blob_as_hex_string(writer, &gas_schedule_blob);
            emitln!(writer, ";");
            emitln!(writer);

            match old_hash {
                Some(old_hash) => {
                    emitln!(
                        writer,
                        "gas_schedule::set_for_next_epoch_check_hash({}, x\"{}\", gas_schedule_blob);",
                        signer_arg,
                        old_hash,
                    );
                },
                None => {
                    emitln!(
                        writer,
                        "gas_schedule::set_for_next_epoch({}, gas_schedule_blob);",
                        signer_arg
                    );
                },
            }
            emitln!(writer, "aptos_governance::reconfigure({});", signer_arg);
        },
    );

    result.push(("gas-schedule".to_string(), proposal));
    Ok(result)
}

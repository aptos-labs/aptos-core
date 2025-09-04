// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This crate implements a script for generating governance proposals to update the
//! on-chain gas schedule. It can be used as both a library and a standalone binary.
//!
//! The generated proposal includes a comment section, listing the contents of the
//! gas schedule in a human readable format.

use anyhow::Result;
use velor_gas_schedule::{
    VelorGasParameters, InitialGasSchedule, ToOnChainGasSchedule, LATEST_GAS_FEATURE_VERSION,
};
use velor_package_builder::PackageBuilder;
use velor_types::on_chain_config::GasScheduleV2;
use clap::Parser;
use move_core_types::account_address::AccountAddress;
use move_model::{code_writer::CodeWriter, emit, emitln, model::Loc};
use std::path::{Path, PathBuf};

fn generate_blob(writer: &CodeWriter, data: &[u8]) {
    emitln!(writer, "vector[");
    writer.indent();
    for (i, b) in data.iter().enumerate() {
        if i % 20 == 0 {
            if i > 0 {
                emitln!(writer);
            }
        } else {
            emit!(writer, " ");
        }
        emit!(writer, "{},", b);
    }
    emitln!(writer);
    writer.unindent();
    emit!(writer, "]")
}

fn generate_script(gas_schedule: &GasScheduleV2) -> Result<String> {
    let gas_schedule_blob = bcs::to_bytes(gas_schedule).unwrap();

    assert!(gas_schedule_blob.len() < 65536);

    let writer = CodeWriter::new(Loc::default());
    emitln!(writer, "// Gas schedule update proposal\n");

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

    emitln!(writer, "script {");
    writer.indent();

    emitln!(writer, "use velor_framework::velor_governance;");
    emitln!(writer, "use velor_framework::gas_schedule;");
    emitln!(writer);

    emitln!(writer, "fun main(proposal_id: u64) {");
    writer.indent();

    emitln!(
        writer,
        "let framework_signer = velor_governance::resolve(proposal_id, @{});\n",
        AccountAddress::ONE,
    );

    emit!(writer, "let gas_schedule_blob: vector<u8> = ");
    generate_blob(&writer, &gas_schedule_blob);
    emitln!(writer, ";\n");

    emitln!(
        writer,
        "gas_schedule::set_gas_schedule(&framework_signer, gas_schedule_blob);"
    );

    writer.unindent();
    emitln!(writer, "}");

    writer.unindent();
    emitln!(writer, "}");

    Ok(writer.process_result(|s| s.to_string()))
}

fn velor_framework_path() -> PathBuf {
    Path::join(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "../framework/velor-framework",
    )
}

/// Command line arguments to the gas schedule update proposal generation tool.
#[derive(Debug, Parser)]
pub struct GenArgs {
    #[clap(short, long)]
    pub output: Option<String>,

    #[clap(short, long)]
    pub gas_feature_version: Option<u64>,
}

/// Constructs the current gas schedule in on-chain format.
pub fn current_gas_schedule(feature_version: u64) -> GasScheduleV2 {
    GasScheduleV2 {
        feature_version,
        entries: VelorGasParameters::initial().to_on_chain_gas_schedule(feature_version),
    }
}

/// Entrypoint for the update proposal generation tool.
pub fn generate_update_proposal(args: &GenArgs) -> Result<()> {
    let mut pack = PackageBuilder::new("GasScheduleUpdate");

    let feature_version = args
        .gas_feature_version
        .unwrap_or(LATEST_GAS_FEATURE_VERSION);

    pack.add_source(
        "update_gas_schedule.move",
        &generate_script(&current_gas_schedule(feature_version))?,
    );
    // TODO: use relative path here
    pack.add_local_dep("VelorFramework", &velor_framework_path().to_string_lossy());

    pack.write_to_disk(args.output.as_deref().unwrap_or("./proposal"))?;

    Ok(())
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    GenArgs::command().debug_assert()
}

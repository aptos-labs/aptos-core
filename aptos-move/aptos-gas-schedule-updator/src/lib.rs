// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This crate implements a script for generating governance proposals to update the
//! on-chain gas schedule. It can be used as both a library and a standalone binary.
//!
//! The generated proposal includes a comment section, listing the contents of the
//! gas schedule in a human readable format.

use std::fs;
use anyhow::Result;
use aptos_gas_schedule::{
    AptosGasParameters, InitialGasSchedule, ToOnChainGasSchedule, LATEST_GAS_FEATURE_VERSION,
};
use aptos_package_builder::PackageBuilder;
use aptos_types::on_chain_config::GasScheduleV2;
use clap::Parser;
use move_core_types::account_address::AccountAddress;
use move_model::{code_writer::CodeWriter, emit, emitln, model::Loc};
use std::path::{Path, PathBuf};

const DEFAULT_GAS_SCHEDULE_SCRIPT_UPDATE_PATH: String = String::from("./proposals");

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

    emitln!(writer, "use supra_framework::supra_governance;");
    emitln!(writer, "use supra_framework::gas_schedule;");
    emitln!(writer);

    emitln!(writer, "fun main(proposal_id: u64) {");
    writer.indent();

    emitln!(
        writer,
        "let framework_signer = supra_governance::supra_resolve(proposal_id, @{});\n",
        AccountAddress::ONE,
    );

    emit!(writer, "let gas_schedule_blob: vector<u8> = ");
    generate_blob(&writer, &gas_schedule_blob);
    emitln!(writer, ";\n");

    emitln!(
        writer,
        "gas_schedule::set_for_next_epoch(&framework_signer, gas_schedule_blob);"
    );
    emitln!(writer, "supra_governance::reconfigure(&framework_signer);");

    writer.unindent();
    emitln!(writer, "}");

    writer.unindent();
    emitln!(writer, "}");

    Ok(writer.process_result(|s| s.to_string()))
}

fn aptos_framework_path() -> PathBuf {
    Path::join(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "../framework/supra-framework",
    )
}

#[derive(Parser, Debug)]
pub enum GasScheduleGenerator {
    #[clap(short, long)]
    GenerateNew(GenerateNewSchedule),
    #[clap(short, long)]
    ScaleCurrent(GenerateNewSchedule),
}

/// Command line arguments to the gas schedule update proposal generation tool.
#[derive(Debug, Parser)]
pub struct GenerateNewSchedule {
    #[clap(short, long, help = "Path to file to write the output script")]
    pub output: Option<String>,

    #[clap(short, long)]
    pub gas_feature_version: Option<u64>,
}

impl GenerateNewSchedule {
    pub fn execute(self) -> Result<()> {
        let feature_version = self
            .gas_feature_version
            .unwrap_or(LATEST_GAS_FEATURE_VERSION);

        let gas_schedule = current_gas_schedule(feature_version);

        generate_update_proposal(&gas_schedule, self.output.unwrap_or(DEFAULT_GAS_SCHEDULE_SCRIPT_UPDATE_PATH))
    }
}


#[derive(Debug, Parser)]
pub struct ScaleCurrentSchedule {
    #[clap(short, long, help = "Path to file to write the output script")]
    pub output: Option<String>,

    #[clap(short, long, help = "Path to JSON file containing the GasScheduleV2 to use")]
    pub current_schedule: String,

    #[clap(short, long, help = "Scale the Minimum Gas Price value with the given factor")]
    pub scale_min_gas_price_by: f64,
}


impl ScaleCurrentSchedule {
    pub fn execute(self) -> Result<()> {
        let json_str = fs::read_to_string(self.current_schedule)?;
        let mut current_schedule = GasScheduleV2::from_json_string(json_str);

        current_schedule.scale_min_gas_price_by(self.scale_min_gas_price_by);

        generate_update_proposal(&current_schedule, self.output.unwrap_or(DEFAULT_GAS_SCHEDULE_SCRIPT_UPDATE_PATH))
    }
}

/// Constructs the current gas schedule in on-chain format.
pub fn current_gas_schedule(feature_version: u64) -> GasScheduleV2 {
    GasScheduleV2 {
        feature_version,
        entries: AptosGasParameters::initial().to_on_chain_gas_schedule(feature_version),
    }
}

/// Entrypoint for the update proposal generation tool.
pub fn generate_update_proposal(gas_schedule: &GasScheduleV2, output_path: String) -> Result<()> {
    let mut pack = PackageBuilder::new("GasScheduleUpdate");

    pack.add_source(
        "update_gas_schedule.move",
        &generate_script(gas_schedule)?,
    );
    // TODO: use relative path here
    pack.add_local_dep("SupraFramework", &aptos_framework_path().to_string_lossy());

    pack.write_to_disk(PathBuf::from(output_path))?;

    Ok(())
}


impl GasScheduleGenerator {
    pub fn execute(self) -> Result<()> {
        match self {
            GasScheduleGenerator::GenerateNew(args) => {
                args.execute()
            }
            GasScheduleGenerator::ScaleCurrent(args) => {
                args.execute()
            }
        }
    }
}


#[test]
fn verify_tool() {
    use clap::CommandFactory;
    GenerateNewSchedule::command().debug_assert()
}

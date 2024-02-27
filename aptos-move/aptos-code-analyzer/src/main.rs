// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::fs;
use std::path::PathBuf;
use aptos_code_analyzer::cost_cfg::InstructionCostCalculator;
use aptos_language_e2e_tests::executor::FakeExecutor;
use move_binary_format::CompiledModule;
use aptos_code_analyzer::types::ModulePass;
use clap::Parser;

#[derive(Parser)]
pub struct Argument {
    path: PathBuf,
}

fn main() {
    let args = Argument::parse();
    let path = args.path.as_path();
    let bytes = fs::read(path).expect("Should be able to read data");

    let exec = FakeExecutor::from_head_genesis();
    let state_view = exec.get_state_view();
    let mut calculator = InstructionCostCalculator::new(state_view);

    let code = CompiledModule::deserialize(&bytes).expect("Success");
    calculator.run_on_module(&code);
}

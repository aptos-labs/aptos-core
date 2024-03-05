// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_code_analyzer::{
    passes::{
        analyze_modules::ModuleAnalysis,
        collect_cost_cfgs::{CollectCostCFGs, Counter, Extension},
    },
    ModulePass,
};
use aptos_language_e2e_tests::executor::FakeExecutor;
use clap::{Parser, Subcommand};
use move_binary_format::CompiledModule;
use std::{
    fs,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

#[derive(Subcommand)]
pub enum Cmd {
    AnalyzeCode {
        output_dir: PathBuf,
    },
    BuildCfgs {
        output_dir: PathBuf,
        #[clap(value_enum)]
        output_extension: Extension,
        #[clap(value_enum)]
        counter: Counter,
    },
}

#[derive(Parser)]
pub struct Argument {
    // Path to a file which contains all paths to bytecode modules.
    state_view_path: PathBuf,
    #[clap(subcommand)]
    cmd: Cmd,
}

fn main() {
    let args = Argument::parse();

    match args.cmd {
        Cmd::AnalyzeCode { output_dir } => {
            let (_, modules) = initialize(args.state_view_path);
            let mut pass = ModuleAnalysis::new(output_dir);
            for module in modules.into_iter() {
                pass.run_on_module(&module)
            }
            pass.finish();
        },
        Cmd::BuildCfgs {
            output_dir,
            output_extension,
            counter,
        } => {
            let (exec, modules) = initialize(args.state_view_path);
            let mut pass = CollectCostCFGs::new(
                &output_dir,
                &output_extension,
                exec.get_state_view(),
                &counter,
            );
            for module in modules.into_iter() {
                pass.run_on_module(&module)
            }
        },
    }
}

fn initialize(state_view_path: PathBuf) -> (FakeExecutor, Vec<CompiledModule>) {
    let state_view_file = File::open(state_view_path.as_path())
        .expect("Must provide a file with paths to all bytecode modules");
    let reader = BufReader::new(state_view_file);

    // State view doesn't have any modules by default.
    let mut exec = FakeExecutor::from_head_genesis();
    let mut modules = Vec::new();

    let mut success_cnt = 0;
    let mut total_cnt = 0;
    for line in reader.lines() {
        let line = line.unwrap();
        let mut iter = line.split_ascii_whitespace();
        let bytecode_path = iter.next().unwrap();

        let bytes = fs::read(bytecode_path).expect("Should be able to read module data");
        match CompiledModule::deserialize(&bytes) {
            Ok(module) => {
                exec.add_module(&module.self_id(), bytes);
                modules.push(module);
                success_cnt += 1;
            },
            Err(e) => {
                // Not much we can do, so log and skip.
                println!("ERROR: Unable to deserialize module: {}", e);
            },
        }
        total_cnt += 1;
    }
    println!("Processed {success_cnt}/{total_cnt} modules...");
    (exec, modules)
}

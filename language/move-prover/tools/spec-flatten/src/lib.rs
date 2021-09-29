// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use std::collections::BTreeMap;
use structopt::StructOpt;

use bytecode::{
    function_target_pipeline::FunctionTargetsHolder, options::ProverOptions,
    pipeline_factory::default_pipeline_with_options,
};
use move_lang::shared::{parse_named_address, AddressBytes};
use move_model::{model::GlobalEnv, options::ModelBuilderOptions, run_model_builder_with_options};

mod flatten;

/// Options passed into the specification flattening tool.
#[derive(StructOpt)]
pub struct FlattenOptions {
    /// Sources of the target modules
    pub srcs: Vec<String>,

    /// Dependencies
    #[structopt(short = "d", long = "dependency")]
    pub deps: Vec<String>,

    /// Do not include default named address
    #[structopt(long = "no-default-named-addresses")]
    pub no_default_named_addresses: bool,

    /// Extra mappings for named address
    #[structopt(short = "a", long = "address", parse(try_from_str = parse_named_address))]
    pub named_addresses_extra: Option<Vec<(String, AddressBytes)>>,
}

//**************************************************************************************************
// Entrypoint
//**************************************************************************************************

pub fn run(options: &FlattenOptions) -> Result<()> {
    let (env, targets) = prepare(options)?;

    // collect spec in target modules
    for (fid, variant) in targets.get_funs_and_variants() {
        if !variant.is_verified() {
            // only care for functions that are marked as verified
            continue;
        }

        let fun_env = env.get_function(fid);
        if !fun_env.module_env.is_target() {
            // only run on specs in target module
            continue;
        }
        if !fun_env.has_unknown_callers() {
            // only run on specs for external-facing functions
            continue;
        }

        let target = targets.get_target(&fun_env, &variant);
        flatten::flatten_spec(target, &targets);
    }

    // everything is OK
    Ok(())
}

fn prepare(options: &FlattenOptions) -> Result<(GlobalEnv, FunctionTargetsHolder)> {
    // build mapping for named addresses
    let mut named_addresses = BTreeMap::new();
    if !options.no_default_named_addresses {
        let default_mapping = [
            ("Std", "0x1"),
            ("DiemFramework", "0x1"),
            ("DiemRoot", "0xA550C18"),
            ("CurrencyInfo", "0xA550C18"),
            ("TreasuryCompliance", "0xB1E55ED"),
            ("VMReserved", "0x0"),
        ];
        named_addresses.extend(
            default_mapping
                .iter()
                .map(|(name, addr)| (name.to_string(), AddressBytes::parse_str(addr).unwrap())),
        );
    }

    // run move model builder
    let env = run_model_builder_with_options(
        &options.srcs,
        &options.deps,
        ModelBuilderOptions::default(),
        named_addresses,
    )?;

    // run bytecode transformation pipeline
    let prover_options = ProverOptions::default();
    let pipeline = default_pipeline_with_options(&prover_options);
    env.set_extension(prover_options);

    let mut targets = FunctionTargetsHolder::default();
    for module_env in env.get_modules() {
        for func_env in module_env.get_functions() {
            targets.add_target(&func_env)
        }
    }
    pipeline.run(&env, &mut targets);

    // return the GlobalEnv
    Ok((env, targets))
}

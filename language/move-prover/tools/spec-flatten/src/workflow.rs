// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use std::collections::BTreeMap;
use structopt::StructOpt;

use bytecode::{
    function_target_pipeline::FunctionTargetsHolder, options::ProverOptions,
    pipeline_factory::default_pipeline_with_options,
};
use move_lang::shared::{parse_named_address, NumericalAddress};
use move_model::{
    ast::Spec,
    model::{FunId, GlobalEnv, QualifiedId, VerificationScope},
    options::ModelBuilderOptions,
    run_model_builder_with_options,
};
use move_prover::{cli::Options as CliOptions, generate_boogie, verify_boogie};

/// Options passed into the workflow pipeline.
#[derive(StructOpt, Clone)]
pub struct WorkflowOptions {
    /// Sources of the target modules
    pub srcs: Vec<String>,

    /// Dependencies
    #[structopt(short = "d", long = "dependency")]
    pub deps: Vec<String>,

    /// Target function
    #[structopt(short, long)]
    pub target: Option<String>,

    /// Do not include default named address
    #[structopt(long = "no-default-named-addresses")]
    pub no_default_named_addresses: bool,

    /// Extra mappings for named address
    #[structopt(short = "a", long = "address", parse(try_from_str = parse_named_address))]
    pub named_addresses_extra: Option<Vec<(String, NumericalAddress)>>,

    /// Verbose mode
    #[structopt(short, long)]
    pub verbose: bool,
}

pub(crate) fn prepare(options: &WorkflowOptions) -> Result<(GlobalEnv, FunctionTargetsHolder)> {
    prepare_with_override(options, BTreeMap::new())
}

pub(crate) fn prepare_with_override(
    options: &WorkflowOptions,
    spec_override: BTreeMap<QualifiedId<FunId>, Spec>,
) -> Result<(GlobalEnv, FunctionTargetsHolder)> {
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
                .map(|(name, addr)| (name.to_string(), NumericalAddress::parse_str(addr).unwrap())),
        );
    }

    // run move model builder
    let mut env = run_model_builder_with_options(
        &options.srcs,
        &options.deps,
        ModelBuilderOptions::default(),
        named_addresses,
    )?;
    if env.has_errors() {
        return Err(anyhow!("Error in model building"));
    }

    // override the spec for functions (if requested)
    for (fun_id, spec) in spec_override {
        let module_data = env
            .module_data
            .iter_mut()
            .find(|m| m.id == fun_id.module_id)
            .ok_or_else(|| {
                anyhow!(
                    "Unable to find module with id `{}`",
                    fun_id.module_id.to_usize()
                )
            })?;
        let function_data = module_data
            .function_data
            .get_mut(&fun_id.id)
            .ok_or_else(|| anyhow!("Unable to find function with id `{:?}`", fun_id.id.symbol()))?;
        function_data.spec = spec;
    }

    // run bytecode transformation pipeline
    let prover_options = get_prover_options(options);
    let pipeline = default_pipeline_with_options(&prover_options);
    env.set_extension(prover_options);

    let mut targets = FunctionTargetsHolder::default();
    for module_env in env.get_modules() {
        for func_env in module_env.get_functions() {
            targets.add_target(&func_env)
        }
    }
    pipeline.run(&env, &mut targets);
    if env.has_errors() {
        return Err(anyhow!("Error in bytecode transformation"));
    }

    // return the GlobalEnv
    Ok((env, targets))
}

pub(crate) fn prove(
    options: &WorkflowOptions,
    env: &GlobalEnv,
    targets: &FunctionTargetsHolder,
) -> Result<bool> {
    let cli_options = get_cli_options(options);

    let code_writer = generate_boogie(env, &cli_options, targets)?;
    if env.has_errors() {
        return Err(anyhow!("Error in boogie translation"));
    }

    verify_boogie(env, &cli_options, targets, code_writer)?;
    Ok(!env.has_errors())
}

//
// utilities
//

fn get_prover_options(options: &WorkflowOptions) -> ProverOptions {
    let verify_scope = match &options.target {
        None => VerificationScope::All,
        Some(target) => VerificationScope::Only(target.clone()),
    };
    ProverOptions {
        verify_scope,
        ..Default::default()
    }
}

fn get_cli_options(options: &WorkflowOptions) -> CliOptions {
    CliOptions {
        move_sources: options.srcs.clone(),
        move_deps: options.deps.clone(),
        prover: get_prover_options(options),
        ..Default::default()
    }
}

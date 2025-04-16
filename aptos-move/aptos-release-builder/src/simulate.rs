// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements the simulation of governance proposals.
//! Currently, it supports only multi-step proposals.
//!
//! It utilizes the remote debugger infrastructure to fetch real chain states
//! for local simulation, but adds another in-memory database to store the new side effects
//! generated by the governance scripts.
//!
//! Normally, governance scripts needs to be approved through on-chain governance
//! before they could be executed. This process involves setting up various states
//! (e.g., staking pool, delegated voter), which can be quite complex.
//!
//! This simulation bypasses these challenges by patching specific Move functions
//! with mock versions, most notably `fun resolve_multi_step_proposal`, thus allowing
//! the governance process to be skipped altogether.
//!
//! In other words, this simulation is intended for checking whether a governance
//! proposal will execute successfully, assuming it gets approved, not whether the
//! governance framework itself is working as intended.

use crate::aptos_framework_path;
use anyhow::{anyhow, bail, Context, Result};
use aptos::{
    common::types::PromptOptions, governance::compile_in_temp_dir, move_tool::FrameworkPackageArgs,
};
use aptos_crypto::HashValue;
use aptos_gas_profiling::GasProfiler;
use aptos_gas_schedule::{AptosGasParameters, FromOnChainGasSchedule};
use aptos_language_e2e_tests::account::AccountData;
use aptos_move_debugger::aptos_debugger::AptosDebugger;
use aptos_rest_client::Client;
use aptos_transaction_simulation::{DeltaStateStore, SimulationStateStore};
use aptos_types::{
    account_address::AccountAddress,
    account_config::ChainIdResource,
    on_chain_config::{ApprovedExecutionHashes, GasScheduleV2, OnChainConfig},
    state_store::TStateView,
    transaction::{ExecutionStatus, Script, TransactionArgument, TransactionStatus},
};
use aptos_vm::{data_cache::AsMoveResolver, move_vm_ext::SessionId, AptosVM};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::{
    module_and_script_storage::AsAptosCodeStorage, module_write_set::ModuleWriteSet,
    storage::change_set_configs::ChangeSetConfigs,
};
use clap::Parser;
use move_binary_format::{
    access::ModuleAccess,
    file_format::{
        AddressIdentifierIndex, Bytecode, FunctionDefinition, FunctionHandle, FunctionHandleIndex,
        IdentifierIndex, ModuleHandle, ModuleHandleIndex, Signature, SignatureIndex,
        SignatureToken, Visibility,
    },
    CompiledModule,
};
use move_core_types::{
    identifier::{IdentStr, Identifier},
    language_storage::ModuleId,
    value::MoveValue,
};
use move_vm_runtime::module_traversal::{TraversalContext, TraversalStorage};
use move_vm_types::gas::UnmeteredGasMeter;
use once_cell::sync::Lazy;
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use url::Url;
use walkdir::WalkDir;

/***************************************************************************************************
 * Compiled Module Helpers
 *
 **************************************************************************************************/
fn find_function_def_by_name<'a>(
    m: &'a mut CompiledModule,
    name: &IdentStr,
) -> Option<&'a mut FunctionDefinition> {
    for (idx, func_def) in m.function_defs.iter().enumerate() {
        let func_handle = m.function_handle_at(func_def.function);
        let func_name = m.identifier_at(func_handle.name);
        if name == func_name {
            return Some(&mut m.function_defs[idx]);
        }
    }
    None
}

fn get_or_add<T: PartialEq>(pool: &mut Vec<T>, val: T) -> usize {
    match pool.iter().position(|elem| elem == &val) {
        Some(idx) => idx,
        None => {
            let idx = pool.len();
            pool.push(val);
            idx
        },
    }
}

#[allow(dead_code)]
fn get_or_add_addr(m: &mut CompiledModule, addr: AccountAddress) -> AddressIdentifierIndex {
    AddressIdentifierIndex::new(get_or_add(&mut m.address_identifiers, addr) as u16)
}

fn get_or_add_ident(m: &mut CompiledModule, ident: Identifier) -> IdentifierIndex {
    IdentifierIndex::new(get_or_add(&mut m.identifiers, ident) as u16)
}

#[allow(dead_code)]
fn get_or_add_module_handle(
    m: &mut CompiledModule,
    addr: AccountAddress,
    name: Identifier,
) -> ModuleHandleIndex {
    let addr = get_or_add_addr(m, addr);
    let name = get_or_add_ident(m, name);
    let module_handle = ModuleHandle {
        address: addr,
        name,
    };
    ModuleHandleIndex::new(get_or_add(&mut m.module_handles, module_handle) as u16)
}

fn get_or_add_signature(m: &mut CompiledModule, sig: Vec<SignatureToken>) -> SignatureIndex {
    SignatureIndex::new(get_or_add(&mut m.signatures, Signature(sig)) as u16)
}

fn find_function_handle_by_name(
    m: &CompiledModule,
    addr: AccountAddress,
    module_name: &IdentStr,
    func_name: &IdentStr,
) -> Option<FunctionHandleIndex> {
    for (idx, func_handle) in m.function_handles().iter().enumerate() {
        let module_handle = m.module_handle_at(func_handle.module);
        if m.address_identifier_at(module_handle.address) == &addr
            && m.identifier_at(module_handle.name) == module_name
            && m.identifier_at(func_handle.name) == func_name
        {
            return Some(FunctionHandleIndex(idx as u16));
        }
    }
    None
}

fn add_simple_native_function(
    m: &mut CompiledModule,
    func_name: Identifier,
    params: Vec<SignatureToken>,
    returns: Vec<SignatureToken>,
) -> Result<FunctionHandleIndex> {
    if let Some(func_handle_idx) =
        find_function_handle_by_name(m, *m.self_addr(), m.self_name(), &func_name)
    {
        return Ok(func_handle_idx);
    }

    let name = get_or_add_ident(m, func_name);
    let parameters = get_or_add_signature(m, params);
    let return_ = get_or_add_signature(m, returns);
    let func_handle = FunctionHandle {
        module: m.self_handle_idx(),
        name,
        parameters,
        return_,
        type_parameters: vec![],
        access_specifiers: None,
        attributes: vec![],
    };
    let func_handle_idx = FunctionHandleIndex(m.function_handles.len() as u16);
    m.function_handles.push(func_handle);

    let func_def = FunctionDefinition {
        function: func_handle_idx,
        visibility: Visibility::Private,
        is_entry: false,
        acquires_global_resources: vec![],
        code: None,
    };
    m.function_defs.push(func_def);

    Ok(func_handle_idx)
}

/***************************************************************************************************
 * Patches
 *
 **************************************************************************************************/
static MODULE_ID_APTOS_GOVERNANCE: Lazy<ModuleId> = Lazy::new(|| {
    ModuleId::new(
        AccountAddress::ONE,
        Identifier::new("aptos_governance").unwrap(),
    )
});

static FUNC_NAME_CREATE_SIGNER: Lazy<Identifier> =
    Lazy::new(|| Identifier::new("create_signer").unwrap());

static FUNC_NAME_RESOLVE_MULTI_STEP_PROPOSAL: Lazy<Identifier> =
    Lazy::new(|| Identifier::new("resolve_multi_step_proposal").unwrap());

const DUMMY_PROPOSAL_ID: u64 = u64::MAX;

const MAGIC_FAILED_NEXT_EXECUTION_HASH_CHECK: u64 = 0xDEADBEEF;

/// Helper to load a module from the state view, deserialize it, modify it with
/// the provided callback, reserialize it and finally write it back.
fn patch_module<F>(
    state_view: &impl SimulationStateStore,
    module_id: &ModuleId,
    modify_module: F,
) -> Result<()>
where
    F: FnOnce(&mut CompiledModule) -> Result<()>,
{
    let mut m = state_view
        .get_module(module_id)?
        .ok_or_else(|| anyhow!("module {} does not exist", module_id))?;

    modify_module(&mut m)?;

    // Sanity check to ensure the correctness of the check
    move_bytecode_verifier::verify_module(&m).map_err(|err| {
        anyhow!(
            "patched module failed to verify -- check if the patch is correct: {}",
            err
        )
    })?;

    state_view.add_module(&m)?;

    Ok(())
}

/// Patches `aptos_framework::aptos_governance::resolve_multi_step_proposal` so that
/// it returns the requested signer directly, skipping the governance process altogether.
fn patch_aptos_governance(
    state_view: &impl SimulationStateStore,
    forbid_next_execution_hash: bool,
) -> Result<()> {
    use Bytecode::*;

    patch_module(state_view, &MODULE_ID_APTOS_GOVERNANCE, |m| {
        // Inject `native fun create_signer`.
        let create_signer_handle_idx = add_simple_native_function(
            m,
            FUNC_NAME_CREATE_SIGNER.clone(),
            vec![SignatureToken::Address],
            vec![SignatureToken::Signer],
        )?;

        // Patch `fun resolve_multi_step_proposal`.
        let sig_u8_idx = get_or_add_signature(m, vec![SignatureToken::U8]);

        let func_def = find_function_def_by_name(m, &FUNC_NAME_RESOLVE_MULTI_STEP_PROPOSAL)
            .ok_or_else(|| {
                anyhow!(
                    "failed to locate `fun {}`",
                    &*FUNC_NAME_RESOLVE_MULTI_STEP_PROPOSAL
                )
            })?;
        func_def.acquires_global_resources = vec![];
        let code = func_def.code.as_mut().ok_or_else(|| {
            anyhow!(
                "`fun {}` must have a Move-defined body",
                &*FUNC_NAME_RESOLVE_MULTI_STEP_PROPOSAL
            )
        })?;

        code.code.clear();
        if forbid_next_execution_hash {
            // If it is needed to forbid a next execution hash, inject additional Move
            // code at the beginning that aborts with a magic number if the vector
            // representing the hash is not empty.
            //
            //     if (!vector::is_empty(&next_execution_hash)) {
            //         abort MAGIC_FAILED_NEXT_EXECUTION_HASH_CHECK;
            //     }
            //
            // The magic number can later be checked in Rust to determine if such violation
            // has happened.
            code.code.extend([
                ImmBorrowLoc(2),
                VecLen(sig_u8_idx),
                LdU64(0),
                Eq,
                BrTrue(7),
                LdU64(MAGIC_FAILED_NEXT_EXECUTION_HASH_CHECK),
                Abort,
            ]);
        }
        // Replace the original logic with `create_signer(signer_address)`, bypassing
        // the governance process.
        code.code
            .extend([MoveLoc(1), Call(create_signer_handle_idx), Ret]);

        Ok(())
    })
}

// Add the hash of the script to the list of approved hashes, so to enable the
// alternative (higher) execution limits.
fn add_script_execution_hash(
    state_view: &impl SimulationStateStore,
    hash: HashValue,
) -> Result<()> {
    let entry = (DUMMY_PROPOSAL_ID, hash.to_vec());

    state_view.modify_on_chain_config(|approved_hashes: &mut ApprovedExecutionHashes| {
        if !approved_hashes.entries.contains(&entry) {
            approved_hashes.entries.push(entry);
        }
        Ok(())
    })
}

/***************************************************************************************************
 * Simulation Workflow
 *
 **************************************************************************************************/
fn force_end_epoch(state_view: &impl SimulationStateStore) -> Result<()> {
    let env = AptosEnvironment::new_with_injected_create_signer_for_gov_sim(&state_view);
    let vm = AptosVM::new(&env, &state_view);
    let resolver = state_view.as_move_resolver();
    let module_storage = state_view.as_aptos_code_storage(&env);

    let gas_schedule =
        GasScheduleV2::fetch_config(&state_view).context("failed to fetch gas schedule v2")?;
    let gas_feature_version = gas_schedule.feature_version;

    let change_set_configs =
        ChangeSetConfigs::unlimited_at_gas_feature_version(gas_feature_version);

    let traversal_storage = TraversalStorage::new();
    let mut sess = vm.new_session(&resolver, SessionId::void(), None);
    sess.execute_function_bypass_visibility(
        &MODULE_ID_APTOS_GOVERNANCE,
        IdentStr::new("force_end_epoch").unwrap(),
        vec![],
        vec![MoveValue::Signer(AccountAddress::ONE)
            .simple_serialize()
            .unwrap()],
        &mut UnmeteredGasMeter,
        &mut TraversalContext::new(&traversal_storage),
        &module_storage,
    )?;
    let mut change_set = sess.finish(&change_set_configs, &module_storage)?;

    change_set.try_materialize_aggregator_v1_delta_set(&resolver)?;
    let (write_set, _events) = change_set
        .try_combine_into_storage_change_set(ModuleWriteSet::empty())
        .expect("Failed to convert to storage ChangeSet")
        .into_inner();

    state_view.apply_write_set(&write_set)?;

    Ok(())
}

pub async fn simulate_multistep_proposal(
    remote_url: Url,
    proposal_dir: &Path,
    proposal_scripts: &[PathBuf],
    profile_gas: bool,
) -> Result<()> {
    println!("Simulating proposal at {}", proposal_dir.display());

    // Compile all scripts.
    println!("Compiling scripts...");
    let mut compiled_scripts = vec![];
    for path in proposal_scripts {
        let framework_package_args = FrameworkPackageArgs::try_parse_from([
            "dummy_executable_name",
            "--framework-local-dir",
            &aptos_framework_path().to_string_lossy(),
            "--skip-fetch-latest-git-deps",
        ])
        .context(
            "failed to parse framework package args for compiling scripts, this should not happen",
        )?;

        let (blob, hash) = compile_in_temp_dir(
            "script",
            path,
            &framework_package_args,
            PromptOptions::yes(),
            None, // bytecode_version
            None, // language_version
            None, // compiler_version
        )
        .with_context(|| format!("failed to compile script {}", path.display()))?;

        compiled_scripts.push((blob, hash));
    }

    // Set up the simulation state view.
    let client = Client::new(remote_url);
    let debugger =
        AptosDebugger::rest_client(client.clone()).context("failed to create AptosDebugger")?;
    let state = client.get_ledger_information().await?.into_inner();

    let state_view = DeltaStateStore::new_with_base(debugger.state_view_at_version(state.version));

    // Create and fund a sender account that is used to send the governance scripts.
    print!("Creating and funding sender account.. ");
    std::io::stdout().flush()?;
    let mut rng = aptos_keygen::KeyGen::from_seed([0; 32]);
    let balance = 100 * 1_0000_0000; // 100 APT
                                     // TODO[Orderless]: Check if this needs modification to accommodate stateless accounts
    let account = AccountData::new_from_seed(&mut rng, balance, Some(0));
    state_view.apply_write_set(&account.to_writeset())?;
    // TODO: should update coin info (total supply)
    println!("done");

    // Execute the governance scripts in sorted order.
    println!("Executing governance scripts...");

    for (script_idx, (script_path, (script_blob, script_hash))) in
        proposal_scripts.iter().zip(compiled_scripts).enumerate()
    {
        // Force-end the epoch so that buffered configuration changes get applied.
        force_end_epoch(&state_view).context("failed to force end epoch")?;

        // Fetch the on-chain configs that are needed for the simulation.
        let chain_id =
            ChainIdResource::fetch_config(&state_view).context("failed to fetch chain id")?;

        let gas_schedule =
            GasScheduleV2::fetch_config(&state_view).context("failed to fetch gas schedule v2")?;
        let gas_feature_version = gas_schedule.feature_version;
        let gas_params = AptosGasParameters::from_on_chain_gas_schedule(
            &gas_schedule.into_btree_map(),
            gas_feature_version,
        )
        .map_err(|err| {
            anyhow!(
                "failed to construct gas params at gas version {}: {}",
                gas_feature_version,
                err
            )
        })?;

        // Patch framework functions to skip the governance process.
        // This is redone every time we execute a script because the previous script could have
        // overwritten the framework.

        // If the script is the last step of the proposal, it MUST NOT have a next execution hash.
        // Set the boolean flag to true to use a modified patch to catch this.
        let forbid_next_execution_hash = script_idx == proposal_scripts.len() - 1;
        patch_aptos_governance(&state_view, forbid_next_execution_hash)
            .context("failed to patch resolve_multistep_proposal")?;

        // Add the hash of the script to the list of approved hashes, so that the
        // alternative (usually higher) execution limits can be used.
        add_script_execution_hash(&state_view, script_hash)
            .context("failed to add script execution hash")?;

        let script_name = script_path.file_name().unwrap().to_string_lossy();
        println!("    {}", script_name);

        // Create a new VM to ensure the loader is clean.
        let env = AptosEnvironment::new_with_injected_create_signer_for_gov_sim(&state_view);
        let vm = AptosVM::new(&env, &state_view);
        let log_context = AdapterLogSchema::new(state_view.id(), 0);

        let resolver = state_view.as_move_resolver();
        let code_storage = state_view.as_aptos_code_storage(&env);

        let txn = account
            .account()
            .transaction()
            .script(Script::new(script_blob, vec![], vec![
                TransactionArgument::U64(DUMMY_PROPOSAL_ID), // dummy proposal id, ignored by the patched function
            ]))
            .chain_id(chain_id.chain_id())
            .sequence_number(script_idx as u64)
            .gas_unit_price(gas_params.vm.txn.min_price_per_gas_unit.into())
            .max_gas_amount(100000)
            .sign();

        let vm_output = if !profile_gas {
            let (_vm_status, vm_output) =
                vm.execute_user_transaction(&resolver, &code_storage, &txn, &log_context);
            vm_output
        } else {
            let (_vm_status, vm_output, gas_profiler) = vm
                .execute_user_transaction_with_modified_gas_meter(
                    &resolver,
                    &code_storage,
                    &txn,
                    &log_context,
                    GasProfiler::new_script,
                )?;

            let gas_log = gas_profiler.finish();
            let report_path = proposal_dir
                .join("gas-profiling")
                .join(script_path.file_stem().unwrap());
            gas_log.generate_html_report(&report_path, format!("Gas Report - {}", script_name))?;

            println!("        Gas report saved to {}", report_path.display());

            vm_output
        };
        // TODO: ensure all scripts trigger reconfiguration.

        println!(
            "{}",
            format!("Fee statement: {:#?}", vm_output.fee_statement())
                .lines()
                .map(|line| format!("        {}", line))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let txn_output = vm_output
            .try_materialize_into_transaction_output(&resolver)
            .context("failed to materialize transaction output")?;

        let txn_status = txn_output.status();
        match txn_status {
            TransactionStatus::Keep(ExecutionStatus::Success) => {
                println!("        Success")
            },
            TransactionStatus::Keep(ExecutionStatus::MoveAbort { code, .. })
                if *code == MAGIC_FAILED_NEXT_EXECUTION_HASH_CHECK =>
            {
                bail!("the last script has a non-zero next execution hash")
            },
            _ => {
                println!(
                    "{}",
                    format!("{:#?}", txn_status)
                        .lines()
                        .map(|line| format!("        {}", line))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
                bail!("failed to execute governance script: {}", script_name)
            },
        }

        let (write_set, _events) = txn_output.into();
        state_view.apply_write_set(&write_set)?;
    }

    println!("All scripts succeeded!");

    Ok(())
}

pub fn collect_proposals(root_dir: &Path) -> Result<Vec<(PathBuf, Vec<PathBuf>)>> {
    let mut result = Vec::new();

    for entry in WalkDir::new(root_dir) {
        let entry = entry.unwrap();
        if entry.path().is_dir() {
            let sub_dir = entry.path();
            let mut move_files = Vec::new();

            for sub_entry in WalkDir::new(sub_dir).min_depth(1).max_depth(1) {
                let sub_entry = sub_entry.unwrap();
                if sub_entry.path().is_file()
                    && sub_entry.path().extension() == Some(std::ffi::OsStr::new("move"))
                {
                    move_files.push(sub_entry.path().to_path_buf());
                }
            }

            if !move_files.is_empty() {
                move_files.sort();
                result.push((sub_dir.to_path_buf(), move_files));
            }
        }
    }

    result.sort_by(|(path1, _), (path2, _)| path1.cmp(path2));

    Ok(result)
}

pub async fn simulate_all_proposals(
    remote_url: Url,
    output_dir: &Path,
    profile_gas: bool,
) -> Result<()> {
    let proposals =
        collect_proposals(output_dir).context("failed to collect proposals for simulation")?;

    if proposals.is_empty() {
        bail!("failed to simulate proposals: no proposals found")
    }

    println!(
        "Found {} proposal{}",
        proposals.len(),
        if proposals.len() == 1 { "" } else { "s" }
    );
    for (proposal_dir, proposal_scripts) in &proposals {
        println!("    {}", proposal_dir.display());

        for script_path in proposal_scripts {
            println!(
                "        {}",
                script_path.file_name().unwrap().to_string_lossy()
            );
        }
    }

    for (proposal_dir, proposal_scripts) in &proposals {
        simulate_multistep_proposal(
            remote_url.clone(),
            proposal_dir,
            proposal_scripts,
            profile_gas,
        )
        .await
        .with_context(|| format!("failed to simulate proposal at {}", proposal_dir.display()))?;
    }

    println!("All proposals succeeded!");

    Ok(())
}

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::Account,
    deps::{PkgDefinition, PkgManifest},
    executor::{
        oneshot::{ExecStatus, OneshotFuzzer},
        sequence::{
            self, Chain, ChainFuzzer, DefUseGraph, SeedInput, SequenceDb, MAX_CHAIN_FUZZERS,
        },
        tracing::{ResourceWrite, TracingExecutor},
    },
    language::LanguageSetting,
    mutate::mutator::TypePool,
    package,
    prep::{canvas::ScriptSignature, datatype::DatatypeDecl, model::Model},
    state::{
        load_auto_state, load_entrypoint_cache, save_auto_state, save_entrypoint_cache,
        PersistedAutoState, PersistedEntrypoint, PersistedEntrypointCache,
        PersistedExecCoverageMap, PersistedMissingDataSignal, PersistedObjectState,
        PersistedOneshotFuzzer, PersistedSeedInput, AUTO_STATE_VERSION, ENTRYPOINT_CACHE_VERSION,
    },
};
use anyhow::Result;
use aptos_vm_environment::prod_configs::set_debugging_enabled;
use legacy_move_compiler::compiled_unit::CompiledUnitEnum;
use log::{debug, info};
use move_core_types::{
    ability::AbilitySet,
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{StructTag, TypeTag as VmTypeTag},
};
use move_vm_runtime::tracing::{clear_tracing_buffer, enable_tracing};
use rand::{rngs::StdRng, SeedableRng};
use serde_json::json;
use sha3::{Digest, Sha3_256};
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

const CHAIN_REBUILD_INTERVAL_SECS: u64 = 60;
const SEQUENCE_MUTATION_INTERVAL_SECS: u64 = 30;
const SEED_SCORE_COVERAGE: u32 = 32;
const SEED_SCORE_DUG: u32 = 20;
const SEED_SCORE_STATE_PROGRESS: u32 = 10;
const SEED_SCORE_SUCCESS: u32 = 4;
const SEED_SCORE_MISSING_DATA: u32 = 6;

#[derive(Default)]
struct MissingDataSignal {
    hits: u64,
    last_seen_iter: u64,
    unresolved_tags: BTreeSet<sequence::ResourceTag>,
}

fn render_resource_write(write: &ResourceWrite) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}",
        write.address,
        write.struct_tag.address,
        write.struct_tag.module,
        write.struct_tag.name,
        write
            .struct_tag
            .type_args
            .iter()
            .map(|arg| format!("{arg:?}"))
            .collect::<Vec<_>>()
            .join(","),
        write.is_resource_group
    )
}

fn entrypoint_identity(sig: &ScriptSignature, code: &[u8]) -> String {
    let mut hasher = Sha3_256::new();
    hasher.update(b"move-fuzz-entrypoint-v1");
    hasher.update(sig.name.as_bytes());
    hasher.update(sig.ident.to_string().as_bytes());
    for generic in &sig.generics {
        hasher.update(format!("{generic:?}").as_bytes());
    }
    for parameter in &sig.parameters {
        hasher.update(parameter.to_string().as_bytes());
    }
    hasher.update(code);
    hex::encode(hasher.finalize())
}

fn entrypoint_identities(entrypoints: &[(ScriptSignature, Vec<u8>)]) -> Vec<String> {
    entrypoints
        .iter()
        .map(|(sig, code)| entrypoint_identity(sig, code))
        .collect()
}

fn render_build_fingerprint(pkg_defs: &[PkgDefinition]) -> Vec<String> {
    let mut entries = Vec::with_capacity(pkg_defs.len());
    for pkg in pkg_defs {
        let info = pkg.package.compiled_package_info();
        entries.push(format!(
            "{:?}|{}|{}|{:?}|{:?}|{:?}",
            pkg.kind,
            info.package_name,
            info.source_digest
                .map(|digest| digest.to_string())
                .unwrap_or_default(),
            info.build_flags.compiler_config.bytecode_version,
            info.build_flags.compiler_config.compiler_version,
            info.build_flags.compiler_config.language_version,
        ));
    }
    entries
}

fn entrypoint_cache_fingerprint(
    pkg_defs: &[PkgDefinition],
    max_trace_depth: usize,
    max_call_repetition: usize,
    max_script_gen_secs_per_function: u64,
) -> String {
    let mut hasher = Sha3_256::new();
    hasher.update(b"move-fuzz-entrypoint-cache-v2");
    hasher.update(max_trace_depth.to_le_bytes());
    hasher.update(max_call_repetition.to_le_bytes());
    hasher.update(max_script_gen_secs_per_function.to_le_bytes());
    for entry in render_build_fingerprint(pkg_defs) {
        hasher.update(entry.as_bytes());
    }
    hex::encode(hasher.finalize())
}

fn entrypoint_priority(
    dug: &DefUseGraph,
    chain: &Chain,
    missing_data_signals: &BTreeMap<usize, MissingDataSignal>,
) -> (usize, u64, usize, usize, usize, usize) {
    let target = chain.target();
    let missing_hits = missing_data_signals
        .get(&target)
        .map_or(0, |signal| signal.hits);
    let missing_resolution = missing_data_signals.get(&target).map_or(0, |signal| {
        chain_missing_data_resolution(dug, chain, signal)
    });
    let unmet = dug.unmet_deps(target).len();
    let defs = dug.defs_of(target).len();
    let producible = dug
        .unmet_deps(target)
        .into_iter()
        .filter(|&type_idx| {
            !dug.producers_of(type_idx).is_empty() || !dug.seed_producers_of(type_idx).is_empty()
        })
        .count();
    (
        missing_resolution,
        missing_hits,
        producible,
        defs,
        unmet,
        chain.len(),
    )
}

fn chain_missing_data_resolution(
    dug: &DefUseGraph,
    chain: &Chain,
    signal: &MissingDataSignal,
) -> usize {
    if chain.len() <= 1 || signal.unresolved_tags.is_empty() {
        return 0;
    }
    let mut resolved = BTreeSet::new();
    for &step in &chain.steps[..chain.len() - 1] {
        resolved.extend(dug.defs_of(step).iter().copied());
    }
    dug.compatible_type_overlap_with_tags(&resolved, &signal.unresolved_tags)
}

fn snapshot_missing_data_signals(
    signals: &BTreeMap<usize, MissingDataSignal>,
) -> Vec<PersistedMissingDataSignal> {
    signals
        .iter()
        .map(|(&script_index, signal)| PersistedMissingDataSignal {
            script_index,
            hits: signal.hits,
            last_seen_iter: signal.last_seen_iter,
            unresolved_tags: signal.unresolved_tags.clone(),
        })
        .collect()
}

fn restore_missing_data_signals(
    signals: Vec<PersistedMissingDataSignal>,
) -> BTreeMap<usize, MissingDataSignal> {
    signals
        .into_iter()
        .map(|signal| {
            (signal.script_index, MissingDataSignal {
                hits: signal.hits,
                last_seen_iter: signal.last_seen_iter,
                unresolved_tags: signal.unresolved_tags,
            })
        })
        .collect()
}

fn target_module_key(entrypoints: &[(ScriptSignature, Vec<u8>)], steps: &[usize]) -> String {
    steps
        .last()
        .map(|step| entrypoints[*step].0.ident.module_name().to_string())
        .unwrap_or_default()
}

fn module_signature_key(entrypoints: &[(ScriptSignature, Vec<u8>)], steps: &[usize]) -> String {
    let mut modules = Vec::new();
    let mut last = None::<String>;
    for &step in steps {
        let module = entrypoints[step].0.ident.module_name().to_string();
        if last.as_ref() != Some(&module) {
            modules.push(module.clone());
            last = Some(module);
        }
    }
    modules.join("->")
}

fn chain_diversity_counts(
    entrypoints: &[(ScriptSignature, Vec<u8>)],
    chain_fuzzers: &[ChainFuzzer],
) -> (BTreeMap<String, usize>, BTreeMap<String, usize>) {
    let mut target_modules = BTreeMap::new();
    let mut module_signatures = BTreeMap::new();
    for fuzzer in chain_fuzzers {
        *target_modules
            .entry(target_module_key(entrypoints, fuzzer.chain_steps()))
            .or_insert(0) += 1;
        *module_signatures
            .entry(module_signature_key(entrypoints, fuzzer.chain_steps()))
            .or_insert(0) += 1;
    }
    (target_modules, module_signatures)
}

fn chain_sort_key(
    entrypoints: &[(ScriptSignature, Vec<u8>)],
    dug: &DefUseGraph,
    chain: &Chain,
    missing_data_signals: &BTreeMap<usize, MissingDataSignal>,
    target_module_counts: &BTreeMap<String, usize>,
    module_signature_counts: &BTreeMap<String, usize>,
) -> (
    Reverse<usize>,
    Reverse<u64>,
    Reverse<usize>,
    Reverse<usize>,
    Reverse<usize>,
    usize,
    usize,
    usize,
) {
    let (missing_resolution, missing_hits, producible, defs, unmet, len) =
        entrypoint_priority(dug, chain, missing_data_signals);
    let target_module = target_module_key(entrypoints, &chain.steps);
    let module_signature = module_signature_key(entrypoints, &chain.steps);
    (
        Reverse(missing_resolution),
        Reverse(missing_hits),
        Reverse(producible),
        Reverse(defs),
        Reverse(unmet),
        target_module_counts
            .get(&target_module)
            .copied()
            .unwrap_or(0),
        module_signature_counts
            .get(&module_signature)
            .copied()
            .unwrap_or(0),
        len,
    )
}

fn diversify_candidates<T, F>(
    entrypoints: &[(ScriptSignature, Vec<u8>)],
    dug: &DefUseGraph,
    mut candidates: Vec<T>,
    missing_data_signals: &BTreeMap<usize, MissingDataSignal>,
    chain_fuzzers: &[ChainFuzzer],
    chain_of: F,
) -> Vec<T>
where
    F: Fn(&T) -> &Chain,
{
    let (mut target_module_counts, mut module_signature_counts) =
        chain_diversity_counts(entrypoints, chain_fuzzers);
    let mut ordered = Vec::with_capacity(candidates.len());

    while !candidates.is_empty() {
        let best_idx = candidates
            .iter()
            .enumerate()
            .min_by_key(|(_, candidate)| {
                chain_sort_key(
                    entrypoints,
                    dug,
                    chain_of(candidate),
                    missing_data_signals,
                    &target_module_counts,
                    &module_signature_counts,
                )
            })
            .map(|(idx, _)| idx)
            .expect("non-empty candidate list");
        let candidate = candidates.swap_remove(best_idx);
        let target_module = target_module_key(entrypoints, &chain_of(&candidate).steps);
        let module_signature = module_signature_key(entrypoints, &chain_of(&candidate).steps);
        *target_module_counts.entry(target_module).or_insert(0) += 1;
        *module_signature_counts.entry(module_signature).or_insert(0) += 1;
        ordered.push(candidate);
    }

    ordered
}

fn execution_seed_score(
    found_new: bool,
    dug_changed: bool,
    progressed_state: bool,
    success: bool,
    missing_data: bool,
) -> u32 {
    let mut score = 0;
    if found_new {
        score += SEED_SCORE_COVERAGE;
    }
    if dug_changed {
        score += SEED_SCORE_DUG;
    }
    if progressed_state {
        score += SEED_SCORE_STATE_PROGRESS;
    }
    if success {
        score += SEED_SCORE_SUCCESS;
    }
    if missing_data {
        score += SEED_SCORE_MISSING_DATA;
    }
    score
}

fn record_missing_data_signal(
    signals: &mut BTreeMap<usize, MissingDataSignal>,
    dug: &DefUseGraph,
    iteration: u64,
    profile: &sequence::ExecResourceProfile,
) {
    let unresolved_tags =
        dug.observed_unresolved_dependency_tags(profile.script_index, &profile.reads);
    if unresolved_tags.is_empty() {
        return;
    }
    let signal = signals.entry(profile.script_index).or_default();
    signal.hits = signal.hits.saturating_add(1);
    signal.last_seen_iter = iteration;
    signal.unresolved_tags.extend(unresolved_tags);
}

fn load_or_generate_entrypoints(
    pkg_defs: &[PkgDefinition],
    model: &Model,
    autogen_manifest: &PkgManifest,
    named_accounts: &BTreeMap<String, Account>,
    language: LanguageSetting,
    max_trace_depth: usize,
    max_call_repetition: usize,
    max_script_gen_secs_per_function: u64,
    path_entrypoint_cache: &Path,
    path_fuzz_stats: &Path,
) -> Result<Vec<(ScriptSignature, Vec<u8>)>> {
    let cache_fingerprint = entrypoint_cache_fingerprint(
        pkg_defs,
        max_trace_depth,
        max_call_repetition,
        max_script_gen_secs_per_function,
    );
    if let Some(cache) = load_entrypoint_cache(path_entrypoint_cache)? {
        if cache.version == ENTRYPOINT_CACHE_VERSION && cache.fingerprint == cache_fingerprint {
            info!(
                "loaded {} cached entrypoints from {}",
                cache.entrypoints.len(),
                path_entrypoint_cache.display()
            );
            return Ok(cache
                .entrypoints
                .into_iter()
                .map(|entry| (entry.signature, entry.code))
                .collect());
        }
    }

    let generated_scripts = model.populate(
        max_trace_depth,
        max_call_repetition,
        max_script_gen_secs_per_function,
        &autogen_manifest.path.join("sources"),
        Some(path_fuzz_stats),
    );
    info!(
        "scripts generated in the autogen package: {}",
        autogen_manifest.path.display()
    );

    let pkg_built = package::build(autogen_manifest, named_accounts, language, false)
        .unwrap_or_else(|why| panic!("unable to build the autogen package: {why}"));
    info!("autogen package built successfully");

    let bytecode_version = pkg_built
        .package
        .compiled_package_info
        .build_flags
        .compiler_config
        .bytecode_version;

    let mut entrypoints = vec![];
    for unit in pkg_built.package.root_compiled_units {
        match unit.unit {
            CompiledUnitEnum::Module(_) => panic!("unexpected module in the autogen package"),
            CompiledUnitEnum::Script(script) => {
                let sig = generated_scripts
                    .iter()
                    .find(|s| s.name == script.name.as_str())
                    .unwrap_or_else(|| {
                        panic!("unable to find a signature for script {}", script.name)
                    });
                let mut code = vec![];
                script
                    .script
                    .serialize_for_version(bytecode_version, &mut code)
                    .unwrap_or_else(|_| panic!("unable to deserialize an autogen CompiledScript"));
                entrypoints.push((sig.clone(), code));
            },
        }
    }
    assert_eq!(entrypoints.len(), generated_scripts.len());

    let cache = PersistedEntrypointCache::new(
        cache_fingerprint,
        entrypoints
            .iter()
            .cloned()
            .map(|(signature, code)| PersistedEntrypoint { signature, code })
            .collect(),
    );
    save_entrypoint_cache(path_entrypoint_cache, &cache)?;
    Ok(entrypoints)
}

fn campaign_fingerprint(
    entrypoints: &[(ScriptSignature, Vec<u8>)],
    initial_resource_writes: &[ResourceWrite],
    max_chain_length: usize,
    max_chain_repetition: usize,
    num_user_accounts: usize,
) -> String {
    let mut hasher = Sha3_256::new();
    hasher.update(b"move-fuzz-auto-state-v1");
    hasher.update(max_chain_length.to_le_bytes());
    hasher.update(max_chain_repetition.to_le_bytes());
    hasher.update(num_user_accounts.to_le_bytes());

    let mut identities = entrypoint_identities(entrypoints);
    identities.sort();
    for identity in identities {
        hasher.update(identity.as_bytes());
    }

    let mut rendered_writes: Vec<_> = initial_resource_writes
        .iter()
        .map(render_resource_write)
        .collect();
    rendered_writes.sort();
    for write in rendered_writes {
        hasher.update(write.as_bytes());
    }

    hex::encode(hasher.finalize())
}

fn chain_instance_identity(steps: &[usize], seed: &[SeedInput]) -> String {
    let mut hasher = Sha3_256::new();
    hasher.update(b"move-fuzz-chain-instance-v1");
    for step in steps {
        hasher.update(step.to_le_bytes());
    }
    for persisted in seed.iter().map(|seed| {
        PersistedSeedInput::try_from_seed(seed)
            .expect("chain identity seeds must remain persistable")
    }) {
        let encoded =
            serde_json::to_vec(&persisted).expect("persisted chain identity seeds must serialize");
        hasher.update((encoded.len() as u64).to_le_bytes());
        hasher.update(encoded);
    }
    hex::encode(hasher.finalize())
}

fn chain_instance_set(chain_fuzzers: &[ChainFuzzer]) -> BTreeSet<String> {
    chain_fuzzers
        .iter()
        .map(|cf| chain_instance_identity(cf.chain_steps(), cf.identity_seed()))
        .collect()
}

fn build_chain_fuzzer(
    executor: &TracingExecutor,
    entrypoints: &[(ScriptSignature, Vec<u8>)],
    type_pool: &TypePool,
    cov_trace_path: &PathBuf,
    dict_string: &[String],
    initial_resource_writes: &[ResourceWrite],
    chain: Chain,
    base_seed: u64,
    chain_seed_nonce: &mut u64,
) -> ChainFuzzer {
    let seed = derive_seed(base_seed, *chain_seed_nonce);
    *chain_seed_nonce = chain_seed_nonce.wrapping_add(1);

    let mut fuzzer = ChainFuzzer::new(
        executor.clone(),
        seed,
        chain,
        entrypoints,
        type_pool.clone(),
        cov_trace_path.clone(),
        dict_string.to_vec(),
    );
    fuzzer.absorb_shared_object_writes(initial_resource_writes);
    fuzzer
}

fn bootstrap_chain_seed(
    chain_steps: &[usize],
    seed_inputs: Vec<SeedInput>,
    seq_db: &SequenceDb,
    dug: &DefUseGraph,
    oneshot_fuzzers: &[OneshotFuzzer],
    rng: &mut StdRng,
) -> Vec<SeedInput> {
    if !seed_inputs.is_empty() {
        return seed_inputs;
    }

    let mut bootstrap_seed = seq_db
        .pick_concrete_prefix_seed(dug, chain_steps, rng)
        .unwrap_or_default();
    if !bootstrap_seed.is_empty() {
        return bootstrap_seed;
    }

    for &step in chain_steps {
        if let Some(seed) = oneshot_fuzzers[step].sample_seed(rng) {
            bootstrap_seed.push(seed);
        } else {
            break;
        }
    }
    bootstrap_seed
}

fn push_chain_fuzzer(
    chain_fuzzers: &mut Vec<ChainFuzzer>,
    seen_chain_instances: &mut BTreeSet<String>,
    executor: &TracingExecutor,
    entrypoints: &[(ScriptSignature, Vec<u8>)],
    type_pool: &TypePool,
    cov_trace_path: &PathBuf,
    dict_string: &[String],
    initial_resource_writes: &[ResourceWrite],
    chain: Chain,
    parent_seed: Vec<SeedInput>,
    base_seed: u64,
    chain_seed_nonce: &mut u64,
) -> bool {
    if chain_fuzzers.len() >= MAX_CHAIN_FUZZERS {
        return false;
    }

    let chain_instance = chain_instance_identity(&chain.steps, &parent_seed);
    if !seen_chain_instances.insert(chain_instance) {
        return false;
    }

    let mut cf = build_chain_fuzzer(
        executor,
        entrypoints,
        type_pool,
        cov_trace_path,
        dict_string,
        initial_resource_writes,
        chain,
        base_seed,
        chain_seed_nonce,
    );
    if !parent_seed.is_empty() {
        cf.import_parent_seed(parent_seed);
    }
    chain_fuzzers.push(cf);
    true
}

#[allow(clippy::too_many_arguments)]
fn spawn_targeted_missing_data_chains(
    chain_fuzzers: &mut Vec<ChainFuzzer>,
    executor: &TracingExecutor,
    entrypoints: &[(ScriptSignature, Vec<u8>)],
    type_pool: &TypePool,
    cov_trace_path: &PathBuf,
    dict_string: &[String],
    initial_resource_writes: &[ResourceWrite],
    seq_db: &SequenceDb,
    dug: &DefUseGraph,
    oneshot_fuzzers: &[OneshotFuzzer],
    missing_data_signals: &BTreeMap<usize, MissingDataSignal>,
    target_script: usize,
    max_chain_length: usize,
    max_chain_repetition: usize,
    base_seed: u64,
    chain_seed_nonce: &mut u64,
    chain_rng: &mut StdRng,
) -> usize {
    let target_scripts = BTreeSet::from([target_script]);
    let mut targeted = sequence::construct_seed_chains_for_targets(
        dug,
        &target_scripts,
        max_chain_length,
        max_chain_repetition,
        12,
        chain_rng,
    );
    targeted = diversify_candidates(
        entrypoints,
        dug,
        targeted,
        missing_data_signals,
        chain_fuzzers,
        |candidate| &candidate.chain,
    );

    let mut seen_chain_instances = chain_instance_set(chain_fuzzers);
    let mut added = 0usize;
    for seed_chain in targeted {
        if chain_fuzzers.len() >= MAX_CHAIN_FUZZERS {
            break;
        }
        let bootstrap_seed = bootstrap_chain_seed(
            &seed_chain.chain.steps,
            seed_chain.seed_inputs,
            seq_db,
            dug,
            oneshot_fuzzers,
            chain_rng,
        );
        if push_chain_fuzzer(
            chain_fuzzers,
            &mut seen_chain_instances,
            executor,
            entrypoints,
            type_pool,
            cov_trace_path,
            dict_string,
            initial_resource_writes,
            seed_chain.chain,
            bootstrap_seed,
            base_seed,
            chain_seed_nonce,
        ) {
            added += 1;
        }
    }
    added
}

struct RestoreParams<'a> {
    executor: &'a TracingExecutor,
    entrypoints: &'a [(ScriptSignature, Vec<u8>)],
    entrypoint_identities: &'a [String],
    type_pool: &'a TypePool,
    cov_trace_path: &'a PathBuf,
    dict_string: &'a [String],
    initial_resource_writes: &'a [ResourceWrite],
    base_seed: u64,
}

struct RuntimeCheckpoint<'a> {
    entrypoint_identities: &'a [String],
    phase2_entered: bool,
    bootstrap_profile_count: usize,
    bootstrap_dug: &'a DefUseGraph,
    dug: Option<&'a DefUseGraph>,
    oneshot_fuzzers: &'a [OneshotFuzzer],
    seq_db: &'a SequenceDb,
    chain_fuzzers: &'a [ChainFuzzer],
    chain_seed_nonce: u64,
    object_state: PersistedObjectState,
    missing_data_signals: &'a BTreeMap<usize, MissingDataSignal>,
}

fn snapshot_auto_state(
    campaign_fingerprint: &str,
    runtime: RuntimeCheckpoint<'_>,
) -> Result<PersistedAutoState> {
    let dug = if runtime.phase2_entered {
        runtime
            .dug
            .expect("live DUG must exist after entering phase 2")
    } else {
        runtime.bootstrap_dug
    };

    let oneshot_fuzzers = runtime
        .oneshot_fuzzers
        .iter()
        .enumerate()
        .map(|(script_index, fuzzer)| {
            Ok(PersistedOneshotFuzzer {
                script_index,
                script_identity: runtime.entrypoint_identities[script_index].clone(),
                replay_log: fuzzer.replay_log_snapshot()?,
                seedpool: fuzzer.seed_record_snapshot()?,
                coverage: PersistedExecCoverageMap::from_exec_coverage_map(
                    &fuzzer.coverage_snapshot(),
                ),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let chain_fuzzers = runtime
        .chain_fuzzers
        .iter()
        .map(|fuzzer| {
            let mut saved = fuzzer.snapshot()?;
            saved.step_identities = fuzzer
                .chain_steps()
                .iter()
                .map(|step| runtime.entrypoint_identities[*step].clone())
                .collect();
            Ok(saved)
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(PersistedAutoState::new(
        campaign_fingerprint.to_string(),
        runtime.entrypoint_identities.to_vec(),
        runtime.phase2_entered,
        runtime.bootstrap_profile_count,
        dug.snapshot()?,
        oneshot_fuzzers,
        runtime.seq_db.snapshot()?,
        chain_fuzzers,
        runtime.chain_seed_nonce,
        runtime.object_state,
        snapshot_missing_data_signals(runtime.missing_data_signals),
    ))
}

fn save_checkpoint(
    path_auto_state: &Path,
    campaign_fingerprint: &str,
    runtime: RuntimeCheckpoint<'_>,
    state_dirty: &mut bool,
) -> Result<()> {
    if !*state_dirty {
        return Ok(());
    }

    let snapshot = snapshot_auto_state(campaign_fingerprint, runtime)?;
    save_auto_state(path_auto_state, &snapshot)?;
    *state_dirty = false;
    Ok(())
}

fn current_object_state(
    oneshot_fuzzers: &[OneshotFuzzer],
    chain_fuzzers: &[ChainFuzzer],
) -> PersistedObjectState {
    PersistedObjectState::merge(
        oneshot_fuzzers
            .iter()
            .map(OneshotFuzzer::object_state_snapshot)
            .chain(chain_fuzzers.iter().map(ChainFuzzer::object_state_snapshot)),
    )
}

fn known_sender_addresses(executor: &TracingExecutor) -> BTreeSet<AccountAddress> {
    executor
        .all_addresses_by_kind()
        .into_values()
        .flatten()
        .collect()
}

fn auto_state_has_known_senders(
    state: &PersistedAutoState,
    known_senders: &BTreeSet<AccountAddress>,
) -> bool {
    let oneshot_ok = state.oneshot_fuzzers.iter().all(|fuzzer| {
        fuzzer
            .seedpool
            .iter()
            .all(|record| known_senders.contains(&record.input.sender))
    });
    let chain_ok = state.chain_fuzzers.iter().all(|fuzzer| {
        fuzzer.seedpool.iter().all(|record| {
            record
                .input
                .iter()
                .all(|seed| known_senders.contains(&seed.sender))
        })
    });
    let seq_db_ok = state.sequence_db.entries.iter().all(|entry| {
        entry
            .seed
            .iter()
            .all(|seed| known_senders.contains(&seed.sender))
    });
    let dug_ok = state
        .dug
        .seed_nodes
        .iter()
        .all(|seed_node| known_senders.contains(&seed_node.seed.sender));
    oneshot_ok && chain_ok && seq_db_ok && dug_ok
}

fn restore_auto_state(
    loaded_state: PersistedAutoState,
    campaign_fingerprint: &str,
    params: RestoreParams<'_>,
    bootstrap_profile_count: &mut usize,
    bootstrap_dug: &mut DefUseGraph,
    phase2_entered: &mut bool,
    dug: &mut Option<DefUseGraph>,
    oneshot_fuzzers: &mut [OneshotFuzzer],
    seq_db: &mut SequenceDb,
    chain_fuzzers: &mut Vec<ChainFuzzer>,
    chain_seed_nonce: &mut u64,
    missing_data_signals: &mut BTreeMap<usize, MissingDataSignal>,
) -> Result<bool> {
    if loaded_state.version != AUTO_STATE_VERSION {
        info!(
            "ignoring persisted fuzz state with unsupported version {}",
            loaded_state.version
        );
        return Ok(false);
    }
    if loaded_state.entrypoint_identities.len() != params.entrypoint_identities.len() {
        info!("ignoring persisted fuzz state because script count changed");
        return Ok(false);
    }
    if loaded_state.oneshot_fuzzers.len() != loaded_state.entrypoint_identities.len() {
        info!("ignoring persisted fuzz state because saved oneshot state is incomplete");
        return Ok(false);
    }

    let mut current_indices = BTreeMap::new();
    for (idx, identity) in params.entrypoint_identities.iter().enumerate() {
        if current_indices.insert(identity.clone(), idx).is_some() {
            info!("ignoring persisted fuzz state because current entrypoints are not unique");
            return Ok(false);
        }
    }

    let mut old_to_new = Vec::with_capacity(loaded_state.entrypoint_identities.len());
    for identity in &loaded_state.entrypoint_identities {
        let Some(&mapped_idx) = current_indices.get(identity) else {
            info!("ignoring persisted fuzz state because generated entrypoints changed");
            return Ok(false);
        };
        old_to_new.push(mapped_idx);
    }
    let known_senders = known_sender_addresses(params.executor);
    if !auto_state_has_known_senders(&loaded_state, &known_senders) {
        info!(
            "ignoring persisted fuzz state because checkpointed sender addresses are no longer available"
        );
        return Ok(false);
    }
    if loaded_state.campaign_fingerprint != campaign_fingerprint {
        info!("ignoring persisted fuzz state because the baseline campaign fingerprint changed");
        return Ok(false);
    }

    for saved in &loaded_state.oneshot_fuzzers {
        if saved.script_index >= loaded_state.entrypoint_identities.len() {
            info!("ignoring persisted fuzz state because a script index is out of range");
            return Ok(false);
        }
    }

    let object_state = loaded_state.object_state;
    *missing_data_signals = restore_missing_data_signals(loaded_state.missing_data_signals);
    for saved in loaded_state.oneshot_fuzzers {
        let Some(&script_index) = current_indices.get(&saved.script_identity) else {
            info!("ignoring persisted fuzz state because a saved script identity is missing");
            return Ok(false);
        };
        let fuzzer = &mut oneshot_fuzzers[script_index];
        fuzzer.replay_checkpoint_log(saved.replay_log)?;
        fuzzer
            .restore_checkpoint_records(saved.seedpool, saved.coverage.into_exec_coverage_map()?)?;
        fuzzer.restore_object_state(&object_state)?;
    }

    *bootstrap_profile_count = loaded_state.bootstrap_profile_count;
    let mut persisted_seq_db = loaded_state.sequence_db;
    persisted_seq_db.remap_script_indices(&old_to_new)?;
    *seq_db = SequenceDb::from_persisted(persisted_seq_db)?;
    *chain_seed_nonce = loaded_state.chain_seed_nonce;

    let mut persisted_dug = loaded_state.dug;
    persisted_dug.remap_script_indices(&old_to_new)?;
    let restored_dug = DefUseGraph::from_persisted(persisted_dug)?;
    if loaded_state.phase2_entered {
        *phase2_entered = true;
        *dug = Some(restored_dug);
        chain_fuzzers.clear();

        let mut seen_chain_instances = BTreeSet::new();
        for saved_chain in loaded_state.chain_fuzzers {
            let remapped_steps = if saved_chain.step_identities.len() == saved_chain.steps.len() {
                let mut steps = Vec::with_capacity(saved_chain.step_identities.len());
                let mut missing_identity = false;
                for identity in &saved_chain.step_identities {
                    let Some(&step_idx) = current_indices.get(identity) else {
                        missing_identity = true;
                        break;
                    };
                    steps.push(step_idx);
                }
                if missing_identity {
                    continue;
                }
                steps
            } else {
                let mut steps = Vec::with_capacity(saved_chain.steps.len());
                let mut invalid = false;
                for old_step in &saved_chain.steps {
                    if *old_step >= old_to_new.len() {
                        invalid = true;
                        break;
                    }
                    steps.push(old_to_new[*old_step]);
                }
                if invalid {
                    continue;
                }
                steps
            };
            let identity_seed = saved_chain
                .identity_seed
                .into_iter()
                .map(PersistedSeedInput::into_seed)
                .collect::<Result<Vec<_>>>()?;
            if remapped_steps.is_empty()
                || !seen_chain_instances
                    .insert(chain_instance_identity(&remapped_steps, &identity_seed))
            {
                continue;
            }

            let chain = Chain {
                steps: remapped_steps,
            };
            let mut fuzzer = build_chain_fuzzer(
                params.executor,
                params.entrypoints,
                params.type_pool,
                params.cov_trace_path,
                params.dict_string,
                params.initial_resource_writes,
                chain,
                params.base_seed,
                chain_seed_nonce,
            );
            fuzzer.set_identity_seed(identity_seed);
            fuzzer.replay_checkpoint_log(saved_chain.replay_log)?;
            fuzzer.restore_checkpoint_records(
                saved_chain.seedpool,
                saved_chain.coverage.into_exec_coverage_map()?,
            )?;
            fuzzer.restore_object_state(&object_state)?;
            chain_fuzzers.push(fuzzer);
        }
    } else {
        *phase2_entered = false;
        *dug = None;
        *bootstrap_dug = restored_dug;
        chain_fuzzers.clear();
    }

    Ok(true)
}

/// Entrypoint for the fuzzer
pub fn entrypoint(
    pkg_defs: Vec<PkgDefinition>,
    named_accounts: BTreeMap<String, Account>,
    language: LanguageSetting,
    autogen_manifest: PkgManifest,
    cov_trace_path: PathBuf,
    seed: Option<u64>,
    max_trace_depth: usize,
    max_call_repetition: usize,
    max_script_gen_secs_per_function: u64,
    num_user_accounts: usize,
    dry_run: bool,
    dict_string: Vec<String>,
    path_fuzz_stats: PathBuf,
    path_auto_state: PathBuf,
    path_entrypoint_cache: PathBuf,
    max_chain_length: usize,
    max_chain_repetition: usize,
    saturation_secs: u64,
) -> Result<()> {
    // build a model on the packages
    let model = Model::new(&pkg_defs);
    if dry_run {
        let generated_scripts = model.populate(
            max_trace_depth,
            max_call_repetition,
            max_script_gen_secs_per_function,
            &autogen_manifest.path.join("sources"),
            Some(path_fuzz_stats.as_path()),
        );
        info!(
            "scripts generated in the autogen package: {}",
            autogen_manifest.path.display()
        );
        info!(
            "dry-run mode: generated {} script(s), stopping before fuzzing loop",
            generated_scripts.len()
        );
        return Ok(());
    }

    let mut entrypoints = load_or_generate_entrypoints(
        &pkg_defs,
        &model,
        &autogen_manifest,
        &named_accounts,
        language,
        max_trace_depth,
        max_call_repetition,
        max_script_gen_secs_per_function,
        &path_entrypoint_cache,
        &path_fuzz_stats,
    )?;

    // build the type pool from the model
    let type_pool = build_type_pool(&model);
    let entrypoints_before_filter = entrypoints.len();
    entrypoints.retain(|(sig, _)| type_pool.can_satisfy_all(&sig.generics));
    let filtered_entrypoints = entrypoints_before_filter - entrypoints.len();
    if filtered_entrypoints > 0 {
        info!(
            "skipping {} script(s) whose generic ability constraints are unsatisfiable",
            filtered_entrypoints
        );
    }
    if entrypoints.is_empty() {
        info!("no fuzzable scripts remain after generic-constraint filtering");
        return Ok(());
    }
    let entrypoint_identities = entrypoint_identities(&entrypoints);

    // enable VM debugging so MOVE_VM_TRACE writes execution traces
    set_debugging_enabled(true);
    enable_tracing(Some(cov_trace_path.to_str().unwrap()));

    // prepare the baseline executor
    let mut executor = TracingExecutor::new();
    for pkg in &pkg_defs {
        executor.add_new_package(pkg)?
    }
    for _ in 0..num_user_accounts {
        executor.add_new_user();
    }

    // scan the full state for resource writes (genesis + provisioning);
    // Mutator::update_object_dict handles the two-pass ObjectGroup filtering
    let initial_resource_writes = executor.scan_all_resource_writes();
    info!(
        "initial state scan found {} resource writes",
        initial_resource_writes.len()
    );
    let campaign_fingerprint = campaign_fingerprint(
        &entrypoints,
        &initial_resource_writes,
        max_chain_length,
        max_chain_repetition,
        num_user_accounts,
    );

    // clear any trace data accumulated during executor setup (module deployments)
    clear_tracing_buffer();

    //
    // stage 1: per-script fuzzing
    //

    let seed_val = seed.unwrap_or(0);
    let mut oneshot_fuzzers = vec![];
    // prepare one fuzzer for each script
    for (idx, (sig, code)) in entrypoints.iter().enumerate() {
        let mut instance = OneshotFuzzer::new(
            executor.clone(),
            derive_seed(seed_val, idx as u64),
            idx,
            sig.clone(),
            code.clone(),
            type_pool.clone(),
            cov_trace_path.clone(),
            dict_string.clone(),
        );
        // seed each fuzzer with initial object discoveries from state scan
        instance.absorb_shared_object_writes(&initial_resource_writes);
        oneshot_fuzzers.push(instance);
    }

    let num_scripts = oneshot_fuzzers.len();

    //
    // Phase state: DUG and chains are built lazily at Phase 2 transition
    //
    let mut phase2_entered = false;
    let mut bootstrap_dug = DefUseGraph::new(num_scripts);
    bootstrap_dug.ingest_initial_writes(&initial_resource_writes);
    let mut bootstrap_profile_count = 0usize;
    let mut last_script_coverage_time: Vec<Instant> = vec![Instant::now(); num_scripts];
    let mut dug: Option<DefUseGraph> = None;
    let mut chain_fuzzers: Vec<ChainFuzzer> = vec![];
    let mut seq_db = SequenceDb::new();
    let mut chain_rng = StdRng::seed_from_u64(derive_seed(seed_val, 0x9E3779B97F4A7C15));
    let mut chain_seed_nonce = 0u64;
    let mut dug_last_marker = 0usize;
    let mut dug_last_seed_marker = 0usize;
    let mut last_chain_reconstruction = Instant::now();
    let mut last_sequence_mutation = Instant::now();
    let mut last_phase2_novelty = Instant::now();
    let mut state_dirty = false;
    let mut missing_data_signals: BTreeMap<usize, MissingDataSignal> = BTreeMap::new();

    match load_auto_state(&path_auto_state) {
        Ok(Some(loaded_state)) => {
            let restored = restore_auto_state(
                loaded_state,
                &campaign_fingerprint,
                RestoreParams {
                    executor: &executor,
                    entrypoints: &entrypoints,
                    entrypoint_identities: &entrypoint_identities,
                    type_pool: &type_pool,
                    cov_trace_path: &cov_trace_path,
                    dict_string: &dict_string,
                    initial_resource_writes: &initial_resource_writes,
                    base_seed: seed_val,
                },
                &mut bootstrap_profile_count,
                &mut bootstrap_dug,
                &mut phase2_entered,
                &mut dug,
                &mut oneshot_fuzzers,
                &mut seq_db,
                &mut chain_fuzzers,
                &mut chain_seed_nonce,
                &mut missing_data_signals,
            )?;
            if restored {
                info!(
                    "resumed fuzz state from {} (phase {}, oneshot corpora {}, chain fuzzers {}, sequence db entries {})",
                    path_auto_state.display(),
                    if phase2_entered { 2 } else { 1 },
                    oneshot_fuzzers.iter().map(OneshotFuzzer::corpus_size).sum::<usize>(),
                    chain_fuzzers.len(),
                    seq_db.len()
                );
                last_chain_reconstruction =
                    Instant::now() - std::time::Duration::from_secs(CHAIN_REBUILD_INTERVAL_SECS);
                last_sequence_mutation = Instant::now()
                    - std::time::Duration::from_secs(SEQUENCE_MUTATION_INTERVAL_SECS);
            }
        },
        Ok(None) => {},
        Err(err) => {
            info!(
                "ignoring persisted fuzz state at {}: {err}",
                path_auto_state.display()
            );
        },
    }

    // startup banner
    eprintln!(
        "=== Move Fuzzer (Phase {}: {}) ===",
        if phase2_entered { 2 } else { 1 },
        if phase2_entered {
            "Multi-transaction"
        } else {
            "Bootstrap"
        }
    );
    eprintln!(
        "scripts: {num_scripts} | seed: {seed_val} | saturation: {saturation_secs}s | max-depth: {max_trace_depth} | max-chain-len: {max_chain_length}"
    );
    eprintln!("{}", "-".repeat(72));
    for (i, fuzzer) in oneshot_fuzzers.iter().enumerate() {
        eprintln!("  [{i:3}] {}", fuzzer.script_desc());
    }
    if phase2_entered && !chain_fuzzers.is_empty() {
        eprintln!("{}", "-".repeat(72));
        for (i, fuzzer) in chain_fuzzers.iter().enumerate() {
            eprintln!("  [chain:{i:3}] {}", fuzzer.script_short_desc());
        }
    }
    eprintln!("{}", "-".repeat(72));
    eprintln!(
        "entering Phase {} mutation loop...\n",
        if phase2_entered { 2 } else { 1 }
    );

    let start_time = Instant::now();
    let mut stats = BTreeMap::new();
    let mut category_counts = BTreeMap::new();
    let mut seen_exec_stats = BTreeSet::new();
    let mut iteration = 0u64;
    let mut total_execs = 0u64;
    let mut last_report = Instant::now();
    let mut last_report_coverage = 0usize;

    loop {
        let mut round_writes = vec![];
        let mut pending_missing_data_targets = BTreeSet::new();

        // energy-based scheduling: prioritize recently-productive fuzzers while
        // still periodically giving full rounds to avoid starvation.
        let run_all_oneshots = !phase2_entered || iteration % 20 == 0;
        let mut oneshot_order: Vec<usize> = (0..oneshot_fuzzers.len()).collect();
        oneshot_order.sort_by_key(|&i| {
            (
                Reverse(missing_data_signals.get(&i).map_or(0, |signal| signal.hits)),
                oneshot_fuzzers[i]
                    .last_new_coverage_time()
                    .map_or(u64::MAX, |t| t.elapsed().as_secs()),
                Reverse(oneshot_fuzzers[i].best_seed_score()),
                Reverse(oneshot_fuzzers[i].corpus_size()),
            )
        });
        let oneshot_budget = if run_all_oneshots {
            oneshot_order.len()
        } else {
            ((oneshot_order.len() * 70) / 100).max(1)
        };

        for idx in oneshot_order.into_iter().take(oneshot_budget) {
            let fuzzer = &mut oneshot_fuzzers[idx];
            let (status, _corpus_size, found_new, writes, profile, seed) = fuzzer.run_one()?;
            let has_successful_writes = !writes.is_empty();
            round_writes.extend(writes);
            // update statistics
            *category_counts.entry(status.category()).or_insert(0) += 1;
            *stats.entry(status.to_string()).or_insert(0) += 1;
            total_execs += 1;
            let success = matches!(status, ExecStatus::Success);
            let missing_data = status.is_missing_data();

            // Feed this execution profile into bootstrap DUG (phase 1) or live DUG (phase 2).
            let dug_changed = if phase2_entered {
                if let Some(ref mut d) = dug {
                    d.add_seed_observation(&profile, seed.clone()).0
                } else {
                    false
                }
            } else {
                bootstrap_profile_count += 1;
                bootstrap_dug.add_seed_observation(&profile, seed.clone()).0
            };

            if missing_data {
                record_missing_data_signal(
                    &mut missing_data_signals,
                    if phase2_entered {
                        dug.as_ref()
                            .expect("phase2 missing-data signal requires live DUG")
                    } else {
                        &bootstrap_dug
                    },
                    iteration,
                    &profile,
                );
                pending_missing_data_targets.insert(profile.script_index);
            }

            let seed_score = execution_seed_score(
                found_new,
                dug_changed,
                has_successful_writes,
                success,
                missing_data,
            );
            if seed_score > 0 {
                fuzzer.remember_seed_with_score(seed.clone(), seed_score);
                state_dirty = true;
            }

            // log new error codes as they are discovered
            if !matches!(status, ExecStatus::Success) && !seen_exec_stats.contains(&status) {
                let desc = fuzzer.script_short_desc();
                info!("[new-status] #{idx} {desc} | {status}");
                seen_exec_stats.insert(status);
            }

            // log new coverage events and reset saturation timer
            if found_new {
                last_script_coverage_time[idx] = Instant::now();
                let desc = fuzzer.script_short_desc();
                let cov = fuzzer.coverage_count();
                info!(
                    "[+cov] #{idx} {desc} | corpus: {} | coverage: {cov}",
                    fuzzer.corpus_size()
                );
            }
            if phase2_entered && (found_new || dug_changed) {
                last_phase2_novelty = Instant::now();
            }

            // Save single-step seeds if they improved coverage or progressed state.
            if found_new || dug_changed || has_successful_writes {
                seq_db.add_entry(vec![idx], vec![seed], std::slice::from_ref(&profile));
                state_dirty = true;
            }
        }

        // run chain fuzzers (Phase 2 only)
        if phase2_entered {
            let run_all_chains = iteration % 10 == 0;
            let mut chain_order: Vec<usize> = (0..chain_fuzzers.len()).collect();
            let (target_module_counts, module_signature_counts) =
                chain_diversity_counts(&entrypoints, &chain_fuzzers);
            chain_order.sort_by_key(|&i| {
                let chain_steps = chain_fuzzers[i].chain_steps();
                let target_script = *chain_steps.last().unwrap_or(&0);
                (
                    Reverse(
                        missing_data_signals
                            .get(&target_script)
                            .map_or(0, |signal| signal.hits),
                    ),
                    target_module_counts
                        .get(&target_module_key(&entrypoints, chain_steps))
                        .copied()
                        .unwrap_or(0),
                    module_signature_counts
                        .get(&module_signature_key(&entrypoints, chain_steps))
                        .copied()
                        .unwrap_or(0),
                    chain_fuzzers[i]
                        .last_new_coverage_time()
                        .map_or(u64::MAX, |t| t.elapsed().as_secs()),
                    Reverse(chain_fuzzers[i].best_seed_score()),
                    Reverse(chain_fuzzers[i].corpus_size()),
                )
            });
            let chain_budget = if run_all_chains {
                chain_order.len()
            } else {
                ((chain_order.len() * 60) / 100).max(1)
            };

            for idx in chain_order.into_iter().take(chain_budget) {
                let fuzzer = &mut chain_fuzzers[idx];
                let (status, _corpus_size, found_new, writes, profiles, seed) =
                    fuzzer.run_one(Some(&seq_db), dug.as_ref())?;
                round_writes.extend(writes);
                *category_counts.entry(status.category()).or_insert(0) += 1;
                *stats.entry(status.to_string()).or_insert(0) += 1;
                total_execs += 1;
                let success = matches!(status, ExecStatus::Success);
                let missing_data = status.is_missing_data();
                let progressed_state = profiles
                    .iter()
                    .any(|profile| profile.succeeded && !profile.writes.is_empty());

                // feed per-step profiles into the DUG
                let mut dug_changed = false;
                debug_assert_eq!(profiles.len(), seed.len());
                for (p, s) in profiles.iter().zip(seed.iter().cloned()) {
                    if let Some(ref mut d) = dug {
                        dug_changed |= d.add_seed_observation(p, s).0;
                    }
                }

                if missing_data {
                    if let Some(last_profile) = profiles.last() {
                        record_missing_data_signal(
                            &mut missing_data_signals,
                            dug.as_ref()
                                .expect("chain missing-data signal requires live DUG"),
                            iteration,
                            last_profile,
                        );
                        pending_missing_data_targets.insert(last_profile.script_index);
                    }
                }

                let seed_score = execution_seed_score(
                    found_new,
                    dug_changed,
                    progressed_state,
                    success,
                    missing_data,
                );
                if seed_score > 0 && !seed.is_empty() {
                    fuzzer.remember_seed_with_score(seed.clone(), seed_score);
                    state_dirty = true;
                }

                let successful_prefix_len = profiles
                    .iter()
                    .take_while(|profile| profile.succeeded)
                    .count();
                if successful_prefix_len > 0 && (found_new || dug_changed || progressed_state) {
                    let steps = fuzzer.chain_steps()[..successful_prefix_len].to_vec();
                    let prefix_seed = seed[..successful_prefix_len].to_vec();
                    seq_db.add_entry(steps, prefix_seed, &profiles[..successful_prefix_len]);
                    state_dirty = true;
                }

                if !matches!(status, ExecStatus::Success) && !seen_exec_stats.contains(&status) {
                    let desc = fuzzer.script_short_desc();
                    info!("[new-status] chain:{idx} {desc} | {status}");
                    seen_exec_stats.insert(status);
                }

                if found_new {
                    let desc = fuzzer.script_short_desc();
                    let cov = fuzzer.coverage_count();
                    info!(
                        "[+cov] chain:{idx} {desc} | corpus: {} | coverage: {cov}",
                        fuzzer.corpus_size()
                    );
                }
                if found_new || dug_changed {
                    last_phase2_novelty = Instant::now();
                }
            }
        }

        if phase2_entered && !pending_missing_data_targets.is_empty() {
            if let Some(ref d) = dug {
                let mut added = 0usize;
                for target_script in pending_missing_data_targets {
                    if chain_fuzzers.len() >= MAX_CHAIN_FUZZERS {
                        break;
                    }
                    added += spawn_targeted_missing_data_chains(
                        &mut chain_fuzzers,
                        &executor,
                        &entrypoints,
                        &type_pool,
                        &cov_trace_path,
                        &dict_string,
                        &initial_resource_writes,
                        &seq_db,
                        d,
                        &oneshot_fuzzers,
                        &missing_data_signals,
                        target_script,
                        max_chain_length,
                        max_chain_repetition,
                        seed_val,
                        &mut chain_seed_nonce,
                        &mut chain_rng,
                    );
                }
                if added > 0 {
                    info!("spawned {added} targeted chain fuzzers from missing-data signals");
                    state_dirty = true;
                }
            }
        }

        // broadcast new object discoveries to all fuzzers
        if !round_writes.is_empty() {
            for fuzzer in oneshot_fuzzers.iter_mut() {
                fuzzer.absorb_shared_object_writes(&round_writes);
            }
            for fuzzer in chain_fuzzers.iter_mut() {
                fuzzer.absorb_shared_object_writes(&round_writes);
            }
            state_dirty = true;
        }

        iteration += 1;

        // report progress every 5 seconds
        if last_report.elapsed().as_secs() >= 5 {
            let elapsed = start_time.elapsed();
            let elapsed_secs = elapsed.as_secs();
            let elapsed_str = fmt_elapsed(elapsed_secs);
            let execs_per_sec = total_execs as f64 / elapsed.as_secs_f64();

            // total corpus and coverage (oneshot + chains)
            let total_corpus: usize = oneshot_fuzzers
                .iter()
                .map(|f| f.corpus_size())
                .sum::<usize>()
                + chain_fuzzers.iter().map(|f| f.corpus_size()).sum::<usize>();
            let oneshot_corpus: usize = oneshot_fuzzers.iter().map(|f| f.corpus_size()).sum();
            let chain_corpus: usize = chain_fuzzers.iter().map(|f| f.corpus_size()).sum();
            let oneshot_avg_seed_score = if oneshot_fuzzers.is_empty() {
                0.0
            } else {
                oneshot_fuzzers
                    .iter()
                    .map(OneshotFuzzer::average_seed_score)
                    .sum::<f64>()
                    / oneshot_fuzzers.len() as f64
            };
            let chain_avg_seed_score = if chain_fuzzers.is_empty() {
                0.0
            } else {
                chain_fuzzers
                    .iter()
                    .map(ChainFuzzer::average_seed_score)
                    .sum::<f64>()
                    / chain_fuzzers.len() as f64
            };
            let best_oneshot_seed_score = oneshot_fuzzers
                .iter()
                .map(OneshotFuzzer::best_seed_score)
                .max()
                .unwrap_or(0);
            let best_chain_seed_score = chain_fuzzers
                .iter()
                .map(ChainFuzzer::best_seed_score)
                .max()
                .unwrap_or(0);
            let missing_data_target_count = missing_data_signals.len();
            let missing_data_hits: u64 = missing_data_signals
                .values()
                .map(|signal| signal.hits)
                .sum();
            let mut global_coverage = BTreeSet::new();
            for f in &oneshot_fuzzers {
                global_coverage.extend(f.coverage_keys());
            }
            for f in &chain_fuzzers {
                global_coverage.extend(f.coverage_keys());
            }
            let total_coverage = global_coverage.len();
            let coverage_delta = total_coverage.saturating_sub(last_report_coverage);
            let growth_rate = if elapsed_secs > 0 {
                total_coverage as f64 / (elapsed_secs as f64 / 60.0)
            } else {
                0.0
            };
            let dug_type_nodes = if phase2_entered {
                dug.as_ref()
                    .map_or(bootstrap_dug.num_types(), DefUseGraph::num_types)
            } else {
                bootstrap_dug.num_types()
            };
            let dug_seed_nodes = if phase2_entered {
                dug.as_ref()
                    .map_or(bootstrap_dug.num_seeds(), DefUseGraph::num_seeds)
            } else {
                bootstrap_dug.num_seeds()
            };
            let (chain_target_module_counts, chain_module_signature_counts) =
                chain_diversity_counts(&entrypoints, &chain_fuzzers);
            let unique_chain_target_modules = chain_target_module_counts.len();
            let unique_chain_module_signatures = chain_module_signature_counts.len();

            // header
            let phase_str = if phase2_entered { "Phase 2" } else { "Phase 1" };
            debug!(
                "[{elapsed_str}] {phase_str} | iter {iteration} | execs: {total_execs} ({execs_per_sec:.1}/s) \
                 | corpus: {total_corpus} | cov: {total_coverage} (+{coverage_delta}) \
                 | growth: {growth_rate:.1}/min | dug-types: {dug_type_nodes} | dug-seeds: {dug_seed_nodes}"
            );

            // per-script table
            let mut script_json = vec![];
            for (i, fuzzer) in oneshot_fuzzers.iter_mut().enumerate() {
                let desc = fuzzer.script_short_desc();
                let exec = fuzzer.exec_count();
                let corp = fuzzer.corpus_size();
                let cov = fuzzer.coverage_count();
                let delta = fuzzer.coverage_delta_since_report();

                let (hot_marker, last_str) = match fuzzer.last_new_coverage_time() {
                    Some(t) if t.elapsed().as_secs() < 30 => ("*", fmt_ago(t)),
                    Some(t) => (" ", fmt_ago(t)),
                    None => (" ", "-".to_string()),
                };

                let delta_str = if delta > 0 {
                    format!("+{delta:<6}")
                } else {
                    " ".repeat(7)
                };

                debug!(
                    "  {hot_marker}[{i:3}] {desc:<45} exec:{exec:<8} corp:{corp:<5} \
                     cov:{cov:<7} {delta_str} last:{last_str}"
                );

                script_json.push(json!({
                    "index": i,
                    "name": desc,
                    "exec_count": exec,
                    "corpus_size": corp,
                    "coverage_count": cov,
                    "coverage_delta": delta,
                    "last_new_coverage": last_str,
                }));
            }

            // per-chain-fuzzer table
            for (i, fuzzer) in chain_fuzzers.iter_mut().enumerate() {
                let desc = fuzzer.script_short_desc();
                let exec = fuzzer.exec_count();
                let corp = fuzzer.corpus_size();
                let cov = fuzzer.coverage_count();
                let delta = fuzzer.coverage_delta_since_report();
                let clen = fuzzer.chain_len();

                let (hot_marker, last_str) = match fuzzer.last_new_coverage_time() {
                    Some(t) if t.elapsed().as_secs() < 30 => ("*", fmt_ago(t)),
                    Some(t) => (" ", fmt_ago(t)),
                    None => (" ", "-".to_string()),
                };

                let delta_str = if delta > 0 {
                    format!("+{delta:<6}")
                } else {
                    " ".repeat(7)
                };

                debug!(
                    "  {hot_marker}[chain:{i:3}] (len={clen}) {desc:<36} exec:{exec:<8} corp:{corp:<5} \
                     cov:{cov:<7} {delta_str} last:{last_str}"
                );

                script_json.push(json!({
                    "index": format!("chain:{i}"),
                    "name": desc,
                    "chain_length": clen,
                    "exec_count": exec,
                    "corpus_size": corp,
                    "coverage_count": cov,
                    "coverage_delta": delta,
                    "last_new_coverage": last_str,
                }));
            }

            // simplified outcome summary by category
            let outcome_parts: Vec<_> = category_counts
                .iter()
                .map(|(k, v)| format!("{k}:{v}"))
                .collect();
            debug!(
                "  outcomes: {} | unique: {} | seq_db: {} entries | missing-data targets: {} hits: {} | chain target modules: {} | chain module signatures: {}",
                outcome_parts.join(" | "),
                seen_exec_stats.len(),
                seq_db.len(),
                missing_data_target_count,
                missing_data_hits,
                unique_chain_target_modules,
                unique_chain_module_signatures
            );

            // write JSON stats atomically
            let stats_json = json!({
                "elapsed_secs": elapsed_secs,
                "iteration": iteration,
                "phase": if phase2_entered { 2 } else { 1 },
                "total_execs": total_execs,
                "execs_per_sec": execs_per_sec,
                "total_corpus": total_corpus,
                "oneshot_corpus": oneshot_corpus,
                "chain_corpus": chain_corpus,
                "total_coverage": total_coverage,
                "coverage_delta": coverage_delta,
                "growth_rate_per_min": growth_rate,
                "bootstrap_profile_count": bootstrap_profile_count,
                "dug_type_nodes": dug_type_nodes,
                "dug_seed_nodes": dug_seed_nodes,
                "chain_fuzzer_count": chain_fuzzers.len(),
                "scripts": script_json,
                "outcomes": &stats,
                "outcome_categories": &category_counts,
                "sequence_db_entries": seq_db.len(),
                "oneshot_avg_seed_score": oneshot_avg_seed_score,
                "chain_avg_seed_score": chain_avg_seed_score,
                "best_oneshot_seed_score": best_oneshot_seed_score,
                "best_chain_seed_score": best_chain_seed_score,
                "missing_data_target_count": missing_data_target_count,
                "missing_data_hits": missing_data_hits,
                "unique_chain_target_modules": unique_chain_target_modules,
                "unique_chain_module_signatures": unique_chain_module_signatures,
            });
            let tmp_path = path_fuzz_stats.with_added_extension("tmp");
            if let Ok(data) = serde_json::to_string_pretty(&stats_json) {
                let _ = fs::write(&tmp_path, data);
                let _ = fs::rename(&tmp_path, &path_fuzz_stats);
            }

            save_checkpoint(
                &path_auto_state,
                &campaign_fingerprint,
                RuntimeCheckpoint {
                    entrypoint_identities: &entrypoint_identities,
                    phase2_entered,
                    bootstrap_profile_count,
                    bootstrap_dug: &bootstrap_dug,
                    dug: dug.as_ref(),
                    oneshot_fuzzers: &oneshot_fuzzers,
                    seq_db: &seq_db,
                    chain_fuzzers: &chain_fuzzers,
                    chain_seed_nonce,
                    object_state: current_object_state(&oneshot_fuzzers, &chain_fuzzers),
                    missing_data_signals: &missing_data_signals,
                },
                &mut state_dirty,
            )?;

            last_report_coverage = total_coverage;
            last_report = Instant::now();

            // Phase 1 → Phase 2 transition check
            let phase1_saturated = last_script_coverage_time
                .iter()
                .all(|t| t.elapsed().as_secs() >= saturation_secs);
            if !phase2_entered && phase1_saturated {
                info!(
                    "Phase 1 saturated per script after {}s ({} execution profiles collected). Transitioning to Phase 2.",
                    start_time.elapsed().as_secs(),
                    bootstrap_profile_count
                );

                // Use the per-execution bootstrap DUG built online in Phase 1.
                let built_dug =
                    std::mem::replace(&mut bootstrap_dug, DefUseGraph::new(num_scripts));
                info!(
                    "DUG: {} type nodes, {} scripts",
                    built_dug.num_types(),
                    built_dug.num_scripts()
                );

                // Construct chains
                let mut chains = sequence::construct_seed_chains(
                    &built_dug,
                    max_chain_length,
                    max_chain_repetition,
                    MAX_CHAIN_FUZZERS,
                    &mut chain_rng,
                );
                chains = diversify_candidates(
                    &entrypoints,
                    &built_dug,
                    chains,
                    &missing_data_signals,
                    &chain_fuzzers,
                    |candidate| &candidate.chain,
                );
                info!("constructed {} chains", chains.len());

                // Create chain fuzzers
                let mut seen_chain_instances = chain_instance_set(&chain_fuzzers);
                for seed_chain in chains {
                    let sequence::SeedChain {
                        chain,
                        seed_inputs,
                        target_seed_id: _,
                    } = seed_chain;
                    let bootstrap_seed = bootstrap_chain_seed(
                        &chain.steps,
                        seed_inputs,
                        &seq_db,
                        &built_dug,
                        &oneshot_fuzzers,
                        &mut chain_rng,
                    );
                    let _ = push_chain_fuzzer(
                        &mut chain_fuzzers,
                        &mut seen_chain_instances,
                        &executor,
                        &entrypoints,
                        &type_pool,
                        &cov_trace_path,
                        &dict_string,
                        &initial_resource_writes,
                        chain,
                        bootstrap_seed,
                        seed_val,
                        &mut chain_seed_nonce,
                    );
                }

                dug_last_marker = built_dug.modification_marker();
                dug_last_seed_marker = built_dug.seed_modification_marker();
                dug = Some(built_dug);
                last_chain_reconstruction = Instant::now();
                last_sequence_mutation = Instant::now();
                last_phase2_novelty = Instant::now();
                phase2_entered = true;
                state_dirty = true;

                // Log chain fuzzers
                eprintln!("\n=== Phase 2: Multi-transaction Fuzz ===");
                eprintln!(
                    "chains: {} | DUG types: {}",
                    chain_fuzzers.len(),
                    dug.as_ref().unwrap().num_types()
                );
                eprintln!("{}", "-".repeat(72));
                for (i, fuzzer) in chain_fuzzers.iter().enumerate() {
                    let desc = fuzzer.script_short_desc();
                    let clen = fuzzer.chain_len();
                    info!("[chain:{i:3}] (len={clen}) {desc}");
                }
                info!("Phase 2 entered with {} chain fuzzers", chain_fuzzers.len());
            }

            // Phase 2 scheduling:
            // 1) DUG-driven chain reconstruction (on DUG change)
            // 2) sequence mutation/extension (periodic, independent of DUG-change gate)
            if phase2_entered {
                let d = dug.as_mut().unwrap();
                if last_chain_reconstruction.elapsed().as_secs() >= CHAIN_REBUILD_INTERVAL_SECS
                    && (d.has_changed_since(dug_last_marker)
                        || d.has_seed_catalog_changed_since(dug_last_seed_marker))
                {
                    dug_last_marker = d.modification_marker();
                    dug_last_seed_marker = d.seed_modification_marker();
                    info!(
                        "DUG updated: {} type nodes, reconstructing chains",
                        d.num_types()
                    );

                    let mut new_chains = sequence::construct_seed_chains(
                        d,
                        max_chain_length,
                        max_chain_repetition,
                        MAX_CHAIN_FUZZERS,
                        &mut chain_rng,
                    );
                    new_chains = diversify_candidates(
                        &entrypoints,
                        d,
                        new_chains,
                        &missing_data_signals,
                        &chain_fuzzers,
                        |candidate| &candidate.chain,
                    );

                    // Deduplicate against both existing chains and any chains added during
                    // this reconstruction pass.
                    let mut seen_chain_instances = chain_instance_set(&chain_fuzzers);

                    let mut new_count = 0usize;
                    for seed_chain in new_chains {
                        let sequence::SeedChain {
                            chain,
                            seed_inputs,
                            target_seed_id: _,
                        } = seed_chain;
                        if chain_fuzzers.len() >= MAX_CHAIN_FUZZERS {
                            break;
                        }
                        let bootstrap_seed = bootstrap_chain_seed(
                            &chain.steps,
                            seed_inputs,
                            &seq_db,
                            d,
                            &oneshot_fuzzers,
                            &mut chain_rng,
                        );
                        if push_chain_fuzzer(
                            &mut chain_fuzzers,
                            &mut seen_chain_instances,
                            &executor,
                            &entrypoints,
                            &type_pool,
                            &cov_trace_path,
                            &dict_string,
                            &initial_resource_writes,
                            chain,
                            bootstrap_seed,
                            seed_val,
                            &mut chain_seed_nonce,
                        ) {
                            new_count += 1;
                        }
                    }

                    if new_count > 0 {
                        info!(
                            "spawned {new_count} new chain fuzzers from DUG reconstruction (total: {})",
                            chain_fuzzers.len()
                        );
                        state_dirty = true;
                    }

                    last_chain_reconstruction = Instant::now();
                }

                if last_sequence_mutation.elapsed().as_secs() >= SEQUENCE_MUTATION_INTERVAL_SECS {
                    let mut seen_chain_instances = chain_instance_set(&chain_fuzzers);

                    // Propose sequence extensions from SequenceDb
                    let mut extensions =
                        seq_db.propose_extensions(d, max_chain_length, max_chain_repetition, 10);
                    extensions = diversify_candidates(
                        &entrypoints,
                        d,
                        extensions,
                        &missing_data_signals,
                        &chain_fuzzers,
                        |candidate| &candidate.0,
                    );
                    let mut ext_count = 0usize;
                    for (ext_chain, parent_seed) in extensions {
                        if chain_fuzzers.len() >= MAX_CHAIN_FUZZERS {
                            break;
                        }
                        if push_chain_fuzzer(
                            &mut chain_fuzzers,
                            &mut seen_chain_instances,
                            &executor,
                            &entrypoints,
                            &type_pool,
                            &cov_trace_path,
                            &dict_string,
                            &initial_resource_writes,
                            ext_chain,
                            parent_seed,
                            seed_val,
                            &mut chain_seed_nonce,
                        ) {
                            ext_count += 1;
                        }
                    }

                    // Sequence-level mutations from SequenceDb
                    let mut mutations =
                        seq_db.propose_mutations(d, max_chain_length, max_chain_repetition, 10);
                    mutations = diversify_candidates(
                        &entrypoints,
                        d,
                        mutations,
                        &missing_data_signals,
                        &chain_fuzzers,
                        |candidate| &candidate.0,
                    );
                    let mut mut_count = 0usize;
                    for (mut_chain, parent_seed) in mutations {
                        if chain_fuzzers.len() >= MAX_CHAIN_FUZZERS {
                            break;
                        }
                        if push_chain_fuzzer(
                            &mut chain_fuzzers,
                            &mut seen_chain_instances,
                            &executor,
                            &entrypoints,
                            &type_pool,
                            &cov_trace_path,
                            &dict_string,
                            &initial_resource_writes,
                            mut_chain,
                            parent_seed,
                            seed_val,
                            &mut chain_seed_nonce,
                        ) {
                            mut_count += 1;
                        }
                    }

                    if ext_count + mut_count > 0 {
                        info!(
                            "spawned {} new chain fuzzers ({ext_count} from extensions, {mut_count} from mutations, total: {})",
                            ext_count + mut_count,
                            chain_fuzzers.len()
                        );
                        state_dirty = true;
                    }

                    last_sequence_mutation = Instant::now();
                }
            }

            if phase2_entered
                && coverage_delta == 0
                && last_phase2_novelty.elapsed().as_secs() >= saturation_secs
            {
                info!(
                    "Phase 2 saturated after {}s without coverage/DUG novelty for {}s. Stopping.",
                    start_time.elapsed().as_secs(),
                    saturation_secs
                );
                save_checkpoint(
                    &path_auto_state,
                    &campaign_fingerprint,
                    RuntimeCheckpoint {
                        entrypoint_identities: &entrypoint_identities,
                        phase2_entered,
                        bootstrap_profile_count,
                        bootstrap_dug: &bootstrap_dug,
                        dug: dug.as_ref(),
                        oneshot_fuzzers: &oneshot_fuzzers,
                        seq_db: &seq_db,
                        chain_fuzzers: &chain_fuzzers,
                        chain_seed_nonce,
                        object_state: current_object_state(&oneshot_fuzzers, &chain_fuzzers),
                        missing_data_signals: &missing_data_signals,
                    },
                    &mut state_dirty,
                )?;
                return Ok(());
            }
        }
    }
}

/// Mix two u64 values into a deterministic, well-scrambled seed.
fn derive_seed(base: u64, salt: u64) -> u64 {
    let mut x = base ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Maximum number of instantiations to generate per generic struct
const MAX_INSTANTIATIONS_PER_STRUCT: usize = 8;
/// Maximum number of rounds to expand generic struct candidates.
///
/// This keeps recursive generic families bounded while still allowing nested
/// shapes like `Outer<Inner<u64>>`.
const MAX_GENERIC_TYPE_POOL_ROUNDS: usize = 3;

/// Build a type pool from the model for generic type argument fuzzing
fn build_type_pool(model: &Model) -> TypePool {
    let mut pool = TypePool::new();
    let mut seen_pool_entries = BTreeSet::new();

    // step 1: collect generic-argument candidates (base types plus one vector layer).
    let mut candidates = BTreeMap::new();

    // add all primitive types as candidates
    let primitives = [
        VmTypeTag::Bool,
        VmTypeTag::U8,
        VmTypeTag::U16,
        VmTypeTag::U32,
        VmTypeTag::U64,
        VmTypeTag::U128,
        VmTypeTag::U256,
        VmTypeTag::Address,
    ];
    for prim in &primitives {
        add_type_candidate(
            &mut pool,
            &mut seen_pool_entries,
            &mut candidates,
            prim.clone(),
            AbilitySet::PRIMITIVES,
        );
    }

    // add non-generic struct types as candidates
    for decl in model.datatype_registry.iter_decls() {
        if !decl.generics.is_empty() {
            continue;
        }
        let struct_tag = StructTag {
            address: decl.ident.address(),
            module: Identifier::new(decl.ident.module_name()).expect("valid identifier"),
            name: Identifier::new(decl.ident.datatype_name()).expect("valid identifier"),
            type_args: vec![],
        };
        add_type_candidate(
            &mut pool,
            &mut seen_pool_entries,
            &mut candidates,
            VmTypeTag::Struct(Box::new(struct_tag)),
            decl.abilities,
        );
    }

    // step 2: instantiate generic structs, reusing earlier instantiations in
    // later rounds so nested generic types remain reachable.
    let generic_structs: Vec<_> = model
        .datatype_registry
        .iter_decls()
        .filter(|decl| !decl.generics.is_empty())
        .collect();
    expand_generic_struct_candidates(
        &mut pool,
        &mut seen_pool_entries,
        &mut candidates,
        &generic_structs,
    );

    pool
}

fn add_type_candidate(
    pool: &mut TypePool,
    seen_pool_entries: &mut BTreeSet<VmTypeTag>,
    candidates: &mut BTreeMap<VmTypeTag, AbilitySet>,
    ty: VmTypeTag,
    abilities: AbilitySet,
) -> bool {
    let inserted = insert_type_entry(pool, seen_pool_entries, candidates, ty.clone(), abilities);

    if !matches!(ty, VmTypeTag::Vector(_)) {
        let vector_abilities = abilities.intersect(AbilitySet::VECTOR);
        insert_type_entry(
            pool,
            seen_pool_entries,
            candidates,
            VmTypeTag::Vector(Box::new(ty)),
            vector_abilities,
        );
    }

    inserted
}

fn insert_type_entry(
    pool: &mut TypePool,
    seen_pool_entries: &mut BTreeSet<VmTypeTag>,
    candidates: &mut BTreeMap<VmTypeTag, AbilitySet>,
    ty: VmTypeTag,
    abilities: AbilitySet,
) -> bool {
    match candidates.entry(ty.clone()) {
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(abilities);
            if seen_pool_entries.insert(ty.clone()) {
                pool.add(ty, abilities);
            }
            true
        },
        std::collections::btree_map::Entry::Occupied(entry) => {
            debug_assert_eq!(*entry.get(), abilities);
            false
        },
    }
}

fn expand_generic_struct_candidates(
    pool: &mut TypePool,
    seen_pool_entries: &mut BTreeSet<VmTypeTag>,
    candidates: &mut BTreeMap<VmTypeTag, AbilitySet>,
    generic_structs: &[&DatatypeDecl],
) {
    for _ in 0..MAX_GENERIC_TYPE_POOL_ROUNDS {
        let snapshot: Vec<_> = candidates
            .iter()
            .map(|(ty, ab)| (ty.clone(), *ab))
            .collect();
        let mut added_any = false;

        for decl in generic_structs {
            let per_param_candidates: Vec<Vec<_>> = decl
                .generics
                .iter()
                .map(|(constraint, _is_phantom)| {
                    snapshot
                        .iter()
                        .filter(|(_, abilities)| constraint.is_subset(*abilities))
                        .collect()
                })
                .collect();

            if per_param_candidates.iter().any(|c| c.is_empty()) {
                continue;
            }

            let max_candidates = per_param_candidates.iter().map(|c| c.len()).max().unwrap();
            let num_instantiations = max_candidates.min(MAX_INSTANTIATIONS_PER_STRUCT);

            for i in 0..num_instantiations {
                let type_args: Vec<_> = per_param_candidates
                    .iter()
                    .map(|cands| cands[i % cands.len()].0.clone())
                    .collect();

                let actual_abilities = compute_instantiated_abilities(
                    decl.abilities,
                    &decl.generics,
                    candidates,
                    &type_args,
                );

                let struct_tag = StructTag {
                    address: decl.ident.address(),
                    module: Identifier::new(decl.ident.module_name()).expect("valid identifier"),
                    name: Identifier::new(decl.ident.datatype_name()).expect("valid identifier"),
                    type_args,
                };
                added_any |= add_type_candidate(
                    pool,
                    seen_pool_entries,
                    candidates,
                    VmTypeTag::Struct(Box::new(struct_tag)),
                    actual_abilities,
                );
            }
        }

        if !added_any {
            break;
        }
    }
}

/// Compute the actual abilities of a generic struct instantiated with concrete type arguments
fn compute_instantiated_abilities(
    declared_abilities: AbilitySet,
    generics: &[(AbilitySet, bool)],
    candidates: &BTreeMap<VmTypeTag, AbilitySet>,
    type_args: &[VmTypeTag],
) -> AbilitySet {
    use move_core_types::ability::Ability;

    // collect abilities of each type argument
    let mut provided_abilities = AbilitySet::ALL;
    for (ty_arg, (_, is_phantom)) in type_args.iter().zip(generics.iter()) {
        if *is_phantom {
            continue;
        }
        let arg_abilities = *candidates
            .get(ty_arg)
            .expect("type argument should come from the candidate pool");
        provided_abilities = provided_abilities.intersect(arg_abilities);
    }

    // apply the same logic as derive_actual_ability
    let mut actual_abilities = AbilitySet::EMPTY;
    for ability in Ability::all() {
        if declared_abilities.has_ability(ability)
            && provided_abilities.has_ability(ability.requires())
        {
            actual_abilities = actual_abilities | ability;
        }
    }
    actual_abilities
}

/// Format a Duration as HH:MM:SS
fn fmt_elapsed(secs: u64) -> String {
    let hh = secs / 3600;
    let mm = (secs % 3600) / 60;
    let ss = secs % 60;
    format!("{hh:02}:{mm:02}:{ss:02}")
}

/// Format a duration since an instant as a human-readable "ago" string
fn fmt_ago(since: Instant) -> String {
    let secs = since.elapsed().as_secs();
    if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else {
        format!("{}h ago", secs / 3600)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        add_type_candidate, campaign_fingerprint, chain_instance_identity,
        chain_missing_data_resolution, entrypoint_cache_fingerprint,
        expand_generic_struct_candidates, record_missing_data_signal, MissingDataSignal, SeedInput,
        MAX_GENERIC_TYPE_POOL_ROUNDS,
    };
    use crate::{
        deps::PkgKind,
        executor::{
            sequence::{Chain, DefUseGraph, ExecResourceProfile, ResourceTag},
            tracing::ResourceWrite,
        },
        mutate::mutator::TypePool,
        prep::{
            canvas::{BasicInput, ScriptSignature},
            datatype::DatatypeDecl,
            ident::{DatatypeIdent, FunctionIdent},
        },
    };
    use move_core_types::{
        ability::AbilitySet,
        account_address::AccountAddress,
        identifier::Identifier,
        language_storage::{StructTag, TypeTag as VmTypeTag},
    };
    use std::collections::{BTreeMap, BTreeSet};

    fn make_decl(name: &str, generics: Vec<(AbilitySet, bool)>) -> DatatypeDecl {
        DatatypeDecl {
            ident: DatatypeIdent::from_struct_tuple(
                AccountAddress::ONE,
                Identifier::new("m").unwrap(),
                Identifier::new(name).unwrap(),
            ),
            generics,
            abilities: AbilitySet::PRIMITIVES,
            kind: PkgKind::Primary,
        }
    }

    fn make_struct(name: &str, type_args: Vec<VmTypeTag>) -> VmTypeTag {
        VmTypeTag::Struct(Box::new(StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("m").unwrap(),
            name: Identifier::new(name).unwrap(),
            type_args,
        }))
    }

    fn make_tag_at(name: &str, account: AccountAddress) -> ResourceTag {
        ResourceTag {
            account,
            struct_tag: StructTag {
                address: AccountAddress::ONE,
                module: Identifier::new("m").unwrap(),
                name: Identifier::new(name).unwrap(),
                type_args: vec![],
            },
        }
    }

    fn make_object_group_tag(account: AccountAddress) -> ResourceTag {
        ResourceTag {
            account,
            struct_tag: StructTag {
                address: AccountAddress::ONE,
                module: Identifier::new("object").unwrap(),
                name: Identifier::new("ObjectGroup").unwrap(),
                type_args: vec![],
            },
        }
    }

    #[test]
    fn generic_type_pool_supports_nested_generic_structs() {
        let mut pool = TypePool::new();
        let mut seen_pool_entries = BTreeSet::new();
        let mut candidates = BTreeMap::new();
        add_type_candidate(
            &mut pool,
            &mut seen_pool_entries,
            &mut candidates,
            VmTypeTag::U64,
            AbilitySet::PRIMITIVES,
        );

        let inner = make_decl("Inner", vec![(AbilitySet::EMPTY, false)]);
        let outer = make_decl("Outer", vec![(AbilitySet::EMPTY, false)]);
        expand_generic_struct_candidates(&mut pool, &mut seen_pool_entries, &mut candidates, &[
            &inner, &outer,
        ]);

        let inner_u64 = make_struct("Inner", vec![VmTypeTag::U64]);
        let outer_inner_u64 = make_struct("Outer", vec![inner_u64.clone()]);
        assert!(
            pool.candidates_for(AbilitySet::EMPTY)
                .contains(&&outer_inner_u64),
            "expected nested generic struct after {MAX_GENERIC_TYPE_POOL_ROUNDS} expansion rounds",
        );

        let outer_vec_inner_u64 =
            make_struct("Outer", vec![VmTypeTag::Vector(Box::new(inner_u64))]);
        assert!(
            pool.candidates_for(AbilitySet::EMPTY)
                .contains(&&outer_vec_inner_u64),
            "expected vector of generated generic struct to be reusable as a type argument",
        );
    }

    #[test]
    fn entrypoint_cache_fingerprint_changes_with_script_generation_limit() {
        let fast = entrypoint_cache_fingerprint(&[], 3, 1, 60);
        let slow = entrypoint_cache_fingerprint(&[], 3, 1, 600);
        let unlimited = entrypoint_cache_fingerprint(&[], 3, 1, 0);

        assert_ne!(fast, slow);
        assert_ne!(slow, unlimited);
        assert_ne!(fast, unlimited);
    }

    #[test]
    fn campaign_fingerprint_is_stable_across_entrypoint_reordering() {
        let entry_a = (
            ScriptSignature {
                name: "script_a".to_string(),
                ident: FunctionIdent::from_function_tuple(
                    AccountAddress::ONE,
                    Identifier::new("m").unwrap(),
                    Identifier::new("a").unwrap(),
                ),
                generics: vec![AbilitySet::EMPTY],
                parameters: vec![BasicInput::Address],
            },
            vec![1u8, 2, 3],
        );
        let entry_b = (
            ScriptSignature {
                name: "script_b".to_string(),
                ident: FunctionIdent::from_function_tuple(
                    AccountAddress::ONE,
                    Identifier::new("m").unwrap(),
                    Identifier::new("b").unwrap(),
                ),
                generics: vec![],
                parameters: vec![BasicInput::U64],
            },
            vec![4u8, 5, 6],
        );

        let fingerprint_ab =
            campaign_fingerprint(&[entry_a.clone(), entry_b.clone()], &[], 5, 2, 4);
        let fingerprint_ba = campaign_fingerprint(&[entry_b, entry_a], &[], 5, 2, 4);
        assert_eq!(fingerprint_ab, fingerprint_ba);
    }

    #[test]
    fn record_missing_data_signal_filters_noise_and_keeps_object_equivalent_reads() {
        let object_a = AccountAddress::from_hex_literal("0x100").unwrap();
        let object_b = AccountAddress::from_hex_literal("0x200").unwrap();
        let ready = make_tag_at("Ready", AccountAddress::from_hex_literal("0x300").unwrap());
        let needed = make_tag_at("Vault", object_a);
        let observed_equivalent = make_tag_at("Vault", object_b);

        let mut dug = DefUseGraph::new(1);
        dug.add_initial_tag(&make_object_group_tag(object_a));
        dug.add_initial_tag(&make_object_group_tag(object_b));
        dug.add_initial_tag(&ready);
        dug.add_use(0, &needed);
        dug.add_use(0, &ready);

        let profile = ExecResourceProfile {
            script_index: 0,
            reads: BTreeSet::from([ready.clone(), observed_equivalent.clone()]),
            writes: BTreeSet::new(),
            succeeded: false,
        };
        let mut signals = BTreeMap::new();
        record_missing_data_signal(&mut signals, &dug, 9, &profile);

        let signal = signals
            .get(&0)
            .expect("expected unresolved missing-data signal");
        assert_eq!(signal.hits, 1);
        assert_eq!(signal.last_seen_iter, 9);
        assert_eq!(
            signal.unresolved_tags,
            BTreeSet::from([observed_equivalent])
        );
    }

    #[test]
    fn chain_missing_data_resolution_counts_object_equivalent_defs() {
        let object_a = AccountAddress::from_hex_literal("0x400").unwrap();
        let object_b = AccountAddress::from_hex_literal("0x500").unwrap();
        let mut dug = DefUseGraph::new(2);
        dug.add_initial_tag(&make_object_group_tag(object_a));
        dug.add_initial_tag(&make_object_group_tag(object_b));
        dug.add_def(0, &make_tag_at("Vault", object_a));

        let signal = MissingDataSignal {
            hits: 1,
            last_seen_iter: 0,
            unresolved_tags: BTreeSet::from([make_tag_at("Vault", object_b)]),
        };

        assert_eq!(
            chain_missing_data_resolution(&dug, &Chain { steps: vec![0, 1] }, &signal),
            1,
        );
    }

    #[test]
    fn chain_instance_identity_distinguishes_bootstrap_seed() {
        let seed_a = vec![SeedInput::new(AccountAddress::ONE, vec![], vec![])];
        let seed_b = vec![SeedInput::new(AccountAddress::TWO, vec![], vec![])];

        assert_eq!(
            chain_instance_identity(&[1, 2], &seed_a),
            chain_instance_identity(&[1, 2], &seed_a),
        );
        assert_ne!(
            chain_instance_identity(&[1, 2], &seed_a),
            chain_instance_identity(&[1, 2], &seed_b),
        );
        assert_ne!(
            chain_instance_identity(&[1, 2], &seed_a),
            chain_instance_identity(&[2, 1], &seed_a),
        );
    }

    #[test]
    fn campaign_fingerprint_changes_with_initial_state() {
        let entry = (
            ScriptSignature {
                name: "script_a".to_string(),
                ident: FunctionIdent::from_function_tuple(
                    AccountAddress::ONE,
                    Identifier::new("m").unwrap(),
                    Identifier::new("a").unwrap(),
                ),
                generics: vec![],
                parameters: vec![],
            },
            vec![1u8, 2, 3],
        );
        let write = ResourceWrite {
            address: AccountAddress::from_hex_literal("0x44").unwrap(),
            struct_tag: StructTag {
                address: AccountAddress::ONE,
                module: Identifier::new("m").unwrap(),
                name: Identifier::new("State").unwrap(),
                type_args: vec![],
            },
            is_resource_group: false,
        };

        let without_state = campaign_fingerprint(&[entry.clone()], &[], 5, 2, 4);
        let with_state = campaign_fingerprint(&[entry], &[write], 5, 2, 4);
        assert_ne!(without_state, with_state);
    }
}

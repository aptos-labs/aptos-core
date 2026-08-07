#![no_main]
#![allow(unexpected_cfgs)]

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#[cfg(fuzzing)]
use aptos_block_executor::code_cache_global_manager::ModuleHotCacheSnapshot;
use aptos_block_executor::{
    code_cache_global_manager::{AptosModuleCacheManager, AptosModuleCacheManagerGuard},
    txn_commit_hook::NoOpTransactionCommitHook,
    txn_provider::default::DefaultTxnProvider,
};
use aptos_crypto::HashValue;
use aptos_transaction_simulation::{
    Account, SimulationStateStore, TransactionBuilder, GENESIS_CHANGE_SET_HEAD,
};
use aptos_types::{
    account_address::AccountAddress,
    block_executor::{
        config::{
            BlockExecutorConfig, BlockExecutorConfigFromOnchain, BlockExecutorLocalConfig,
            BlockExecutorModuleCacheLocalConfig,
        },
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    chain_id::ChainId,
    on_chain_config::{BlockGasLimitType, FeatureFlag, Features, TimedFeaturesBuilder},
    transaction::{
        signature_verified_transaction::into_signature_verified_block, AuxiliaryInfo,
        EntryFunction, ExecutionStatus, PersistedAuxiliaryInfo, ReplayProtector, Script,
        SignedTransaction, Transaction, TransactionArgument, TransactionOutput, TransactionPayload,
        TransactionStatus,
    },
    vm_status::{StatusCode, StatusType, VMStatus},
    write_set::WriteSet,
};
use aptos_vm::block_executor::AptosVMBlockExecutorWrapper;
#[cfg(fuzzing)]
use aptos_vm_environment::environment::set_vm_config_override_for_fuzzing;
use aptos_vm_environment::{prod_configs, prod_configs::LATEST_GAS_FEATURE_VERSION};
use blockstm_executor::{assert_outputs_equal, BlockFuzzExecutor};
use blockv2_fuzz_config::{
    apply_verifier_config_overrides, env_optional_bool, env_optional_u32, env_optional_u64,
    env_optional_usize, env_u64, module_cache_config_from_env,
};
use fuzzer::{BlockExecVariantV2, RunnableBlockStateV2, RunnableBlockTransactionV2, UserAccount};
use libfuzzer_sys::{fuzz_target, Corpus};
use move_binary_format::{
    access::ModuleAccess, deserializer::DeserializerConfig, file_format::CompiledModule,
    file_format_common::VERSION_MAX,
};
use move_core_types::language_storage::ModuleId;
use once_cell::sync::Lazy;
use std::{
    collections::{btree_map::Entry, BTreeMap, HashMap},
    env,
    hash::Hash,
    sync::atomic::{AtomicU64, Ordering},
};

mod blockstm_executor;
mod blockv2_fuzz_config;
mod utils;
use utils::vm::{
    checked_module_self_id, filter_bad_modules, filter_bad_tx, group_modules_by_address_topo,
    has_invalid_split_blocks, is_split_block, publish_transaction_payload_with_package_names,
    resolve_function_name, resolve_module_ref, resolve_module_refs, script_bytecode_version,
    verify_module_fast, verify_script_fast,
};

#[cfg(fuzzing)]
type HotCacheSnapshot = ModuleHotCacheSnapshot;
#[cfg(not(fuzzing))]
struct HotCacheSnapshot;

static VM_WRITE_SET: Lazy<WriteSet> = Lazy::new(|| GENESIS_CHANGE_SET_HEAD.write_set().clone());
// A single module cache manager and version counter are shared by both the sequential and the
// Block-STM v2 executions of each fuzz case. Using one manager (rather than one per mode) keeps the
// Aptos framework hot-loaded in the global module cache while guaranteeing that both executions see
// an identical execution environment. The framework's verified modules are bound to the environment
// they were verified under (interned struct names, type pool, verifier cache), so the framework can
// only stay warm if its environment is kept; the snapshot/rollback below resets everything else (the
// fuzz case's own published modules) to a clean framework-only baseline before each execution.
//
// Two independent managers would each accumulate their own interned/verified state across fuzz
// iterations and drift apart, making the sequential-vs-v2 comparison non-hermetic (e.g. spurious
// divergences that only reproduce deep into a fuzzing session, never on the input in isolation).
static MODULE_CACHE_MANAGER: Lazy<AptosModuleCacheManager> =
    Lazy::new(AptosModuleCacheManager::new);
static NEXT_VERSION: AtomicU64 = AtomicU64::new(1);

const MIN_CONCURRENCY_LEVEL: usize = 2;
const MAX_CONCURRENCY_LEVEL: usize = 8;
const MAX_BLOCK_ACCOUNTS: usize = 256;
const MAX_BLOCK_MODULES: usize = 32;
const MAX_BLOCK_TXNS: usize = 24;
const MAX_SECONDARY_SIGNERS: usize = 10;
const DEFAULT_GAS_UNIT_PRICE: u64 = 100;
const DEFAULT_MAX_GAS_AMOUNT: u64 = 2_000_000;

static FUZZ_GAS_UNIT_PRICE: Lazy<u64> =
    Lazy::new(|| env_u64("APTOS_FUZZ_GAS_UNIT_PRICE", DEFAULT_GAS_UNIT_PRICE));
static FUZZ_MAX_GAS_AMOUNT: Lazy<u64> =
    Lazy::new(|| env_u64("APTOS_FUZZ_MAX_GAS_AMOUNT", DEFAULT_MAX_GAS_AMOUNT));
static FUZZ_EFFECTIVE_BLOCK_GAS_LIMIT: Lazy<Option<u64>> =
    Lazy::new(|| env_optional_u64("APTOS_FUZZ_EFFECTIVE_BLOCK_GAS_LIMIT"));
static FUZZ_BLOCK_OUTPUT_LIMIT: Lazy<Option<u64>> =
    Lazy::new(|| env_optional_u64("APTOS_FUZZ_BLOCK_OUTPUT_LIMIT"));
static FUZZ_CONFLICT_PENALTY_WINDOW: Lazy<Option<u32>> =
    Lazy::new(|| env_optional_u32("APTOS_FUZZ_CONFLICT_PENALTY_WINDOW"));
static FUZZ_EXECUTION_GAS_EFFECTIVE_MULTIPLIER: Lazy<Option<u64>> =
    Lazy::new(|| env_optional_u64("APTOS_FUZZ_EXECUTION_GAS_EFFECTIVE_MULTIPLIER"));
static FUZZ_IO_GAS_EFFECTIVE_MULTIPLIER: Lazy<Option<u64>> =
    Lazy::new(|| env_optional_u64("APTOS_FUZZ_IO_GAS_EFFECTIVE_MULTIPLIER"));
static FUZZ_USE_GRANULAR_RESOURCE_GROUP_CONFLICTS: Lazy<Option<bool>> =
    Lazy::new(|| env_optional_bool("APTOS_FUZZ_USE_GRANULAR_RESOURCE_GROUP_CONFLICTS"));
static FUZZ_MAX_IDENTIFIER_SIZE: Lazy<Option<u64>> =
    Lazy::new(|| env_optional_u64("APTOS_FUZZ_MAX_IDENTIFIER_SIZE"));
static FUZZ_MAX_CONCURRENCY_LEVEL: Lazy<usize> = Lazy::new(|| {
    let max_concurrency =
        env_optional_usize("APTOS_FUZZ_MAX_CONCURRENCY_LEVEL").unwrap_or(MAX_CONCURRENCY_LEVEL);
    assert!(
        max_concurrency >= MIN_CONCURRENCY_LEVEL,
        "APTOS_FUZZ_MAX_CONCURRENCY_LEVEL must be at least {MIN_CONCURRENCY_LEVEL}"
    );
    max_concurrency
});
static FUZZ_ALLOW_FALLBACK: Lazy<bool> =
    Lazy::new(|| env_optional_bool("APTOS_FUZZ_ALLOW_FALLBACK").unwrap_or(false));
static FUZZ_ASYNC_RUNTIME_CHECKS: Lazy<bool> =
    Lazy::new(|| env_optional_bool("APTOS_FUZZ_ASYNC_RUNTIME_CHECKS").unwrap_or(true));
static FUZZ_PARANOID_REF_CHECKS: Lazy<bool> =
    Lazy::new(|| env_optional_bool("APTOS_FUZZ_PARANOID_REF_CHECKS").unwrap_or(true));
#[cfg(fuzzing)]
static CONFIGURE_FUZZ_VM_CONFIG: Lazy<()> = Lazy::new(|| {
    set_vm_config_override_for_fuzzing(Box::new(blockv2_fuzz_config::apply_vm_config_overrides));
});
#[cfg(not(fuzzing))]
static CONFIGURE_FUZZ_VM_CONFIG: Lazy<()> = Lazy::new(|| {});
static FUZZ_ENABLE_LAYOUT_CACHES: Lazy<bool> = Lazy::new(|| {
    env_optional_bool("APTOS_FUZZ_ENABLE_LAYOUT_CACHES")
        .unwrap_or_else(|| env::var_os("APTOS_FUZZ_MAX_LAYOUT_CACHE_SIZE").is_some())
});
static FUZZ_MODULE_CACHE_CONFIG: Lazy<BlockExecutorModuleCacheLocalConfig> =
    Lazy::new(module_cache_config_from_env);

/// Which executor lane(s) each fuzz case exercises, chosen via the `FUZZER_EXEC_MODE` environment
/// variable (default `v2+seq`):
///   - `v2+seq`: run Block-STM v2 (parallel) and sequential, assert their outputs match.
///   - `seq`:    run sequential only (no comparison; just status/invariant checks).
///   - `v2`:     run Block-STM v2 (parallel) only.
#[derive(Clone, Copy)]
enum ExecMode {
    V2AndSeq,
    Seq,
    V2,
}

static EXEC_MODE: Lazy<ExecMode> = Lazy::new(|| {
    let raw = env::var("FUZZER_EXEC_MODE").unwrap_or_else(|_| "v2+seq".to_string());
    match raw.as_str() {
        "v2+seq" => ExecMode::V2AndSeq,
        "seq" => ExecMode::Seq,
        "v2" => ExecMode::V2,
        other => panic!("unknown FUZZER_EXEC_MODE={other:?}; expected one of: v2+seq, seq, v2"),
    }
});

fn require_fuzzing_cfg() {
    #[cfg(not(fuzzing))]
    {
        eprintln!(
            "move_aptosvm_blockstm_v2_compare must be built with cfg(fuzzing); \
             refusing to run a non-fuzzing build"
        );
        std::process::abort();
    }
}

fn next_metadata(counter: &AtomicU64) -> TransactionSliceMetadata {
    let end = counter.fetch_add(1, Ordering::Relaxed);
    TransactionSliceMetadata::chunk(end - 1, end)
}

fn create_user_account(vm: &mut BlockFuzzExecutor, account: UserAccount) -> Account {
    if account.is_inited_and_funded {
        vm.create_accounts(1, account.fund_amount(), 0).remove(0)
    } else {
        vm.new_unfunded_account()
    }
}

fn slot_account(accounts: &[Account], slot: u8) -> &Account {
    assert!(
        !accounts.is_empty(),
        "account slots checked at fuzz case boundary"
    );
    &accounts[slot as usize % accounts.len()]
}

fn publish_modules_sender(modules: &[CompiledModule]) -> Result<AccountAddress, Corpus> {
    let sender = *modules.first().ok_or(Corpus::Keep)?.address();
    if modules.iter().any(|module| module.address() != &sender) {
        return Err(Corpus::Keep);
    }
    Ok(sender)
}

fn ensure_account_at<'a>(
    vm: &mut BlockFuzzExecutor,
    accounts_by_address: &'a mut HashMap<AccountAddress, Account>,
    address: AccountAddress,
) -> &'a Account {
    accounts_by_address
        .entry(address)
        .or_insert_with(|| vm.new_account_at(address))
}

fn next_sequence_number<K: Copy + Eq + Hash>(sequences: &mut HashMap<K, u64>, key: K) -> u64 {
    let next = sequences.entry(key).or_insert(0);
    *next += 1;
    *next - 1
}

fn transaction_builder(sender: &Account, replay_protector: ReplayProtector) -> TransactionBuilder {
    let builder = sender
        .transaction()
        .gas_unit_price(*FUZZ_GAS_UNIT_PRICE)
        .max_gas_amount(*FUZZ_MAX_GAS_AMOUNT);
    match replay_protector {
        ReplayProtector::SequenceNumber(sequence_number) => {
            builder.sequence_number(sequence_number)
        },
        ReplayProtector::Nonce(_) => builder,
    }
}

fn apply_replay_protector(
    payload: TransactionPayload,
    replay_protector: ReplayProtector,
) -> TransactionPayload {
    match replay_protector {
        ReplayProtector::SequenceNumber(_) => payload,
        ReplayProtector::Nonce(nonce) => payload.set_replay_protection_nonce(nonce),
    }
}

fn next_replay_protector(
    orderless: bool,
    sequence_by_address: &mut HashMap<AccountAddress, u64>,
    nonce_by_address: &mut HashMap<AccountAddress, u64>,
    sender: AccountAddress,
) -> ReplayProtector {
    if orderless {
        ReplayProtector::Nonce(next_sequence_number(nonce_by_address, sender))
    } else {
        ReplayProtector::SequenceNumber(next_sequence_number(sequence_by_address, sender))
    }
}

fn secondary_signer_accounts(
    slot_accounts: &[Account],
    secondary_slots: &[u8],
) -> Result<Vec<Account>, Corpus> {
    if secondary_slots.len() > MAX_SECONDARY_SIGNERS {
        return Err(Corpus::Keep);
    }
    Ok(secondary_slots
        .iter()
        .map(|slot| slot_account(slot_accounts, *slot).clone())
        .collect())
}

fn build_signed_transaction(
    vm: &mut BlockFuzzExecutor,
    modules: &[CompiledModule],
    slot_accounts: &[Account],
    accounts_by_address: &mut HashMap<AccountAddress, Account>,
    sequence_by_address: &mut HashMap<AccountAddress, u64>,
    nonce_by_address: &mut HashMap<AccountAddress, u64>,
    package_names_by_module: &BTreeMap<ModuleId, String>,
    input: &RunnableBlockTransactionV2,
) -> Result<SignedTransaction, Corpus> {
    // Sequence numbers are bumped eagerly: any error below rejects the entire fuzz case, so the
    // sequence maps are never observed in a half-updated state.
    let (sender_acc, payload) = match &input.exec_variant {
        BlockExecVariantV2::Script {
            _script,
            _type_args,
            _args,
        } => {
            let mut script_bytes = vec![];
            _script
                .serialize_for_version(Some(script_bytecode_version(_script)), &mut script_bytes)
                .map_err(|_| Corpus::Keep)?;
            let args = _args
                .iter()
                .cloned()
                .map(TransactionArgument::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| Corpus::Keep)?;
            let sender_acc = slot_account(slot_accounts, input.sender_slot).clone();
            (
                sender_acc,
                TransactionPayload::Script(Script::new(script_bytes, _type_args.clone(), args)),
            )
        },
        BlockExecVariantV2::CallFunction {
            _module_idx,
            _function,
            _type_args,
            _args,
        } => {
            let module = resolve_module_ref(modules, *_module_idx)?;
            let module_id = checked_module_self_id(module);
            let function_name = resolve_function_name(module, *_function)?;
            let sender_acc = slot_account(slot_accounts, input.sender_slot).clone();
            (
                sender_acc,
                TransactionPayload::EntryFunction(EntryFunction::new(
                    module_id,
                    function_name,
                    _type_args.clone(),
                    _args.clone(),
                )),
            )
        },
        BlockExecVariantV2::Publish { _module_idxs } => {
            let _modules = resolve_module_refs(modules, _module_idxs)?;
            let sender = publish_modules_sender(&_modules)?;
            let package_name = package_name_for_modules(&_modules, package_names_by_module);
            let sender_acc = ensure_account_at(vm, accounts_by_address, sender).clone();
            (
                sender_acc,
                publish_transaction_payload_with_package_names(
                    &_modules,
                    package_name,
                    package_names_by_module,
                ),
            )
        },
        BlockExecVariantV2::SplitBlock => {
            unreachable!("split transactions have alredy been removed")
        },
    };

    let replay_protector = next_replay_protector(
        input.orderless,
        sequence_by_address,
        nonce_by_address,
        *sender_acc.address(),
    );
    let payload = apply_replay_protector(payload, replay_protector);
    let raw_tx = transaction_builder(&sender_acc, replay_protector)
        .payload(payload)
        .raw();
    let signed = if let Some(fee_payer_slot) = input.fee_payer_slot {
        let secondary = secondary_signer_accounts(slot_accounts, &input.secondary_slots)?;
        let fee_payer = slot_account(slot_accounts, fee_payer_slot).clone();
        raw_tx
            .sign_fee_payer(
                &sender_acc.privkey,
                secondary.iter().map(|acc| *acc.address()).collect(),
                secondary.iter().map(|acc| &acc.privkey).collect(),
                *fee_payer.address(),
                &fee_payer.privkey,
            )
            .map_err(|_| Corpus::Keep)?
            .into_inner()
    } else if input.secondary_slots.is_empty() {
        raw_tx
            .sign(
                &sender_acc.privkey,
                sender_acc.pubkey.as_ed25519().expect("ed25519 public key"),
            )
            .map_err(|_| Corpus::Keep)?
            .into_inner()
    } else {
        let secondary = secondary_signer_accounts(slot_accounts, &input.secondary_slots)?;
        raw_tx
            .sign_multi_agent(
                &sender_acc.privkey,
                secondary.iter().map(|acc| *acc.address()).collect(),
                secondary.iter().map(|acc| &acc.privkey).collect(),
            )
            .map_err(|_| Corpus::Keep)?
            .into_inner()
    };
    Ok(signed)
}

fn build_publish_transaction(
    vm: &mut BlockFuzzExecutor,
    accounts_by_address: &mut HashMap<AccountAddress, Account>,
    sequence_by_address: &mut HashMap<AccountAddress, u64>,
    package_names_by_module: &BTreeMap<ModuleId, String>,
    modules: &[CompiledModule],
) -> Result<SignedTransaction, Corpus> {
    let sender = publish_modules_sender(modules)?;
    let package_name = package_name_for_modules(modules, package_names_by_module);
    let sequence = next_sequence_number(sequence_by_address, sender);
    let publisher = ensure_account_at(vm, accounts_by_address, sender);
    Ok(
        transaction_builder(publisher, ReplayProtector::SequenceNumber(sequence))
            .payload(publish_transaction_payload_with_package_names(
                modules,
                package_name,
                package_names_by_module,
            ))
            .sign(),
    )
}

fn apply_fuzz_feature_overrides(features: &mut Features) {
    // known false positives with this feature:
    features.disable(FeatureFlag::ENABLE_RESOURCE_ACCESS_CONTROL);
}

fn configure_fuzz_features(vm: &BlockFuzzExecutor) {
    let mut features = vm.state_store().get_features().unwrap_or_default();
    apply_fuzz_feature_overrides(&mut features);
    vm.state_store().set_features(features).unwrap();
}

fn onchain_config() -> BlockExecutorConfigFromOnchain {
    let mut config = BlockExecutorConfigFromOnchain::on_but_large_for_test();

    match &mut config.block_gas_limit_type {
        BlockGasLimitType::ComplexLimitV1 {
            effective_block_gas_limit,
            execution_gas_effective_multiplier,
            io_gas_effective_multiplier,
            conflict_penalty_window,
            use_granular_resource_group_conflicts,
            block_output_limit,
            ..
        } => {
            if let Some(limit) = *FUZZ_EFFECTIVE_BLOCK_GAS_LIMIT {
                *effective_block_gas_limit = limit;
            }
            if let Some(limit) = *FUZZ_BLOCK_OUTPUT_LIMIT {
                *block_output_limit = Some(limit);
            }
            if let Some(window) = *FUZZ_CONFLICT_PENALTY_WINDOW {
                *conflict_penalty_window = window;
            }
            if let Some(multiplier) = *FUZZ_EXECUTION_GAS_EFFECTIVE_MULTIPLIER {
                *execution_gas_effective_multiplier = multiplier;
            }
            if let Some(multiplier) = *FUZZ_IO_GAS_EFFECTIVE_MULTIPLIER {
                *io_gas_effective_multiplier = multiplier;
            }
            if let Some(use_granular) = *FUZZ_USE_GRANULAR_RESOURCE_GROUP_CONFLICTS {
                *use_granular_resource_group_conflicts = use_granular;
            }
        },
        BlockGasLimitType::Limit(existing) => {
            if let Some(limit) = *FUZZ_EFFECTIVE_BLOCK_GAS_LIMIT {
                *existing = limit;
            }
        },
        BlockGasLimitType::NoLimit => {
            if let Some(limit) = *FUZZ_EFFECTIVE_BLOCK_GAS_LIMIT {
                config.block_gas_limit_type = BlockGasLimitType::Limit(limit);
            }
        },
    }

    config
}

fn block_executor_config(
    blockstm_v2: bool,
    parallel: bool,
    block_size: usize,
) -> BlockExecutorConfig {
    BlockExecutorConfig {
        local: BlockExecutorLocalConfig {
            // Selects the Block-STM v2 scheduler for parallel execution.
            blockstm_v2,
            // Scale the parallel run's workers to the block size, clamped to
            // [MIN_CONCURRENCY_LEVEL, APTOS_FUZZ_MAX_CONCURRENCY_LEVEL]: at least the minimum so it always
            // runs multi-threaded (a single worker wouldn't exercise the parallel scheduler or the
            // races we compare against sequential), and at most the maximum so surplus workers
            // don't busy-wait on the fuzzer's small blocks. Sequential execution always uses one
            // worker.
            concurrency_level: if parallel {
                block_size.clamp(MIN_CONCURRENCY_LEVEL, *FUZZ_MAX_CONCURRENCY_LEVEL)
            } else {
                1
            },
            allow_fallback: *FUZZ_ALLOW_FALLBACK,
            discard_failed_blocks: false,
            module_cache_config: FUZZ_MODULE_CACHE_CONFIG.clone(),
            enable_pre_write: true,
        },
        onchain: onchain_config(),
    }
}

fn lock_hot_cache_manager<'a>(
    vm: &BlockFuzzExecutor,
    manager: &'a AptosModuleCacheManager,
    config: &BlockExecutorConfig,
    metadata: TransactionSliceMetadata,
    phase: &str,
) -> AptosModuleCacheManagerGuard<'a> {
    let guard = manager
        .try_lock(
            vm.get_state_view(),
            &config.local.module_cache_config,
            metadata,
        )
        .unwrap_or_else(|error| {
            panic!(
                "module cache manager try_lock failed {phase}; \
             check whether RuntimeEnvironment::struct_name_index_map_size() failed, \
             otherwise this is a fuzzer harness/cache problem: {error:?}"
            );
        });
    if let AptosModuleCacheManagerGuard::None { .. } = &guard {
        panic!(
            "module cache manager returned None guard {phase}; \
             the fuzzer requires the shared hot cache lock and this indicates unexpected \
             concurrent access or a harness bug"
        );
    }
    guard
}

fn snapshot_hot_cache(
    vm: &BlockFuzzExecutor,
    manager: &AptosModuleCacheManager,
    config: &BlockExecutorConfig,
    metadata: TransactionSliceMetadata,
) -> HotCacheSnapshot {
    let guard = lock_hot_cache_manager(vm, manager, config, metadata, "before hot-cache snapshot");
    #[cfg(fuzzing)]
    {
        return guard.snapshot_hot_cache();
    }
    #[cfg(not(fuzzing))]
    {
        drop(guard);
        HotCacheSnapshot
    }
}

fn rollback_hot_cache(
    vm: &BlockFuzzExecutor,
    manager: &AptosModuleCacheManager,
    config: &BlockExecutorConfig,
    metadata: TransactionSliceMetadata,
    snapshot: HotCacheSnapshot,
) {
    let guard = lock_hot_cache_manager(vm, manager, config, metadata, "before hot-cache rollback");
    #[cfg(fuzzing)]
    {
        let mut guard = guard;
        guard.rollback_hot_cache(snapshot);
    }
    #[cfg(not(fuzzing))]
    {
        let _ = snapshot;
        drop(guard);
    }
}

struct BlockExecutionArtifacts {
    outputs: Vec<TransactionOutput>,
    state_hash: HashValue,
}

fn state_store_hash(vm: &BlockFuzzExecutor) -> HashValue {
    let state = vm.state_store().to_btree_map();
    let bytes = bcs::to_bytes(&state).expect("state store must serialize");
    HashValue::sha3_256_of(&bytes)
}

fn execute_block(
    vm: &BlockFuzzExecutor,
    block: Vec<SignedTransaction>,
    manager: &AptosModuleCacheManager,
    metadata: TransactionSliceMetadata,
    config: BlockExecutorConfig,
) -> Vec<TransactionOutput> {
    let txn_block: Vec<Transaction> = block
        .into_iter()
        .map(Transaction::UserTransaction)
        .collect();
    let signature_verified_block = into_signature_verified_block(txn_block);
    let auxiliary_info = (0..signature_verified_block.len() as u32)
        .map(|i| {
            AuxiliaryInfo::new(
                PersistedAuxiliaryInfo::V1 {
                    transaction_index: i,
                },
                None,
            )
        })
        .collect::<Vec<_>>();
    let txn_provider = DefaultTxnProvider::new(signature_verified_block, auxiliary_info);
    let output =
        AptosVMBlockExecutorWrapper::execute_block::<_, NoOpTransactionCommitHook<VMStatus>, _>(
            &txn_provider,
            vm.get_state_view(),
            manager,
            config,
            metadata,
            None,
        )
        .unwrap_or_else(|error| {
            panic!("block execution failed: {error:?}");
        });
    output.into_transaction_outputs_forced()
}

fn execute_blocks_with_cache_rollback(
    vm: &BlockFuzzExecutor,
    blocks: Vec<Vec<SignedTransaction>>,
    manager: &AptosModuleCacheManager,
    next_version: &AtomicU64,
    blockstm_v2: bool,
    parallel: bool,
) -> BlockExecutionArtifacts {
    if blocks.is_empty() {
        return BlockExecutionArtifacts {
            outputs: vec![],
            state_hash: state_store_hash(vm),
        };
    }

    let max_block_size = blocks.iter().map(Vec::len).max().unwrap_or(1);
    tdbg!(
        "exec start",
        blockstm_v2,
        parallel,
        blocks.len(),
        max_block_size
    );
    let snapshot_config = block_executor_config(blockstm_v2, parallel, max_block_size);
    // Each fuzz case starts from a fresh genesis state, so writes produced by these blocks must not
    // stay in the hot module cache. The snapshot keeps prefetched framework modules warm while the
    // rollback removes modules published by the current fuzz case.
    tdbg!("snapshot hot cache", blockstm_v2, parallel);
    let snapshot = snapshot_hot_cache(vm, manager, &snapshot_config, next_metadata(next_version));

    let mut all_outputs = Vec::new();
    for block in blocks {
        tdbg!("execute block", blockstm_v2, parallel, block.len(), &block);
        let config = block_executor_config(blockstm_v2, parallel, block.len());
        let outputs = execute_block(vm, block, manager, next_metadata(next_version), config);
        tdbg!("block outputs", outputs.len());
        vm.apply_transaction_outputs(&outputs);
        all_outputs.extend(outputs);
    }

    tdbg!("rollback hot cache", blockstm_v2, parallel);
    rollback_hot_cache(
        vm,
        manager,
        &snapshot_config,
        next_metadata(next_version),
        snapshot,
    );
    tdbg!("exec end", blockstm_v2, parallel, all_outputs.len());
    BlockExecutionArtifacts {
        outputs: all_outputs,
        state_hash: state_store_hash(vm),
    }
}

/// Compares two executor lanes that must agree and returns the second lane's outputs for status
/// checking. Panics on a divergence — the differential bug this fuzzer hunts for.
fn compare_runs(
    first: BlockExecutionArtifacts,
    first_name: &str,
    second: BlockExecutionArtifacts,
    second_name: &str,
) -> Result<Vec<TransactionOutput>, Corpus> {
    tdbg!(
        "comparing runs",
        first_name,
        first.outputs.len(),
        second_name,
        second.outputs.len()
    );
    assert_outputs_equal(&first.outputs, first_name, &second.outputs, second_name);
    assert_eq!(
        first.state_hash, second.state_hash,
        "{first_name} and {second_name} produced different final state hashes",
    );
    Ok(second.outputs)
}

fn is_invariant_or_unknown(status_code: StatusCode) -> bool {
    matches!(
        status_code.status_type(),
        StatusType::InvariantViolation | StatusType::Unknown
    )
}

fn check_status_code_for_invariant_violation(
    status_code: &StatusCode,
    source: &str,
    output: &TransactionOutput,
) {
    if is_invariant_or_unknown(*status_code)
        // TODO: DOUBLE CHECK THESE AND FIX
        && *status_code != StatusCode::TYPE_RESOLUTION_FAILURE
        && *status_code != StatusCode::STORAGE_ERROR
        && *status_code != StatusCode::VERIFICATION_ERROR
    {
        panic!(
            "invariant violation via {source}: {:?}, {:?}",
            status_code,
            output.auxiliary_data()
        );
    }
}

fn check_output_status(output: &TransactionOutput) -> Result<(), Corpus> {
    match tdbg!(output.status()) {
        TransactionStatus::Keep(ExecutionStatus::Success) => Ok(()),
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(status_code))) => {
            check_status_code_for_invariant_violation(status_code, "ExecutionStatus", output);
            Ok(())
        },
        TransactionStatus::Keep(_) => Ok(()),
        TransactionStatus::Discard(status_code) => {
            check_status_code_for_invariant_violation(status_code, "TransactionStatus", output);
            Ok(())
        },
        TransactionStatus::Retry => Ok(()),
    }
}

/// Records a dependency module under its id, rejecting ambiguous preloads. Different versions of
/// the same module id are allowed in the block itself as publish transactions, but genesis preload
/// can contain only one version for a given id.
fn register_preload_module(
    modules_by_id: &mut BTreeMap<ModuleId, CompiledModule>,
    module: &CompiledModule,
) -> Result<(), Corpus> {
    let module_id = checked_module_self_id(module);
    match modules_by_id.entry(module_id) {
        Entry::Occupied(entry) if entry.get() != module => Err(Corpus::Keep),
        Entry::Occupied(_) => Ok(()),
        Entry::Vacant(entry) => {
            entry.insert(module.clone());
            Ok(())
        },
    }
}

fn package_name_for_modules<'a>(
    modules: &[CompiledModule],
    package_names_by_module: &'a BTreeMap<ModuleId, String>,
) -> &'a str {
    for module in modules {
        let module_id = checked_module_self_id(module);
        if let Some(package_name) = package_names_by_module.get(&module_id) {
            return package_name.as_str();
        }
    }

    panic!("package name must exist for published module group")
}

fn build_package_names_by_module(packages: &[Vec<CompiledModule>]) -> BTreeMap<ModuleId, String> {
    let mut package_names_by_module = BTreeMap::new();
    for (package_idx, modules) in packages.iter().enumerate() {
        let module_ids = modules
            .iter()
            .map(checked_module_self_id)
            .collect::<Vec<_>>();
        let package_name = module_ids
            .iter()
            .find_map(|module_id| package_names_by_module.get(module_id))
            .cloned()
            .unwrap_or_else(|| format!("package_{package_idx}"));

        for module_id in module_ids {
            package_names_by_module
                .entry(module_id)
                .or_insert_with(|| package_name.clone());
        }
    }

    package_names_by_module
}

fn build_signed_transaction_blocks(
    vm: &mut BlockFuzzExecutor,
    modules: &[CompiledModule],
    account_inputs: &[UserAccount],
    dependency_publish_packages: &[Vec<CompiledModule>],
    package_names_by_module: &BTreeMap<ModuleId, String>,
    transaction_blocks: &[Vec<RunnableBlockTransactionV2>],
) -> Result<Vec<Vec<SignedTransaction>>, Corpus> {
    if transaction_blocks.is_empty() {
        return Err(Corpus::Keep);
    }

    let slot_accounts = account_inputs
        .iter()
        .copied()
        .map(|account| create_user_account(vm, account))
        .collect::<Vec<_>>();
    let mut accounts_by_address = slot_accounts
        .iter()
        .cloned()
        .map(|account| (*account.address(), account))
        .collect::<HashMap<_, _>>();
    let mut sequence_by_address = HashMap::new();
    let mut nonce_by_address = HashMap::new();
    let mut signed_blocks = Vec::with_capacity(transaction_blocks.len());

    if !dependency_publish_packages.is_empty() {
        let mut block = Vec::with_capacity(dependency_publish_packages.len());
        for group in dependency_publish_packages {
            block.push(build_publish_transaction(
                vm,
                &mut accounts_by_address,
                &mut sequence_by_address,
                package_names_by_module,
                group,
            )?);
        }
        signed_blocks.push(block);
    }

    for transaction_block in transaction_blocks {
        let mut block = Vec::with_capacity(transaction_block.len());

        for tx in transaction_block {
            block.push(build_signed_transaction(
                vm,
                modules,
                &slot_accounts,
                &mut accounts_by_address,
                &mut sequence_by_address,
                &mut nonce_by_address,
                package_names_by_module,
                tx,
            )?);
        }

        if block.is_empty() || block.len() > MAX_BLOCK_TXNS {
            return Err(Corpus::Keep);
        }
        signed_blocks.push(block);
    }

    Ok(signed_blocks)
}

fn run_case(mut input: RunnableBlockStateV2) -> Result<(), Corpus> {
    tdbg!(&input);

    tdbg!("filtering fuzz case");
    if input.accounts.is_empty()
        || input.accounts.len() > MAX_BLOCK_ACCOUNTS
        || input.modules.len() > MAX_BLOCK_MODULES
        || input.transactions.is_empty()
        || input.transactions.len() > MAX_BLOCK_TXNS
    {
        return Err(Corpus::Keep);
    }

    if has_invalid_split_blocks(&input.transactions) {
        return Err(Corpus::Keep);
    }

    // fail fast
    tdbg!("checking module ids and transaction shape");
    filter_bad_modules(&mut input.modules)?;
    for tx in &input.transactions {
        filter_bad_tx(&tx.exec_variant)?;
    }

    tdbg!("collecting in-block published modules");
    let mut in_block_published_modules = Vec::new();
    for tx in &input.transactions {
        match &tx.exec_variant {
            BlockExecVariantV2::Script { .. } => {},
            BlockExecVariantV2::Publish { _module_idxs } => {
                let _modules = resolve_module_refs(&input.modules, _module_idxs)?;
                for module in _modules {
                    if !in_block_published_modules.contains(&module) {
                        in_block_published_modules.push(module);
                    }
                }
            },
            BlockExecVariantV2::CallFunction { .. } => (),
            BlockExecVariantV2::SplitBlock => (),
        }
    }

    tdbg!("building preload module set");
    let mut preload_modules_by_id = BTreeMap::new();
    for module in &input.modules {
        if !in_block_published_modules.contains(module) {
            register_preload_module(&mut preload_modules_by_id, module)?;
        }
    }

    let timed_features = TimedFeaturesBuilder::enable_all().build();
    let mut features = Features::default();
    apply_fuzz_feature_overrides(&mut features);
    let mut verifier_config = prod_configs::aptos_prod_verifier_config(
        LATEST_GAS_FEATURE_VERSION,
        &features,
        &timed_features,
    );
    apply_verifier_config_overrides(&mut verifier_config);
    let deserializer_config =
        DeserializerConfig::new(VERSION_MAX, FUZZ_MAX_IDENTIFIER_SIZE.unwrap_or(255));

    // consider maybe allowing verifier failures??

    tdbg!("verifying scripts");
    for tx in &input.transactions {
        match &tx.exec_variant {
            BlockExecVariantV2::Script { _script, .. } => {
                verify_script_fast(_script, &verifier_config, &deserializer_config)?;
            },
            BlockExecVariantV2::Publish { _module_idxs } => {},
            BlockExecVariantV2::CallFunction { .. } => (),
            BlockExecVariantV2::SplitBlock => (),
        }
    }

    tdbg!("verifying modules");
    for module in &input.modules {
        verify_module_fast(module, &verifier_config, &deserializer_config)?;
    }

    tdbg!("configuring VM checks");
    prod_configs::set_async_runtime_checks(*FUZZ_ASYNC_RUNTIME_CHECKS);
    prod_configs::set_paranoid_ref_checks(*FUZZ_PARANOID_REF_CHECKS);
    Lazy::force(&CONFIGURE_FUZZ_VM_CONFIG);
    prod_configs::set_layout_caches(*FUZZ_ENABLE_LAYOUT_CACHES);

    tdbg!("topologically ordering and grouping preload modules");
    let dependency_publish_packages =
        group_modules_by_address_topo(preload_modules_by_id.values().cloned().collect())?;

    tdbg!("building package names");
    let mut publish_packages = dependency_publish_packages.clone();
    for tx in &input.transactions {
        if let BlockExecVariantV2::Publish { _module_idxs } = &tx.exec_variant {
            publish_packages.push(resolve_module_refs(&input.modules, _module_idxs)?);
        }
    }
    let package_names_by_module = build_package_names_by_module(&publish_packages);
    tdbg!("building transaction blocks");
    let transaction_blocks = input
        .transactions
        .as_slice()
        .split(is_split_block)
        .map(<[RunnableBlockTransactionV2]>::to_vec)
        .collect::<Vec<_>>();
    let transaction_block_sizes = transaction_blocks.iter().map(Vec::len).collect::<Vec<_>>();
    tdbg!("transaction block sizes", &transaction_block_sizes);
    tdbg!("transaction blocks", &transaction_blocks);

    // Every lane shares the same warm manager/version counter. Each call snapshots the
    // framework-only hot cache, executes all split blocks in that lane, and rolls back the fuzz
    // case's published modules. Each lane signs transactions from a fresh genesis state, so
    // sequential/v1/v2 comparisons do not share account or committed transaction state.
    let run = |blockstm_v2: bool, parallel: bool| -> Result<BlockExecutionArtifacts, Corpus> {
        tdbg!("building signed tx blocks", blockstm_v2, parallel);
        let mut vm = BlockFuzzExecutor::from_genesis(&VM_WRITE_SET, ChainId::mainnet());
        configure_fuzz_features(&vm);
        let blocks = build_signed_transaction_blocks(
            &mut vm,
            &input.modules,
            &input.accounts,
            &dependency_publish_packages,
            &package_names_by_module,
            &transaction_blocks,
        )?;
        let block_sizes = blocks.iter().map(Vec::len).collect::<Vec<_>>();
        tdbg!("signed block sizes", blockstm_v2, parallel, &block_sizes);
        tdbg!("signed blocks", blockstm_v2, parallel, &blocks);
        tdbg!("executing signed tx blocks", blockstm_v2, parallel);
        Ok(execute_blocks_with_cache_rollback(
            &vm,
            blocks,
            &MODULE_CACHE_MANAGER,
            &NEXT_VERSION,
            blockstm_v2,
            parallel,
        ))
    };

    let checked_outputs = match *EXEC_MODE {
        ExecMode::V2AndSeq => compare_runs(
            run(false, false)?,
            "sequential",
            run(true, true)?,
            "blockstm_v2",
        )?,
        ExecMode::Seq => run(false, false)?.outputs,
        ExecMode::V2 => run(true, true)?.outputs,
    };

    tdbg!("checking output statuses", checked_outputs.len());
    for output in checked_outputs {
        check_output_status(&output)?;
    }

    Ok(())
}

fuzz_target!(
    init: {
        require_fuzzing_cfg();
        Lazy::force(&VM_WRITE_SET);
        Lazy::force(&MODULE_CACHE_MANAGER);
        Lazy::force(&EXEC_MODE);
        Lazy::force(&FUZZ_GAS_UNIT_PRICE);
        Lazy::force(&FUZZ_MAX_GAS_AMOUNT);
        Lazy::force(&FUZZ_EFFECTIVE_BLOCK_GAS_LIMIT);
        Lazy::force(&FUZZ_BLOCK_OUTPUT_LIMIT);
        Lazy::force(&FUZZ_CONFLICT_PENALTY_WINDOW);
        Lazy::force(&FUZZ_EXECUTION_GAS_EFFECTIVE_MULTIPLIER);
        Lazy::force(&FUZZ_IO_GAS_EFFECTIVE_MULTIPLIER);
        Lazy::force(&FUZZ_USE_GRANULAR_RESOURCE_GROUP_CONFLICTS);
        Lazy::force(&FUZZ_MAX_IDENTIFIER_SIZE);
        Lazy::force(&FUZZ_MAX_CONCURRENCY_LEVEL);
        Lazy::force(&FUZZ_ALLOW_FALLBACK);
        Lazy::force(&FUZZ_ASYNC_RUNTIME_CHECKS);
        Lazy::force(&FUZZ_PARANOID_REF_CHECKS);
        Lazy::force(&FUZZ_ENABLE_LAYOUT_CACHES);
        Lazy::force(&FUZZ_MODULE_CACHE_CONFIG);
    },
    |fuzz_data: RunnableBlockStateV2| -> Corpus {
        run_case(fuzz_data).err().unwrap_or(Corpus::Keep)
    }
);

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{framework_usage_html, replay_on_archive};
use anyhow::{Context, Result};
use aptos_crypto::HashValue;
use aptos_types::transaction::{ExecutionStatus, TransactionStatus, Version};
use aptos_vm::function_usage::{
    FunctionId, FunctionUsageSink, TransactionFunctionUsage, UsageCallKind,
};
use clap::Parser;
use move_binary_format::{access::ModuleAccess, file_format::Visibility};
use move_core_types::{account_address::AccountAddress, language_storage::ModuleId};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};
use tempfile::NamedTempFile;

const SCHEMA_VERSION: u64 = 5;

// Caller and root-function identities originate in replayed transactions. Keep the detailed
// call-path map bounded so a stream of uniquely addressed wrapper modules cannot exhaust a
// replay worker. Per-function totals are collected independently and remain complete.
const MAX_USAGE_DETAIL_ROWS: usize = 100_000;

// Preserve the originating transaction entry function independently for every framework
// function. A per-callee limit prevents a popular function with many distinct wrappers from
// consuming the attribution budget needed by rare functions.
const MAX_ROOT_FUNCTION_ROWS_PER_FUNCTION: usize = 32;

// Root entry functions are also transaction-controlled identities. Keep a separate, smaller
// budget for the active-caller summary so it cannot reintroduce the unbounded growth avoided
// above.
const MAX_ACTIVE_ENTRY_FUNCTION_CALLER_ROWS: usize = 25_000;

#[derive(Parser)]
#[group(id = "FrameworkUsageOpt")]
pub struct Opt {
    #[clap(flatten)]
    replay: replay_on_archive::Opt,

    #[clap(long, value_parser, help = "Path to the framework usage JSON report")]
    output: PathBuf,

    #[clap(
        long,
        value_parser,
        help = "Optional path for a self-contained HTML deprecation analysis report"
    )]
    html_output: Option<PathBuf>,
}

impl Opt {
    pub async fn run(self) -> Result<()> {
        if let Some(html_output) = &self.html_output {
            anyhow::ensure!(
                html_output != &self.output,
                "--output and --html-output must be different paths"
            );
        }
        self.replay
            .run_with_function_usage(self.output, self.html_output)
            .await
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
enum TransactionOutcome {
    Success,
    Abort,
    Discard,
    Retry,
}

impl From<&TransactionStatus> for TransactionOutcome {
    fn from(status: &TransactionStatus) -> Self {
        match status {
            TransactionStatus::Keep(ExecutionStatus::Success) => Self::Success,
            TransactionStatus::Keep(_) => Self::Abort,
            TransactionStatus::Discard(_) => Self::Discard,
            TransactionStatus::Retry => Self::Retry,
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct UsageKey {
    callee: FunctionId,
    caller: Option<FunctionId>,
    root_function: Option<FunctionId>,
    call_kind: UsageCallKind,
    outcome: TransactionOutcome,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct FunctionUsageKey {
    callee: FunctionId,
    outcome: TransactionOutcome,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct RootFunctionUsageKey {
    root_function: Option<FunctionId>,
    outcome: TransactionOutcome,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct ActiveEntryFunctionCallerKey {
    address: AccountAddress,
    entry_function: FunctionId,
    outcome: TransactionOutcome,
}

#[derive(Clone, Debug, Default, Serialize)]
struct UsageCounts {
    invocation_count: u64,
    transaction_count: u64,
    first_version: Version,
    last_version: Version,
}

#[derive(Clone, Debug, Default, Serialize)]
struct ActiveEntryFunctionCallerCounts {
    framework_invocation_count: u64,
    transaction_count: u64,
    first_version: Version,
    last_version: Version,
}

#[derive(Clone, Debug, Serialize)]
struct UsageRow {
    #[serde(flatten)]
    key: UsageKey,
    #[serde(flatten)]
    counts: UsageCounts,
}

#[derive(Clone, Debug, Serialize)]
struct FunctionInventoryRow {
    module_id: ModuleId,
    function_name: String,
    visibility: String,
    is_entry: bool,
    is_native: bool,
    type_parameter_count: usize,
}

#[derive(Serialize)]
struct FrameworkUsageReport {
    schema_version: u64,
    start_version: Version,
    end_version: Version,
    start_timestamp_usecs: u64,
    end_timestamp_usecs: u64,
    git_sha: String,
    target_modules: Vec<ModuleId>,
    processed_transaction_count: u64,
    transaction_usage_records: u64,
    usage_detail_row_limit: usize,
    usage_detail_truncated: bool,
    dropped_usage_invocation_count: u64,
    dropped_usage_transaction_count: u64,
    root_function_row_limit_per_function: usize,
    root_function_usage_truncated: bool,
    dropped_root_function_usage_invocation_count: u64,
    dropped_root_function_usage_transaction_count: u64,
    active_entry_function_caller_row_limit: usize,
    active_entry_function_callers_truncated: bool,
    dropped_active_entry_function_framework_invocation_count: u64,
    dropped_active_entry_function_transaction_count: u64,
    functions: Vec<FunctionInventoryRow>,
    function_usage: Vec<FunctionUsageRow>,
    root_function_usage: Vec<RootFunctionUsageRow>,
    usage: Vec<UsageRow>,
    active_entry_function_callers: Vec<ActiveEntryFunctionCallerRow>,
}

#[derive(Clone, Debug, Serialize)]
struct FunctionUsageRow {
    #[serde(flatten)]
    key: FunctionUsageKey,
    #[serde(flatten)]
    counts: UsageCounts,
}

#[derive(Clone, Debug, Serialize)]
struct RootFunctionUsageRow {
    callee: FunctionId,
    #[serde(flatten)]
    key: RootFunctionUsageKey,
    #[serde(flatten)]
    counts: UsageCounts,
}

#[derive(Clone, Debug, Serialize)]
struct ActiveEntryFunctionCallerRow {
    #[serde(flatten)]
    key: ActiveEntryFunctionCallerKey,
    #[serde(flatten)]
    counts: ActiveEntryFunctionCallerCounts,
}

#[derive(Default)]
struct CollectorState {
    pending: HashMap<HashValue, TransactionFunctionUsage>,
    function_usage: BTreeMap<FunctionUsageKey, UsageCounts>,
    root_function_usage: BTreeMap<FunctionId, BTreeMap<RootFunctionUsageKey, UsageCounts>>,
    usage: BTreeMap<UsageKey, UsageCounts>,
    active_entry_function_callers:
        BTreeMap<ActiveEntryFunctionCallerKey, ActiveEntryFunctionCallerCounts>,
    transaction_usage_records: u64,
    dropped_usage_invocation_count: u64,
    dropped_usage_transaction_count: u64,
    dropped_root_function_usage_invocation_count: u64,
    dropped_root_function_usage_transaction_count: u64,
    dropped_active_entry_function_framework_invocation_count: u64,
    dropped_active_entry_function_transaction_count: u64,
    ledger_timestamps: Option<(u64, u64)>,
}

pub(crate) struct FrameworkUsageCollector {
    start_version: Version,
    end_version: Version,
    target_modules: BTreeSet<ModuleId>,
    inventory: Vec<FunctionInventoryRow>,
    usage_detail_row_limit: usize,
    root_function_row_limit_per_function: usize,
    active_entry_function_caller_row_limit: usize,
    state: Mutex<CollectorState>,
}

impl FrameworkUsageCollector {
    pub(crate) fn new(start_version: Version, end_version: Version) -> Self {
        Self::new_with_usage_detail_row_limit(start_version, end_version, MAX_USAGE_DETAIL_ROWS)
    }

    fn new_with_usage_detail_row_limit(
        start_version: Version,
        end_version: Version,
        usage_detail_row_limit: usize,
    ) -> Self {
        let mut target_modules = BTreeSet::new();
        let mut inventory = vec![];

        for module in aptos_cached_packages::head_release_bundle().compiled_modules() {
            let module_id = module.self_id();
            target_modules.insert(module_id.clone());
            for definition in module.function_defs() {
                let handle = module.function_handle_at(definition.function);
                inventory.push(FunctionInventoryRow {
                    module_id: module_id.clone(),
                    function_name: module.identifier_at(handle.name).to_string(),
                    visibility: visibility_name(definition.visibility).to_owned(),
                    is_entry: definition.is_entry,
                    is_native: definition.code.is_none(),
                    type_parameter_count: handle.type_parameters.len(),
                });
            }
        }
        inventory.sort_by(|left, right| {
            (&left.module_id, &left.function_name).cmp(&(&right.module_id, &right.function_name))
        });
        inventory.dedup_by(|left, right| {
            left.module_id == right.module_id && left.function_name == right.function_name
        });

        Self {
            start_version,
            end_version,
            target_modules,
            inventory,
            usage_detail_row_limit,
            root_function_row_limit_per_function: MAX_ROOT_FUNCTION_ROWS_PER_FUNCTION,
            active_entry_function_caller_row_limit: MAX_ACTIVE_ENTRY_FUNCTION_CALLER_ROWS,
            state: Mutex::new(CollectorState::default()),
        }
    }

    fn lock_state(&self) -> MutexGuard<'_, CollectorState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(crate) fn assign_version(
        &self,
        transaction_hash: HashValue,
        version: Version,
    ) -> Result<()> {
        let mut state = self.lock_state();
        let Some(transaction) = state.pending.remove(&transaction_hash) else {
            // Transactions which fail before entering a Move payload legitimately produce no
            // usage record.
            return Ok(());
        };

        state.transaction_usage_records = state
            .transaction_usage_records
            .checked_add(1)
            .context("transaction usage record count overflow")?;

        let outcome = TransactionOutcome::from(&transaction.status);
        let root_function = transaction.root_function.clone();
        let framework_invocation_count = u64::try_from(transaction.calls.len())
            .context("framework invocation count does not fit in u64")?;
        let entry_function_with_module = root_function
            .as_ref()
            .filter(|_| framework_invocation_count > 0)
            .and_then(|entry_function| {
                entry_function
                    .module_id
                    .as_ref()
                    .map(|module_id| (entry_function, module_id))
            });
        if let Some((entry_function, module_id)) = entry_function_with_module {
            let key = ActiveEntryFunctionCallerKey {
                address: *module_id.address(),
                entry_function: entry_function.clone(),
                outcome: outcome.clone(),
            };
            if state.active_entry_function_callers.contains_key(&key)
                || state.active_entry_function_callers.len()
                    < self.active_entry_function_caller_row_limit
            {
                merge_active_entry_function_caller_counts(
                    &mut state.active_entry_function_callers,
                    key,
                    framework_invocation_count,
                    version,
                )?;
            } else {
                state.dropped_active_entry_function_framework_invocation_count = state
                    .dropped_active_entry_function_framework_invocation_count
                    .checked_add(framework_invocation_count)
                    .context("dropped active entry function framework invocation count overflow")?;
                state.dropped_active_entry_function_transaction_count = state
                    .dropped_active_entry_function_transaction_count
                    .checked_add(1)
                    .context("dropped active entry function transaction count overflow")?;
            }
        }
        let mut per_transaction = BTreeMap::<UsageKey, u64>::new();
        let mut per_function = BTreeMap::<FunctionUsageKey, u64>::new();
        let mut per_root_function =
            BTreeMap::<FunctionId, BTreeMap<RootFunctionUsageKey, u64>>::new();
        let mut usage_detail_truncated = false;
        let mut root_function_usage_truncated = false;
        for call in transaction.calls {
            let function_key = FunctionUsageKey {
                callee: call.callee.clone(),
                outcome: outcome.clone(),
            };
            let function_count = per_function.entry(function_key).or_default();
            *function_count = function_count
                .checked_add(1)
                .context("per-transaction function invocation count overflow")?;

            let root_key = RootFunctionUsageKey {
                root_function: root_function.clone(),
                outcome: outcome.clone(),
            };
            let retained_roots = state.root_function_usage.get(&call.callee);
            let pending_roots = per_root_function.entry(call.callee.clone()).or_default();
            if retained_roots.is_some_and(|roots| roots.contains_key(&root_key))
                || pending_roots.contains_key(&root_key)
                || retained_roots.map_or(0, BTreeMap::len) + pending_roots.len()
                    < self.root_function_row_limit_per_function
            {
                let count = pending_roots.entry(root_key).or_default();
                *count = count
                    .checked_add(1)
                    .context("per-transaction root function invocation count overflow")?;
            } else {
                state.dropped_root_function_usage_invocation_count = state
                    .dropped_root_function_usage_invocation_count
                    .checked_add(1)
                    .context("dropped root function usage invocation count overflow")?;
                root_function_usage_truncated = true;
            }

            let key = UsageKey {
                callee: call.callee,
                caller: call.caller,
                root_function: root_function.clone(),
                call_kind: call.kind,
                outcome: outcome.clone(),
            };
            if state.usage.contains_key(&key)
                || per_transaction.contains_key(&key)
                || state.usage.len().saturating_add(per_transaction.len())
                    < self.usage_detail_row_limit
            {
                let count = per_transaction.entry(key).or_default();
                *count = count
                    .checked_add(1)
                    .context("per-transaction invocation count overflow")?;
            } else {
                state.dropped_usage_invocation_count = state
                    .dropped_usage_invocation_count
                    .checked_add(1)
                    .context("dropped usage invocation count overflow")?;
                usage_detail_truncated = true;
            }
        }

        if usage_detail_truncated {
            state.dropped_usage_transaction_count = state
                .dropped_usage_transaction_count
                .checked_add(1)
                .context("dropped usage transaction count overflow")?;
        }
        if root_function_usage_truncated {
            state.dropped_root_function_usage_transaction_count = state
                .dropped_root_function_usage_transaction_count
                .checked_add(1)
                .context("dropped root function usage transaction count overflow")?;
        }

        merge_usage_counts(&mut state.function_usage, per_function, version)?;
        merge_nested_usage_counts(&mut state.root_function_usage, per_root_function, version)?;
        merge_usage_counts(&mut state.usage, per_transaction, version)?;
        Ok(())
    }

    pub(crate) fn set_ledger_timestamps(
        &self,
        start_timestamp_usecs: u64,
        end_timestamp_usecs: u64,
    ) -> Result<()> {
        anyhow::ensure!(
            start_timestamp_usecs <= end_timestamp_usecs,
            "framework usage ledger timestamps are out of order"
        );
        let mut state = self.lock_state();
        anyhow::ensure!(
            state.ledger_timestamps.is_none(),
            "framework usage ledger timestamps were already assigned"
        );
        state.ledger_timestamps = Some((start_timestamp_usecs, end_timestamp_usecs));
        Ok(())
    }

    pub(crate) fn write_report(&self, output: &Path, html_output: Option<&Path>) -> Result<()> {
        let state = self.lock_state();
        anyhow::ensure!(
            state.pending.is_empty(),
            "{} transaction usage records were not assigned a ledger version",
            state.pending.len()
        );
        let processed_transaction_count = self
            .end_version
            .checked_sub(self.start_version)
            .and_then(|count| count.checked_add(1))
            .context("invalid or overflowing report version range")?;
        let (start_timestamp_usecs, end_timestamp_usecs) = state
            .ledger_timestamps
            .context("framework usage ledger timestamps were not assigned")?;
        let report = FrameworkUsageReport {
            schema_version: SCHEMA_VERSION,
            start_version: self.start_version,
            end_version: self.end_version,
            start_timestamp_usecs,
            end_timestamp_usecs,
            git_sha: aptos_build_info::get_git_hash(),
            target_modules: self.target_modules.iter().cloned().collect(),
            processed_transaction_count,
            transaction_usage_records: state.transaction_usage_records,
            usage_detail_row_limit: self.usage_detail_row_limit,
            usage_detail_truncated: state.dropped_usage_invocation_count > 0,
            dropped_usage_invocation_count: state.dropped_usage_invocation_count,
            dropped_usage_transaction_count: state.dropped_usage_transaction_count,
            root_function_row_limit_per_function: self.root_function_row_limit_per_function,
            root_function_usage_truncated: state.dropped_root_function_usage_invocation_count > 0,
            dropped_root_function_usage_invocation_count: state
                .dropped_root_function_usage_invocation_count,
            dropped_root_function_usage_transaction_count: state
                .dropped_root_function_usage_transaction_count,
            active_entry_function_caller_row_limit: self.active_entry_function_caller_row_limit,
            active_entry_function_callers_truncated: state
                .dropped_active_entry_function_transaction_count
                > 0,
            dropped_active_entry_function_framework_invocation_count: state
                .dropped_active_entry_function_framework_invocation_count,
            dropped_active_entry_function_transaction_count: state
                .dropped_active_entry_function_transaction_count,
            functions: self.inventory.clone(),
            function_usage: state
                .function_usage
                .iter()
                .map(|(key, counts)| FunctionUsageRow {
                    key: key.clone(),
                    counts: counts.clone(),
                })
                .collect(),
            root_function_usage: state
                .root_function_usage
                .iter()
                .flat_map(|(callee, roots)| {
                    roots.iter().map(|(key, counts)| RootFunctionUsageRow {
                        callee: callee.clone(),
                        key: key.clone(),
                        counts: counts.clone(),
                    })
                })
                .collect(),
            usage: state
                .usage
                .iter()
                .map(|(key, counts)| UsageRow {
                    key: key.clone(),
                    counts: counts.clone(),
                })
                .collect(),
            active_entry_function_callers: state
                .active_entry_function_callers
                .iter()
                .map(|(key, counts)| ActiveEntryFunctionCallerRow {
                    key: key.clone(),
                    counts: counts.clone(),
                })
                .collect(),
        };

        write_json_report(output, &report)?;
        if let Some(html_output) = html_output {
            let report_json = serde_json::to_string(&report)
                .context("serializing framework usage data for HTML report")?;
            framework_usage_html::write(html_output, &report_json)?;
        }
        Ok(())
    }
}

fn write_json_report(output: &Path, report: &FrameworkUsageReport) -> Result<()> {
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    anyhow::ensure!(
        parent.exists(),
        "output directory {:?} does not exist",
        parent
    );
    let mut writer = BufWriter::new(
        NamedTempFile::new_in(parent)
            .with_context(|| format!("creating temporary report in {:?}", parent))?,
    );
    serde_json::to_writer_pretty(&mut writer, report)
        .context("serializing framework usage report")?;
    writer
        .flush()
        .context("flushing temporary framework usage report")?;
    let file = writer
        .into_inner()
        .map_err(|error| error.into_error())
        .context("closing temporary framework usage report")?;
    file.as_file()
        .sync_all()
        .context("syncing temporary framework usage report")?;
    file.persist(output)
        .map_err(|error| error.error)
        .with_context(|| format!("renaming temporary report to {:?}", output))?;
    Ok(())
}

fn merge_usage_counts<K: Ord>(
    aggregate: &mut BTreeMap<K, UsageCounts>,
    transaction: BTreeMap<K, u64>,
    version: Version,
) -> Result<()> {
    for (key, invocation_count) in transaction {
        let counts = aggregate.entry(key).or_insert(UsageCounts {
            invocation_count: 0,
            transaction_count: 0,
            first_version: version,
            last_version: version,
        });
        counts.invocation_count = counts
            .invocation_count
            .checked_add(invocation_count)
            .context("invocation count overflow")?;
        counts.transaction_count = counts
            .transaction_count
            .checked_add(1)
            .context("transaction count overflow")?;
        counts.first_version = counts.first_version.min(version);
        counts.last_version = counts.last_version.max(version);
    }
    Ok(())
}

fn merge_nested_usage_counts<OuterKey: Ord, InnerKey: Ord>(
    aggregate: &mut BTreeMap<OuterKey, BTreeMap<InnerKey, UsageCounts>>,
    transaction: BTreeMap<OuterKey, BTreeMap<InnerKey, u64>>,
    version: Version,
) -> Result<()> {
    for (outer_key, counts) in transaction {
        merge_usage_counts(aggregate.entry(outer_key).or_default(), counts, version)?;
    }
    Ok(())
}

fn merge_active_entry_function_caller_counts(
    aggregate: &mut BTreeMap<ActiveEntryFunctionCallerKey, ActiveEntryFunctionCallerCounts>,
    key: ActiveEntryFunctionCallerKey,
    framework_invocation_count: u64,
    version: Version,
) -> Result<()> {
    let counts = aggregate
        .entry(key)
        .or_insert(ActiveEntryFunctionCallerCounts {
            framework_invocation_count: 0,
            transaction_count: 0,
            first_version: version,
            last_version: version,
        });
    counts.framework_invocation_count = counts
        .framework_invocation_count
        .checked_add(framework_invocation_count)
        .context("active entry function framework invocation count overflow")?;
    counts.transaction_count = counts
        .transaction_count
        .checked_add(1)
        .context("active entry function transaction count overflow")?;
    counts.first_version = counts.first_version.min(version);
    counts.last_version = counts.last_version.max(version);
    Ok(())
}

impl FunctionUsageSink for FrameworkUsageCollector {
    fn is_target_module(&self, module_id: &ModuleId) -> bool {
        self.target_modules.contains(module_id)
    }

    fn record_transaction(&self, usage: TransactionFunctionUsage) {
        self.lock_state()
            .pending
            .insert(usage.transaction_hash, usage);
    }
}

fn visibility_name(visibility: Visibility) -> &'static str {
    match visibility {
        Visibility::Private => "private",
        Visibility::Public => "public",
        Visibility::Friend => "friend",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_types::transaction::TransactionStatus;
    use move_core_types::account_address::AccountAddress;
    use std::fs::File;

    #[test]
    fn aggregates_invocations_and_distinct_transactions() {
        let collector = FrameworkUsageCollector::new(10, 20);
        let module_id = collector.target_modules.iter().next().unwrap().clone();
        let callee = FunctionId {
            module_id: Some(module_id.clone()),
            function_name: "target".to_owned(),
        };
        let root_function = FunctionId {
            module_id: Some(module_id),
            function_name: "entry".to_owned(),
        };
        let call = aptos_vm::function_usage::FunctionCall {
            caller: None,
            callee: callee.clone(),
            kind: UsageCallKind::Call,
        };
        collector.record_transaction(TransactionFunctionUsage {
            transaction_hash: HashValue::zero(),
            sender: AccountAddress::ONE,
            multisig_address: None,
            root_function: Some(root_function),
            status: TransactionStatus::Keep(ExecutionStatus::Success),
            calls: vec![call.clone(), call],
        });
        collector.assign_version(HashValue::zero(), 12).unwrap();
        collector.set_ledger_timestamps(1_000, 2_000).unwrap();

        let state = collector.lock_state();
        let function_counts = state.function_usage.values().next().unwrap();
        assert_eq!(function_counts.invocation_count, 2);
        assert_eq!(function_counts.transaction_count, 1);
        assert_eq!(function_counts.first_version, 12);
        assert_eq!(function_counts.last_version, 12);
        let call_path_counts = state.usage.values().next().unwrap();
        assert_eq!(call_path_counts.invocation_count, 2);
        assert_eq!(call_path_counts.transaction_count, 1);
        let root_function_counts = state
            .root_function_usage
            .values()
            .next()
            .unwrap()
            .values()
            .next()
            .unwrap();
        assert_eq!(root_function_counts.invocation_count, 2);
        assert_eq!(root_function_counts.transaction_count, 1);
        let active_entry_function_counts =
            state.active_entry_function_callers.values().next().unwrap();
        assert_eq!(active_entry_function_counts.framework_invocation_count, 2);
        assert_eq!(active_entry_function_counts.transaction_count, 1);
        drop(state);

        let output_dir = aptos_temppath::TempPath::new();
        output_dir.create_as_dir().unwrap();
        let output = output_dir.path().join("usage.json");
        let html_output = output_dir.path().join("usage.html");
        collector.write_report(&output, Some(&html_output)).unwrap();
        let report: serde_json::Value =
            serde_json::from_reader(File::open(output).unwrap()).unwrap();
        assert_eq!(report["schema_version"], SCHEMA_VERSION);
        assert_eq!(report["processed_transaction_count"], 11);
        assert_eq!(report["start_timestamp_usecs"], 1_000);
        assert_eq!(report["end_timestamp_usecs"], 2_000);
        assert_eq!(report["usage_detail_row_limit"], MAX_USAGE_DETAIL_ROWS);
        assert_eq!(report["usage_detail_truncated"], false);
        assert_eq!(report["dropped_usage_invocation_count"], 0);
        assert_eq!(report["dropped_usage_transaction_count"], 0);
        assert_eq!(
            report["root_function_row_limit_per_function"],
            MAX_ROOT_FUNCTION_ROWS_PER_FUNCTION
        );
        assert_eq!(report["root_function_usage_truncated"], false);
        assert_eq!(
            report["root_function_usage"][0]["root_function"]["function_name"],
            "entry"
        );
        assert_eq!(report["root_function_usage"][0]["invocation_count"], 2);
        assert_eq!(
            report["active_entry_function_caller_row_limit"],
            MAX_ACTIVE_ENTRY_FUNCTION_CALLER_ROWS
        );
        assert_eq!(report["active_entry_function_callers_truncated"], false);
        assert_eq!(
            report["active_entry_function_callers"][0]["entry_function"]["function_name"],
            "entry"
        );
        assert_eq!(
            report["active_entry_function_callers"][0]["framework_invocation_count"],
            2
        );
        assert_eq!(report["function_usage"][0]["invocation_count"], 2);
        assert_eq!(report["usage"][0]["transaction_count"], 1);
        let html = std::fs::read_to_string(html_output).unwrap();
        assert!(html.contains("Framework deprecation evidence"));
        assert!(html.contains("target"));
        assert!(html.contains("report-data"));
    }

    #[test]
    fn caps_untrusted_call_path_details_without_losing_function_totals() {
        let collector = FrameworkUsageCollector::new_with_usage_detail_row_limit(10, 20, 1);
        let module_id = collector.target_modules.iter().next().unwrap().clone();
        let callee = FunctionId {
            module_id: Some(module_id),
            function_name: "target".to_owned(),
        };
        let make_call = |caller_name: &str| aptos_vm::function_usage::FunctionCall {
            caller: Some(FunctionId {
                module_id: None,
                function_name: caller_name.to_owned(),
            }),
            callee: callee.clone(),
            kind: UsageCallKind::Call,
        };
        let record = |transaction_hash, call| {
            collector.record_transaction(TransactionFunctionUsage {
                transaction_hash,
                sender: AccountAddress::ONE,
                multisig_address: None,
                root_function: None,
                status: TransactionStatus::Keep(ExecutionStatus::Success),
                calls: vec![call],
            });
        };

        record(HashValue::zero(), make_call("first"));
        collector.assign_version(HashValue::zero(), 10).unwrap();
        record(HashValue::from_u64(1), make_call("second"));
        collector
            .assign_version(HashValue::from_u64(1), 11)
            .unwrap();
        record(HashValue::zero(), make_call("first"));
        collector.assign_version(HashValue::zero(), 12).unwrap();

        let state = collector.lock_state();
        let function_counts = state.function_usage.values().next().unwrap();
        assert_eq!(function_counts.invocation_count, 3);
        assert_eq!(function_counts.transaction_count, 3);
        assert_eq!(state.usage.len(), 1);
        let call_path_counts = state.usage.values().next().unwrap();
        assert_eq!(call_path_counts.invocation_count, 2);
        assert_eq!(call_path_counts.transaction_count, 2);
        assert_eq!(state.dropped_usage_invocation_count, 1);
        assert_eq!(state.dropped_usage_transaction_count, 1);
        let root_function_counts = state
            .root_function_usage
            .values()
            .next()
            .unwrap()
            .values()
            .next()
            .unwrap();
        assert_eq!(root_function_counts.invocation_count, 3);
        assert_eq!(root_function_counts.transaction_count, 3);
        assert_eq!(state.dropped_root_function_usage_invocation_count, 0);
    }

    #[test]
    fn applies_root_function_limit_independently_per_callee() {
        let mut collector = FrameworkUsageCollector::new(10, 20);
        collector.root_function_row_limit_per_function = 1;
        let module_id = collector.target_modules.iter().next().unwrap().clone();
        let first_callee = FunctionId {
            module_id: Some(module_id.clone()),
            function_name: "first_target".to_owned(),
        };
        let second_callee = FunctionId {
            module_id: Some(module_id.clone()),
            function_name: "second_target".to_owned(),
        };
        let record = |transaction_hash, version, callee: FunctionId, root_name: &str| {
            collector.record_transaction(TransactionFunctionUsage {
                transaction_hash,
                sender: AccountAddress::ONE,
                multisig_address: None,
                root_function: Some(FunctionId {
                    module_id: Some(module_id.clone()),
                    function_name: root_name.to_owned(),
                }),
                status: TransactionStatus::Keep(ExecutionStatus::Success),
                calls: vec![aptos_vm::function_usage::FunctionCall {
                    caller: None,
                    callee,
                    kind: UsageCallKind::Call,
                }],
            });
            collector.assign_version(transaction_hash, version).unwrap();
        };

        record(HashValue::zero(), 10, first_callee.clone(), "first_root");
        record(HashValue::from_u64(1), 11, first_callee, "dropped_root");
        record(HashValue::from_u64(2), 12, second_callee, "second_root");

        let state = collector.lock_state();
        assert_eq!(state.root_function_usage.len(), 2);
        assert!(state
            .root_function_usage
            .values()
            .all(|roots| roots.len() == 1));
        assert_eq!(state.dropped_root_function_usage_invocation_count, 1);
        assert_eq!(state.dropped_root_function_usage_transaction_count, 1);
    }

    #[test]
    fn does_not_overwrite_json_output_when_html_output_is_its_old_temp_path() {
        let collector = FrameworkUsageCollector::new(10, 10);
        collector.set_ledger_timestamps(1_000, 1_000).unwrap();

        let output_dir = aptos_temppath::TempPath::new();
        output_dir.create_as_dir().unwrap();
        let output = output_dir.path().join("usage.tmp");
        let html_output = output_dir.path().join("usage");
        collector.write_report(&output, Some(&html_output)).unwrap();

        let report: serde_json::Value =
            serde_json::from_reader(File::open(output).unwrap()).unwrap();
        assert_eq!(report["schema_version"], SCHEMA_VERSION);
        assert!(std::fs::read_to_string(html_output)
            .unwrap()
            .contains("Framework deprecation evidence"));
    }
}

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
use move_core_types::language_storage::ModuleId;
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};

const SCHEMA_VERSION: u64 = 1;

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

#[derive(Clone, Debug, Default, Serialize)]
struct UsageCounts {
    invocation_count: u64,
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
    git_sha: String,
    target_modules: Vec<ModuleId>,
    processed_transaction_count: u64,
    transaction_usage_records: u64,
    functions: Vec<FunctionInventoryRow>,
    function_usage: Vec<FunctionUsageRow>,
    usage: Vec<UsageRow>,
}

#[derive(Clone, Debug, Serialize)]
struct FunctionUsageRow {
    #[serde(flatten)]
    key: FunctionUsageKey,
    #[serde(flatten)]
    counts: UsageCounts,
}

#[derive(Default)]
struct CollectorState {
    pending: HashMap<HashValue, TransactionFunctionUsage>,
    function_usage: BTreeMap<FunctionUsageKey, UsageCounts>,
    usage: BTreeMap<UsageKey, UsageCounts>,
    transaction_usage_records: u64,
}

pub(crate) struct FrameworkUsageCollector {
    start_version: Version,
    end_version: Version,
    target_modules: BTreeSet<ModuleId>,
    inventory: Vec<FunctionInventoryRow>,
    state: Mutex<CollectorState>,
}

impl FrameworkUsageCollector {
    pub(crate) fn new(start_version: Version, end_version: Version) -> Self {
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
        let mut per_transaction = BTreeMap::<UsageKey, u64>::new();
        let mut per_function = BTreeMap::<FunctionUsageKey, u64>::new();
        for call in transaction.calls {
            let function_key = FunctionUsageKey {
                callee: call.callee.clone(),
                outcome: outcome.clone(),
            };
            let function_count = per_function.entry(function_key).or_default();
            *function_count = function_count
                .checked_add(1)
                .context("per-transaction function invocation count overflow")?;

            let key = UsageKey {
                callee: call.callee,
                caller: call.caller,
                root_function: transaction.root_function.clone(),
                call_kind: call.kind,
                outcome: outcome.clone(),
            };
            let count = per_transaction.entry(key).or_default();
            *count = count
                .checked_add(1)
                .context("per-transaction invocation count overflow")?;
        }

        merge_usage_counts(&mut state.function_usage, per_function, version)?;
        merge_usage_counts(&mut state.usage, per_transaction, version)?;
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
        let report = FrameworkUsageReport {
            schema_version: SCHEMA_VERSION,
            start_version: self.start_version,
            end_version: self.end_version,
            git_sha: aptos_build_info::get_git_hash(),
            target_modules: self.target_modules.iter().cloned().collect(),
            processed_transaction_count,
            transaction_usage_records: state.transaction_usage_records,
            functions: self.inventory.clone(),
            function_usage: state
                .function_usage
                .iter()
                .map(|(key, counts)| FunctionUsageRow {
                    key: key.clone(),
                    counts: counts.clone(),
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
    let tmp_output = output.with_extension("tmp");
    let writer = BufWriter::new(
        File::create(&tmp_output)
            .with_context(|| format!("creating temporary report {:?}", tmp_output))?,
    );
    serde_json::to_writer_pretty(writer, report).context("serializing framework usage report")?;
    std::fs::rename(&tmp_output, output)
        .with_context(|| format!("renaming report {:?} to {:?}", tmp_output, output))?;
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

    #[test]
    fn aggregates_invocations_and_distinct_transactions() {
        let collector = FrameworkUsageCollector::new(10, 20);
        let module_id = collector.target_modules.iter().next().unwrap().clone();
        let callee = FunctionId {
            module_id: Some(module_id),
            function_name: "target".to_owned(),
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
            root_function: None,
            status: TransactionStatus::Keep(ExecutionStatus::Success),
            calls: vec![call.clone(), call],
        });
        collector.assign_version(HashValue::zero(), 12).unwrap();

        let state = collector.lock_state();
        let function_counts = state.function_usage.values().next().unwrap();
        assert_eq!(function_counts.invocation_count, 2);
        assert_eq!(function_counts.transaction_count, 1);
        assert_eq!(function_counts.first_version, 12);
        assert_eq!(function_counts.last_version, 12);
        let call_path_counts = state.usage.values().next().unwrap();
        assert_eq!(call_path_counts.invocation_count, 2);
        assert_eq!(call_path_counts.transaction_count, 1);
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
        assert_eq!(report["function_usage"][0]["invocation_count"], 2);
        assert_eq!(report["usage"][0]["transaction_count"], 1);
        let html = std::fs::read_to_string(html_output).unwrap();
        assert!(html.contains("Framework deprecation evidence"));
        assert!(html.contains("target"));
        assert!(html.contains("report-data"));
    }
}

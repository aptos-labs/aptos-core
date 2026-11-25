// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Runs a test on V1 through the transactional test runner and panics on findings. Shared by
//! the VM fuzz targets.
//!
//! Each test runs under the primary config (extra runtime type and reference checks), under one
//! alternate config picked by the test's hash, and once more as an equivalent rewrite (see
//! `masm_smith::rewrite`). Findings:
//!   - a runner error or panic, including a value that does not survive a serialization round
//!     trip;
//!   - an invariant-violation status from code the verifier accepted;
//!   - any difference between the primary and alternate runs: status, location, offsets, return
//!     values, gas, or writes;
//!   - any difference between the test and its rewrite.
//!
//! Environment:
//!   - `MASM_SMITH_REWRITES`: unset runs every rewrite, `none` runs none, and a comma-separated
//!     list of rewrite names runs only those.
//!   - `MASM_SMITH_RICH_FIELD_PROB`: overrides [`GEN_CONFIG`]'s `rich_field_prob`.

use crate::{state_feedback, stats};
use masm_smith::{
    rewrite::{self, Agreement, Rewrite},
    Config, MasmTest,
};
use move_core_types::vm_status::{
    StatusCode, INVARIANT_VIOLATION_STATUS_MAX_CODE, INVARIANT_VIOLATION_STATUS_MIN_CODE,
};
use move_transactional_test_runner::vm_test_harness::{self, TestRunConfig};
use move_vm_runtime::config::VMConfig;
use serde_json::json;
use std::{collections::BTreeSet, fmt, io::Write, path::Path, str::FromStr, sync::LazyLock};

/// Generator config for VM targets. `MASM_SMITH_RICH_FIELD_PROB` overrides
/// [`Config::rich_field_prob`].
pub static GEN_CONFIG: LazyLock<Config> = LazyLock::new(|| {
    let mut config = Config::vm_fuzzing();
    if let Some(prob) = env_parse("MASM_SMITH_RICH_FIELD_PROB") {
        config.rich_field_prob = prob;
    }
    config
});

/// Parses a Unicode environment value, returning `None` if it is absent or not Unicode.
/// Panics if a present Unicode value cannot be parsed as `T`.
pub fn env_parse<T: FromStr>(name: &str) -> Option<T> {
    std::env::var(name).ok().map(|value| {
        value.parse().unwrap_or_else(|_| {
            panic!(
                "{name} must be a {}, not `{value}`",
                std::any::type_name::<T>()
            )
        })
    })
}

/// Default gas budget for run tasks that do not specify one.
pub const GAS_BUDGET: u64 = 100_000_000;

static REWRITES: LazyLock<Vec<Rewrite>> =
    LazyLock::new(|| match std::env::var("MASM_SMITH_REWRITES") {
        Err(_) => Rewrite::ALL.to_vec(),
        Ok(names) if names == "none" => Vec::new(),
        Ok(names) => names
            .split(',')
            .map(|name| {
                Rewrite::from_name(name.trim())
                    .unwrap_or_else(|| panic!("unknown rewrite `{name}` in MASM_SMITH_REWRITES"))
            })
            .collect(),
    });

/// A VM configuration that must agree with every other on verified code.
pub struct RunConfig {
    pub name: &'static str,
    pub config: TestRunConfig,
}

pub static PRIMARY: LazyLock<RunConfig> = LazyLock::new(|| {
    // SAFETY: runs once, before any runner reads the environment, and no other thread reads
    // or writes it.
    unsafe {
        // Makes the runner write captured output to the `.exp` sidecar instead of diffing it.
        std::env::set_var("UPDATE_BASELINE", "1");
        // Includes the VM error message in the captured output.
        std::env::set_var("MOVE_TEST_DEBUG", "1");
    }
    RunConfig {
        name: "primary",
        config: TestRunConfig::default()
            .with_runtime_ref_checks()
            .with_effects_report()
            .with_serde_round_trip(),
    }
});

static ALTERNATES: LazyLock<Vec<RunConfig>> = LazyLock::new(|| {
    let primary = &PRIMARY.config;
    let with_vm_config = |name, vm_config: VMConfig| RunConfig {
        name,
        config: TestRunConfig {
            vm_config,
            ..primary.clone()
        },
    };
    vec![
        // Type checks deferred to a replay of the recorded trace.
        RunConfig {
            name: "tracing",
            config: TestRunConfig {
                tracing: true,
                ..primary.clone()
            },
        },
        with_vm_config("trusted-optimized", VMConfig {
            optimize_trusted_code: true,
            ..primary.vm_config.clone()
        }),
        with_vm_config("unchecked", VMConfig {
            paranoid_type_checks: false,
            paranoid_ref_checks: false,
            ..primary.vm_config.clone()
        }),
        with_vm_config("eager-loading", VMConfig {
            enable_lazy_loading: false,
            ..primary.vm_config.clone()
        }),
        // Only caches that must not change behaviour. `enable_struct_layout_local_cache` is a
        // version-gated change to how layout size limits count repeated structs, not a cache.
        with_vm_config("uncached", VMConfig {
            enable_function_caches: false,
            enable_function_caches_for_native_dynamic_dispatch: false,
            enable_layout_caches: false,
            ..primary.vm_config.clone()
        }),
    ]
});

pub static INVARIANT_STATUS_NAMES: LazyLock<BTreeSet<String>> = LazyLock::new(|| {
    (INVARIANT_VIOLATION_STATUS_MIN_CODE..=INVARIANT_VIOLATION_STATUS_MAX_CODE)
        .filter_map(|code| StatusCode::try_from(code).ok())
        .map(|status| format!("{status:?}"))
        .collect()
});

/// Runs `test` under the primary and one alternate config and panics on any finding.
pub fn run(test: &MasmTest) {
    // The runner re-assembles the text, so binary-level recipes cannot be applied here.
    if test.post_process.is_some() {
        return;
    }
    let text = with_gas_budget(&test.render());
    if text.trim().is_empty() {
        return;
    }
    let primary = run_text(&PRIMARY, PRIMARY.name, &text);
    state_feedback::record("v1", &primary);
    let failures = task_failures(&primary);
    for failure in &failures {
        let status = &failure.status;
        if INVARIANT_STATUS_NAMES.contains(status) {
            panic!(
                "vm_oracle: invariant violation {status} on verifier-accepted code\n--output--\n{primary}\n--full--\n{text}"
            );
        }
    }

    let rewritten = check_rewrite(test, &text, &primary);

    let alternate = &ALTERNATES[alternate_index(&text)];
    let other = run_text(alternate, alternate.name, &text);
    if let Some((task, left, right)) = first_difference(&primary, &other) {
        panic!(
            "vm_oracle: configs primary and {} disagree on task {task}\n--primary--\n{left}\n--{}--\n{right}\n--primary output--\n{primary}\n--{} output--\n{other}\n--full--\n{text}",
            alternate.name, alternate.name, alternate.name
        );
    }

    if let Some(path) = stats::STATS_PATH.as_ref() {
        write_stats(path, &primary, &failures, rewritten);
    }
}

/// Runs a rewrite of `test` under the primary config and panics unless it behaves like the
/// original. Returns the rewrite and whether the comparison was conclusive, or `None` when no
/// enabled rewrite applies.
fn check_rewrite(test: &MasmTest, text: &str, primary: &str) -> Option<(Rewrite, bool)> {
    let (rewrite, variant) =
        rewrite::choose_and_apply(test, &REWRITES, text_hash("rewrite", text))?;
    let name = rewrite.name();
    let variant_text = with_gas_budget(&variant.render());
    let output = run_text(&PRIMARY, name, &variant_text);
    for failure in task_failures(&output) {
        let status = &failure.status;
        if INVARIANT_STATUS_NAMES.contains(status) {
            panic!(
                "vm_oracle: invariant violation {status} in rewrite {name}\n--output--\n{output}\n--original text--\n{text}\n--{name} text--\n{variant_text}"
            );
        }
    }
    match rewrite::compare_outputs(primary, &output) {
        Agreement::Same => Some((rewrite, true)),
        Agreement::Inconclusive { .. } => Some((rewrite, false)),
        Agreement::VariantRejected {
            task,
            original,
            variant,
        } => panic!(
            "vm_oracle: rewrite {name} produced code that fails to publish at task {task}\n--original--\n{original}\n--{name}--\n{variant}\n--{name} text--\n{variant_text}"
        ),
        Agreement::Differ {
            task,
            original,
            variant,
        } => panic!(
            "vm_oracle: rewrite {name} disagrees on task {task}\n--original--\n{original}\n--{name}--\n{variant}\n--original output--\n{primary}\n--{name} output--\n{output}\n--original text--\n{text}\n--{name} text--\n{variant_text}"
        ),
    }
}

/// Runs `text` under `run_config` and returns the captured output. `label` names the run in
/// panic messages.
fn run_text(run_config: &RunConfig, label: &str, text: &str) -> String {
    let (result, output) = run_in_temp_file(text, "masm", |source, _| {
        vm_test_harness::run_test_with_config(run_config.config.clone(), source)
    });
    if let Err(err) = &result {
        panic!("vm_oracle: runner returned an error under {label}: {err}\n--full--\n{text}");
    }
    output.unwrap_or_default()
}

/// Writes `text` to a temporary `.{extension}` file and calls `run` with its path and the path
/// of the `.exp` output next to it. Returns `run`'s result and the output it wrote, if any.
pub fn run_in_temp_file<E>(
    text: &str,
    extension: &str,
    run: impl FnOnce(&Path, &Path) -> Result<(), E>,
) -> (Result<(), E>, Option<String>) {
    let mut source = tempfile::Builder::new()
        .suffix(&format!(".{extension}"))
        .tempfile()
        .expect("create temp file");
    source.write_all(text.as_bytes()).expect("write the source");
    source.flush().expect("flush the source");
    let exp_path = source.path().with_extension("exp");
    let result = run(source.path(), &exp_path);
    let output = std::fs::read_to_string(&exp_path).ok();
    let _ = std::fs::remove_file(&exp_path);
    (result, output)
}

/// Adds `--gas-budget` to every `run` directive that lacks one, so all executions are metered.
pub fn with_gas_budget(text: &str) -> String {
    with_run_flag(text, "--gas-budget", GAS_BUDGET)
}

/// Appends `flag value` to every `run` directive that lacks `flag`.
pub fn with_run_flag(text: &str, flag: &str, value: impl fmt::Display) -> String {
    let mut out = String::with_capacity(text.len() + 256);
    for line in text.lines() {
        out.push_str(line);
        if is_run_directive(line) && !line.contains(flag) {
            out.push_str(&format!(" {flag} {value}"));
        }
        out.push('\n');
    }
    out
}

/// Whether `line` is a `run` task directive.
fn is_run_directive(line: &str) -> bool {
    line.trim_start()
        .strip_prefix("//#")
        .and_then(|rest| rest.trim_start().strip_prefix("run"))
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
}

fn alternate_index(text: &str) -> usize {
    (state_feedback::hash(&text) % ALTERNATES.len() as u64) as usize
}

/// A hash of `text` for choices independent of [`alternate_index`].
pub fn text_hash(salt: &str, text: &str) -> u64 {
    state_feedback::hash(&(salt, text))
}

/// Returns the first task whose comparable output differs, with both versions.
fn first_difference(left: &str, right: &str) -> Option<(usize, String, String)> {
    let left_tasks = comparable_tasks(left);
    let right_tasks = comparable_tasks(right);
    let count = left_tasks.len().max(right_tasks.len());
    (0..count).find_map(|task| {
        let left_task = left_tasks.get(task).cloned().unwrap_or_default();
        let right_task = right_tasks.get(task).cloned().unwrap_or_default();
        (left_task != right_task).then_some((task, left_task, right_task))
    })
}

/// Splits runner output into per-task outputs. Error records keep only the fields that must
/// agree across configs; VM messages and execution state (stack traces) legitimately differ.
fn comparable_tasks(output: &str) -> Vec<String> {
    const ERROR_FIELDS: [&str; 5] = [
        "major_status: ",
        "sub_status: ",
        "location: ",
        "indices: ",
        "offsets: ",
    ];
    let mut tasks: Vec<String> = Vec::new();
    let mut in_error_record = false;
    for line in output.lines() {
        if line.starts_with("task ") {
            tasks.push(String::new());
            in_error_record = false;
            continue;
        }
        let Some(task) = tasks.last_mut() else {
            continue;
        };
        if line.starts_with("Error: ") {
            in_error_record = line.ends_with('{');
        } else if in_error_record {
            if line == "}" {
                in_error_record = false;
            } else if !ERROR_FIELDS
                .iter()
                .any(|field| line.trim_start().starts_with(field))
            {
                continue;
            }
        }
        task.push_str(line);
        task.push('\n');
    }
    tasks
}

fn write_stats(
    path: &Path,
    output: &str,
    failures: &[TaskFailure],
    rewritten: Option<(Rewrite, bool)>,
) {
    let tasks = output
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|count| count.parse::<u64>().ok())
        .unwrap_or(0);
    let writes = |kind: &str| {
        output
            .lines()
            .filter(|line| line.starts_with(&format!("write {kind} ")))
            .count()
    };
    let mut published = BTreeSet::new();
    let upgrades = output
        .lines()
        .filter_map(|line| line.split_once("publish [module ").map(|(_, rest)| rest))
        .filter(|module| !published.insert(module.to_owned()))
        .count();
    let record = json!({
        "tasks": tasks,
        "writes": { "new": writes("new"), "modify": writes("modify"), "delete": writes("delete") },
        "upgrades": upgrades,
        "rewrite": rewritten.map(|(rewrite, conclusive)| {
            json!({ "name": rewrite.name(), "conclusive": conclusive })
        }),
        "failures": failures
            .iter()
            .map(|failure| {
                json!({
                    "kind": failure.kind,
                    "status": failure.status,
                    "message": failure.message,
                })
            })
            .collect::<Vec<_>>(),
    });
    stats::append_stats(path, &record);
}

struct TaskFailure {
    kind: &'static str,
    status: String,
    message: String,
}

/// Collects each `Error:` line in the runner output with the `major_status` and `message`
/// that follow it. Errors without a VM status (e.g. assembly failures) report `NO_STATUS` and
/// keep the error line as the message.
fn task_failures(output: &str) -> Vec<TaskFailure> {
    const MAX_MESSAGE_LEN: usize = 240;
    let mut failures: Vec<TaskFailure> = Vec::new();
    for line in output.lines() {
        if let Some(message) = line.strip_prefix("Error: ") {
            let kind = if message.starts_with("Unable to publish module") {
                "publish"
            } else if message.starts_with("Script execution failed") {
                "script"
            } else if message.starts_with("Function execution failed") {
                "call"
            } else {
                "other"
            };
            failures.push(TaskFailure {
                kind,
                status: "NO_STATUS".to_owned(),
                message: message.chars().take(MAX_MESSAGE_LEN).collect(),
            });
        } else if let Some(last) = failures.last_mut() {
            let line = line.trim();
            if let Some(status) = line.strip_prefix("major_status: ") {
                last.status = status.trim_end_matches(',').to_owned();
            } else if let Some(message) = line.strip_prefix("message: ") {
                last.message = message.chars().take(MAX_MESSAGE_LEN).collect();
            }
        }
    }
    failures
}

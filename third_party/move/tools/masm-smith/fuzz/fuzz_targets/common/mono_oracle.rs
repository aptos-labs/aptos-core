// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Runs a test on V1 and on MonoMove (v2), and again on v2 with a small heap, and reports
//! differences that are not in [`ALLOWED_DIVERGENCES`].
//!
//! Two comparisons, task by task:
//!   - **cross-vm:** V1 and v2 must agree on success or failure, return values, resource writes,
//!     and error details. Gas and GC counts are not compared.
//!   - **small-heap:** v2 runs again with a heap of at most 16 KiB, which forces garbage
//!     collection. Its output must match v2's normal run.
//!
//! A comparison stops early where the runs may legitimately part ways: one side runs out of a
//! resource, v2 lacks a feature listed in [`KNOWN_GAPS`], or the difference is allowed. An
//! invariant-violation status, or v2 running out of memory where V1 succeeds, is always a
//! finding.
//!
//! Environment:
//!   - `MASM_SMITH_DIFF_FINDINGS=<dir>`: save up to [`SAMPLES_PER_SIGNATURE`] findings per
//!     signature in `<dir>` and keep fuzzing. Unset, a finding panics.
//!   - `MASM_SMITH_DIFF_EXPECT=<signature>`: panic on that signature only, for test-case
//!     reduction.
//!   - `MASM_SMITH_DIFF_HEAP_SIZE=<bytes>`: fix the small heap's size, which is otherwise picked
//!     from the text's hash.
//!   - `MASM_SMITH_STATS=<file>`: append one JSON record per input, with each run's time.

use crate::{
    mono_allowlist::{AllowedDivergence, ALLOWED_DIVERGENCES},
    state_feedback, stats,
    vm_oracle::{self, GAS_BUDGET, INVARIANT_STATUS_NAMES, PRIMARY},
};
use masm_smith::{Config, MasmTest};
use move_transactional_test_runner::{
    framework::{BaselineTarget, UpdatePolicy},
    vm_test_harness,
};
use serde_json::json;
use std::{
    collections::BTreeMap,
    fmt,
    path::PathBuf,
    sync::LazyLock,
    time::{Duration, Instant},
};

/// Generator config: `vm_oracle`'s, without the operations v2 does not support.
pub static GEN_CONFIG: LazyLock<Config> = LazyLock::new(|| Config {
    enable_function_value_ops: false,
    enable_type_name_get: false,
    ..vm_oracle::GEN_CONFIG.clone()
});

const CROSS_VM: &str = "cross-vm";
const SMALL_HEAP: &str = "small-heap";

/// v2 heap sizes for the small-heap arm, in bytes. Below 32 KiB no region is pooled, so every
/// collection frees the old space and ASan reports a stale pointer into it. The smallest sizes
/// leave many runs out of heap, which only stops the comparison.
const SMALL_HEAP_SIZES: [usize; 14] = [
    64, 96, 128, 200, 256, 384, 512, 768, 1024, 1536, 2048, 4096, 8192, 16384,
];

/// Statuses for which v2 reports its own message and no sub-status, since it cannot reconstruct
/// V1's.
const MONO_MESSAGE_STATUSES: [&str; 4] = [
    "STRUCT_VARIANT_MISMATCH",
    "FUNCTION_RESOLUTION_FAILURE",
    "CODE_DESERIALIZATION_ERROR",
    "VALUE_SERIALIZATION_ERROR",
];

/// Statuses of a resource running out.
const EXHAUSTION_STATUSES: [&str; 4] = [
    "OUT_OF_GAS",
    "EXECUTION_STACK_OVERFLOW",
    "CALL_STACK_OVERFLOW",
    "MEMORY_LIMIT_EXCEEDED",
];

/// Prefixes of the error messages v2 reports for a feature it lacks.
const GAP_PREFIXES: [&str; 3] = [
    "unsupported by MonoVM: ",
    "the MonoVM transactional session cannot ",
    "the MonoVM adapter runs unmetered",
];

/// Prefixes of the v2 feature gaps that stop the comparison without a finding.
const KNOWN_GAPS: [&str; 6] = [
    "unsupported by MonoVM: function values are not yet supported",
    "unsupported by MonoVM: reference parameters",
    "unsupported by MonoVM: generic function not instantiated",
    "unsupported by MonoVM: not all types are concrete",
    // Text inputs may still call it.
    "unsupported by MonoVM: Function 0x1::type_name::get is a native and cannot be loaded as code",
    // Only text inputs carry their own gas budgets.
    "the MonoVM adapter runs unmetered",
];

/// Longest status message or error line kept in a signature.
const MAX_KEY_LEN: usize = 120;

/// Findings recorded per signature. A signature can cover distinct bugs, such as two wrong
/// return values.
const SAMPLES_PER_SIGNATURE: usize = 4;

static FINDINGS_DIR: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
    let dir = PathBuf::from(std::env::var_os("MASM_SMITH_DIFF_FINDINGS")?);
    std::fs::create_dir_all(&dir).expect("create the findings directory");
    Some(dir)
});

static EXPECTED_SIGNATURE: LazyLock<Option<String>> =
    LazyLock::new(|| std::env::var("MASM_SMITH_DIFF_EXPECT").ok());

static HEAP_SIZE: LazyLock<Option<usize>> =
    LazyLock::new(|| vm_oracle::env_parse("MASM_SMITH_DIFF_HEAP_SIZE"));

/// Runs a generated test on both VMs and reports any finding.
pub fn run_test(test: &MasmTest) {
    // The runners re-assemble the text, so binary-level recipes cannot be applied here.
    if test.post_process.is_some() {
        return;
    }
    let text = test.render();
    if text.trim().is_empty() {
        return;
    }
    run(&text, "masm", true);
}

/// Runs the transactional test source `text`, whose file extension is `extension`, on both VMs
/// and reports any finding. A source V1's runner rejects is skipped: text inputs need not be
/// well-formed.
pub fn run_source(text: &str, extension: &str) {
    run(text, extension, false);
}

/// Runs `text` on both VMs and under a small heap, and reports any finding. A V1 runner error is
/// a finding only for a `generated` test.
fn run(text: &str, extension: &str, generated: bool) {
    let (v1_result, v1_elapsed) =
        timed(|| run_on(Vm::V1, &vm_oracle::with_gas_budget(text), extension));
    let v1_output = match v1_result {
        Ok(output) => output,
        Err(err) if generated => {
            panic!("mono_oracle: the V1 runner returned an error: {err}\n--full--\n{text}")
        },
        Err(_) => return,
    };
    let mut v1_tasks = parse_tasks(&v1_output);
    drop_unchanged_writes(&mut v1_tasks);
    let v1_out_of_gas = v1_tasks
        .iter()
        .position(|task| task.vm_status() == Some("OUT_OF_GAS"));
    // Unmetered, v2 need not stop where V1 ran out of gas.
    let v2_text = match v1_out_of_gas {
        Some(index) => {
            let first_line = v1_tasks[index]
                .first_line
                .expect("task headers name their lines");
            text.lines()
                .take(first_line.saturating_sub(1))
                .map(|line| format!("{line}\n"))
                .collect()
        },
        None => text.to_owned(),
    };
    let (v2_output, v2_elapsed) = timed(|| run_v2(&v2_text, extension));
    let v2_tasks = parse_tasks(&v2_output);
    let heap_size = HEAP_SIZE.unwrap_or_else(|| {
        let index = vm_oracle::text_hash("heap size", text) % SMALL_HEAP_SIZES.len() as u64;
        SMALL_HEAP_SIZES[usize::try_from(index).expect("an index below the length fits")]
    });
    let (small_heap_output, small_heap_elapsed) = timed(|| {
        run_v2(
            &vm_oracle::with_run_flag(&v2_text, "--heap-size", heap_size),
            extension,
        )
    });
    let small_heap_tasks = parse_tasks(&small_heap_output);
    state_feedback::record("v1", &v1_output);
    state_feedback::record("v2", &v2_output);
    state_feedback::record("v2 small heap", &small_heap_output);

    let cross_vm = compare_cross_vm(&v1_tasks, &v2_tasks, v1_out_of_gas);
    let small_heap = compare_small_heap(&v2_tasks, &small_heap_tasks);
    write_stats(
        &v1_tasks,
        &v2_tasks,
        &cross_vm,
        heap_size,
        &small_heap_tasks,
        &small_heap,
        &Elapsed {
            v1: v1_elapsed,
            v2: v2_elapsed,
            small_heap: small_heap_elapsed,
        },
    );
    if let End::Finding { task, signature } = &cross_vm.end {
        let detail = finding_detail(
            signature,
            *task,
            ("V1", &v1_tasks, &v1_output),
            ("v2", &v2_tasks, &v2_output),
        );
        report(signature, &detail, text, extension);
    }
    if let End::Finding { task, signature } = &small_heap.end {
        let detail = finding_detail(
            signature,
            *task,
            ("v2", &v2_tasks, &v2_output),
            (
                &format!("v2 with a {heap_size}-byte heap (MASM_SMITH_DIFF_HEAP_SIZE={heap_size})"),
                &small_heap_tasks,
                &small_heap_output,
            ),
        );
        report(signature, &detail, text, extension);
    }
}

/// Runs `action`, returning its result and how long it took.
fn timed<T>(action: impl FnOnce() -> T) -> (T, Duration) {
    let started = Instant::now();
    let result = action();
    (result, started.elapsed())
}

/// Runs `text` on v2 and returns the captured output, which is empty when the text has no task:
/// running out of gas in the first task leaves v2 nothing to run. Panics if the runner returns an
/// error.
fn run_v2(text: &str, extension: &str) -> String {
    if !text
        .lines()
        .any(|line| line.trim_start().starts_with("//#"))
    {
        return String::new();
    }
    run_on(Vm::V2, text, extension).unwrap_or_else(|err| {
        panic!("mono_oracle: the v2 runner returned an error: {err}\n--full--\n{text}")
    })
}

/// The finding `signature` at `task` with both sides' outputs, each given as its name, tasks and
/// whole output.
fn finding_detail(
    signature: &str,
    task: usize,
    left: (&str, &[Task], &str),
    right: (&str, &[Task], &str),
) -> String {
    let mut detail = signature.to_owned();
    for (name, tasks, _) in [left, right] {
        let raw = tasks
            .get(task)
            .map_or("<missing>", |task| task.raw.as_str());
        detail.push_str(&format!("\n--{name}, task {task}--\n{raw}"));
    }
    for (name, _, output) in [left, right] {
        detail.push_str(&format!("\n--{name}, output--\n{output}"));
    }
    detail
}

#[derive(Clone, Copy)]
enum Vm {
    V1,
    V2,
}

impl fmt::Display for Vm {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Vm::V1 => "V1",
            Vm::V2 => "v2",
        })
    }
}

/// Runs `text` on `vm` under the primary config and returns the captured output, or the
/// runner's error.
fn run_on(vm: Vm, text: &str, extension: &str) -> Result<String, String> {
    let (result, output) = vm_oracle::run_in_temp_file(text, extension, |source, exp_path| {
        let config = PRIMARY.config.clone();
        match vm {
            Vm::V1 => {
                vm_test_harness::run_test_with_config(config, source).map_err(|err| err.to_string())
            },
            Vm::V2 => mono_move_testsuite::run_transactional_test(
                config,
                source,
                &BaselineTarget::at(exp_path, UpdatePolicy::CreateOrUpdate),
            )
            .map_err(|err| err.to_string()),
        }
    });
    result
        .map(|()| output.unwrap_or_else(|| panic!("mono_oracle: the {vm} runner wrote no output")))
}

/// One task's output.
struct Task {
    /// All of the task's lines, header included.
    raw: String,
    /// The header, without the gas budget `vm_oracle` adds and any heap size.
    header: String,
    /// First line of the task's directive in the source, 1-based.
    first_line: Option<usize>,
    kind: &'static str,
    /// The first error the task reports.
    failure: Option<Failure>,
    /// Lines other than the header, gas used, the GC count and the failure: return values,
    /// resource writes and views.
    lines: Vec<String>,
    gc_count: Option<u64>,
}

impl Task {
    fn vm_status(&self) -> Option<&str> {
        match &self.failure {
            Some(Failure::Vm(error)) => Some(&error.major_status),
            Some(Failure::Gap(_) | Failure::Error(_)) | None => None,
        }
    }

    fn is_exhausted(&self) -> bool {
        self.vm_status()
            .is_some_and(|status| EXHAUSTION_STATUSES.contains(&status))
    }
}

enum Failure {
    Vm(VmError),
    /// A feature v2 lacks: the error message.
    Gap(String),
    /// Any other error: the error line.
    Error(String),
}

/// A `VMError` as the runners render it, without its execution state.
#[derive(Default)]
struct VmError {
    /// What failed, e.g. `Function execution failed with`.
    context: String,
    message: String,
    major_status: String,
    sub_status: String,
    location: String,
    indices: String,
    offsets: String,
}

/// Splits runner output into tasks.
fn parse_tasks(output: &str) -> Vec<Task> {
    let mut tasks: Vec<Task> = Vec::new();
    // The field of the error record being read, if inside one.
    let mut record_field: Option<&'static str> = None;
    for line in output.lines() {
        if line.starts_with("task ") {
            tasks.push(parse_header(line));
            record_field = None;
            continue;
        }
        let Some(task) = tasks.last_mut() else {
            continue;
        };
        task.raw.push('\n');
        task.raw.push_str(line);
        if let Some(field) = record_field {
            let Some(Failure::Vm(error)) = task.failure.as_mut() else {
                unreachable!("a record field is read only after its record starts");
            };
            record_field = read_record_line(error, field, line);
            continue;
        }
        if line.starts_with("gas used: ") {
            continue;
        }
        if let Some(count) = line.strip_prefix("gc count: ") {
            task.gc_count = Some(count.parse().expect("the GC count is a number"));
            continue;
        }
        let error = line.strip_prefix("Error: ");
        match error {
            Some(message) if task.failure.is_none() => {
                if let Some(context) = message.strip_suffix(" VMError: {") {
                    task.failure = Some(Failure::Vm(VmError {
                        context: context.to_owned(),
                        ..VmError::default()
                    }));
                    record_field = Some("");
                } else if GAP_PREFIXES
                    .iter()
                    .any(|prefix| message.starts_with(prefix))
                {
                    task.failure = Some(Failure::Gap(message.to_owned()));
                } else {
                    task.failure = Some(Failure::Error(message.to_owned()));
                }
            },
            Some(_) | None => task.lines.push(line.to_owned()),
        }
    }
    for task in &mut tasks {
        if let Some(Failure::Vm(error)) = task.failure.as_mut() {
            for value in [
                &mut error.message,
                &mut error.major_status,
                &mut error.sub_status,
                &mut error.location,
                &mut error.indices,
                &mut error.offsets,
            ] {
                if let Some(stripped) = value.strip_suffix(',') {
                    *value = stripped.to_owned();
                }
            }
        }
    }
    tasks
}

/// Removes the writes that store a resource's current bytes back, given every task's writes in
/// order. Tests start with no resources, so earlier tasks' writes determine each current value.
fn drop_unchanged_writes(tasks: &mut [Task]) {
    let mut current: BTreeMap<(String, String), String> = BTreeMap::new();
    for task in tasks {
        task.lines.retain(|line| {
            let Some(("modify", address, tag, value)) = parse_write(line) else {
                return true;
            };
            current
                .get(&(address.to_owned(), tag.to_owned()))
                .is_none_or(|existing| existing != value)
        });
        for line in &task.lines {
            let Some((kind, address, tag, value)) = parse_write(line) else {
                continue;
            };
            let key = (address.to_owned(), tag.to_owned());
            if kind == "delete" {
                current.remove(&key);
            } else {
                current.insert(key, value.to_owned());
            }
        }
    }
}

/// Splits `write <kind> <address> <struct tag> <hex value>` into its parts. A struct tag may
/// contain spaces; a deletion has an empty value.
fn parse_write(line: &str) -> Option<(&str, &str, &str, &str)> {
    let (kind, rest) = line.strip_prefix("write ")?.split_once(' ')?;
    let (address, rest) = rest.split_once(' ')?;
    let (tag, value) = rest.rsplit_once(' ')?;
    Some((kind, address, tag, value))
}

/// Parses `task <n> lines <first>-<last>: <command>`.
fn parse_header(line: &str) -> Task {
    let header = without_heap_size(&line.replace(&format!(" --gas-budget {GAS_BUDGET}"), ""));
    let first_line = line
        .split_once(" lines ")
        .and_then(|(_, range)| range.split('-').next())
        .and_then(|first| first.parse().ok());
    let command = header
        .split_once(": ")
        .map_or("", |(_, command)| command.trim());
    let kind = if command.starts_with("publish") {
        "publish"
    } else if command.starts_with("run") && command.contains(" [script") {
        "script"
    } else if command.starts_with("run") {
        "call"
    } else {
        "other"
    };
    Task {
        raw: line.to_owned(),
        header,
        first_line,
        kind,
        failure: None,
        lines: Vec::new(),
        gc_count: None,
    }
}

/// `header` without its `--heap-size` option.
fn without_heap_size(header: &str) -> String {
    match header.split_once(" --heap-size ") {
        Some((before, after)) => match after.split_once(' ') {
            Some((_, rest)) => format!("{before} {rest}"),
            None => before.to_owned(),
        },
        None => header.to_owned(),
    }
}

/// Adds one line of an error record to `error`, given the field the previous line was in (empty
/// for the record's first line), and returns the field the next line continues, or `None` once
/// the record ends. A message may span several lines.
fn read_record_line(error: &mut VmError, field: &'static str, line: &str) -> Option<&'static str> {
    if line == "}" {
        return None;
    }
    const FIELDS: [&str; 7] = [
        "message",
        "major_status",
        "sub_status",
        "location",
        "indices",
        "offsets",
        "exec_state",
    ];
    let started = FIELDS.iter().find_map(|name| {
        line.strip_prefix("    ")
            .and_then(|rest| rest.strip_prefix(name))
            .and_then(|rest| rest.strip_prefix(": "))
            .map(|value| (*name, value))
    });
    let (field, value, continued) = match started {
        Some((name, value)) => (name, value, false),
        None => (field, line, true),
    };
    let slot = match field {
        "message" => &mut error.message,
        "major_status" => &mut error.major_status,
        "sub_status" => &mut error.sub_status,
        "location" => &mut error.location,
        "indices" => &mut error.indices,
        "offsets" => &mut error.offsets,
        // Execution state is never compared.
        "exec_state" => return Some(field),
        other => panic!("mono_oracle: unexpected line `{line}` in error record field `{other}`"),
    };
    if continued {
        slot.push('\n');
    }
    slot.push_str(value);
    Some(field)
}

/// How the comparison of two outputs ended.
enum End {
    Agreed,
    /// The comparison stopped at `task` with no verdict on the remaining tasks.
    Cut {
        task: usize,
        kind: &'static str,
        reason: String,
    },
    Allowed {
        task: usize,
        divergence: &'static AllowedDivergence,
    },
    Finding {
        task: usize,
        signature: String,
    },
}

struct Comparison {
    /// Number of leading tasks on which the two sides agree, at most the end's task.
    compared: usize,
    end: End,
}

/// What comparing one task's two outputs found.
enum Verdict {
    Agree,
    Cut {
        kind: &'static str,
        reason: String,
    },
    /// The VMs diverge; the first differing field.
    Differ(&'static str),
}

/// Fields that are findings even when their signature is allowed.
const NEVER_ALLOWED: [&str; 5] = [
    "v1-invariant",
    "v2-invariant",
    "return values",
    "writes",
    "output",
];

fn is_invariant(task: Option<&Task>) -> bool {
    task.and_then(Task::vm_status)
        .is_some_and(|status| INVARIANT_STATUS_NAMES.contains(status))
}

/// Compares V1's and v2's outputs. An invariant-violation status on either VM is a finding at
/// any task, even past a cut: verified code must never reach one.
fn compare_cross_vm(v1: &[Task], v2: &[Task], v1_out_of_gas: Option<usize>) -> Comparison {
    let comparison = compare_tasks(v1, v2, v1_out_of_gas);
    let invariant = (0..v1.len().max(v2.len())).find_map(|index| {
        let (left, right) = (v1.get(index), v2.get(index));
        let field = if is_invariant(left) {
            "v1-invariant"
        } else if is_invariant(right) {
            "v2-invariant"
        } else {
            return None;
        };
        Some((index, divergence_end(CROSS_VM, index, left, right, field)))
    });
    match invariant {
        Some((task, end)) => Comparison {
            compared: comparison.compared.min(task),
            end,
        },
        None => comparison,
    }
}

/// Compares V1's and v2's tasks in order, up to the task where V1 ran out of gas, and ends at the
/// first difference or cut.
fn compare_tasks(v1: &[Task], v2: &[Task], v1_out_of_gas: Option<usize>) -> Comparison {
    let limit = v1_out_of_gas.unwrap_or(v1.len());
    for (index, left) in v1.iter().enumerate().take(limit) {
        let right = v2.get(index);
        let verdict = match right {
            Some(right) => compare_task(left, right),
            None => Verdict::Differ("task count"),
        };
        let end = match verdict {
            Verdict::Agree => continue,
            Verdict::Cut { kind, reason } => End::Cut {
                task: index,
                kind,
                reason,
            },
            Verdict::Differ(field) => divergence_end(CROSS_VM, index, Some(left), right, field),
        };
        return Comparison {
            compared: index,
            end,
        };
    }
    let end = match v1_out_of_gas {
        Some(task) => End::Cut {
            task,
            kind: "v1 out of gas",
            reason: String::new(),
        },
        None if v2.len() > v1.len() => {
            divergence_end(CROSS_VM, v1.len(), None, v2.get(v1.len()), "task count")
        },
        None => End::Agreed,
    };
    Comparison {
        compared: limit,
        end,
    }
}

/// Compares v2's small-heap run with its base run: they must match exactly, GC counts aside. A
/// small-heap invariant-violation status is a finding at any task unless the base run reports the
/// same status there, which makes it the cross-vm arm's finding.
fn compare_small_heap(base: &[Task], small: &[Task]) -> Comparison {
    let new_invariant = |index: &usize| {
        let base_status = base.get(*index).and_then(Task::vm_status);
        let small_status = small.get(*index).and_then(Task::vm_status);
        is_invariant(small.get(*index)) && base_status != small_status
    };
    if let Some(task) = (0..small.len()).find(new_invariant) {
        return Comparison {
            compared: task,
            end: divergence_end(
                SMALL_HEAP,
                task,
                base.get(task),
                small.get(task),
                "v2-invariant",
            ),
        };
    }
    for index in 0..base.len().max(small.len()) {
        let (Some(left), Some(right)) = (base.get(index), small.get(index)) else {
            return Comparison {
                compared: index,
                end: divergence_end(
                    SMALL_HEAP,
                    index,
                    base.get(index),
                    small.get(index),
                    "task count",
                ),
            };
        };
        if right.vm_status() == Some("MEMORY_LIMIT_EXCEEDED")
            && left.vm_status() != Some("MEMORY_LIMIT_EXCEEDED")
        {
            return Comparison {
                compared: index,
                end: End::Cut {
                    task: index,
                    kind: "heap exhausted",
                    reason: String::new(),
                },
            };
        }
        if let Some(field) = same_vm_difference(left, right) {
            return Comparison {
                compared: index,
                end: divergence_end(SMALL_HEAP, index, Some(left), Some(right), field),
            };
        }
    }
    Comparison {
        compared: base.len(),
        end: End::Agreed,
    }
}

/// The first field in which two runs of a task on v2 differ.
fn same_vm_difference(left: &Task, right: &Task) -> Option<&'static str> {
    if left.header != right.header {
        return Some("header");
    }
    outcome_difference(left, right, false)
}

/// The first field in which two tasks' failures or output lines differ. `cross_vm` relaxes the
/// VM error comparison as [`vm_error_difference`] describes.
fn outcome_difference(left: &Task, right: &Task, cross_vm: bool) -> Option<&'static str> {
    let failure_difference = match (&left.failure, &right.failure) {
        (None, Some(_)) | (Some(_), None) => Some("outcome"),
        (None, None) => None,
        (Some(Failure::Vm(left)), Some(Failure::Vm(right))) => {
            vm_error_difference(left, right, cross_vm)
        },
        (Some(Failure::Gap(left)), Some(Failure::Gap(right)))
        | (Some(Failure::Error(left)), Some(Failure::Error(right)))
            if left == right =>
        {
            None
        },
        (
            Some(Failure::Vm(_) | Failure::Gap(_) | Failure::Error(_)),
            Some(Failure::Vm(_) | Failure::Gap(_) | Failure::Error(_)),
        ) => Some("error"),
    };
    failure_difference.or_else(|| lines_difference(&left.lines, &right.lines))
}

/// Ends a comparison at a difference in `field`, as allowed or as a finding. The signature is
/// `arm|task kind|left status|right status|field`, where left and right are V1 and v2, or v2's
/// base and small-heap runs.
fn divergence_end(
    arm: &str,
    task: usize,
    left: Option<&Task>,
    right: Option<&Task>,
    field: &'static str,
) -> End {
    let kind = left.or(right).map_or("other", |task| task.kind);
    let signature = format!(
        "{arm}|{kind}|{}|{}|{field}",
        status_key(left),
        status_key(right)
    );
    let allowed = ALLOWED_DIVERGENCES
        .iter()
        .find(|divergence| divergence.matches(&signature));
    match allowed {
        Some(divergence) if !NEVER_ALLOWED.contains(&field) => End::Allowed { task, divergence },
        Some(_) | None => End::Finding { task, signature },
    }
}

fn compare_task(v1: &Task, v2: &Task) -> Verdict {
    if v1.header != v2.header {
        return Verdict::Differ("header");
    }
    if let Some(Failure::Gap(message)) = &v2.failure {
        return if KNOWN_GAPS.iter().any(|gap| message.starts_with(gap)) {
            Verdict::Cut {
                kind: "unsupported",
                reason: message.clone(),
            }
        } else {
            Verdict::Differ("unsupported")
        };
    }
    match (v1.is_exhausted(), v2.is_exhausted()) {
        // Neither commits anything.
        (true, true) => return Verdict::Agree,
        // Generated programs allocate far less than v2's heap holds, so running out of it where
        // V1 succeeds suggests memory that is never reclaimed.
        (false, true)
            if v1.failure.is_none() && v2.vm_status() == Some("MEMORY_LIMIT_EXCEEDED") =>
        {
            return Verdict::Differ("exhaustion")
        },
        (true, false) | (false, true) => {
            return Verdict::Cut {
                kind: "exhaustion",
                reason: format!("{} vs {}", status_key(Some(v1)), status_key(Some(v2))),
            }
        },
        (false, false) => {},
    }
    match outcome_difference(v1, v2, true) {
        Some(field) => Verdict::Differ(field),
        None => Verdict::Agree,
    }
}

/// The kind of the first line that differs between two tasks' output lines.
fn lines_difference(left: &[String], right: &[String]) -> Option<&'static str> {
    let count = left.len().max(right.len());
    let (left, right) = (0..count)
        .map(|index| (left.get(index), right.get(index)))
        .find(|(left, right)| left != right)?;
    let either_starts_with = |prefix: &str| {
        [left, right]
            .into_iter()
            .flatten()
            .any(|line| line.starts_with(prefix))
    };
    Some(
        if either_starts_with("return values: ") {
            "return values"
        } else if either_starts_with("write ") {
            "writes"
        } else {
            "output"
        },
    )
}

/// The first field in which two VM errors differ. Under `cross_vm`, `v2` may omit a native abort's
/// code offset, and for [`MONO_MESSAGE_STATUSES`] its message and sub-status are not compared.
fn vm_error_difference(v1: &VmError, v2: &VmError, cross_vm: bool) -> Option<&'static str> {
    let mono_message = cross_vm && MONO_MESSAGE_STATUSES.contains(&v2.major_status.as_str());
    // v2 records no instruction for an abort raised by a native, which lives in the stdlib.
    let native_abort = cross_vm
        && v2.major_status == "ABORTED"
        && v2.offsets == "[]"
        && v2.location.starts_with("0x1::");
    if v1.context != v2.context {
        Some("context")
    } else if v1.major_status != v2.major_status {
        Some("major_status")
    } else if !mono_message && v1.sub_status != v2.sub_status {
        Some("sub_status")
    } else if v1.location != v2.location {
        Some("location")
    } else if v1.indices != v2.indices {
        Some("indices")
    } else if !native_abort && v1.offsets != v2.offsets {
        Some("offsets")
    } else if !mono_message && v1.message != v2.message {
        Some("message")
    } else {
        None
    }
}

/// A task's outcome as it appears in a signature.
fn status_key(task: Option<&Task>) -> String {
    let Some(task) = task else {
        return "missing".to_owned();
    };
    match &task.failure {
        None => "ok".to_owned(),
        // The status alone does not identify the failure.
        Some(Failure::Vm(error))
            if error.major_status == "UNKNOWN_RUNTIME_STATUS"
                || INVARIANT_STATUS_NAMES.contains(&error.major_status) =>
        {
            format!("{}({})", error.major_status, mask(&error.message))
        },
        Some(Failure::Vm(error)) => error.major_status.clone(),
        Some(Failure::Gap(message)) => format!("gap({})", mask(message)),
        Some(Failure::Error(message)) => format!("error({})", mask(message)),
    }
}

/// `text` with numbers and addresses replaced, so it names a class of failures, and without the
/// signature's field separator.
fn mask(text: &str) -> String {
    let mut masked = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '0' && chars.peek() == Some(&'x') {
            chars.next();
            while chars.next_if(char::is_ascii_hexdigit).is_some() {}
            masked.push_str("0x_");
        } else if ch.is_ascii_digit() {
            while chars.next_if(char::is_ascii_digit).is_some() {}
            masked.push('#');
        } else if ch == '|' || ch == '\n' {
            masked.push(' ');
        } else {
            masked.push(ch);
        }
    }
    masked.chars().take(MAX_KEY_LEN).collect()
}

/// Records or raises the finding `signature`, as the environment selects.
fn report(signature: &str, detail: &str, text: &str, extension: &str) {
    if let Some(expected) = EXPECTED_SIGNATURE.as_ref() {
        if expected == signature {
            panic!("mono_oracle: {detail}\n--full--\n{text}");
        }
        return;
    }
    let Some(dir) = FINDINGS_DIR.as_ref() else {
        panic!("mono_oracle: {detail}\n--full--\n{text}");
    };
    let name = format!("{:016x}", vm_oracle::text_hash("finding", signature));
    // Both files are complete under private names before a sample is claimed, so a fuzzer that
    // dies midway leaves no empty record behind.
    let stage = |extension: &str, contents: &str| {
        let path = dir.join(format!(".{name}.{}.{extension}", std::process::id()));
        std::fs::write(&path, contents).expect("stage the finding");
        path
    };
    let staged_record = stage("txt", &format!("{detail}\n--source--\n{text}"));
    let staged_source = stage(extension, text);
    for sample in 0..SAMPLES_PER_SIGNATURE {
        // Linking fails when the name exists, so each sample is claimed by one fuzzer.
        match std::fs::hard_link(&staged_record, dir.join(format!("{name}.{sample}.txt"))) {
            Ok(()) => {
                std::fs::rename(
                    &staged_source,
                    dir.join(format!("{name}.{sample}.{extension}")),
                )
                .expect("publish the finding's source");
                break;
            },
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {},
            Err(err) => panic!(
                "mono_oracle: cannot record a finding in {}: {err}",
                dir.display()
            ),
        }
    }
    let _ = std::fs::remove_file(&staged_record);
    let _ = std::fs::remove_file(&staged_source);
}

/// Wall-clock time of each run of an input.
struct Elapsed {
    v1: Duration,
    v2: Duration,
    small_heap: Duration,
}

/// Appends the stats record of an input to the `MASM_SMITH_STATS` file, if set.
fn write_stats(
    v1: &[Task],
    v2: &[Task],
    cross_vm: &Comparison,
    heap_size: usize,
    small_heap_tasks: &[Task],
    small_heap: &Comparison,
    elapsed: &Elapsed,
) {
    let Some(path) = stats::STATS_PATH.as_ref() else {
        return;
    };
    let micros = |duration: Duration| u64::try_from(duration.as_micros()).unwrap_or(u64::MAX);
    let statuses = |tasks: &[Task]| {
        tasks
            .iter()
            .map(|task| status_key(Some(task)))
            .collect::<Vec<_>>()
    };
    let gc_counts = |tasks: &[Task]| {
        tasks
            .iter()
            .filter_map(|task| task.gc_count)
            .collect::<Vec<_>>()
    };
    let (end, task, detail) = end_json(&cross_vm.end);
    let (small_heap_end, small_heap_task, small_heap_detail) = end_json(&small_heap.end);
    let record = json!({
        "tasks": v1.len(),
        "compared": cross_vm.compared,
        "end": end,
        "task": task,
        "detail": detail,
        "gc_counts": gc_counts(v2),
        "v1": statuses(v1),
        "v2": statuses(v2),
        "small_heap": {
            "heap_size": heap_size,
            "compared": small_heap.compared,
            "end": small_heap_end,
            "task": small_heap_task,
            "detail": small_heap_detail,
            "gc_counts": gc_counts(small_heap_tasks),
            "statuses": statuses(small_heap_tasks),
        },
        "micros": {
            "v1": micros(elapsed.v1),
            "v2": micros(elapsed.v2),
            "small_heap": micros(elapsed.small_heap),
        },
    });
    stats::append_stats(path, &record);
}

/// How a comparison ended, as the stats record shows it: the kind of end, its task, and its detail.
fn end_json(end: &End) -> (&'static str, Option<usize>, serde_json::Value) {
    match end {
        End::Agreed => ("agreed", None, json!(null)),
        End::Cut { task, kind, reason } => (
            "cut",
            Some(*task),
            json!({ "kind": kind, "reason": reason }),
        ),
        End::Allowed { task, divergence } => ("allowed", Some(*task), json!(divergence.signature)),
        End::Finding { task, signature } => ("finding", Some(*task), json!(signature)),
    }
}

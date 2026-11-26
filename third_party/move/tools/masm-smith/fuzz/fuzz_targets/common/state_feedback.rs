// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Coverage feedback from transaction outcomes, so the fuzzer keeps inputs that reach new VM
//! states even when they cover no new code.
//!
//! Each task's kind, status, position, change from the previous task's status, kinds of writes
//! (and whether they follow a module upgrade), and rough gas and GC counts become features in
//! libFuzzer's extra counters, kept apart per VM. Linux only; a no-op elsewhere.
//! `MASM_SMITH_NO_STATE_FEEDBACK` turns it off, for comparison runs.

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::LazyLock,
};

static ENABLED: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("MASM_SMITH_NO_STATE_FEEDBACK").is_none());

const COUNTERS: usize = 1 << 14;

#[cfg(target_os = "linux")]
#[used]
#[unsafe(link_section = "__libfuzzer_extra_counters")]
static mut EXTRA_COUNTERS: [u8; COUNTERS] = [0; COUNTERS];

/// Records the features of one run's captured output. `vm` names the VM that produced it, so
/// the same outcome on two VMs counts as two features.
pub fn record(vm: &str, output: &str) {
    if !*ENABLED {
        return;
    }
    for feature in features(output) {
        set(hash(&(vm, feature)));
    }
}

fn set(feature: u64) {
    let index = (feature % COUNTERS as u64) as usize;
    #[cfg(target_os = "linux")]
    // SAFETY: the fuzzer runs inputs on one thread, and libFuzzer only reads the counters
    // between inputs.
    unsafe {
        let counters = &raw mut EXTRA_COUNTERS;
        (*counters)[index] = 1;
    }
    #[cfg(not(target_os = "linux"))]
    let _ = index;
}

/// The features of one run: hashes of small tuples describing each task's outcome.
fn features(output: &str) -> Vec<u64> {
    let mut features = Vec::new();
    let mut previous = String::from("start");
    let mut upgraded = false;
    let mut published: Vec<String> = Vec::new();
    for (index, task) in tasks(output).iter().enumerate() {
        let position = index.min(15);
        if let Some(module) = task.published_module.as_ref() {
            upgraded |= published.contains(module);
            published.push(module.clone());
        }
        let status = task.status.as_deref().unwrap_or("ok");
        features.push(hash(&("task", position, task.kind, status)));
        features.push(hash(&("transition", &previous, task.kind, status)));
        features.push(hash(&("status", task.kind, status, upgraded)));
        for write in &task.writes {
            features.push(hash(&("write", write, upgraded)));
            features.push(hash(&("write_at", write, position)));
        }
        if let Some(gas) = task.gas_used {
            let magnitude = 64 - gas.leading_zeros();
            features.push(hash(&("gas", task.kind, status, magnitude)));
        }
        if let Some(collections) = task.gc_count {
            let magnitude = 64 - collections.leading_zeros();
            features.push(hash(&("gc", task.kind, status, magnitude)));
        }
        previous = format!("{}:{status}", task.kind);
    }
    features
}

/// The outcome-relevant parts of one task's output.
struct Task {
    kind: &'static str,
    published_module: Option<String>,
    status: Option<String>,
    writes: Vec<&'static str>,
    gas_used: Option<u64>,
    gc_count: Option<u64>,
}

fn tasks(output: &str) -> Vec<Task> {
    let mut tasks: Vec<Task> = Vec::new();
    for line in output.lines() {
        if let Some(header) = line.strip_prefix("task ") {
            let command = header
                .split_once(": ")
                .map_or("", |(_, command)| command.trim());
            let (kind, published_module) = if command.starts_with("publish") {
                let module = command
                    .split_once("[module ")
                    .and_then(|(_, rest)| rest.strip_suffix(']'))
                    .map(str::to_owned);
                ("publish", module)
            } else if command.starts_with("run ") && command.contains("::") {
                ("call", None)
            } else {
                ("script", None)
            };
            tasks.push(Task {
                kind,
                published_module,
                status: None,
                writes: Vec::new(),
                gas_used: None,
                gc_count: None,
            });
            continue;
        }
        let Some(task) = tasks.last_mut() else {
            continue;
        };
        let trimmed = line.trim();
        if let Some(status) = trimmed.strip_prefix("major_status: ") {
            task.status = Some(status.trim_end_matches(',').to_owned());
        } else if let Some(reason) = line.strip_prefix("Error: unsupported by MonoVM: ") {
            task.status = Some(format!("unsupported: {reason}"));
        } else if line.starts_with("Error: ") && task.status.is_none() {
            task.status = Some("ERROR".to_owned());
        } else if let Some(gas) = line.strip_prefix("gas used: ") {
            task.gas_used = gas.parse().ok();
        } else if let Some(collections) = line.strip_prefix("gc count: ") {
            task.gc_count = collections.parse().ok();
        } else if let Some(write) = line.strip_prefix("write ") {
            let kind = match write.split_whitespace().next() {
                Some("new") => "new",
                Some("modify") => "modify",
                Some("delete") => "delete",
                _ => "other",
            };
            task.writes.push(kind);
        }
    }
    tasks
}

pub fn hash(value: &impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#[cfg(target_os = "linux")]
use libc::{cpu_set_t, sched_setaffinity, setpriority, PRIO_PROCESS};

#[cfg(target_os = "linux")]
pub(crate) fn new_cpu_set() -> cpu_set_t {
    unsafe { std::mem::zeroed::<cpu_set_t>() }
}

#[cfg(target_os = "linux")]
pub(crate) fn pin_cpu_set(cpu_set: cpu_set_t) -> impl Fn() + Send + Sync + 'static {
    move || {
        unsafe {
            sched_setaffinity(
                0, // Defaults to current thread
                std::mem::size_of::<cpu_set_t>(),
                &cpu_set,
            );
        };
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn set_thread_nice_value(nice_value: i32) -> impl Fn() + Send + Sync + 'static {
    move || unsafe {
        setpriority(PRIO_PROCESS, 0, nice_value);
    }
}

/// Builds a `cpu_set_t` containing the given logical CPU IDs.
///
/// IDs outside the kernel's `CPU_SETSIZE` (typically 1024) are silently
/// dropped — writing to those bits would be out-of-bounds.
#[cfg(target_os = "linux")]
pub fn cpu_set_from_ids(ids: &[usize]) -> cpu_set_t {
    let mut set = new_cpu_set();
    let max = libc::CPU_SETSIZE as usize;
    for &id in ids {
        if id < max {
            unsafe { libc::CPU_SET(id, &mut set) };
        }
    }
    set
}

/// Outcome of planning a physical-core pin for a thread pool.
#[derive(Debug, Clone)]
pub enum PhysicalCorePinPlan {
    /// Not supported on this platform (non-Linux).
    NotSupported,
    /// Couldn't enumerate allowed CPUs or sysfs topology.
    DetectionFailed,
    /// Fewer physical cores are available than threads requested; pinning
    /// would oversubscribe and hurt performance, so skip it.
    SkippedFewerCores {
        physical_cores: usize,
        requested_threads: usize,
    },
    /// Will pin threads so the effective CPU set is exactly these logical CPU
    /// IDs — one representative per physical core, intersected with the
    /// process's allowed CPUs.
    Apply { cpu_ids: Vec<usize> },
}

impl PhysicalCorePinPlan {
    pub fn applied_cpus(&self) -> Option<&[usize]> {
        match self {
            PhysicalCorePinPlan::Apply { cpu_ids } => Some(cpu_ids),
            _ => None,
        }
    }
}

/// Returns the set of logical CPU IDs representing distinct *physical* cores
/// among the CPUs currently allowed to run this process.
///
/// Properties:
/// - Respects `sched_getaffinity()`, so taskset / numactl / cgroup (cpuset)
///   restrictions are honored — we never return a CPU the kernel won't let us
///   run on. `core_affinity::get_core_ids()` is the source of the allowed set
///   and is itself `sched_getaffinity`-based on Linux.
/// - For each physical core with at least one allowed hyperthread sibling,
///   returns exactly one representative logical CPU (the lowest-numbered
///   allowed sibling). This is what lets a caller avoid scheduling onto two
///   siblings of the same physical core.
/// - Reads `/sys/devices/system/cpu/cpuN/topology/thread_siblings_list` to
///   identify siblings. If sysfs topology for a CPU is unavailable, that CPU
///   is treated as its own physical core (degrades gracefully).
///
/// Returns `None` if we cannot enumerate allowed CPUs at all (should be rare).
#[cfg(target_os = "linux")]
pub fn physical_core_cpu_ids() -> Option<Vec<usize>> {
    use std::collections::BTreeSet;

    let allowed: BTreeSet<usize> = core_affinity::get_core_ids()?
        .into_iter()
        .map(|c| c.id)
        .collect();
    if allowed.is_empty() {
        return None;
    }

    let mut representatives = BTreeSet::<usize>::new();
    let mut processed = BTreeSet::<usize>::new();

    for &cpu in &allowed {
        if processed.contains(&cpu) {
            continue;
        }
        let siblings = read_thread_siblings(cpu).unwrap_or_else(|| vec![cpu]);
        let mut allowed_siblings: Vec<usize> = siblings
            .into_iter()
            .filter(|c| allowed.contains(c))
            .collect();
        if allowed_siblings.is_empty() {
            // Shouldn't happen — `cpu` itself is in `allowed` — but guard anyway.
            allowed_siblings.push(cpu);
        }
        for c in &allowed_siblings {
            processed.insert(*c);
        }
        let rep = *allowed_siblings.iter().min().expect("non-empty by above");
        representatives.insert(rep);
    }

    Some(representatives.into_iter().collect())
}

#[cfg(target_os = "linux")]
fn read_thread_siblings(cpu: usize) -> Option<Vec<usize>> {
    let path = format!(
        "/sys/devices/system/cpu/cpu{}/topology/thread_siblings_list",
        cpu
    );
    let contents = std::fs::read_to_string(path).ok()?;
    parse_cpu_list(contents.trim())
}

/// Parses a Linux cpulist (as used by sysfs topology files and
/// `/proc/self/status`'s `Cpus_allowed_list`): comma-separated entries, each
/// being either `N` or `lo-hi`. Returns sorted, deduped IDs. Returns `None`
/// on malformed input or empty string.
#[cfg(target_os = "linux")]
fn parse_cpu_list(s: &str) -> Option<Vec<usize>> {
    use std::collections::BTreeSet;
    if s.is_empty() {
        return None;
    }
    let mut out = BTreeSet::<usize>::new();
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            return None;
        }
        if let Some((lo, hi)) = part.split_once('-') {
            let lo: usize = lo.trim().parse().ok()?;
            let hi: usize = hi.trim().parse().ok()?;
            if hi < lo {
                return None;
            }
            for i in lo..=hi {
                out.insert(i);
            }
        } else {
            out.insert(part.parse().ok()?);
        }
    }
    Some(out.into_iter().collect())
}

/// Build a thread-start hook that pins the calling thread's CPU affinity to
/// one logical CPU per physical core (among CPUs allowed to the process).
///
/// Intended to be plugged into a rayon `start_handler` or tokio
/// `on_thread_start`. The returned closure is cheap to call per thread — the
/// underlying `cpu_set_t` is captured once.
///
/// The plan is returned alongside the hook so the caller can log / emit
/// metrics describing what happened. If `num_threads > available_physical_cores`,
/// the hook is a no-op (we'd oversubscribe).
///
/// When this returns a no-op hook (non-Linux, detection failure, or skip),
/// threads inherit the default OS scheduling, which is the intended fallback.
#[cfg(target_os = "linux")]
pub fn physical_core_pin_hook(
    num_threads: usize,
) -> (PhysicalCorePinPlan, impl Fn() + Send + Sync + 'static) {
    let (plan, cpu_set) = match physical_core_cpu_ids() {
        None => (PhysicalCorePinPlan::DetectionFailed, None),
        Some(ids) if ids.is_empty() => (PhysicalCorePinPlan::DetectionFailed, None),
        Some(ids) if ids.len() < num_threads => (
            PhysicalCorePinPlan::SkippedFewerCores {
                physical_cores: ids.len(),
                requested_threads: num_threads,
            },
            None,
        ),
        Some(ids) => {
            let set = cpu_set_from_ids(&ids);
            (PhysicalCorePinPlan::Apply { cpu_ids: ids }, Some(set))
        },
    };
    let hook = move || {
        if let Some(set) = cpu_set {
            unsafe {
                sched_setaffinity(0, std::mem::size_of::<cpu_set_t>(), &set);
            }
        }
    };
    (plan, hook)
}

#[cfg(not(target_os = "linux"))]
pub fn physical_core_pin_hook(
    _num_threads: usize,
) -> (PhysicalCorePinPlan, impl Fn() + Send + Sync + 'static) {
    (PhysicalCorePinPlan::NotSupported, || {})
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn parse_cpu_list_single() {
        assert_eq!(parse_cpu_list("0"), Some(vec![0]));
        assert_eq!(parse_cpu_list("42"), Some(vec![42]));
    }

    #[test]
    fn parse_cpu_list_range() {
        assert_eq!(parse_cpu_list("0-3"), Some(vec![0, 1, 2, 3]));
        assert_eq!(parse_cpu_list("7-7"), Some(vec![7]));
    }

    #[test]
    fn parse_cpu_list_mixed() {
        assert_eq!(parse_cpu_list("0,2,4-6,8"), Some(vec![0, 2, 4, 5, 6, 8]));
        // Typical thread_siblings_list on an SMT machine:
        assert_eq!(parse_cpu_list("0,64"), Some(vec![0, 64]));
    }

    #[test]
    fn parse_cpu_list_dedup_and_sort() {
        assert_eq!(parse_cpu_list("3,1,3,0-1"), Some(vec![0, 1, 3]));
    }

    #[test]
    fn parse_cpu_list_malformed() {
        assert_eq!(parse_cpu_list(""), None);
        assert_eq!(parse_cpu_list(","), None);
        assert_eq!(parse_cpu_list("a"), None);
        assert_eq!(parse_cpu_list("5-3"), None);
        assert_eq!(parse_cpu_list("1,,2"), None);
    }

    #[test]
    fn physical_core_cpu_ids_nonempty() {
        // On any Linux host there should be at least one physical core.
        let ids = physical_core_cpu_ids().expect("sched_getaffinity should succeed");
        assert!(!ids.is_empty());
        // The representatives should be a subset of the allowed CPU set, and
        // each should be a valid CPU id.
        let allowed: std::collections::BTreeSet<usize> = core_affinity::get_core_ids()
            .unwrap()
            .into_iter()
            .map(|c| c.id)
            .collect();
        for id in &ids {
            assert!(allowed.contains(id), "rep {} not in allowed set", id);
        }
        // Representatives should be <= allowed (since we fold SMT siblings).
        assert!(ids.len() <= allowed.len());
    }
}

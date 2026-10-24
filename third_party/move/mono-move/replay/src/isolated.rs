// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Runs MonoMove (V2) in a child process, under a memory and a time limit.
//!
//! A MonoMove bug can exhaust memory or never finish (e.g. aptos-labs/aptos-core#20699), which
//! `catch_unwind` cannot contain: in process, one such transaction would take the whole capture or
//! comparison down. In a child process it becomes a result, [`V2Outcome::Crashed`], and the run
//! goes on.
//!
//! The child is this binary, re-run with the hidden `v2-worker` subcommand. It builds the patched
//! state from a [`ReplayInput`] exactly as the parent does for V1. Protocol:
//! - The parent writes the request to the child's stdin, length-prefixed, so the child never waits
//!   for an end of input that a sibling holding a leaked pipe end could withhold.
//! - The child moves its stdout aside as a private channel (stray prints go to stderr) and sends
//!   length-prefixed [`Frame`]s on it: each key the first time MonoMove reads it, then the outcome.
//!   Reads are reported as they happen, so a child killed mid-run still leaves them.
//! - Both sides enforce the limits: the parent kills the child, and a watchdog thread in the child
//!   exits it, so a child whose parent died cannot run unbounded.
//! - Children run only while their memory limits fit a shared budget, a share of the host's (or its
//!   container's) memory, so that concurrent children cannot exhaust it together.
//!
//! A failure of the harness itself (the worker cannot be started, or breaks the protocol) is a
//! [`HarnessError`], not a crash: it says nothing about MonoMove, and callers stop on it.

use crate::{
    comparison::UnsupportedKind,
    overrides::{OverrideConfig, PatchedState},
    panic_message, v2,
};
use anyhow::{anyhow, bail, Context, Result};
use aptos_infallible::Mutex;
use aptos_transaction_simulation::InMemoryStateStore;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{
        AuxiliaryInfo, PersistedAuxiliaryInfo, Transaction, TransactionOutput, TransactionStatus,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, VecDeque},
    fmt,
    fs::File,
    io::{Read, Write},
    panic::{catch_unwind, AssertUnwindSafe},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{mpsc, Arc, Condvar, OnceLock, PoisonError},
    time::{Duration, Instant},
};

/// The subcommand that runs one MonoMove replay as a child process.
pub const WORKER_SUBCOMMAND: &str = "v2-worker";

/// How often each side checks the limits.
const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// How long the parent waits, after the child exits by itself, for its outcome: it is already in
/// the pipe, so this only bounds a parent too loaded to read it.
const RESULT_TIMEOUT: Duration = Duration::from_secs(60);

/// How long the parent waits for the rest of a killed child's output, which a process the child
/// started could hold open.
const DRAIN_TIMEOUT: Duration = Duration::from_secs(2);

/// How much of the child's stderr is kept for a crash's reason.
const STDERR_TAIL: usize = 4_096;

/// The most distinct keys a run may read: past it, MonoMove is reading without end.
const MAX_READS: usize = 1_000_000;

/// The largest frame the parent accepts; a larger length is a corrupt stream.
const MAX_FRAME_BYTES: u64 = 1 << 34;

/// Exit codes of a child that stopped itself at a limit.
const EXIT_TIME_LIMIT: i32 = 97;
const EXIT_MEMORY_LIMIT: i32 = 98;

/// The version of the protocol between parent and worker, which the worker announces first: a
/// worker of another version is a harness failure, not a MonoMove one.
const PROTOCOL_VERSION: u32 = 2;

/// A crashed run's reads are completed only up to this many: past it, MonoMove is taken to have
/// run away, and its reads would only flood completion.
pub const CRASH_FETCH_LIMIT: usize = 10_000;

/// The share of the host's memory all concurrent children may use together.
const HOST_MEMORY_SHARE: (u64, u64) = (3, 4);

const MB: u64 = 1024 * 1024;

/// The least a replay reserves for its worker's start-up, beyond MonoMove's limit: the allowance
/// for the smallest request (see [`startup_memory_bytes`]).
pub const MIN_STARTUP_MB: u64 = 257;

/// A failure of the isolation harness, as opposed to anything MonoMove did.
#[derive(Debug)]
pub struct HarnessError(String);

impl fmt::Display for HarnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MonoMove harness: {}", self.0)
    }
}

impl std::error::Error for HarnessError {}

/// Whether `err` is a failure of the harness, on which callers stop rather than recording a result.
pub fn is_harness_error(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| cause.is::<HarnessError>())
}

/// The limits one MonoMove run is held to.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct V2Limits {
    pub memory_mb: u64,
    /// MonoMove's time limit, from when the worker starts it (its `Hello`).
    pub timeout: Duration,
    /// How long a worker may take to receive its request and start MonoMove: a large request
    /// cannot eat into MonoMove's own time limit, and a worker that misses this is a harness
    /// failure.
    pub startup_timeout: Duration,
}

impl Default for V2Limits {
    fn default() -> Self {
        Self {
            memory_mb: 4_096,
            timeout: Duration::from_secs(120),
            startup_timeout: Duration::from_secs(120),
        }
    }
}

impl V2Limits {
    /// The limits, with the memory limit lowered to the memory budget all children share, so that
    /// one run can always start. Every command uses the same limits, so that a record is held to
    /// the same limit when its state is completed as when it is compared.
    pub fn fit_to_host(self) -> Self {
        // The host's share, not a budget set with `set_memory_budget_mb`: that one only limits how
        // many replays run at once, and must not change what each is allowed.
        // Room is left for the worker's start-up allowance, which its reservation includes.
        let fit_mb = host_memory_budget_mb().saturating_sub(MIN_STARTUP_MB);
        if fit_mb >= self.memory_mb {
            return self;
        }
        eprintln!(
            "note: lowering the MonoMove memory limit from {} MB to {} MB, to fit this host's \
             share of its memory",
            self.memory_mb, fit_mb
        );
        Self {
            memory_mb: fit_mb.max(1),
            ..self
        }
    }

    fn memory_bytes(&self) -> u64 {
        self.memory_mb.saturating_mul(MB)
    }

    fn time_limit_reason(&self) -> String {
        format!(
            "MonoMove exceeded the time limit ({} s)",
            self.timeout.as_secs()
        )
    }

    fn memory_limit_reason(&self) -> String {
        format!("MonoMove exceeded the memory limit ({} MB)", self.memory_mb)
    }
}

/// Why a worker that used `bytes` before `Hello` was stopped.
fn startup_overrun(bytes: u64, startup_bytes: u64) -> String {
    format!(
        "the worker used {} MB to receive its request, more than its {} MB",
        bytes / MB,
        startup_bytes / MB
    )
}

/// Why a worker that read more than [`MAX_READS`] keys was stopped.
fn read_overflow() -> String {
    format!("MonoMove read more than {} distinct keys", MAX_READS)
}

static BUDGET: OnceLock<u64> = OnceLock::new();

/// Sets the memory all concurrent children of this process may use together, in MB, instead of
/// the share of the host's memory. The budget is per process: processes sharing a host should
/// split it between them. Must be called before the first replay.
pub fn set_memory_budget_mb(mb: u64) -> Result<()> {
    BUDGET
        .set(mb)
        .map_err(|_| anyhow!("the MonoMove memory budget is already set"))
}

/// The memory all concurrent children of this process may use together, in MB: by default a
/// share of the host's (or container's) memory, unbounded if that is unknown.
fn memory_budget_mb() -> u64 {
    *BUDGET.get_or_init(host_memory_budget_mb)
}

/// A share of the host's (or container's) memory, in MB; unbounded if that is unknown.
fn host_memory_budget_mb() -> u64 {
    static HOST: OnceLock<u64> = OnceLock::new();
    *HOST.get_or_init(|| {
        let (num, den) = HOST_MEMORY_SHARE;
        match host_memory_bytes() / MB {
            0 => u64::MAX,
            mb => mb.saturating_mul(num) / den,
        }
    })
}

/// The memory this process may use: the host's, or its container's limit if lower.
fn host_memory_bytes() -> u64 {
    use sysinfo::SystemExt;
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    let total = system.total_memory();
    // Without the process's own cgroup, at least the root's limit applies.
    let cgroups = std::fs::read_to_string("/proc/self/cgroup")
        .unwrap_or_else(|_| "0::/\n1:memory:/\n".to_string());
    total.min(cgroup_memory_limit(&cgroups, Path::new("/sys/fs/cgroup")))
}

/// The lowest memory limit on the path of this process's cgroup, from its own cgroup up to the
/// root, as `/proc/self/cgroup` (`cgroups`) names it under `root`: a limit applies to every cgroup
/// below it. cgroup v2 names the path on its `0::` line and limits it in `memory.max`; v1 names it
/// on the `memory` controller's line and limits it in `memory/…/memory.limit_in_bytes`. "max", or
/// v1's huge default, means no limit.
fn cgroup_memory_limit(cgroups: &str, root: &Path) -> u64 {
    let mut lowest = u64::MAX;
    for line in cgroups.lines() {
        let mut fields = line.splitn(3, ':');
        let (Some(_), Some(controllers), Some(path)) =
            (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        let (base, file) = if controllers.is_empty() {
            (root.to_path_buf(), "memory.max")
        } else if controllers
            .split(',')
            .any(|controller| controller == "memory")
        {
            (root.join("memory"), "memory.limit_in_bytes")
        } else {
            continue;
        };
        let mut dir = base.join(path.trim_start_matches('/'));
        loop {
            if let Some(limit) = std::fs::read_to_string(dir.join(file))
                .ok()
                .and_then(|limit| limit.trim().parse::<u64>().ok())
                .filter(|limit| *limit > 0)
            {
                lowest = lowest.min(limit);
            }
            if dir == base || !dir.pop() {
                break;
            }
        }
    }
    lowest
}

/// Memory reserved from the budget for one running child; released on drop.
struct Reservation(u64);

static RESERVED: std::sync::Mutex<u64> = std::sync::Mutex::new(0);
static RELEASED: Condvar = Condvar::new();

impl Reservation {
    /// Waits until `mb` fits in the budget next to the running children; a child always runs if
    /// none else is, so that a limit above the budget cannot wait forever.
    fn acquire(mb: u64) -> Self {
        let budget = memory_budget_mb();
        let mut reserved = RESERVED.lock().unwrap_or_else(PoisonError::into_inner);
        while *reserved > 0 && reserved.saturating_add(mb) > budget {
            reserved = RELEASED
                .wait(reserved)
                .unwrap_or_else(PoisonError::into_inner);
        }
        *reserved = reserved.saturating_add(mb);
        Self(mb)
    }
}

impl Drop for Reservation {
    fn drop(&mut self) {
        let mut reserved = RESERVED.lock().unwrap_or_else(PoisonError::into_inner);
        *reserved = reserved.saturating_sub(self.0);
        RELEASED.notify_all();
    }
}

/// Everything a replay of one record runs on. Both VMs build their state from it the same way,
/// through [`ReplayInput::patched`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayInput {
    /// The record's state, before the overrides.
    pub values: Vec<(StateKey, StateValue)>,
    pub absent: BTreeSet<StateKey>,
    pub config: OverrideConfig,
    pub txn: Transaction,
    pub aux_info: PersistedAuxiliaryInfo,
}

impl ReplayInput {
    /// The state every replay of the record runs on.
    pub fn patched(&self) -> Result<PatchedState> {
        PatchedState::new(
            InMemoryStateStore::new_with_state_values(self.values.iter().cloned()),
            &self.config,
            &self.absent,
        )
    }
}

/// What one MonoMove run did, and the keys it read.
#[derive(Debug)]
pub struct V2Report {
    pub outcome: V2Outcome,
    /// Every key MonoMove read, also for a crash: reported as they were read.
    pub reads: BTreeSet<StateKey>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum V2Outcome {
    /// MonoMove ran.
    Ran(V2Ran),
    /// The executor could not be set up.
    SetupFailed(String),
    Panicked(String),
    /// The child exceeded a limit or died; the reason says which.
    Crashed(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct V2Ran {
    /// The materialized output; `Err` only when materialization failed.
    pub output: std::result::Result<TransactionOutput, String>,
    /// The gap MonoMove hit, which makes the output meaningless to compare.
    #[serde(with = "json_string")]
    pub unsupported: Option<UnsupportedKind>,
    /// For a gap hit while writing the output: the status the execution committed with, final
    /// before the gap.
    pub output_gap_status: Option<TransactionStatus>,
    pub vm_error: Option<String>,
}

/// BCS cannot decode the internally tagged [`UnsupportedKind`], so it crosses as JSON.
mod json_string {
    use crate::comparison::UnsupportedKind;
    use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(
        kind: &Option<UnsupportedKind>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        kind.as_ref()
            .map(|kind| serde_json::to_string(kind).map_err(serde::ser::Error::custom))
            .transpose()?
            .serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<UnsupportedKind>, D::Error> {
        Option::<String>::deserialize(deserializer)?
            .map(|json| serde_json::from_str(&json).map_err(D::Error::custom))
            .transpose()
    }
}

/// What the parent sends the child, borrowed to serialize it; [`Request`] is the same, owned.
#[derive(Serialize)]
struct RequestRef<'a> {
    input: &'a ReplayInput,
    limits: &'a V2Limits,
}

#[derive(Deserialize)]
struct Request {
    input: ReplayInput,
    limits: V2Limits,
}

/// What the child sends the parent, in order: `Hello` once its channel is up, reads as they happen,
/// then one `Done` or `Failed`.
#[derive(Serialize, Deserialize)]
enum Frame {
    /// The worker started and speaks this protocol version: from here on, its exit is MonoMove's.
    /// With the worker's memory then, its state prepared: MonoMove's memory limit applies above it.
    Hello(u32, u64),
    Read(StateKey),
    Done(V2Outcome),
    /// The worker could not run the request: a harness failure.
    Failed(String),
}

/// Runs MonoMove on `input` in a child process held to `limits`: this binary, re-run as the
/// worker. `Err` only for a [`HarnessError`].
pub fn run_v2(input: &ReplayInput, limits: &V2Limits) -> Result<V2Report> {
    let worker = worker_binary().map_err(harness)?;
    run_v2_with(worker, input, limits)
}

/// [`run_v2`] with `worker` as the child binary, which must handle [`WORKER_SUBCOMMAND`]. Tests
/// pass the built binary, since their running binary is the test harness.
pub fn run_v2_with(worker: &Path, input: &ReplayInput, limits: &V2Limits) -> Result<V2Report> {
    // Reserved first, so that a replay waiting for the budget does not already hold its request;
    // sized from the request without building it.
    let request_bytes = bcs::serialized_size(&RequestRef { input, limits })
        .map_err(|err| harness(err.into()))? as u64;
    let startup_bytes = startup_memory_bytes(request_bytes);
    // The most the worker may hold at once: its start-up allowance (which bounds the baseline it
    // keeps after `Hello` too), plus MonoMove's limit above that baseline.
    let _reservation =
        Reservation::acquire(limits.memory_mb.saturating_add(startup_bytes.div_ceil(MB)));
    let request =
        bcs::to_bytes(&RequestRef { input, limits }).map_err(|err| harness(err.into()))?;
    let mut child = spawn(worker).map_err(harness)?;
    let result = supervise(&mut child, request, startup_bytes, limits).map_err(harness);
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

fn harness(err: anyhow::Error) -> anyhow::Error {
    if is_harness_error(&err) {
        return err;
    }
    anyhow::Error::new(HarnessError(format!("{:#}", err)))
}

/// Identifies the worker binary now, so that a rebuild after a command starts is noticed too;
/// commands that replay MonoMove call it first.
pub fn pin_worker() -> Result<()> {
    worker_binary().map(|_| ()).map_err(harness)
}

/// This binary, as the worker. A rebuild during a long run must not swap the MonoMove the
/// comparison runs: on Linux `/proc/self/exe` is the running binary even once its file is
/// replaced; elsewhere the file is identified on first use, and a run whose binary changed since
/// stops rather than mixing two MonoMoves.
fn worker_binary() -> Result<&'static Path> {
    static WORKER: OnceLock<std::result::Result<(PathBuf, Option<FileIdentity>), String>> =
        OnceLock::new();
    let (path, identity) = WORKER
        .get_or_init(|| {
            if cfg!(target_os = "linux") {
                return Ok((PathBuf::from("/proc/self/exe"), None));
            }
            let path = std::env::current_exe().map_err(|err| format!("{}", err))?;
            let identity = FileIdentity::of(&path).map_err(|err| format!("{:#}", err))?;
            Ok((path, Some(identity)))
        })
        .as_ref()
        .map_err(|err| anyhow!("failed to locate the running binary: {}", err))?;
    if let Some(identity) = identity {
        if FileIdentity::of(path)? != *identity {
            bail!(
                "{:?} changed during the run (rebuilt?); its MonoMove may differ, so the run stops",
                path
            );
        }
    }
    Ok(path)
}

/// What identifies a file's contents without reading them: replacing or rewriting it changes one.
#[derive(Debug, PartialEq, Eq)]
struct FileIdentity {
    device: u64,
    inode: u64,
    len: u64,
    modified: Option<std::time::SystemTime>,
}

impl FileIdentity {
    fn of(path: &Path) -> Result<Self> {
        use std::os::unix::fs::MetadataExt;
        let metadata =
            std::fs::metadata(path).with_context(|| format!("failed to inspect {:?}", path))?;
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            len: metadata.len(),
            modified: metadata.modified().ok(),
        })
    }
}

fn spawn(worker: &Path) -> Result<Child> {
    let mut attempt = 0;
    loop {
        match Command::new(worker)
            .arg(WORKER_SUBCOMMAND)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => return Ok(child),
            // A just-written executable can be briefly busy while another thread forks.
            Err(err) if err.raw_os_error() == Some(libc::ETXTBSY) && attempt < 10 => {
                attempt += 1;
                std::thread::sleep(Duration::from_millis(20));
            },
            Err(err) => {
                return Err(err).with_context(|| format!("failed to start {:?}", worker));
            },
        }
    }
}

/// What the parent collected from the child's channel.
#[derive(Default)]
struct Collected {
    /// When the worker announced itself, and its memory baseline then: before it, an exit is a
    /// harness failure, and MonoMove's time limit counts from it.
    hello: Option<(Instant, u64)>,
    reads: BTreeSet<StateKey>,
    /// Whether the child read more than [`MAX_READS`] distinct keys.
    overflowed: bool,
    done: Option<V2Outcome>,
    failed: Option<String>,
    malformed: Option<String>,
}

fn supervise(
    child: &mut Child,
    request: Vec<u8>,
    startup_bytes: u64,
    limits: &V2Limits,
) -> Result<V2Report> {
    // The request is written on a thread, so a child that dies before reading it all cannot
    // block the parent; a write error then just means the child is gone.
    let mut stdin = child.stdin.take().context("child stdin")?;
    std::thread::spawn(move || {
        let _ = stdin.write_all(&(request.len() as u64).to_le_bytes());
        let _ = stdin.write_all(&request);
    });

    // The channel's reader signals once the outcome arrives or the channel ends; the stderr
    // reader, once stderr ends.
    let collected = Arc::new(Mutex::new(Collected::default()));
    let (settled, settled_rx) = mpsc::channel();
    let mut channel = std::io::BufReader::new(child.stdout.take().context("child stdout")?);
    let sink = collected.clone();
    std::thread::spawn(move || read_frames(&mut channel, &sink, &settled));

    let tail = Arc::new(Mutex::new(VecDeque::with_capacity(STDERR_TAIL)));
    let (stderr_done, stderr_done_rx) = mpsc::channel();
    let mut stderr = child.stderr.take().context("child stderr")?;
    let sink = tail.clone();
    std::thread::spawn(move || {
        keep_tail(&mut stderr, &sink);
        let _ = stderr_done.send(());
    });

    let mut memory = MemoryProbe::new();
    let started = Instant::now();
    let mut killed = None;
    let mut killed_before_hello = false;
    // Whether the previous poll found the worker over its start-up allowance before `Hello`.
    let mut over_startup = false;
    // When the parent first saw the worker's outcome.
    let mut reported_at = None;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        // Memory first, then a snapshot of the channel, which also keeps the reader unblocked while
        // memory is measured. A `Hello` the worker sent may not have been read yet, so an overrun of
        // the start-up allowance stops it only if it lasts to the next poll, by when it would have.
        let bytes = memory.bytes(child.id());
        let (overflowed, hello, broken, reported) = {
            let collected = collected.lock();
            (
                collected.overflowed,
                collected.hello,
                collected.failed.is_some() || collected.malformed.is_some(),
                collected.done.is_some(),
            )
        };
        // A baseline above the start-up allowance escaped it between two samples: the worker would
        // hold more than it reserved, so it is stopped, as at start-up.
        if let Some((_, baseline)) = hello
            && baseline > startup_bytes
        {
            killed = Some(startup_overrun(baseline, startup_bytes));
            killed_before_hello = true;
            let _ = child.kill();
            break child.wait()?;
        }
        if broken {
            // The worker cannot be trusted past a broken protocol: it is stopped at once, and the
            // failure reported below.
            let _ = child.kill();
            break child.wait()?;
        }
        // Once the worker has reported its outcome it is no longer held to the limits: one exceeded
        // before would have stopped it, and time spent reading the outcome is not MonoMove's. It is
        // only given a moment to exit.
        if reported {
            if reported_at.get_or_insert_with(Instant::now).elapsed() > DRAIN_TIMEOUT {
                let _ = child.kill();
                break child.wait()?;
            }
            std::thread::sleep(POLL_INTERVAL);
            continue;
        }
        if overflowed {
            killed = Some(read_overflow());
        } else if let Some(reason) =
            bytes.and_then(|bytes| over_memory(hello, bytes, startup_bytes, limits))
        {
            if hello.is_some() || over_startup {
                killed = Some(reason);
            } else {
                over_startup = true;
            }
        } else {
            // Within its allowance: an overrun must last two consecutive polls.
            over_startup = false;
            if let Some(reason) = timed_out(hello.map(|(at, _)| at), started, limits) {
                killed = Some(reason);
            }
        }
        if killed.is_some() {
            killed_before_hello = hello.is_none();
            let _ = child.kill();
            break child.wait()?;
        }
        std::thread::sleep(POLL_INTERVAL);
    };
    // A child that exited by itself has already written its outcome, so it is waited for; a
    // killed child's channel could be held open by a process it started, so only briefly.
    let wait = if killed.is_some() {
        DRAIN_TIMEOUT
    } else {
        RESULT_TIMEOUT
    };
    let _ = settled_rx.recv_timeout(wait);
    let _ = stderr_done_rx.recv_timeout(DRAIN_TIMEOUT);

    let collected = std::mem::take(&mut *collected.lock());
    if let Some(err) = collected.failed {
        bail!("the worker could not run the request: {}", err);
    }
    if let Some(err) = collected.malformed {
        bail!("the worker broke the protocol: {}", err);
    }
    // A worker that never announced itself did not get to run MonoMove: whatever ended it is the
    // harness's, a kill at a limit included, and so is a kill decided before `Hello`, even if a
    // `Hello` was already on its way.
    if collected.hello.is_none() || killed_before_hello {
        bail!(
            "the worker did not start ({}{}): {}",
            status,
            killed
                .map(|reason| format!(", {}", reason))
                .unwrap_or_default(),
            stderr_text(&tail.lock())
        );
    }
    // A worker that exited before a poll saw its baseline is held to the start-up allowance too.
    if let Some((_, baseline)) = collected.hello
        && baseline > startup_bytes
    {
        bail!("{}", startup_overrun(baseline, startup_bytes));
    }
    let outcome = if collected.overflowed {
        // Past the cap, the reads are incomplete whatever the outcome: MonoMove ran away.
        V2Outcome::Crashed(read_overflow())
    } else {
        // A kill at a limit, decided before the outcome was read, wins over an outcome the worker
        // managed to write: the run exceeded the limit, which is the verdict.
        match (killed, collected.done) {
            (Some(reason), _) => V2Outcome::Crashed(reason),
            (None, Some(outcome)) => outcome,
            (None, None) => V2Outcome::Crashed(crash_reason(status, limits, &tail.lock())?),
        }
    };
    Ok(V2Report {
        outcome,
        reads: collected.reads,
    })
}

/// What a worker may use before `Hello`, while it receives and decodes its request of
/// `request_bytes`: the request raw and decoded, with room to spare. MonoMove's own limit applies
/// only from `Hello`, since the request is the harness's data.
fn startup_memory_bytes(request_bytes: u64) -> u64 {
    request_bytes
        .saturating_mul(3)
        .saturating_add((MIN_STARTUP_MB - 1) * MB)
}

/// Why the child must be stopped for memory, if it must: before `Hello` it is held to
/// `startup_bytes`, after it to MonoMove's memory limit above the baseline `Hello` reported.
fn over_memory(
    hello: Option<(Instant, u64)>,
    bytes: u64,
    startup_bytes: u64,
    limits: &V2Limits,
) -> Option<String> {
    match hello {
        None => (bytes > startup_bytes).then(|| startup_overrun(bytes, startup_bytes)),
        Some((_, baseline)) => (bytes.saturating_sub(baseline) > limits.memory_bytes())
            .then(|| limits.memory_limit_reason()),
    }
}

/// Why the child must be stopped for time, if it must: before `Hello` (at `hello`) it is held to the
/// startup limit, from `started`; after it, MonoMove is held to its time limit, from `Hello`.
fn timed_out(hello: Option<Instant>, started: Instant, limits: &V2Limits) -> Option<String> {
    match hello {
        None => (started.elapsed() > limits.startup_timeout).then(|| {
            format!(
                "the worker did not start MonoMove within {} s",
                limits.startup_timeout.as_secs()
            )
        }),
        Some(hello) => (hello.elapsed() > limits.timeout).then(|| limits.time_limit_reason()),
    }
}

/// Why a child that exited without an outcome did so.
fn crash_reason(status: ExitStatus, limits: &V2Limits, tail: &VecDeque<u8>) -> Result<String> {
    match status.code() {
        Some(EXIT_TIME_LIMIT) => Ok(limits.time_limit_reason()),
        Some(EXIT_MEMORY_LIMIT) => Ok(limits.memory_limit_reason()),
        // A worker that exits normally always sends an outcome first.
        Some(0) => bail!("the worker exited without a result"),
        _ => Ok(format!(
            "MonoMove's process died ({}): {}",
            status,
            stderr_text(tail)
        )),
    }
}

fn stderr_text(tail: &VecDeque<u8>) -> String {
    let (front, back) = tail.as_slices();
    String::from_utf8_lossy(&[front, back].concat())
        .trim()
        .to_string()
}

/// Reads frames into `sink`, signalling `settled` once the outcome arrives or the channel ends.
fn read_frames(channel: &mut impl Read, sink: &Mutex<Collected>, settled: &mpsc::Sender<()>) {
    loop {
        let mut len = [0u8; 8];
        // A frame cut short means the child died while sending it, which its exit explains.
        if channel.read_exact(&mut len).is_err() {
            break;
        }
        let len = u64::from_le_bytes(len);
        if len > MAX_FRAME_BYTES {
            sink.lock().malformed = Some(format!("a frame of {} bytes", len));
            break;
        }
        // Grown as the bytes arrive, so a corrupt length allocates only what was actually sent.
        let mut bytes = Vec::new();
        match channel.by_ref().take(len).read_to_end(&mut bytes) {
            Ok(read) if read as u64 == len => {},
            Ok(_) | Err(_) => break,
        }
        let mut collected = sink.lock();
        match bcs::from_bytes::<Frame>(&bytes) {
            Ok(Frame::Hello(version, baseline)) if version == PROTOCOL_VERSION => {
                collected.hello = Some((Instant::now(), baseline))
            },
            Ok(Frame::Hello(version, _)) => {
                collected.malformed = Some(format!(
                    "a worker of protocol version {}, not {}",
                    version, PROTOCOL_VERSION
                ));
                break;
            },
            Ok(Frame::Read(key)) => {
                if collected.reads.len() < MAX_READS {
                    collected.reads.insert(key);
                } else {
                    collected.overflowed = true;
                }
            },
            Ok(Frame::Done(outcome)) => {
                collected.done = Some(outcome);
                let _ = settled.send(());
            },
            Ok(Frame::Failed(err)) => {
                collected.failed = Some(err);
                let _ = settled.send(());
            },
            Err(err) => {
                // A `Hello` of another layout still names its version: its first field.
                collected.malformed = Some(match (bytes.first(), bytes.get(1..5)) {
                    (Some(0), Some(version)) => format!(
                        "a worker of protocol version {}, not {}",
                        u32::from_le_bytes(version.try_into().expect("four bytes")),
                        PROTOCOL_VERSION
                    ),
                    _ => format!("undecodable frame: {}", err),
                });
                break;
            },
        }
    }
    let _ = settled.send(());
}

/// Keeps only the last [`STDERR_TAIL`] bytes, so a child writing without end costs the parent
/// nothing.
fn keep_tail(stderr: &mut impl Read, sink: &Mutex<VecDeque<u8>>) {
    let mut buffer = [0u8; 4_096];
    while let Ok(read) = stderr.read(&mut buffer) {
        if read == 0 {
            return;
        }
        let mut tail = sink.lock();
        tail.extend(&buffer[..read]);
        let excess = tail.len().saturating_sub(STDERR_TAIL);
        tail.drain(..excess);
    }
}

/// Measures a process's memory: on macOS its physical footprint, which counts compressed
/// memory; elsewhere its resident set.
struct MemoryProbe {
    #[cfg(not(target_os = "macos"))]
    system: sysinfo::System,
}

impl MemoryProbe {
    fn new() -> Self {
        #[cfg(not(target_os = "macos"))]
        use sysinfo::SystemExt;
        Self {
            #[cfg(not(target_os = "macos"))]
            system: sysinfo::System::new(),
        }
    }

    #[cfg(target_os = "macos")]
    fn bytes(&mut self, pid: u32) -> Option<u64> {
        // SAFETY: all-zero bytes are a valid `rusage_info_v2`, a plain struct of integers.
        let mut info: libc::rusage_info_v2 = unsafe { std::mem::zeroed() };
        // SAFETY: `info` is the struct `RUSAGE_INFO_V2` asks the kernel to fill, and it outlives
        // the call.
        let result = unsafe {
            libc::proc_pid_rusage(
                pid as libc::c_int,
                libc::RUSAGE_INFO_V2,
                &mut info as *mut libc::rusage_info_v2 as *mut libc::rusage_info_t,
            )
        };
        (result == 0).then_some(info.ri_phys_footprint)
    }

    #[cfg(not(target_os = "macos"))]
    fn bytes(&mut self, pid: u32) -> Option<u64> {
        use sysinfo::{PidExt, ProcessExt, ProcessRefreshKind, SystemExt};
        let pid = sysinfo::Pid::from_u32(pid);
        if !self
            .system
            .refresh_process_specifics(pid, ProcessRefreshKind::new())
        {
            return None;
        }
        self.system.process(pid).map(|process| process.memory())
    }
}

/// Runs MonoMove on `input` in this process, calling `on_read` with each key the first time it
/// is read. What the worker runs; also what tests compare the isolated run against.
pub fn run_in_process(input: &ReplayInput, on_read: &(dyn Fn(&StateKey) + Sync)) -> V2Report {
    match input.patched() {
        Ok(state) => run_on(input, &state, on_read),
        Err(err) => V2Report {
            outcome: V2Outcome::SetupFailed(format!("failed to prepare the state: {:#}", err)),
            reads: BTreeSet::new(),
        },
    }
}

/// Runs MonoMove on `input`'s transaction against `state`, its patched state.
fn run_on(
    input: &ReplayInput,
    state: &PatchedState,
    on_read: &(dyn Fn(&StateKey) + Sync),
) -> V2Report {
    let view = state.view_reporting(on_read);
    let aux_info = AuxiliaryInfo::new(input.aux_info, None);
    let run = catch_unwind(AssertUnwindSafe(|| {
        v2::execute(&view, &input.txn, &aux_info)
    }));
    let reads = view.into_reads();
    let outcome = match run {
        Ok(Ok(run)) => {
            let output_gap_status = match &run.unsupported {
                Some(v2::Unsupported::Output { status, .. }) => Some(status.clone()),
                Some(v2::Unsupported::Vm(_) | v2::Unsupported::Transaction(_)) | None => None,
            };
            V2Outcome::Ran(V2Ran {
                output: run
                    .output
                    .map(transferable)
                    .map_err(|err| format!("{:#}", err)),
                unsupported: run.unsupported.map(UnsupportedKind::from),
                output_gap_status,
                vm_error: run.vm_error,
            })
        },
        Ok(Err(err)) => V2Outcome::SetupFailed(format!("{:#}", err)),
        Err(panic) => V2Outcome::Panicked(panic_message(&panic)),
    };
    V2Report { outcome, reads }
}

/// `output` with its write set as V1, so that it crosses to the parent whole: a V0 write set skips
/// its hot-state and native-position buckets in BCS.
fn transferable(mut output: TransactionOutput) -> TransactionOutput {
    output.convert_write_set_to_v1();
    output
}

/// The child: reads a [`Request`] from stdin, runs MonoMove on it, and reports on its channel.
pub fn worker_main() -> Result<()> {
    // Without a channel the parent never hears `Hello`, so it reports a harness failure.
    let channel = Mutex::new(take_stdout_as_channel()?);
    // A write that fails means the parent is gone: nothing is left to report to.
    let send = |frame: &Frame| -> std::result::Result<(), bcs::Error> {
        // One write per frame: the length, then the frame.
        let mut bytes = vec![0u8; 8];
        bcs::serialize_into(&mut bytes, frame)?;
        let len = bytes.len().saturating_sub(8) as u64;
        bytes[..8].copy_from_slice(&len.to_le_bytes());
        let mut channel = channel.lock();
        let _ = channel.write_all(&bytes);
        let _ = channel.flush();
        Ok(())
    };
    let (request, state, baseline, watchdog) = match prepare_worker() {
        Ok(prepared) => prepared,
        Err(err) => {
            let _ = send(&Frame::Failed(format!("{:#}", err)));
            return Ok(());
        },
    };
    // From here on, MonoMove runs: whatever ends this process is MonoMove's. The watchdog's clock
    // starts with it, as the parent's does when it receives `Hello`.
    let _ = send(&Frame::Hello(PROTOCOL_VERSION, baseline));
    let _ = watchdog.send(baseline);
    let report = run_on(&request.input, &state, &|key| {
        let _ = send(&Frame::Read(key.clone()));
    });
    // An output that cannot be serialized (e.g. a type tag too deep for BCS) is MonoMove's
    // output at fault: it is replaced by the error, keeping what else the run reported.
    let done = Frame::Done(report.outcome);
    if let Err(err) = send(&done) {
        let reason = format!("MonoMove's output could not be serialized: {}", err);
        let fallback = match done {
            Frame::Done(V2Outcome::Ran(run)) => V2Outcome::Ran(V2Ran {
                output: Err(reason.clone()),
                ..run
            }),
            Frame::Done(_) | Frame::Hello(..) | Frame::Read(_) | Frame::Failed(_) => {
                V2Outcome::Panicked(reason.clone())
            },
        };
        if send(&Frame::Done(fallback)).is_err() {
            let _ = send(&Frame::Done(V2Outcome::Panicked(reason)));
        }
    }
    Ok(())
}

/// What the worker needs before `Hello`: its request, the state, the memory baseline and an armed
/// watchdog. The state is the harness's data: it is prepared, and the watchdog armed, before
/// `Hello`, so that failing at either is the harness's, and MonoMove's memory limit is measured
/// above it.
fn prepare_worker() -> Result<(Request, PatchedState, u64, mpsc::Sender<u64>)> {
    let request = read_request()?;
    let state = request
        .input
        .patched()
        .context("failed to prepare the state")?;
    // Without a baseline, the state's own memory would count as MonoMove's: a harness failure.
    let baseline = MemoryProbe::new()
        .bytes(std::process::id())
        .context("failed to measure the worker's memory")?;
    let watchdog = start_watchdog(request.limits)?;
    Ok((request, state, baseline, watchdog))
}

fn read_request() -> Result<Request> {
    let mut stdin = std::io::stdin().lock();
    let mut len = [0u8; 8];
    stdin.read_exact(&mut len).context("no request")?;
    let mut bytes = vec![0u8; usize::try_from(u64::from_le_bytes(len))?];
    stdin.read_exact(&mut bytes).context("truncated request")?;
    bcs::from_bytes(&bytes).context("undecodable request")
}

/// Arms the watchdog, so that the limits hold even if the parent is gone: once sent the memory
/// baseline at `Hello`, it starts its clock and exits the process at MonoMove's time limit, or when
/// its memory grows past the limit above the baseline.
fn start_watchdog(limits: V2Limits) -> Result<mpsc::Sender<u64>> {
    let (start, started_rx) = mpsc::channel::<u64>();
    std::thread::Builder::new()
        .name("v2-watchdog".to_string())
        .spawn(move || {
            let Ok(baseline) = started_rx.recv() else {
                return;
            };
            let started = Instant::now();
            let mut memory = MemoryProbe::new();
            loop {
                if started.elapsed() > limits.timeout {
                    exit_now(EXIT_TIME_LIMIT);
                }
                if memory
                    .bytes(std::process::id())
                    .is_some_and(|bytes| bytes.saturating_sub(baseline) > limits.memory_bytes())
                {
                    exit_now(EXIT_MEMORY_LIMIT);
                }
                std::thread::sleep(POLL_INTERVAL);
            }
        })
        .context("failed to start the watchdog")?;
    Ok(start)
}

/// Ends the worker with `code` at once. Unlike `std::process::exit`, it runs no exit handlers or
/// static destructors, which would tear down state MonoMove is still using on the main thread.
fn exit_now(code: i32) -> ! {
    // SAFETY: `_exit` only ends the process; nothing runs after it.
    unsafe { libc::_exit(code) }
}

/// Moves stdout aside as the channel to the parent, and points stdout at stderr, so that nothing
/// else this process prints can corrupt the channel.
fn take_stdout_as_channel() -> Result<File> {
    use std::os::fd::FromRawFd;
    // SAFETY: `dup` and `dup2` act on this process's own descriptors 1 and 2; this runs first
    // thing in the worker, before anything else writes to stdout. The duplicated descriptor is
    // owned by the returned `File` alone.
    unsafe {
        let channel = libc::dup(1);
        if channel < 0 {
            bail!(
                "failed to duplicate stdout: {}",
                std::io::Error::last_os_error()
            );
        }
        if libc::dup2(2, 1) < 0 {
            bail!(
                "failed to redirect stdout: {}",
                std::io::Error::last_os_error()
            );
        }
        Ok(File::from_raw_fd(channel))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_output_crosses_to_the_parent_whole() {
        use aptos_types::{
            state_store::state_key::StateKey,
            transaction::{ExecutionStatus, TransactionAuxiliaryData},
            write_set::{NativePositionOp, WriteOp, WriteSetMut},
        };
        let mut write_set = WriteSetMut::new(vec![])
            .freeze()
            .expect("write set freezes");
        write_set.add_native_positions(
            [(
                StateKey::raw(b"P"),
                NativePositionOp::from_write_op(WriteOp::legacy_modification(vec![1].into())),
            )]
            .into_iter()
            .collect(),
        );
        let output = TransactionOutput::new(
            write_set,
            vec![],
            0,
            TransactionStatus::Keep(ExecutionStatus::Success),
            TransactionAuxiliaryData::default(),
        );
        let crossed = |output: &TransactionOutput| {
            bcs::from_bytes::<TransactionOutput>(&bcs::to_bytes(output).expect("serialize"))
                .expect("deserialize")
        };
        // As built, a V0 write set loses the bucket on the way; made transferable, it keeps it.
        assert!(!crossed(&output).write_set().has_native_positions());
        assert!(crossed(&transferable(output))
            .write_set()
            .has_native_positions());
    }

    #[test]
    fn replacing_a_file_changes_its_identity() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("worker");
        std::fs::write(&path, b"old build").expect("write");
        let before = FileIdentity::of(&path).expect("identity");
        assert_eq!(FileIdentity::of(&path).expect("identity"), before);
        // A rebuild writes a new file and moves it into place.
        let new = dir.path().join("worker.new");
        std::fs::write(&new, b"new build!").expect("write");
        std::fs::rename(&new, &path).expect("rename");
        assert_ne!(FileIdentity::of(&path).expect("identity"), before);
    }

    #[test]
    fn the_lowest_limit_on_the_cgroup_path_applies() {
        let root = tempfile::tempdir().expect("tempdir");
        let write = |dir: &str, file: &str, limit: &str| {
            let dir = root.path().join(dir);
            std::fs::create_dir_all(&dir).expect("dir");
            std::fs::write(dir.join(file), limit).expect("limit");
        };
        // v2: the root is unlimited, a parent cgroup limits 8 GiB, the process's own 16 GiB.
        write("", "memory.max", "max\n");
        write("kubepods", "memory.max", "8589934592\n");
        write("kubepods/pod", "memory.max", "17179869184\n");
        assert_eq!(
            cgroup_memory_limit("0::/kubepods/pod\n", root.path()),
            8_589_934_592
        );
        // v1: the memory controller's own line; other controllers are ignored.
        write("memory/job", "memory.limit_in_bytes", "4294967296\n");
        assert_eq!(
            cgroup_memory_limit("5:cpu,cpuacct:/other\n4:memory:/job\n", root.path()),
            4_294_967_296
        );
        // No limit anywhere.
        assert_eq!(cgroup_memory_limit("0::/\n", root.path()), u64::MAX);
    }

    #[test]
    fn harness_errors_are_recognized_through_context() {
        let err = harness(anyhow!("spawn failed")).context("capturing version 1");
        assert!(is_harness_error(&err));
        assert!(!is_harness_error(&anyhow!("a MonoMove failure")));
    }
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! MonoMove replays in a child process: results and reads cross the process boundary, a child
//! that exceeds a limit or dies after it started MonoMove becomes a crash that keeps the reads it
//! reported, and a worker that never started is a harness error rather than a verdict on MonoMove.
//!
//! The stand-in workers are shell scripts that speak the protocol by hand: a frame is a `u64`
//! little-endian length, then the BCS of the frame, whose variants are `Hello(u32, u64)` (0),
//! `Read(StateKey)` (1), `Done` (2) and `Failed` (3).

use aptos_crypto::HashValue;
use aptos_transaction_simulation::InMemoryStateStore;
use aptos_types::{
    block_metadata::BlockMetadata,
    state_store::state_key::StateKey,
    transaction::{PersistedAuxiliaryInfo, Transaction},
};
use mono_move_replay::{
    isolated::{run_in_process, run_v2_with, ReplayInput, V2Limits, V2Outcome},
    overrides::OverrideConfig,
};
use move_core_types::account_address::AccountAddress;
use std::{
    collections::BTreeSet,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::Duration,
};

fn empty_input() -> ReplayInput {
    ReplayInput {
        values: vec![],
        absent: BTreeSet::new(),
        config: OverrideConfig::default(),
        txn: Transaction::StateCheckpoint(Default::default()),
        aux_info: PersistedAuxiliaryInfo::None,
    }
}

/// Limits with MonoMove's `memory_mb` and `timeout_secs`, and a start-up limit generous enough for
/// a loaded machine.
fn limits(memory_mb: u64, timeout_secs: u64) -> V2Limits {
    V2Limits {
        memory_mb,
        timeout: Duration::from_secs(timeout_secs),
        startup_timeout: Duration::from_secs(30),
    }
}

fn worker() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_mono-move-replay"))
}

/// One frame as the worker writes it: `variant` and its BCS-encoded payload.
fn frame(variant: u8, payload: &[u8]) -> Vec<u8> {
    let mut body = vec![variant];
    body.extend(payload);
    let mut bytes = (body.len() as u64).to_le_bytes().to_vec();
    bytes.extend(body);
    bytes
}

/// The `Hello` a worker of the current protocol (2) sends once it starts MonoMove, with a memory
/// baseline of 0.
fn hello() -> Vec<u8> {
    let mut payload = 2u32.to_le_bytes().to_vec();
    payload.extend(0u64.to_le_bytes());
    frame(0, &payload)
}

/// A stand-in worker: a shell script that first writes `frames` to its channel, then runs `body`.
fn script(dir: &Path, name: &str, frames: &[u8], body: &str) -> PathBuf {
    let frames_path = dir.join(format!("{}.frames", name));
    std::fs::write(&frames_path, frames).expect("frames");
    let path = dir.join(name);
    std::fs::write(
        &path,
        format!("#!/bin/sh\ncat '{}'\n{}\n", frames_path.display(), body),
    )
    .expect("write script");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    path
}

fn crash_reason(outcome: V2Outcome) -> String {
    match outcome {
        V2Outcome::Crashed(reason) => reason,
        other => panic!("expected a crash, got {:?}", other),
    }
}

#[test]
fn the_worker_runs_monomove_like_it_runs_in_process() {
    // A block prologue on genesis state: MonoMove executes and reads state.
    let input = ReplayInput {
        values: InMemoryStateStore::from_head_genesis()
            .to_btree_map()
            .into_iter()
            .collect(),
        absent: BTreeSet::new(),
        config: OverrideConfig::default(),
        txn: Transaction::BlockMetadata(BlockMetadata::new(
            HashValue::zero(),
            1,
            0,
            AccountAddress::ZERO,
            vec![],
            vec![],
            1,
        )),
        aux_info: PersistedAuxiliaryInfo::None,
    };
    let isolated = run_v2_with(&worker(), &input, &limits(4_096, 120)).expect("harness");
    let in_process = run_in_process(&input, &|_| {});
    let V2Outcome::Ran(isolated_run) = isolated.outcome else {
        panic!("MonoMove did not run: {:?}", isolated.outcome);
    };
    let V2Outcome::Ran(in_process_run) = in_process.outcome else {
        panic!("MonoMove did not run in process: {:?}", in_process.outcome);
    };
    assert!(!isolated.reads.is_empty(), "the reads did not cross over");
    assert_eq!(isolated.reads, in_process.reads);
    assert_eq!(
        format!("{:?}", isolated_run),
        format!("{:?}", in_process_run)
    );
}

#[test]
fn a_child_over_the_time_limit_is_a_crash() {
    let dir = tempfile::tempdir().expect("tempdir");
    let worker = script(dir.path(), "slow", &hello(), "exec sleep 30");
    // Long enough for the script to send `Hello` even while other tests load the machine.
    let report = run_v2_with(&worker, &empty_input(), &limits(4_096, 3)).expect("harness");
    let reason = crash_reason(report.outcome);
    assert!(reason.contains("time limit (3 s)"), "{reason}");
}

#[test]
fn an_outcome_read_before_a_limit_stands() {
    let dir = tempfile::tempdir().expect("tempdir");
    // The worker writes a valid `Done`, then stays alive past MonoMove's time limit: the outcome
    // was read first, so it stands, and the worker is stopped once it has had a moment to exit.
    let mut frames = hello();
    frames.extend(frame(2, &[1, 1, b'x']));
    let worker = script(dir.path(), "done-then-hangs", &frames, "exec sleep 30");
    let report = run_v2_with(&worker, &empty_input(), &limits(4_096, 1)).expect("harness");
    assert!(
        matches!(report.outcome, V2Outcome::SetupFailed(ref message) if message == "x"),
        "{:?}",
        report.outcome
    );
}

#[test]
fn a_child_over_the_memory_limit_is_a_crash() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Holds about 300 MB in a shell variable, then waits.
    let worker = script(
        dir.path(),
        "hungry",
        &hello(),
        "x=$(head -c 300000000 /dev/zero | tr '\\0' a); sleep 30",
    );
    let report = run_v2_with(&worker, &empty_input(), &limits(50, 60)).expect("harness");
    let reason = crash_reason(report.outcome);
    assert!(reason.contains("memory limit (50 MB)"), "{reason}");
}

#[test]
fn a_child_that_dies_is_a_crash_with_its_stderr() {
    let dir = tempfile::tempdir().expect("tempdir");
    let worker = script(
        dir.path(),
        "dies",
        &hello(),
        "echo 'fatal runtime error: stack overflow' >&2; exit 3",
    );
    let report = run_v2_with(&worker, &empty_input(), &limits(4_096, 60)).expect("harness");
    let reason = crash_reason(report.outcome);
    assert!(reason.contains("process died"), "{reason}");
    assert!(reason.contains("stack overflow"), "{reason}");
}

#[test]
fn a_killed_childs_reads_survive() {
    let key = StateKey::raw(b"read before the crash");
    let mut frames = hello();
    frames.extend(frame(1, &bcs::to_bytes(&key).expect("key")));
    let dir = tempfile::tempdir().expect("tempdir");
    let worker = script(dir.path(), "reads-then-hangs", &frames, "exec sleep 30");
    let report = run_v2_with(&worker, &empty_input(), &limits(4_096, 3)).expect("harness");
    assert!(crash_reason(report.outcome).contains("time limit"));
    assert_eq!(report.reads, BTreeSet::from([key]));
}

#[test]
fn a_child_that_exits_without_a_result_is_a_harness_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let worker = script(dir.path(), "silent", &hello(), "cat > /dev/null");
    let err = run_v2_with(&worker, &empty_input(), &limits(4_096, 60)).expect_err("harness");
    assert!(err.to_string().contains("without a result"), "{err}");
}

#[test]
fn a_worker_that_dies_before_it_starts_is_a_harness_error() {
    // E.g. a worker binary of another version rejecting its command line.
    let dir = tempfile::tempdir().expect("tempdir");
    let worker = script(
        dir.path(),
        "usage",
        &[],
        "echo 'error: unexpected argument' >&2; exit 2",
    );
    let err = run_v2_with(&worker, &empty_input(), &limits(4_096, 60)).expect_err("harness");
    assert!(err.to_string().contains("did not start"), "{err}");
    assert!(err.to_string().contains("unexpected argument"), "{err}");
}

#[test]
fn a_worker_killed_before_it_starts_is_a_harness_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let worker = script(dir.path(), "stuck", &[], "exec sleep 30");
    let limits = V2Limits {
        startup_timeout: Duration::from_secs(1),
        ..limits(4_096, 60)
    };
    let err = run_v2_with(&worker, &empty_input(), &limits).expect_err("harness");
    assert!(err.to_string().contains("did not start"), "{err}");
}

#[test]
fn a_worker_of_another_protocol_version_is_a_harness_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let worker = script(
        dir.path(),
        "old",
        &frame(
            0,
            &[&999u32.to_le_bytes()[..], &0u64.to_le_bytes()[..]].concat(),
        ),
        "exec sleep 30",
    );
    let err = run_v2_with(&worker, &empty_input(), &limits(4_096, 3)).expect_err("harness");
    assert!(err.to_string().contains("protocol version 999"), "{err}");
}

#[test]
fn a_worker_that_cannot_start_is_a_harness_error() {
    let err = run_v2_with(
        Path::new("/nonexistent/worker"),
        &empty_input(),
        &limits(4_096, 60),
    )
    .expect_err("harness");
    assert!(err.to_string().contains("failed to start"), "{err}");
}

#[test]
fn monomoves_time_limit_counts_from_hello() {
    // The worker takes 2 s to start (e.g. a large request), then MonoMove finishes 2 s later: 4 s
    // after the spawn, but within its 3 s limit, which counts from Hello. A limit counted from
    // the spawn would have killed it.
    let dir = tempfile::tempdir().expect("tempdir");
    let hello_path = dir.path().join("hello");
    std::fs::write(&hello_path, hello()).expect("hello");
    // `Done(SetupFailed("x"))`: variant 2, then `SetupFailed` (1) and the string.
    let done_path = dir.path().join("done");
    std::fs::write(&done_path, frame(2, &[1, 1, b'x'])).expect("done");
    let worker = script(
        dir.path(),
        "late",
        &[],
        &format!(
            "sleep 2; cat '{}'; sleep 2; cat '{}'",
            hello_path.display(),
            done_path.display()
        ),
    );
    let limits = V2Limits {
        memory_mb: 4_096,
        timeout: Duration::from_secs(3),
        startup_timeout: Duration::from_secs(10),
    };
    let report = run_v2_with(&worker, &empty_input(), &limits).expect("harness");
    assert!(
        matches!(report.outcome, V2Outcome::SetupFailed(ref message) if message == "x"),
        "{:?}",
        report.outcome
    );
}

#[test]
fn a_worker_using_too_much_memory_to_start_is_a_harness_error() {
    // Before `Hello` the worker only receives its request: its memory is bounded by the request's
    // size, not by MonoMove's limit, and exceeding it is the harness's failure.
    let dir = tempfile::tempdir().expect("tempdir");
    let worker = script(
        dir.path(),
        "bloated",
        &[],
        "x=$(head -c 400000000 /dev/zero | tr '\\0' a); sleep 30",
    );
    let err = run_v2_with(&worker, &empty_input(), &limits(4_096, 60)).expect_err("harness");
    assert!(err.to_string().contains("to receive its request"), "{err}");
}

#[test]
fn a_baseline_above_the_startup_allowance_is_a_harness_error() {
    // A worker that crossed its start-up allowance just before `Hello` would hold more than it
    // reserved: it is stopped as at start-up.
    let mut payload = 2u32.to_le_bytes().to_vec();
    payload.extend((64u64 << 30).to_le_bytes());
    let dir = tempfile::tempdir().expect("tempdir");
    let worker = script(dir.path(), "heavy", &frame(0, &payload), "exec sleep 30");
    let err = run_v2_with(&worker, &empty_input(), &limits(4_096, 60)).expect_err("harness");
    assert!(err.to_string().contains("to receive its request"), "{err}");
}

#[test]
fn a_fast_worker_with_a_baseline_above_its_allowance_is_a_harness_error() {
    // The same, for a worker that sends its outcome and exits before a poll sees the baseline.
    let mut payload = 2u32.to_le_bytes().to_vec();
    payload.extend((64u64 << 30).to_le_bytes());
    let mut frames = frame(0, &payload);
    frames.extend(frame(2, &[1, 1, b'x']));
    let dir = tempfile::tempdir().expect("tempdir");
    let worker = script(dir.path(), "heavy-fast", &frames, "exit 0");
    let err = run_v2_with(&worker, &empty_input(), &limits(4_096, 60)).expect_err("harness");
    assert!(err.to_string().contains("to receive its request"), "{err}");
}

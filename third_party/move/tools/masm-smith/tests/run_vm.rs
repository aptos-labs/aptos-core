// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Runs generated tests on the V1 transactional runner. A run may fail or abort, but a panic
//! fails the test, and the sweep must include at least three clean runs. Tests that need
//! bytecode edits are skipped.

use arbitrary::Unstructured;
use masm_smith::{generate, Config};
use move_transactional_test_runner::vm_test_harness;
use std::{io::Write, path::Path};

const SEED_BYTES: usize = 192;
const NUM_SEEDS: u32 = 64;

/// Each seed's outcome after running through the VM harness.
#[derive(Clone, Copy, Debug, Default)]
struct Outcome {
    /// Generation failed, rendered text was empty, or the test required binary mutations.
    skipped: bool,
    /// Captured output has no error or abort marker; a runner error with no output is excluded.
    clean: bool,
    /// At least one task printed an `Error:` (VM-level error like deserialization or
    /// linking failure).
    vm_error: bool,
    /// At least one task aborted at runtime (`MoveAbort` / `abort` in output).
    vm_abort: bool,
}

fn seed_for(index: u32) -> Vec<u8> {
    masm_smith::seeded_bytes(
        u64::from(index).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xC6BC_2796_92B5_C323,
        SEED_BYTES,
    )
}

/// Classify the runner's captured output. The transactional runner emits structured task
/// output; we look for the conventional markers it uses for VM errors and aborts.
fn classify(captured: &str) -> Outcome {
    let mut outcome = Outcome::default();
    let lower = captured.to_ascii_lowercase();
    // VM-level errors surface as "Error: ..." lines (deserialization, link failures,
    // unexpected VM exit, etc.). The runner also writes "Error: " when a publish or run
    // task fails for any reason.
    if captured.contains("Error: ") {
        outcome.vm_error = true;
    }
    // Recognize the abort markers used by VM error output.
    if lower.contains("moveabort") || captured.contains("ABORTED") || lower.contains("abort code") {
        outcome.vm_abort = true;
    }
    if !outcome.vm_error && !outcome.vm_abort {
        outcome.clean = true;
    }
    outcome
}

/// Runs a generated test and classifies its captured output. The caller must set
/// `UPDATE_BASELINE=1` so the runner writes its `.exp` sidecar. Panics propagate.
fn run_generated(i: u32) -> Outcome {
    let mut outcome = Outcome::default();
    let seed = seed_for(i);
    let mut u = Unstructured::new(&seed);
    let Ok(test) = generate(&mut u, &Config::default()) else {
        outcome.skipped = true;
        return outcome;
    };
    let text = test.render();
    if text.trim().is_empty() {
        outcome.skipped = true;
        return outcome;
    }
    // Skip recipe-bearing tests — `run_test` re-assembles the `.masm` internally, so any
    // binary-only mutation we'd apply would be lost. The verifier-targeted recipe paths
    // are covered by `tests/dimensions.rs`.
    if test.post_process.is_some() {
        outcome.skipped = true;
        return outcome;
    }
    let mut tmp = tempfile::Builder::new()
        .suffix(".masm")
        .tempfile()
        .expect("tempfile");
    tmp.write_all(text.as_bytes()).expect("write masm");
    tmp.flush().expect("flush masm");
    let masm_path = tmp.path().to_path_buf();
    // `run_test_impl` writes the captured output to `<path>.exp` when UPDATE_BASELINE is
    // set, and returns `Ok` instead of bailing on a baseline mismatch.
    let result = vm_test_harness::run_test(&masm_path);
    let exp_path = masm_path.with_extension("exp");
    let captured = std::fs::read_to_string(&exp_path).unwrap_or_default();
    // Clean up sidecar.
    let _ = std::fs::remove_file(&exp_path);
    // With `UPDATE_BASELINE` set, an `Err` that wrote no output means the runner failed
    // before running any task, which is not a clean run.
    if result.is_err() && captured.is_empty() {
        outcome.vm_error = true;
        return outcome;
    }
    classify(&captured)
}

#[test]
fn vm_sweep_classifies_outcomes() {
    // Setting the env var globally is safe here because this binary's `#[test]`s don't
    // race against the env var: `render_and_assemble_one_seed` doesn't touch the runner.
    // SAFETY: env var mutation is unsafe in 2024-edition Rust because of pthread races;
    // we accept that risk in tests where we own the process. The Drop guard restores the
    // prior value on panic.
    struct EnvGuard {
        prior: Option<std::ffi::OsString>,
    }
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe {
                match self.prior.take() {
                    Some(v) => std::env::set_var("UPDATE_BASELINE", v),
                    None => std::env::remove_var("UPDATE_BASELINE"),
                }
            }
        }
    }
    let _guard = EnvGuard {
        prior: std::env::var_os("UPDATE_BASELINE"),
    };
    unsafe { std::env::set_var("UPDATE_BASELINE", "1") };

    let mut skipped = 0u32;
    let mut clean = 0u32;
    let mut vm_error = 0u32;
    let mut vm_abort = 0u32;
    for i in 0..NUM_SEEDS {
        let o = run_generated(i);
        if o.skipped {
            skipped += 1;
            continue;
        }
        if o.clean {
            clean += 1;
        }
        if o.vm_error {
            vm_error += 1;
        }
        if o.vm_abort {
            vm_abort += 1;
        }
    }
    // Drop of `_guard` restores the prior env state.

    eprintln!(
        "run_vm sweep: total={NUM_SEEDS} skipped={skipped} clean={clean} vm_error={vm_error} \
         vm_abort={vm_abort}"
    );
    let executed = NUM_SEEDS - skipped;
    assert!(
        executed > 0,
        "every one of {NUM_SEEDS} seeds was skipped — generator failing to produce content"
    );
    // Require successful runs as well as tolerating individual errors and aborts.
    assert!(
        clean >= 3,
        "fewer than 3 clean VM executions across {executed} executed seeds — \
         generator may be regressing on VM-runnable content (clean={clean}, \
         vm_error={vm_error}, vm_abort={vm_abort})"
    );
}

/// Sanity check: we can render to a real temp file. Kept separate from the sweep so a
/// filesystem hiccup doesn't mask the sweep's assertion.
#[test]
fn render_and_assemble_one_seed() {
    let seed = seed_for(0);
    let mut u = Unstructured::new(&seed);
    let Ok(test) = generate(&mut u, &Config::default()) else {
        return;
    };
    let text = test.render();
    if text.trim().is_empty() {
        return;
    }
    let mut tmp = tempfile::Builder::new()
        .suffix(".masm")
        .tempfile()
        .expect("tempfile");
    tmp.write_all(text.as_bytes()).expect("write masm");
    tmp.flush().expect("flush masm");
    assert!(Path::new(tmp.path()).exists());
}

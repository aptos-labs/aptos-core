// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Periodically checks for newer CLI releases and notifies interactive users.

use super::aptos::{parse_stable_version, InstallationMethod};
use anyhow::Result;
use aptos_cli_common::global_folder;
use aptos_telemetry::service::is_env_variable_true;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    io::{self, stderr, stdout, ErrorKind, IsTerminal, Write},
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tempfile::NamedTempFile;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

const DISABLE_ENV_VAR: &str = "APTOS_DISABLE_UPDATE_CHECK";
const STATE_FILE: &str = "update_check.json";
const CHECK_INTERVAL: Duration = Duration::from_secs(3 * 24 * 60 * 60);
/// Retry delay when `finish` has not received a lookup result. Consecutive misses wait
/// `CHECK_INTERVAL` instead, so unreachable hosts cost the budget once per interval.
const RETRY_DELAY: Duration = Duration::from_secs(24 * 60 * 60);
/// Time budget for waiting, measured from the start of the lookup.
const CHECK_BUDGET: Duration = Duration::from_secs(1);

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct UpdateCheckState {
    /// Unix time in seconds at or after which the next check is due.
    next_check_secs: u64,
    /// Whether the last lookup failed or `finish` did not receive its result.
    last_lookup_missed: bool,
}

impl UpdateCheckState {
    fn after(now_secs: u64, delay: Duration, last_lookup_missed: bool) -> Self {
        Self {
            next_check_secs: now_secs.saturating_add(delay.as_secs()),
            last_lookup_missed,
        }
    }
}

/// A background release lookup used to display an update notice after the command's output.
pub struct UpdateCheck {
    /// Version of the running binary.
    current: Version,
    upgrade_hint: &'static str,
    state_dir: PathBuf,
    /// When `finish` stops waiting for the lookup.
    deadline: Instant,
    /// Outcome of the lookup running on a background thread.
    receiver: mpsc::Receiver<Result<Version>>,
}

impl UpdateCheck {
    /// Starts the lookup in the background if this is an interactive run and a check is due.
    pub fn start() -> Option<Self> {
        if is_env_variable_true(DISABLE_ENV_VAR)
            || is_env_variable_true("CI")
            || !(stdout().is_terminal() && stderr().is_terminal())
        {
            return None;
        }
        let current = parse_stable_version(env!("CARGO_PKG_VERSION"))?;
        let method = InstallationMethod::from_env().ok()?;
        let upgrade_hint = method.upgrade_hint()?;

        let state_dir = global_folder().ok()?;
        let now_secs = now_secs()?;
        let last_lookup_missed = match read_state(&state_dir) {
            // Delay the first check when no schedule exists.
            Ok(None) => {
                let _ = write_state(
                    &state_dir,
                    &UpdateCheckState::after(now_secs, CHECK_INTERVAL, false),
                );
                return None;
            },
            Ok(Some(state)) if !is_check_due(state.next_check_secs, now_secs) => return None,
            Ok(Some(state)) => state.last_lookup_missed,
            Err(_) => false,
        };
        // Record this lookup as missed now; `finish` clears that and defers the next check by a
        // full interval if it receives a version.
        let delay = if last_lookup_missed {
            CHECK_INTERVAL
        } else {
            RETRY_DELAY
        };
        write_state(&state_dir, &UpdateCheckState::after(now_secs, delay, true)).ok()?;

        let deadline = Instant::now().checked_add(CHECK_BUDGET)?;
        let (sender, receiver) = mpsc::sync_channel(1);
        // Must not panic: `node run-localnet` exits the process on a panic in any thread.
        thread::Builder::new()
            .name("update-check".to_string())
            .spawn(move || {
                let _ = sender.send(method.fetch_latest());
            })
            .ok()?;
        Some(Self {
            current,
            upgrade_hint,
            state_dir,
            deadline,
            receiver,
        })
    }

    /// Prints a notice for a newer release, waiting only for the remaining lookup budget.
    /// A result already in the channel is used even if the budget has elapsed.
    pub fn finish(self) {
        let timeout = self.deadline.saturating_duration_since(Instant::now());
        // Errors keep the retry that `start` scheduled.
        let Ok(Ok(latest)) = self.receiver.recv_timeout(timeout) else {
            return;
        };
        if let Some(now_secs) = now_secs() {
            let _ = write_state(
                &self.state_dir,
                &UpdateCheckState::after(now_secs, CHECK_INTERVAL, false),
            );
        }
        if let Some(message) = notice_message(&self.current, &latest, self.upgrade_hint) {
            let _ = print_notice(&message);
        }
    }
}

/// Prints `message` in yellow, preceded by a blank line.
fn print_notice(message: &str) -> io::Result<()> {
    let mut writer = StandardStream::stderr(ColorChoice::Auto);
    writeln!(writer)?;
    writer.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
    let written = writeln!(writer, "{}", message);
    writer.reset()?;
    written
}

fn now_secs() -> Option<u64> {
    Some(SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs())
}

/// Treats a check more than one interval ahead as a clock rollback and checks immediately.
fn is_check_due(next_check_secs: u64, now_secs: u64) -> bool {
    now_secs >= next_check_secs
        || next_check_secs > now_secs.saturating_add(CHECK_INTERVAL.as_secs())
}

/// Returns `Ok(None)` if no check has been scheduled, and an error if the state is unreadable.
fn read_state(dir: &Path) -> Result<Option<UpdateCheckState>> {
    match std::fs::read(dir.join(STATE_FILE)) {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Fails if `dir` does not exist: the check never creates `~/.aptos`, so a first run under `sudo`
/// cannot leave it owned by root.
fn write_state(dir: &Path, state: &UpdateCheckState) -> Result<()> {
    // Rename avoids writing directly to a state file owned by root after a `sudo` run.
    let mut file = NamedTempFile::new_in(dir)?;
    serde_json::to_writer(&mut file, state)?;
    file.persist(dir.join(STATE_FILE))?;
    Ok(())
}

fn notice_message(current: &Version, latest: &Version, upgrade_hint: &str) -> Option<String> {
    if latest <= current {
        return None;
    }
    Some(format!(
        "A new version of the Aptos CLI is available: {} -> {}\n{}\nSet {}=1 to disable this check.",
        current, latest, upgrade_hint, DISABLE_ENV_VAR
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use InstallationMethod::*;

    const DAY_SECS: u64 = 24 * 60 * 60;

    #[test]
    fn check_is_due_when_scheduled_or_after_clock_change() {
        let now_secs = 1_000 * DAY_SECS;
        assert!(!is_check_due(now_secs + 1, now_secs));
        assert!(!is_check_due(now_secs + 3 * DAY_SECS, now_secs));
        assert!(is_check_due(now_secs, now_secs));
        assert!(is_check_due(now_secs - 1, now_secs));
        assert!(is_check_due(now_secs + 3 * DAY_SECS + 1, now_secs));
    }

    #[test]
    fn state_file_round_trip() {
        let home = tempfile::tempdir().expect("temp dir should be created");
        let dir = home.path().join(".aptos");
        assert_eq!(read_state(&dir).expect("missing state reads"), None);

        let retry = UpdateCheckState::after(42, RETRY_DELAY, true);
        assert!(write_state(&dir, &retry).is_err());
        assert!(!dir.exists());

        std::fs::create_dir(&dir).expect("state dir should be created");
        write_state(&dir, &retry).expect("state should be written");
        assert_eq!(read_state(&dir).expect("state reads"), Some(retry));

        let scheduled = UpdateCheckState::after(43, CHECK_INTERVAL, false);
        write_state(&dir, &scheduled).expect("state should be overwritten");
        assert_eq!(read_state(&dir).expect("state reads"), Some(scheduled));

        std::fs::write(dir.join(STATE_FILE), "garbage").expect("garbage should be written");
        assert!(read_state(&dir).is_err());
    }

    #[test]
    fn notice_message_per_installation_method() {
        let current = Version::new(9, 4, 0);
        let latest = Version::new(9, 5, 1);
        let notice =
            |method: InstallationMethod| notice_message(&current, &latest, method.upgrade_hint()?);
        assert_eq!(
            notice(Other).expect("Other installs get a notice"),
            "A new version of the Aptos CLI is available: 9.4.0 -> 9.5.1\n\
             To upgrade, run: aptos update aptos\n\
             Set APTOS_DISABLE_UPDATE_CHECK=1 to disable this check."
        );
        assert!(notice(Homebrew)
            .expect("Homebrew installs get a notice")
            .contains("\nTo upgrade, run: brew upgrade aptos\n"));
        assert!(notice(VersionManager)
            .expect("version manager installs get a notice")
            .contains(
            "\nTo upgrade, use the version manager that installed the Aptos CLI (asdf or mise).\n"
        ));
        assert_eq!(notice(Source), None);
        assert_eq!(notice(PackageManager), None);

        let hint = Other.upgrade_hint().expect("Other installs have a hint");
        assert_eq!(notice_message(&latest, &latest, hint), None);
        assert_eq!(notice_message(&Version::new(9, 5, 2), &latest, hint), None);
    }
}

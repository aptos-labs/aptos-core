// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![cfg(unix)]

use serde_json::Value;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Output},
};

struct Repo {
    dir: tempfile::TempDir,
}

impl Repo {
    fn new() -> Self {
        let repo = Self {
            dir: tempfile::tempdir().unwrap(),
        };
        repo.write(
            "Cargo.toml",
            "[workspace]\nresolver = '2'\nmembers = ['move/core', 'api']\n",
        );
        repo.write(
            "move/core/Cargo.toml",
            "[package]\nname = 'move-core'\nversion = '0.1.0'\nedition = '2021'\n",
        );
        repo.write("move/core/src/lib.rs", "// baseline\n");
        repo.write("move/core/fixtures/input.move", "// fixture\n");
        repo.write("api/Cargo.toml", "[package]\nname = 'test-api'\nversion = '0.1.0'\nedition = '2021'\n[dependencies]\nmove-core = { path = '../move/core' }\n");
        repo.write("api/src/lib.rs", "// baseline\n");
        repo.write(".config/test-subsystems.toml", "version = 1\nunmatched_changes = 'legacy'\nignored_paths = ['**/*.md']\n[subsystems.move]\nroots = ['move']\nselection = 'affected'\nrelated_test_roots = ['api']\ne2e_tests = ['cli-e2e']\n[e2e_tests.cli-e2e]\naffected_packages = ['test-api']\ninput_paths = ['move/core/fixtures/**']\n[e2e_tests.node-api-compatibility]\naffected_packages = ['test-api']\n");
        repo.write(".gitignore", "/target\n/bin\n/runner-args\n");
        repo.git(&["init", "-q"]);
        repo.git(&["add", "."]);
        repo.git(&[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-qm",
            "baseline",
        ]);
        let sha = String::from_utf8(repo.git(&["rev-parse", "HEAD"]).stdout).unwrap();
        let metadata = Command::new(env!("CARGO"))
            .current_dir(repo.root())
            .args(["metadata", "--format-version", "1", "--offline"])
            .output()
            .unwrap();
        assert!(
            metadata.status.success(),
            "{}",
            String::from_utf8_lossy(&metadata.stderr)
        );
        repo.write(
            &format!("target/aptos-x-tool/metadata-{}.json", sha.trim()),
            &String::from_utf8(metadata.stdout).unwrap(),
        );
        repo.write("bin/cargo", "#!/bin/sh\nif [ \"$1\" = nextest ]; then\n  printf '%s\\n' \"$@\" > \"$RUNNER_ARGS\"\n  exit \"${RUNNER_EXIT:-0}\"\nfi\nexec \"$REAL_CARGO\" \"$@\"\n");
        fs::set_permissions(
            repo.root().join("bin/cargo"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        repo
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn write(&self, path: &str, value: &str) {
        let path = self.root().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, value).unwrap();
    }

    fn git(&self, args: &[&str]) -> Output {
        let output = Command::new("git")
            .current_dir(self.root())
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_aptos-cargo-cli"));
        command
            .current_dir(self.root())
            .args(["--base", "HEAD"])
            .env_remove("APTOS_TEST_DETERMINATOR")
            .env("REAL_CARGO", env!("CARGO"))
            .env("RUNNER_ARGS", self.root().join("runner-args"))
            .env(
                "PATH",
                std::env::join_paths(
                    std::iter::once(self.root().join("bin"))
                        .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
                )
                .unwrap(),
            );
        command
    }

    fn plan(&self, mode: &str) -> Value {
        let output = self
            .command()
            .args(["--determinator", mode, "test-plan", "--format", "json"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

#[test]
fn execution_matches_plan_and_propagates_failure() {
    let repo = Repo::new();
    repo.write("move/core/src/lib.rs", "// changed\n");
    let plan = repo.plan("subsystem");
    assert_eq!(
        plan["packages"],
        serde_json::json!(["move-core", "test-api"])
    );
    let result = repo
        .command()
        .env("APTOS_TEST_DETERMINATOR", "subsystem")
        .args(["targeted-unit-tests", "--profile", "ci", "--locked"])
        .env("RUNNER_EXIT", "7")
        .output()
        .unwrap();
    assert!(!result.status.success());
    let args = fs::read_to_string(repo.root().join("runner-args")).unwrap();
    assert_eq!(
        args,
        format!("nextest\nrun\n--no-tests=warn\n--profile\nci\n--locked\n-p\npath+file://{}/move/core#move-core@0.1.0\n-p\npath+file://{}/api#test-api@0.1.0\n", repo.root().display(), repo.root().display())
    );
}

#[test]
fn empty_selection_skips_runner_and_untracked_files() {
    let repo = Repo::new();
    repo.write(
        "move/core/untracked.move",
        "// untracked files are not analyzed\n",
    );
    let output = repo
        .command()
        .env("APTOS_TEST_DETERMINATOR", "subsystem")
        .arg("targeted-unit-tests")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!repo.root().join("runner-args").exists());
}

#[test]
fn compare_still_runs_legacy_when_config_is_invalid() {
    let repo = Repo::new();
    repo.write("move/core/src/lib.rs", "// changed\n");
    repo.write(".config/test-subsystems.toml", "broken = [");
    let plan = repo.plan("compare");
    assert!(plan["comparison_error"].is_string());
    assert_eq!(plan["e2e_tests"], repo.plan("legacy")["e2e_tests"]);
    assert_eq!(plan["packages"], repo.plan("legacy")["packages"]);
    let output = repo
        .command()
        .env("APTOS_TEST_DETERMINATOR", "compare")
        .arg("targeted-unit-tests")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(repo.root().join("runner-args").exists());
    let output = repo
        .command()
        .env("APTOS_TEST_DETERMINATOR", "subsystem")
        .arg("test-plan")
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn cli_overrides_environment_and_git_failures_are_visible() {
    let repo = Repo::new();
    let output = repo
        .command()
        .env("APTOS_TEST_DETERMINATOR", "subsystem")
        .args(["--determinator", "legacy", "test-plan", "--format", "json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let plan: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(plan["mode"], "legacy");
    let output = Command::new(env!("CARGO_BIN_EXE_aptos-cargo-cli"))
        .current_dir(repo.root())
        .args(["--base", "no-such-ref", "changed-files"])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn rename_reports_both_paths_even_from_a_member_directory() {
    let repo = Repo::new();
    repo.git(&["mv", "move/core/fixtures/input.move", "api/input.move"]);
    let output = repo
        .command()
        .current_dir(repo.root().join("api"))
        .args([
            "--determinator",
            "subsystem",
            "test-plan",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let plan: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        plan["changed_paths"],
        serde_json::json!(["api/input.move", "move/core/fixtures/input.move"])
    );
}

#[test]
fn e2e_json_reports_execution_comparison_and_legacy_fallback() {
    let repo = Repo::new();
    repo.write("move/core/src/lib.rs", "// changed\n");
    let subsystem = repo.plan("subsystem");
    assert_eq!(subsystem["schema_version"], 1);
    assert!(subsystem["e2e_tests"]["cli-e2e"].is_array());
    assert!(subsystem["e2e_tests"]["node-api-compatibility"].is_null());
    let compare = repo.plan("compare");
    assert_eq!(compare["e2e_tests"], repo.plan("legacy")["e2e_tests"]);
    let legacy_names = compare["e2e_tests"]
        .as_object()
        .unwrap()
        .keys()
        .filter(|name| *name != "cli-e2e")
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(compare["e2e_legacy_only"], serde_json::json!(legacy_names));
    repo.write("api/src/lib.rs", "// outside configured source roots\n");
    assert!(repo.plan("subsystem")["e2e_tests"]["node-api-compatibility"].is_array());
}

#[test]
fn list_e2e_tests_does_not_require_a_workspace_or_valid_base() {
    let dir = tempfile::tempdir().unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_aptos-cargo-cli"))
        .current_dir(dir.path())
        .args([
            "--base",
            "no-such-ref",
            "list-e2e-tests",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let listed: Value = serde_json::from_slice(&result.stdout).unwrap();
    for name in [
        "cli-e2e",
        "node-api-compatibility",
        "execution-performance",
        "faucet-integration",
        "forge-e2e",
        "forge-compatibility",
    ] {
        assert!(listed[name]["description"].is_string(), "{name}");
        assert!(listed[name]["workflow"].is_string(), "{name}");
    }
    for name in [
        "flow-evaluation",
        "mono-move-parity",
        "mono-move-performance",
        "forge-framework-upgrade",
        "forge-consensus-only-performance",
        "forge-multiregion",
    ] {
        assert!(
            listed[name].is_null(),
            "manual suite {name} must not be configurable"
        );
    }
}

#[test]
fn unknown_e2e_references_and_definitions_fail_the_cli() {
    let repo = Repo::new();
    let path = repo.root().join(".config/test-subsystems.toml");
    let original = fs::read_to_string(&path).unwrap();
    // Manual suites are unregistered names; one representative covers them.
    for name in ["does-not-exist", "mono-move-parity"] {
        for config in [
            original.replace(
                "e2e_tests = ['cli-e2e']",
                &format!("e2e_tests = ['{name}']"),
            ),
            format!("{original}\n[e2e_tests.{name}]\naffected_packages = ['test-api']\n"),
        ] {
            fs::write(&path, config).unwrap();
            for mode in ["subsystem", "compare"] {
                let result = repo
                    .command()
                    .args(["--determinator", mode, "test-plan"])
                    .output()
                    .unwrap();
                assert!(
                    !result.status.success(),
                    "{name} must be rejected in {mode}"
                );
                assert!(String::from_utf8_lossy(&result.stderr).contains("Unknown E2E"));
            }
        }
    }
}

#[test]
fn policy_change_cannot_ignore_itself_or_remove_full_e2e_coverage() {
    let repo = Repo::new();
    repo.write("api/src/lib.rs", "// outside subsystem source roots\n");
    repo.write(".config/test-subsystems.toml", "version = 1\nunmatched_changes = 'legacy'\nglobal_test_inputs = []\nignored_paths = ['**']\n[subsystems.move]\nroots = ['move']\nselection = 'affected'\n");
    let plan = repo.plan("subsystem");
    assert_eq!(
        plan["packages"],
        serde_json::json!(["move-core", "test-api"])
    );
    assert_eq!(plan["e2e_tests"], repo.plan("legacy")["e2e_tests"]);
}

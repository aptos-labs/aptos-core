// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The Verex backend (`--lean`): the Lean-based verifier of
//! `third_party/move/lean/verex-move`, which verifies a Move file or package
//! from its typed-AST export and reports every message at its position in the
//! Move sources.

use anyhow::{bail, Result};
use log::info;
use move_model::model::{GlobalEnv, ModuleId, VerificationScope};
use move_model_exchange::{
    ast::{Pragma, PragmaValue, Value, XastModule},
    dump_ast_module, module_closure,
};
use sha2::{digest, Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};

const VEREX_MOVE_EXE_ENV: &str = "VEREX_MOVE_EXE";
const VEREX_MOVE_HOME_ENV: &str = "VEREX_MOVE_HOME";
const LAKE_EXE_ENV: &str = "LAKE";
/// Set where the verifier must be available, so tests fail instead of skip.
const VEREX_MOVE_REQUIRED_ENV: &str = "VEREX_MOVE_REQUIRED";
/// The Lean package of the verifier within an Aptos Core checkout.
const VEREX_MOVE_PACKAGE: &str = "third_party/move/lean/verex-move";
/// The verifier's executable within its built Lean package.
const VEREX_MOVE_BINARY: &str = ".lake/build/bin/verex-move";
/// The Lake target building the executable.
const VEREX_MOVE_TARGET: &str = "verex-move";

/// The verifier could not be run, or exited without reporting verification
/// completion: a failure of the harness, not a verification result.
#[derive(Debug)]
pub struct HarnessFailure(String);

impl fmt::Display for HarnessFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for HarnessFailure {}

/// How the verifier is run: the program, the arguments before the verifier's
/// own, and the environment the run needs. It runs in the current directory,
/// where the package's source paths, as the export records them, resolve.
#[derive(Debug)]
pub struct Runner {
    pub program: PathBuf,
    pub prefix_args: Vec<String>,
    pub env: Vec<(String, String)>,
}

/// Finds the verifier's Lean package by walking up from `start`.
fn find_package(start: &Path) -> Option<PathBuf> {
    let mut dir = Some(start);
    while let Some(candidate) = dir {
        let package = candidate.join(VEREX_MOVE_PACKAGE);
        if ["lakefile.lean", "lakefile.toml"]
            .iter()
            .any(|lakefile| package.join(lakefile).is_file())
        {
            return Some(package);
        }
        dir = candidate.parent();
    }
    None
}

/// The verifier's Lean package: `VEREX_MOVE_HOME`, or the one of the
/// enclosing Aptos Core checkout, searched upward from `source` and the
/// current directory.
pub fn package(source: &Path) -> Option<PathBuf> {
    match std::env::var(VEREX_MOVE_HOME_ENV) {
        Ok(home) => Some(PathBuf::from(home)),
        Err(_) => std::fs::canonicalize(source)
            .ok()
            .and_then(|dir| find_package(&dir))
            .or_else(|| {
                std::env::current_dir()
                    .ok()
                    .and_then(|dir| find_package(&dir))
            }),
    }
}

/// An executable on `PATH`.
fn find_in_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

/// The `lake` executable: the `LAKE` environment variable, `PATH`, or the
/// elan installation.
fn find_lake() -> Option<PathBuf> {
    if let Ok(path) = std::env::var(LAKE_EXE_ENV) {
        return Some(PathBuf::from(path));
    }
    if let Some(path) = find_in_path("lake") {
        return Some(path);
    }
    let home = std::env::var("HOME").ok()?;
    let candidate = PathBuf::from(home).join(".elan/bin/lake");
    candidate.is_file().then_some(candidate)
}

/// The toolchain `package` pins, for elan, which otherwise selects it by the
/// current directory: the package being verified, not the verifier's.
fn toolchain_env(package: &Path) -> Vec<(String, String)> {
    let mut env = vec![];
    if let Ok(toolchain) = std::fs::read_to_string(package.join("lean-toolchain")) {
        let toolchain = toolchain.trim();
        if !toolchain.is_empty() {
            env.push(("ELAN_TOOLCHAIN".to_string(), toolchain.to_string()));
        }
    }
    env
}

/// `lake` for `package`, under the toolchain it pins.
fn lake_command(package: &Path) -> Result<Command> {
    let Some(lake) = find_lake() else {
        bail!(
            "Could not find `lake` to run the Verex Move verifier. Install Lean through elan \
             (`./scripts/dev_setup.sh -p -l`), or set {} to its path.",
            LAKE_EXE_ENV,
        );
    };
    let mut command = Command::new(lake);
    command
        .arg("--dir")
        .arg(package)
        .envs(toolchain_env(package));
    Ok(command)
}

/// A private copy of the built verifier: its executable and the module files
/// it loads, which a rebuild of the live Lean packages leaves intact.
#[derive(Debug)]
pub struct Snapshot {
    /// The executable within the snapshot.
    pub program: PathBuf,
    /// The environment the executable runs in: the one `lake env` provides,
    /// with the module path leading into the snapshot.
    pub env: Vec<(String, String)>,
}

/// Builds and copies a coherent verifier. A stable pass must include the build
/// freshness check and the whole copy: if any source or build artifact changed,
/// discard the copy and retry. This also detects concurrent builds that do not
/// participate in a lock owned by this test harness.
pub fn snapshot(package: &Path, into: &Path) -> Result<Snapshot> {
    for _ in 0..3 {
        let env = lake_environment(package)?;
        let mut roots = BTreeSet::from([package.to_path_buf()]);
        for (_, value) in env.iter().filter(|(name, _)| name == "LEAN_PATH") {
            for dir in std::env::split_paths(value) {
                if let Some(lake) = dir
                    .ancestors()
                    .find(|p| p.file_name().is_some_and(|n| n == ".lake"))
                {
                    if let Some(root) = lake.parent() {
                        roots.insert(root.to_path_buf());
                    }
                }
            }
        }
        let before = snapshot_inventory(&roots)?;
        // The first environment discovers the roots. Check it again inside
        // the inventoried interval, so a configuration change during that
        // discovery cannot select untracked dependencies for the copy.
        if env != lake_environment(package)? {
            continue;
        }
        let result = snapshot_copy(package, into, env.clone());
        // Read the final environment before the final inventory: any config
        // or artifact changes during this check must be inventoried too.
        let after_env = lake_environment(package)?;
        if before == snapshot_inventory(&roots)? && env == after_env {
            return result;
        }
        // Only these directories are owned by this operation.
        for name in ["lib", "bin"] {
            let path = into.join(name);
            if path.exists() {
                std::fs::remove_dir_all(path)?;
            }
        }
    }
    bail!("Verex sources or build artifacts kept changing while taking the verifier snapshot; retry after the concurrent build finishes")
}

/// The content digest of each inventoried file. Metadata cannot replace it: a
/// same-length rewrite within one timestamp tick keeps size, inode and times.
type Inventory = BTreeMap<PathBuf, digest::Output<Sha256>>;

fn snapshot_inventory(roots: &BTreeSet<PathBuf>) -> Result<Inventory> {
    fn visit(dir: &Path, found: &mut Inventory) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let ty = entry.file_type()?;
            if ty.is_dir() {
                if entry.file_name() != ".git" {
                    visit(&path, found)?;
                }
            } else {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if ![
                    ".lean",
                    ".toml",
                    ".json",
                    ".olean",
                    ".olean.server",
                    ".olean.private",
                    ".trace",
                    ".hash",
                ]
                .iter()
                .any(|suffix| name.ends_with(suffix))
                    && name != "lean-toolchain"
                    && name != VEREX_MOVE_TARGET
                {
                    continue;
                }
                let digest = Sha256::digest(&std::fs::read(&path)?);
                found.insert(path, digest);
            }
        }
        Ok(())
    }
    let mut found = BTreeMap::new();
    for root in roots {
        visit(root, &mut found)?;
    }
    Ok(found)
}

fn lake_environment(package: &Path) -> Result<Vec<(String, String)>> {
    let env = lake_command(package)?.arg("env").output()?;
    if !env.status.success() {
        bail!(
            "`lake env` in {} failed with {}:\n{}",
            package.display(),
            env.status,
            String::from_utf8_lossy(&env.stderr)
        );
    }
    let mut environment: Vec<_> = String::from_utf8_lossy(&env.stdout)
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
    environment.sort_unstable();
    Ok(environment)
}

fn snapshot_copy(package: &Path, into: &Path, mut env: Vec<(String, String)>) -> Result<Snapshot> {
    let build = lake_command(package)?
        .arg("query")
        .arg(VEREX_MOVE_TARGET)
        .stderr(std::process::Stdio::inherit())
        .output()?;
    if !build.status.success() {
        bail!(
            "building the Verex Move verifier in {} failed with {}",
            package.display(),
            build.status
        );
    }
    let built = String::from_utf8_lossy(&build.stdout);
    let binary = built
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .ok_or_else(|| anyhow::anyhow!("`lake query {}` named no executable", VEREX_MOVE_TARGET))?;
    let program = into.join("bin").join(VEREX_MOVE_TARGET);
    std::fs::create_dir_all(program.parent().expect("bin directory"))?;
    std::fs::copy(binary, &program)?;
    // Ask the verifier for the import closure it actually loads. Test libraries
    // and Lake's hash/trace files are neither loaded nor copied.
    let modules = Command::new(&program)
        .envs(env.iter().map(|(k, v)| (k, v)))
        .arg("snapshot-modules")
        .output()?;
    if !modules.status.success() {
        bail!(
            "reading the verifier's import closure failed with {}:\n{}",
            modules.status,
            String::from_utf8_lossy(&modules.stderr)
        );
    }
    let modules: Vec<PathBuf> = String::from_utf8_lossy(&modules.stdout)
        .lines()
        .map(PathBuf::from)
        .collect();
    if modules.is_empty() {
        bail!("the verifier reported an empty import closure");
    }
    let mut copied = 0;
    for (name, value) in &mut env {
        if name != "LEAN_PATH" {
            continue;
        }
        let mut dirs = vec![];
        for dir in std::env::split_paths(value) {
            if dir.components().any(|c| c.as_os_str() == ".lake") {
                let copy = into.join("lib").join(copied.to_string());
                copied += 1;
                std::fs::create_dir_all(&copy)?;
                for module in &modules {
                    let Ok(relative) = module.strip_prefix(&dir) else {
                        continue;
                    };
                    for extension in ["olean", "olean.server", "olean.private"] {
                        let source = module.with_extension(extension);
                        if source.is_file() {
                            let target = copy.join(relative).with_extension(extension);
                            std::fs::create_dir_all(target.parent().expect("module directory"))?;
                            std::fs::copy(source, target)?;
                        }
                    }
                }
                dirs.push(copy);
            } else {
                dirs.push(dir);
            }
        }
        *value = std::env::join_paths(dirs)?.to_string_lossy().into_owned();
    }
    Ok(Snapshot { program, env })
}

/// What a run of the verifier is asked besides its sources.
#[derive(Clone, Debug, Default)]
pub struct Settings {
    /// The heartbeat budget of a function without `pragma heartbeats`, in
    /// thousands of Lean `maxHeartbeats` units; the verifier's own default
    /// when absent.
    pub heartbeats: Option<u64>,
    /// Verify every function automatically, its authored proof and `pragma
    /// verify = manual` ignored: whether a proof is still needed.
    pub ignore_proofs: bool,
}

/// Resolves how to run the verifier.
///
/// `VEREX_MOVE_EXE` names the executable and is run as is, so its caller
/// provides `LEAN_PATH`. Otherwise the verifier's Lean package is
/// `VEREX_MOVE_HOME` or the one of the enclosing Aptos Core checkout,
/// searched upward from `source` and the current directory, and its
/// built executable runs through `lake --dir <package> env`, which supplies
/// the module path of its dependencies, under the toolchain the package pins.
pub fn runner(source: &Path) -> Result<Runner> {
    if let Ok(path) = std::env::var(VEREX_MOVE_EXE_ENV) {
        return Ok(Runner {
            program: PathBuf::from(path),
            prefix_args: vec![],
            env: vec![],
        });
    }
    let Some(package) = package(source) else {
        bail!(
            "Could not find the Verex Move verifier. Build it with `cd {} && lake build \
             verex-move` inside an Aptos Core checkout, or set {} to its Lean package or {} to \
             its executable.",
            VEREX_MOVE_PACKAGE,
            VEREX_MOVE_HOME_ENV,
            VEREX_MOVE_EXE_ENV,
        );
    };
    let binary = package.join(VEREX_MOVE_BINARY);
    if !binary.is_file() {
        bail!(
            "The Verex Move verifier is not built at {}. Build it with `cd {} && lake build \
             verex-move`.",
            binary.display(),
            package.display(),
        );
    }
    let Some(lake) = find_lake() else {
        bail!(
            "Could not find `lake` to run the Verex Move verifier. Install Lean through elan \
             (`./scripts/dev_setup.sh -p -l`), or set {} to its path.",
            LAKE_EXE_ENV,
        );
    };
    Ok(Runner {
        program: lake,
        prefix_args: vec![
            "--dir".to_string(),
            package.display().to_string(),
            "env".to_string(),
            binary.display().to_string(),
        ],
        env: toolchain_env(&package),
    })
}

/// Whether the verifier can be run from here; tests skip when it cannot,
/// unless `VEREX_MOVE_REQUIRED` is set.
pub fn verifier_available() -> bool {
    let available = std::env::current_dir()
        .map_err(anyhow::Error::from)
        .and_then(|dir| runner(&dir));
    match available {
        Ok(_) => true,
        Err(err) if std::env::var_os(VEREX_MOVE_REQUIRED_ENV).is_some() => {
            panic!("{} is set: {:#}", VEREX_MOVE_REQUIRED_ENV, err)
        },
        Err(_) => false,
    }
}

/// The modules of `source` whose source file name contains `filter`: for a
/// package directory, the modules under its `sources`; for a Move file, the
/// modules it declares.
fn target_modules(
    model: &GlobalEnv,
    source: &Path,
    filter: Option<&str>,
) -> Result<BTreeSet<ModuleId>> {
    let canonical = std::fs::canonicalize(source)?;
    let package_sources = if canonical.is_dir() {
        Some(std::fs::canonicalize(source.join("sources"))?)
    } else {
        None
    };
    let selected: BTreeSet<ModuleId> = model
        .get_modules()
        .filter(|module| {
            let path = Path::new(module.get_source_path());
            path.file_name().is_some_and(|name| {
                filter.is_none_or(|filter| name.to_string_lossy().contains(filter))
            }) && std::fs::canonicalize(path).is_ok_and(|path| match &package_sources {
                Some(sources) => path.starts_with(sources),
                None => path == canonical,
            })
        })
        .map(|module| module.get_id())
        .collect();
    if selected.is_empty() {
        match filter {
            Some(filter) => bail!(
                "no module of {} matches the filter `{}`",
                source.display(),
                filter
            ),
            None => bail!("{} holds no module", source.display()),
        }
    }
    Ok(selected)
}

/// Under an exclusive verification scope (`--verify-only`, `--only`, or a
/// module's), each function of a target module is exported with
/// `pragma verify` set as the Move Prover selects it: true for those the
/// scope names, whatever they declare, and false for the others, which their
/// callers still read as the verifier reads any unverified function.
fn apply_scope(
    model: &GlobalEnv,
    module: ModuleId,
    dumped: &mut XastModule,
    scope: &VerificationScope,
) {
    if !scope.is_exclusive() {
        return;
    }
    let module_env = model.get_module(module);
    for function in dumped.functions.iter_mut() {
        let Some(function_env) = module_env
            .get_functions()
            .find(|f| *f.get_simple_name_string() == function.name)
        else {
            continue;
        };
        // The specification's own pragmas and the resolved ones alike.
        let verify = Pragma {
            name: "verify".to_string(),
            value: PragmaValue::Value(Value::Bool(function_env.should_verify(scope))),
        };
        for pragmas in [&mut function.spec.pragmas, &mut function.pragmas] {
            pragmas.retain(|pragma| pragma.name != "verify");
            pragmas.push(verify.clone());
        }
    }
}

/// Verifies `source`, a Move file or package directory of `model`, with the
/// Verex verifier: the modules of `source` (with `filter`, those whose file
/// name contains it) and what verifying them reads are exported in the
/// typed-AST exchange format, the verifier reads the export, verifies those
/// modules (the functions an exclusive `scope` selects, `apply_scope`) with
/// the others linked as dependencies, each function within
/// `settings.heartbeats` unless its `pragma heartbeats` says otherwise (every
/// function automatically with `settings.ignore_proofs`), writes their
/// VerexSL rendering where the source's proof files find it (the verifier
/// chooses the place: `build/` of a package, `.verex/` beside a file of no
/// package), and reports its messages, one per line in the Move sources'
/// coordinates, to `writer`. Fails when the verifier
/// reports an error, and with a `HarnessFailure` when it cannot be run or
/// exits without completing verification. Logs the time since `start_time` spent building the
/// model, exporting it, and verifying it; the verifier reports its phases.
pub fn verify(
    model: &GlobalEnv,
    source: &Path,
    filter: Option<&str>,
    scope: &VerificationScope,
    settings: &Settings,
    writer: &mut impl Write,
    start_time: Instant,
) -> Result<()> {
    let build_duration = start_time.elapsed();
    let now = Instant::now();
    let runner = runner(source)?;
    let export = tempfile::tempdir()?;
    let targets = target_modules(model, source, filter)?;
    let selection = module_closure(model, &targets);
    for module in model.get_modules() {
        // A bytecode-only dependency has no AST to export.
        if module.get_source_path().is_empty() || !selection.contains(&module.get_id()) {
            continue;
        }
        let mut dumped = dump_ast_module(model, module.get_id())?;
        if targets.contains(&module.get_id()) {
            apply_scope(model, module.get_id(), &mut dumped, scope);
        }
        let name = module.get_full_name_str().replace("::", "_");
        std::fs::write(
            export.path().join(format!("{}.xast.json", name)),
            dumped.to_pretty_json() + "\n",
        )?;
    }
    // The paths the verifier receives are absolute but for the source, which
    // the export records relative to the current directory.
    let mut command = Command::new(&runner.program);
    command
        .envs(runner.env.iter().map(|(name, value)| (name, value)))
        .args(&runner.prefix_args)
        .arg("verify")
        .arg(source)
        .arg("--export")
        .arg(export.path());
    if let Some(filter) = filter {
        command.arg("--filter").arg(filter);
    }
    if let Some(heartbeats) = settings.heartbeats {
        command.arg("--heartbeats").arg(heartbeats.to_string());
    }
    if settings.ignore_proofs {
        command.arg("--ignore-proofs");
    }
    let export_duration = now.elapsed();
    let now = Instant::now();
    let run = command.output().map_err(|e| {
        HarnessFailure(format!(
            "cannot run the Verex Move verifier `{}`: {}",
            runner.program.display(),
            e
        ))
    })?;
    let stdout = String::from_utf8_lossy(&run.stdout);
    let stderr = String::from_utf8_lossy(&run.stderr);
    for line in stdout.lines() {
        writeln!(writer, "{}", line)?;
    }
    // The verifier reports on its run, the rendering it wrote and its wall
    // time per phase, on stderr.
    if run.status.success() {
        for line in stderr.lines() {
            info!("{}", line);
        }
    }
    info!(
        "{:.2}s build, {:.2}s export, {:.2}s verex-move, total {:.2}s",
        build_duration.as_secs_f64(),
        export_duration.as_secs_f64(),
        now.elapsed().as_secs_f64(),
        start_time.elapsed().as_secs_f64()
    );
    if run.status.success() {
        Ok(())
    } else if verification_failed(&run) {
        bail!("exiting with verification errors")
    } else {
        Err(HarnessFailure(format!(
            "the Verex Move verifier exited with {}:\n{}{}",
            run.status, stdout, stderr
        ))
        .into())
    }
}

/// Exit 1 is a verification result only after the CLI explicitly completed.
/// Signals and other exit codes remain harness failures even with diagnostics.
fn verification_failed(output: &std::process::Output) -> bool {
    output.status.code() == Some(1)
        && String::from_utf8_lossy(&output.stderr)
            .lines()
            .any(|line| line == "verex-move: verification complete")
        && String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line.contains(": error: "))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::{fs::PermissionsExt, process::ExitStatusExt};

    #[cfg(unix)]
    #[test]
    fn incomplete_or_crashed_verification_is_a_harness_failure() {
        let mut output = std::process::Output {
            status: std::process::ExitStatus::from_raw(1 << 8),
            stdout: b"test.move:1:1: error: unproved\n".to_vec(),
            stderr: vec![],
        };
        assert!(!verification_failed(&output));
        output.stderr = b"verex-move: verification complete\n".to_vec();
        assert!(verification_failed(&output));
        for status in [9, 11, 137 << 8, 2 << 8] {
            output.status = std::process::ExitStatus::from_raw(status);
            assert!(!verification_failed(&output));
        }
    }

    #[test]
    fn snapshot_inventory_detects_rebuilds_and_source_changes() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let roots = BTreeSet::from([temp.path().to_path_buf()]);
        std::fs::write(temp.path().join("Module.lean"), "old source")?;
        std::fs::write(temp.path().join("Module.olean"), "old artifact")?;
        let before = snapshot_inventory(&roots)?;
        assert_eq!(before, snapshot_inventory(&roots)?);
        std::fs::write(temp.path().join("Module.lean"), "new source")?;
        assert_ne!(before, snapshot_inventory(&roots)?);
        let before = snapshot_inventory(&roots)?;
        std::fs::write(temp.path().join("Module.olean"), "new artifact")?;
        assert_ne!(before, snapshot_inventory(&roots)?);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn snapshot_retries_when_lake_environment_changes_before_inventory() -> Result<()> {
        // Use a child process to give Lake its own environment without changing
        // the environment of other tests running in this process.
        if let Some(case) = std::env::var_os("VEREX_SNAPSHOT_RACE_CASE") {
            let case = PathBuf::from(case);
            let snapshot = snapshot(&case.join("package"), &case.join("snapshot"))?;
            let lean_path = snapshot
                .env
                .iter()
                .find(|(name, _)| name == "LEAN_PATH")
                .unwrap();
            let copied = std::env::split_paths(&lean_path.1).next().unwrap();
            assert_eq!(
                std::fs::read_to_string(copied.join("Dependency.olean"))?,
                "new"
            );
            // The stale environment is rejected before building or copying.
            assert_eq!(std::fs::read_to_string(case.join("queries"))?, "1\n");
            return Ok(());
        }
        let case = tempfile::tempdir()?;
        let package = case.path().join("package");
        std::fs::create_dir(&package)?;
        for generation in ["old", "new"] {
            let modules = case.path().join(generation).join(".lake/build/lib/lean");
            std::fs::create_dir_all(&modules)?;
            std::fs::write(modules.join("Dependency.olean"), generation)?;
        }
        let verifier = package.join(VEREX_MOVE_TARGET);
        std::fs::write(
            &verifier,
            "#!/bin/sh\nprintf '%s/Dependency.olean\\n' \"$LEAN_PATH\"\n",
        )?;
        std::fs::set_permissions(&verifier, std::fs::Permissions::from_mode(0o755))?;
        let lake = case.path().join("lake");
        std::fs::write(
            &lake,
            r#"#!/bin/sh
if [ "$1" = --dir ]; then shift 2; fi
case "$1" in
  env)
    generation=new
    if [ ! -f "$VEREX_SNAPSHOT_RACE_CASE/configured" ]; then
      generation=old
      touch "$VEREX_SNAPSHOT_RACE_CASE/configured"
      # The old environment has been computed; the new configuration is
      # already on disk before the caller can start inventorying files.
      printf '{}\n' > "$VEREX_SNAPSHOT_RACE_CASE/package/lake-manifest.json"
    fi
    printf 'LEAN_PATH=%s/%s/.lake/build/lib/lean\n' "$VEREX_SNAPSHOT_RACE_CASE" "$generation"
    ;;
  query)
    count=0
    if [ -f "$VEREX_SNAPSHOT_RACE_CASE/queries" ]; then
      count=$(cat "$VEREX_SNAPSHOT_RACE_CASE/queries")
    fi
    echo $((count + 1)) > "$VEREX_SNAPSHOT_RACE_CASE/queries"
    printf '%s/package/verex-move\n' "$VEREX_SNAPSHOT_RACE_CASE"
    ;;
  *) exit 2 ;;
esac
"#,
        )?;
        std::fs::set_permissions(&lake, std::fs::Permissions::from_mode(0o755))?;
        let output = Command::new(std::env::current_exe()?)
            .args([
                "--exact",
                "verex::tests::snapshot_retries_when_lake_environment_changes_before_inventory",
                "--nocapture",
            ])
            .env("VEREX_SNAPSHOT_RACE_CASE", case.path())
            .env(LAKE_EXE_ENV, lake)
            .output()?;
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(())
    }
}

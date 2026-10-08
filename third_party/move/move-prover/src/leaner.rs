// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The Leaner backend (`--lean`): the Lean-based verifier of
//! `third_party/move/lean/leaner-move`, which verifies a Move file or package
//! from its typed-AST export and reports every message at its position in the
//! Move sources.

use anyhow::{anyhow, bail, Result};
use log::info;
use move_model::model::{GlobalEnv, ModuleId, VerificationScope};
use move_model_exchange::{
    ast::{Pragma, PragmaValue, Value, XastModule},
    dump_ast_module, module_closure,
};
use std::{
    collections::BTreeSet,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};

const LEANER_MOVE_EXE_ENV: &str = "LEANER_MOVE_EXE";
const LEANER_MOVE_HOME_ENV: &str = "LEANER_MOVE_HOME";
const LAKE_EXE_ENV: &str = "LAKE";
/// The Lean package of the verifier within an Aptos Core checkout.
const LEANER_MOVE_PACKAGE: &str = "third_party/move/lean/leaner-move";
/// The verifier's executable within its built Lean package.
const LEANER_MOVE_BINARY: &str = ".lake/build/bin/leaner-move";

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
        let package = candidate.join(LEANER_MOVE_PACKAGE);
        if package.join(LEANER_MOVE_BINARY).is_file() {
            return Some(package);
        }
        dir = candidate.parent();
    }
    None
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

/// Resolves how to run the verifier.
///
/// `LEANER_MOVE_EXE` names the executable and is run as is, so its caller
/// provides `LEAN_PATH`. Otherwise the verifier's Lean package is
/// `LEANER_MOVE_HOME` or the one of the enclosing Aptos Core checkout,
/// searched upward from `source` and the current directory, and its
/// built executable runs through `lake --dir <package> env`, which supplies
/// the module path of its dependencies, under the toolchain the package pins.
pub fn runner(source: &Path) -> Result<Runner> {
    if let Ok(path) = std::env::var(LEANER_MOVE_EXE_ENV) {
        return Ok(Runner {
            program: PathBuf::from(path),
            prefix_args: vec![],
            env: vec![],
        });
    }
    let package = match std::env::var(LEANER_MOVE_HOME_ENV) {
        Ok(home) => Some(PathBuf::from(home)),
        Err(_) => std::fs::canonicalize(source)
            .ok()
            .and_then(|dir| find_package(&dir))
            .or_else(|| {
                std::env::current_dir()
                    .ok()
                    .and_then(|dir| find_package(&dir))
            }),
    };
    let Some(package) = package else {
        bail!(
            "Could not find the Leaner Move verifier. Build it with `cd {} && lake build \
             leaner-move` inside an Aptos Core checkout, or set {} to its Lean package or {} to \
             its executable.",
            LEANER_MOVE_PACKAGE,
            LEANER_MOVE_HOME_ENV,
            LEANER_MOVE_EXE_ENV,
        );
    };
    let binary = package.join(LEANER_MOVE_BINARY);
    if !binary.is_file() {
        bail!(
            "The Leaner Move verifier is not built at {}. Build it with `cd {} && lake build \
             leaner-move`.",
            binary.display(),
            package.display(),
        );
    }
    let Some(lake) = find_lake() else {
        bail!(
            "Could not find `lake` to run the Leaner Move verifier. Install Lean through elan \
             (`./scripts/dev_setup.sh -p -l`), or set {} to its path.",
            LAKE_EXE_ENV,
        );
    };
    // elan selects the toolchain by the current directory, which is the
    // package being verified, not the verifier's: the pinned one is asked for.
    let mut env = vec![];
    if let Ok(toolchain) = std::fs::read_to_string(package.join("lean-toolchain")) {
        let toolchain = toolchain.trim();
        if !toolchain.is_empty() {
            env.push(("ELAN_TOOLCHAIN".to_string(), toolchain.to_string()));
        }
    }
    Ok(Runner {
        program: lake,
        prefix_args: vec![
            "--dir".to_string(),
            package.display().to_string(),
            "env".to_string(),
            binary.display().to_string(),
        ],
        env,
    })
}

/// Whether the verifier can be run from here; tests skip when it cannot.
pub fn verifier_available() -> bool {
    std::env::current_dir()
        .map(|dir| runner(&dir).is_ok())
        .unwrap_or(false)
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
/// Leaner verifier: the modules of `source` (with `filter`, those whose file
/// name contains it) and what verifying them reads are exported in the
/// typed-AST exchange format, the verifier reads the export, verifies those
/// modules (the functions an exclusive `scope` selects, `apply_scope`) with
/// the others linked as dependencies, each function within
/// `heartbeats` unless its `pragma heartbeats` says otherwise, writes their
/// LeanerLang rendering to `output`, and reports its messages, one per line in
/// the Move sources' coordinates, to `writer`. Fails when the verifier
/// reports an error. Logs the time since `start_time` spent building the
/// model, exporting it, and verifying it; the verifier reports its phases.
pub fn verify(
    model: &GlobalEnv,
    source: &Path,
    filter: Option<&str>,
    scope: &VerificationScope,
    heartbeats: Option<u64>,
    output: &Path,
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
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // The paths the verifier receives are absolute but for the source, which
    // the export records relative to the current directory.
    let output = std::path::absolute(output)?;
    let mut command = Command::new(&runner.program);
    command
        .envs(runner.env.iter().map(|(name, value)| (name, value)))
        .args(&runner.prefix_args)
        .arg("verify")
        .arg(source)
        .arg("--export")
        .arg(export.path())
        .arg("--output")
        .arg(&output);
    if let Some(filter) = filter {
        command.arg("--filter").arg(filter);
    }
    if let Some(heartbeats) = heartbeats {
        command.arg("--heartbeats").arg(heartbeats.to_string());
    }
    let export_duration = now.elapsed();
    let now = Instant::now();
    let run = command.output().map_err(|e| {
        anyhow!(
            "cannot run the Leaner Move verifier `{}`: {}",
            runner.program.display(),
            e
        )
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
        "{:.2}s build, {:.2}s export, {:.2}s leaner-move, total {:.2}s",
        build_duration.as_secs_f64(),
        export_duration.as_secs_f64(),
        now.elapsed().as_secs_f64(),
        start_time.elapsed().as_secs_f64()
    );
    if run.status.success() {
        Ok(())
    } else if stdout.lines().any(|line| line.contains(": error: ")) {
        bail!("exiting with verification errors")
    } else {
        bail!(
            "the Leaner Move verifier exited with {}:\n{}{}",
            run.status,
            stdout,
            stderr
        )
    }
}

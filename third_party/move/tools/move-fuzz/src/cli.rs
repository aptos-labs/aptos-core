// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Command-line surface of the fuzzer.
//!
//! Defines the `FuzzCommand` subcommands (list, build, test, exec, auto) and
//! `run_on`, which prepares logging, the working and state directories, and the
//! resolved project before dispatching to each command.

use crate::{
    deps::{self, PkgDeclaration, PkgDefinition, PkgKind, PkgManifest, Project},
    fuzzer,
    language::LanguageSetting,
    package,
    simulator::Simulator,
    state::{AUTO_STATE_FILENAME, ENTRYPOINT_CACHE_FILENAME, PACKAGE_BUILD_CACHE_DIR},
    testnet::{execute_runbook, provision_simulator},
};
use anyhow::{anyhow, bail, Context, Result};
use aptos_framework::extended_checks;
use aptos_vm::natives;
use clap::{Parser, Subcommand};
use fs_extra::dir;
use log::{info, warn, LevelFilter};
use move_model::model::GlobalEnv;
use move_unit_test::test_validation;
use regex::Regex;
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::Instant,
};
use tempfile::TempDir;
use walkdir::WalkDir;

/// Commands for move-fuzz
const DEFAULT_MAX_SCRIPT_GEN_SECS_PER_FUNCTION: u64 = 600;

#[derive(Subcommand)]
pub enum FuzzCommand {
    /// List collected packages in the project
    List,

    /// Build the packages
    Build {
        /// Filter on package level
        #[clap(flatten)]
        pkg_filter: FilterPackage,

        /// Development build (e.g., including tests)
        #[clap(long)]
        dev: bool,
    },

    /// Run unit tests in the packages (locally, without network)
    Test {
        /// Filter on package level
        #[clap(flatten)]
        pkg_filter: FilterPackage,

        /// Test case filter
        #[clap(long)]
        test_filter: Option<String>,

        /// Gas metering
        #[clap(long)]
        gas: bool,

        /// Single thread
        #[clap(long)]
        single_thread: bool,
    },

    /// Run end-to-end runbook in a locally simulated network
    Exec {
        /// Path to runbook file or a directory
        #[clap(long)]
        runbook: Option<PathBuf>,

        /// Use realistic gas settings
        #[clap(long)]
        realistic_gas: bool,
    },

    /// Run the entire fuzz testing on the project
    Auto {
        /// Mark the main packages to fuzz
        #[clap(flatten)]
        pkg_filter: FilterPackage,

        /// Seed for all randomness in the fuzzing process
        #[clap(long)]
        seed: Option<u64>,

        /// Max trace depth
        #[clap(long, default_value = "3")]
        max_trace_depth: usize,

        /// Max call repetition
        #[clap(long, default_value = "1")]
        max_call_repetition: usize,

        /// Max seconds to spend generating scripts for one primary function.
        /// Set to 0 to disable the wall-clock cap.
        #[clap(long, default_value_t = DEFAULT_MAX_SCRIPT_GEN_SECS_PER_FUNCTION)]
        max_script_gen_secs_per_function: u64,

        /// Number of user accounts to simulate
        #[clap(long, default_value = "3")]
        num_user_accounts: usize,

        /// Generate and compile the driver scripts, then stop (do not enter fuzzing loop)
        #[clap(long)]
        dry_run: bool,

        /// Path to a string dictionary file (one string per line)
        #[clap(long)]
        string_dict: Option<PathBuf>,

        /// Directory for persistent fuzz state and stats.
        /// Defaults to `<project>/.move-fuzz`.
        #[clap(long)]
        state_dir: Option<PathBuf>,

        /// Wipe any persisted fuzz state, including cached package builds, before starting.
        #[clap(long)]
        reset_state: bool,

        /// Max length of dependency chains for multi-transaction fuzzing
        #[clap(long, default_value = "5")]
        max_chain_length: usize,

        /// Max times a script can repeat within a single chain
        #[clap(long, default_value = "2")]
        max_chain_repetition: usize,

        /// Seconds without new coverage before transitioning from Phase 1 to Phase 2
        #[clap(long, default_value = "120")]
        saturation_secs: u64,

        /// Hard wall-clock budget for the whole campaign, in seconds. When it
        /// elapses the fuzzer stops in whichever phase it is in, after writing
        /// a final checkpoint. The budget is per invocation: a resumed run
        /// starts a fresh one. 0 means no limit.
        #[clap(long, default_value = "0")]
        max_total_secs: u64,

        /// Hard cap on the number of mutation-loop rounds across both phases.
        /// When it is reached the fuzzer stops, after writing a final
        /// checkpoint. Counted per invocation. 0 means no limit.
        #[clap(long, default_value = "0")]
        max_iterations: u64,
    },
}

/// Package-level filter
#[derive(Parser)]
pub struct FilterPackage {
    /// Include dependencies
    #[clap(long)]
    include_deps: bool,

    /// Include Aptos Framework packages
    #[clap(long)]
    include_framework: bool,

    /// Allow-list
    #[clap(long)]
    include_pkg: Option<Vec<String>>,

    /// Deny-list
    #[clap(long)]
    exclude_pkg: Option<Vec<String>>,
}

impl FilterPackage {
    pub fn apply(&self, pkgs: Vec<PkgDeclaration>) -> Result<Vec<PkgDeclaration>> {
        let include_regex = match self.include_pkg.as_ref() {
            None => None,
            Some(patterns) => Some(
                patterns
                    .iter()
                    .map(|p| {
                        Regex::new(&format!("^{p}$"))
                            .map_err(|e| anyhow!("invalid regex '{p}': {e}"))
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
        };
        let exclude_regex = match self.exclude_pkg.as_ref() {
            None => None,
            Some(patterns) => Some(
                patterns
                    .iter()
                    .map(|p| {
                        Regex::new(&format!("^{p}$"))
                            .map_err(|e| anyhow!("invalid regex '{p}': {e}"))
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
        };

        // filtering logic: include first then exclude
        let mut filtered = vec![];
        for pkg in pkgs {
            // filter based on kind
            match &pkg.kind {
                PkgKind::Framework if !self.include_framework => {
                    continue;
                },
                PkgKind::Dependency if !self.include_deps => {
                    continue;
                },
                PkgKind::Primary | PkgKind::Dependency | PkgKind::Framework => (),
            }

            // filter based on name
            let manifest = &pkg.manifest;
            match include_regex.as_ref() {
                None => (),
                Some(regexes) => {
                    if regexes.iter().all(|r| !r.is_match(&manifest.name)) {
                        continue;
                    }
                },
            }
            match exclude_regex.as_ref() {
                None => (),
                Some(regexes) => {
                    if regexes.iter().any(|r| r.is_match(&manifest.name)) {
                        continue;
                    }
                },
            }

            // if the control flow reaches here, we need to include this package
            filtered.push(pkg);
        }

        Ok(filtered)
    }
}

/// Entrypoint on move-fuzz from the CLI
pub fn run_on(
    path: PathBuf,
    subdirs: Vec<PathBuf>,
    language: LanguageSetting,
    name_aliases: Vec<String>,
    resource_accounts: Vec<String>,
    in_place: bool,
    skip_deps_update: bool,
    verbose: u8,
    command: FuzzCommand,
) -> Result<()> {
    // initialize logging
    env_logger::builder()
        .format_timestamp(None)
        .format_target(false)
        .format_module_path(false)
        .filter_level(match verbose {
            0 => LevelFilter::Warn,
            1 => LevelFilter::Info,
            2 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        })
        .init();
    info!("analyzing project at path: {}", path.to_string_lossy());

    // sanity check paths
    if !path.exists() {
        bail!("project path does not exist: {}", path.display());
    }
    let project_root = path.canonicalize()?;
    for item in &subdirs {
        let path_subdir = path.join(item);
        if !path_subdir.exists() {
            bail!(
                "project subdirectory does not exist: {}",
                path_subdir.display()
            );
        }
    }

    // construct the named aliases
    let address_aliases = build_address_aliases(name_aliases)?;

    // mark resource accounts created from regular addresses
    let mut resource_mapping = BTreeMap::new();
    for item in resource_accounts {
        let (resource, base, seed) = split_on_char(&item, '=')
            .and_then(|(resource, rest)| {
                split_on_char(rest, ':').map(|(base, seed)| (resource, base, seed))
            })
            .ok_or_else(|| anyhow!("invalid resource declaration: {item}"))?;

        resource_mapping.insert(resource.to_string(), (base.to_string(), seed.to_string()));
    }

    // copy over the workspace
    let tempdir = if in_place {
        None
    } else {
        let dir = TempDir::new()?;
        dir::copy(
            &path,
            dir.path(),
            &dir::CopyOptions::new().content_only(true),
        )?;
        Some(dir)
    };
    let workdir = tempdir
        .as_ref()
        .map_or(path.as_path(), |d| d.path())
        .canonicalize()?;

    // resolve the project
    let project = deps::resolve(
        &workdir,
        subdirs
            .into_iter()
            .map(|p| {
                workdir
                    .join(p)
                    .canonicalize()
                    .expect("canonicalized path in work directory")
            })
            .collect(),
        language,
        address_aliases.into_iter().collect(),
        resource_mapping,
        skip_deps_update,
    )?;

    // execute the command
    match command {
        FuzzCommand::List => {
            cmd_list(project);
        },
        FuzzCommand::Build { pkg_filter, dev } => {
            cmd_build(project, pkg_filter, dev)?;
        },
        FuzzCommand::Test {
            pkg_filter,
            test_filter,
            gas,
            single_thread,
        } => {
            cmd_test(project, pkg_filter, test_filter, gas, single_thread)?;
        },
        FuzzCommand::Exec {
            runbook,
            realistic_gas,
        } => match runbook {
            None => cmd_exec(&project, None, realistic_gas)?,
            Some(path) => {
                let mut targets = vec![];
                if path.is_file() {
                    targets.push(path);
                } else {
                    for entry in WalkDir::new(&path) {
                        let entry = entry?;
                        if entry.path().extension().is_some_and(|ext| ext == "json") {
                            targets.push(entry.path().to_owned());
                        }
                    }
                }
                for target in targets {
                    cmd_exec(&project, Some(&target), realistic_gas)?;
                }
            },
        },
        FuzzCommand::Auto {
            pkg_filter,
            seed,
            max_trace_depth,
            max_call_repetition,
            max_script_gen_secs_per_function,
            num_user_accounts,
            dry_run,
            string_dict,
            state_dir,
            reset_state,
            max_chain_length,
            max_chain_repetition,
            saturation_secs,
            max_total_secs,
            max_iterations,
        } => {
            cmd_auto(
                &project_root,
                &workdir,
                project,
                pkg_filter,
                seed,
                max_trace_depth,
                max_call_repetition,
                max_script_gen_secs_per_function,
                num_user_accounts,
                dry_run,
                string_dict,
                state_dir,
                reset_state,
                max_chain_length,
                max_chain_repetition,
                saturation_secs,
                max_total_secs,
                max_iterations,
            )?;
        },
    }

    // clean-up
    if let Some(dir) = tempdir {
        dir.close()?;
    }

    // done
    Ok(())
}

fn cmd_list(project: Project) {
    for pkg in project.pkgs {
        println!(
            "{} [{}] :{:?}",
            pkg.manifest.name, pkg.manifest.version, pkg.kind
        );
    }
}

fn cmd_build(project: Project, pkg_filter: FilterPackage, dev_mode: bool) -> Result<()> {
    let Project {
        pkgs,
        named_accounts,
        language,
    } = project;

    for pkg in pkg_filter.apply(pkgs)? {
        package::build(&pkg.manifest, &named_accounts, language, dev_mode)?;
    }

    Ok(())
}

fn cmd_test(
    project: Project,
    pkg_filter: FilterPackage,
    test_filter: Option<String>,
    gas: bool,
    single_thread: bool,
) -> Result<()> {
    let Project {
        pkgs,
        named_accounts,
        language,
    } = project;

    // configure hooks ahead of time for unit tests
    natives::configure_for_unit_test();
    configure_extended_checks_for_unit_test();

    // run tests on each of the packages
    for pkg in pkg_filter.apply(pkgs)? {
        package::unit_test(
            &pkg.manifest,
            &named_accounts,
            language,
            test_filter.as_deref(),
            gas,
            single_thread,
        )?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_auto(
    project_root: &Path,
    workdir: &Path,
    project: Project,
    mut pkg_filter: FilterPackage,
    seed: Option<u64>,
    max_trace_depth: usize,
    max_call_repetition: usize,
    max_script_gen_secs_per_function: u64,
    num_user_accounts: usize,
    dry_run: bool,
    path_string_dict: Option<PathBuf>,
    state_dir: Option<PathBuf>,
    reset_state: bool,
    max_chain_length: usize,
    max_chain_repetition: usize,
    saturation_secs: u64,
    max_total_secs: u64,
    max_iterations: u64,
) -> Result<()> {
    // we need to see all packages unless the package is explicitly excluded
    if !pkg_filter.include_framework {
        pkg_filter.include_framework = true;
        info!("fuzzer overrides the `--include-framework` flag and sets it to true");
    }
    if !pkg_filter.include_deps {
        pkg_filter.include_deps = true;
        info!("fuzzer overrides the `--include-deps` flag and sets it to true");
    }

    // build all packages initially, this is also a sanity check on the packages
    let Project {
        pkgs,
        named_accounts,
        language,
    } = project;
    let state_dir = prepare_state_dir(project_root, state_dir, reset_state)?;
    let path_fuzz_stats = state_dir.join("fuzz_stats.json");
    let package_cache_root = state_dir.join(PACKAGE_BUILD_CACHE_DIR);
    let package_build_started = Instant::now();

    let mut autogen_deps = BTreeMap::new();
    let mut pkg_defs = vec![];
    let mut package_fingerprints = BTreeMap::new();
    let filtered_pkgs = pkg_filter.apply(pkgs)?;
    let total_packages = filtered_pkgs.len();
    write_frontend_stats(
        &path_fuzz_stats,
        "building_packages",
        total_packages,
        0,
        None,
        package_build_started.elapsed().as_secs_f64(),
    );
    for (processed_packages, pkg_decl) in filtered_pkgs.into_iter().enumerate() {
        let manifest = &pkg_decl.manifest;
        let existing = autogen_deps.insert(manifest.name.clone(), manifest.clone());
        assert!(existing.is_none());
        let manifest_identity = stable_project_path(project_root, workdir, &manifest.path)
            .display()
            .to_string();
        let cache_slot = package::package_cache_slot_dir(
            &package_cache_root,
            &manifest.name,
            &manifest_identity,
        );
        let dependency_fingerprints = manifest
            .deps
            .keys()
            .map(|dep_name| {
                let fingerprint: &String = package_fingerprints.get(dep_name).ok_or_else(|| {
                    anyhow!(
                        "missing package build fingerprint for dependency {dep_name} of {}",
                        manifest.name
                    )
                })?;
                Ok((dep_name.clone(), fingerprint.clone()))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let cache_fingerprint = package::build_cache_fingerprint(
            manifest,
            &named_accounts,
            language,
            false,
            &dependency_fingerprints,
        )?;

        let package = match package::load_build_cache(
            &cache_slot,
            &manifest.path,
            &manifest_identity,
            &manifest.name,
            &cache_fingerprint,
        ) {
            Ok(Some(package)) => {
                info!(
                    "loaded cached package build for {} from {}",
                    manifest.name,
                    cache_slot.display()
                );
                package
            },
            Ok(None) => {
                log::debug!("compiling package {}", manifest.name);
                let pkg_built = package::build(manifest, &named_accounts, language, false)?;
                if let Err(err) = package::save_build_cache(
                    &cache_slot,
                    manifest,
                    &pkg_built,
                    &manifest_identity,
                    &cache_fingerprint,
                ) {
                    warn!(
                        "failed to persist package build cache for {}: {err:#}",
                        manifest.name
                    );
                }
                package::FuzzPackage::from_built(pkg_built)
            },
            Err(err) => {
                warn!(
                    "failed to load package build cache for {}: {err:#}; rebuilding",
                    manifest.name
                );
                let pkg_built = package::build(manifest, &named_accounts, language, false)?;
                if let Err(save_err) = package::save_build_cache(
                    &cache_slot,
                    manifest,
                    &pkg_built,
                    &manifest_identity,
                    &cache_fingerprint,
                ) {
                    warn!(
                        "failed to persist package build cache for {}: {save_err:#}",
                        manifest.name
                    );
                }
                package::FuzzPackage::from_built(pkg_built)
            },
        };
        package_fingerprints.insert(manifest.name.clone(), cache_fingerprint);
        write_frontend_stats(
            &path_fuzz_stats,
            "building_packages",
            total_packages,
            processed_packages + 1,
            Some(manifest.name.as_str()),
            package_build_started.elapsed().as_secs_f64(),
        );

        // NOTE: as `pkgs` are in the topological order of the dependency graph, so are `pkg_defs`
        pkg_defs.push(PkgDefinition {
            kind: pkg_decl.kind,
            manifest_path: manifest.path.clone(),
            package,
        });
    }

    write_frontend_stats(
        &path_fuzz_stats,
        "preparing_autogen",
        total_packages,
        total_packages,
        None,
        package_build_started.elapsed().as_secs_f64(),
    );

    // prepare the autogen package directory to host derived Move code
    let autogen_dir = prepare_autogen_dir(workdir)?;

    let autogen_name = "Autogen".to_string();
    let autogen_deps_str = autogen_deps
        .iter()
        .map(|(key, val)| format!("{key} = {{ local = \"{}\" }}", val.path.display()))
        .collect::<Vec<_>>()
        .join("\n");
    let autogen_toml = format!(
        r#"
[package]
name = "{autogen_name}"
version = "1.0.0"
upgrade_policy = "compatible"
authors = []

[dependencies]
{autogen_deps_str}
"#
    );
    fs::write(autogen_dir.join("Move.toml"), autogen_toml)?;
    fs::create_dir(autogen_dir.join("sources"))?;

    // create a manifest for the autogen package
    let autogen_manifest = PkgManifest {
        name: autogen_name,
        path: autogen_dir,
        version: (1, 0, 0).into(),
        deps: autogen_deps,
        named_addresses: BTreeMap::new(),
    };

    // prepare for coverage tracing
    let cov_trace_path = workdir.join("cov.trace");

    // load string dictionary if provided
    let dict_string = match path_string_dict {
        Some(path) => {
            let content = fs::read_to_string(&path)
                .map_err(|e| anyhow!("failed to read string dictionary {}: {e}", path.display()))?;
            content.lines().map(|s| s.to_string()).collect()
        },
        None => vec![],
    };

    // done with preparation, now call the fuzzer
    fuzzer::entrypoint(
        pkg_defs,
        named_accounts,
        language,
        autogen_manifest,
        cov_trace_path,
        seed,
        max_trace_depth,
        max_call_repetition,
        max_script_gen_secs_per_function,
        num_user_accounts,
        dry_run,
        dict_string,
        path_fuzz_stats,
        state_dir.join(AUTO_STATE_FILENAME),
        state_dir.join(ENTRYPOINT_CACHE_FILENAME),
        max_chain_length,
        max_chain_repetition,
        saturation_secs,
        max_total_secs,
        max_iterations,
    )
}

fn write_frontend_stats(
    path_fuzz_stats: &Path,
    stage: &str,
    total_packages: usize,
    processed_packages: usize,
    current_package: Option<&str>,
    elapsed_secs: f64,
) {
    let payload = json!({
        "stage": stage,
        "total_packages": total_packages,
        "processed_packages": processed_packages,
        "current_package": current_package,
        "elapsed_secs": elapsed_secs,
    });
    if let Ok(encoded) = serde_json::to_vec_pretty(&payload) {
        let _ = fs::write(path_fuzz_stats, encoded);
    }
}

fn cmd_exec(project: &Project, runbook: Option<&Path>, realistic_gas: bool) -> Result<()> {
    // initialize the simulator
    let mut simulator = Simulator::new(project.language, realistic_gas)?;
    provision_simulator(&mut simulator, project)?;

    // execute the runbook in the simulator
    let result = match runbook {
        None => Ok(()),
        Some(path) => execute_runbook(&mut simulator, path),
    };

    // clean-up either on success or on failure
    simulator.destroy()?;

    // return the execution result
    result
}

/// Utility: split on a given char
fn split_on_char(s: &str, sep: char) -> Option<(&str, &str)> {
    let mut iter = s.split(sep);
    let p1 = iter.next()?;
    let p2 = iter.next()?;
    if p1.is_empty() || p2.is_empty() || iter.next().is_some() {
        return None;
    }
    Some((p1, p2))
}

fn build_address_aliases(name_aliases: Vec<String>) -> Result<Vec<BTreeSet<String>>> {
    let mut address_aliases: Vec<BTreeSet<String>> = vec![];
    for item in name_aliases {
        let (lhs, rhs) = split_on_char(&item, '=')
            .ok_or_else(|| anyhow!("invalid alias declaration: {item}"))?;

        let lhs_pos = address_aliases.iter().position(|set| set.contains(lhs));
        let rhs_pos = address_aliases.iter().position(|set| set.contains(rhs));

        match (lhs_pos, rhs_pos) {
            (None, None) => {
                address_aliases.push([lhs.to_string(), rhs.to_string()].into_iter().collect());
            },
            (Some(lhs_idx), None) => {
                address_aliases
                    .get_mut(lhs_idx)
                    .expect("alias set exists")
                    .insert(rhs.to_string());
            },
            (None, Some(rhs_idx)) => {
                address_aliases
                    .get_mut(rhs_idx)
                    .expect("alias set exists")
                    .insert(lhs.to_string());
            },
            (Some(lhs_idx), Some(rhs_idx)) if lhs_idx == rhs_idx => {},
            (Some(lhs_idx), Some(rhs_idx)) => {
                let (dst_idx, src_idx) = if lhs_idx < rhs_idx {
                    (lhs_idx, rhs_idx)
                } else {
                    (rhs_idx, lhs_idx)
                };
                let mut src_set = address_aliases.swap_remove(src_idx);
                address_aliases
                    .get_mut(dst_idx)
                    .expect("destination alias set exists")
                    .append(&mut src_set);
            },
        }
    }
    Ok(address_aliases)
}

/// Name of the file move-fuzz drops into every directory it owns and may later
/// delete. Deletion is gated on this marker and nothing else: a directory
/// without it is somebody else's, whatever it happens to be called.
const OWNERSHIP_MARKER: &str = ".move-fuzz-owned";

const OWNERSHIP_MARKER_CONTENT: &str = "\
# Created by `move fuzz`. This directory is regenerated on every run and any
# file in it may be deleted without warning. Remove this marker to make
# move-fuzz refuse to touch the directory.
";

/// Claim `dir` for move-fuzz by writing the ownership marker into it.
fn claim_dir(dir: &Path) -> Result<()> {
    fs::write(dir.join(OWNERSHIP_MARKER), OWNERSHIP_MARKER_CONTENT)
        .with_context(|| format!("unable to mark {} as move-fuzz owned", dir.display()))
}

fn is_owned_dir(dir: &Path) -> bool {
    dir.join(OWNERSHIP_MARKER).is_file()
}

fn is_empty_dir(dir: &Path) -> Result<bool> {
    Ok(fs::read_dir(dir)
        .with_context(|| format!("unable to read {}", dir.display()))?
        .next()
        .is_none())
}

/// Recursively delete a directory move-fuzz previously claimed.
///
/// Refuses anything without the ownership marker. move-fuzz has no way to tell
/// a user's directory from its own except by the marker it wrote itself, so an
/// unmarked directory is reported rather than guessed at.
fn remove_owned_dir(dir: &Path, what: &str, remedy: &str) -> Result<()> {
    if !dir.is_dir() {
        bail!("{what} exists but is not a directory: {}", dir.display());
    }
    if is_empty_dir(dir)? {
        return fs::remove_dir(dir).with_context(|| format!("unable to remove {}", dir.display()));
    }
    if !is_owned_dir(dir) {
        bail!(
            "refusing to delete {what} at {}: it is not empty and has no `{OWNERSHIP_MARKER}` \
             marker, so it was not created by move fuzz. {remedy}",
            dir.display()
        );
    }
    fs::remove_dir_all(dir).with_context(|| format!("unable to remove {}", dir.display()))
}

fn prepare_autogen_dir(workdir: &Path) -> Result<PathBuf> {
    let autogen_dir = workdir.join("autogen");
    if autogen_dir.exists() {
        remove_owned_dir(
            &autogen_dir,
            "the autogen directory",
            "Move it aside; move fuzz needs to own this directory to write generated drivers.",
        )?;
    }
    fs::create_dir_all(&autogen_dir)?;
    claim_dir(&autogen_dir)?;
    Ok(autogen_dir)
}

fn prepare_state_dir(
    project_root: &Path,
    state_dir: Option<PathBuf>,
    reset_state: bool,
) -> Result<PathBuf> {
    let state_dir = resolve_state_dir(project_root, state_dir);

    // A state directory that contains the project would put the project itself
    // inside move-fuzz's regenerated-output area, where `--reset-state` deletes
    // it. Reject the overlap instead of relying on the marker check below to
    // catch it, so the mistake is named for what it is.
    let resolved_state = state_dir
        .canonicalize()
        .unwrap_or_else(|_| state_dir.clone());
    let resolved_project = project_root
        .canonicalize()
        .unwrap_or_else(|_| project_root.to_path_buf());
    if resolved_project.starts_with(&resolved_state) {
        bail!(
            "--state-dir {} contains the project at {}; move-fuzz treats its state directory as \
             disposable and would delete the project with it",
            state_dir.display(),
            project_root.display()
        );
    }

    if state_dir.exists() {
        if reset_state {
            remove_owned_dir(
                &state_dir,
                "the state directory",
                "Move it aside, or point --state-dir somewhere else.",
            )?;
        } else if !is_owned_dir(&state_dir) && !is_empty_dir(&state_dir)? {
            bail!(
                "state directory {} is not empty and has no `{OWNERSHIP_MARKER}` marker, so it \
                 was not created by move fuzz. Point --state-dir at a fresh directory.",
                state_dir.display()
            );
        }
    }
    fs::create_dir_all(&state_dir)?;
    claim_dir(&state_dir)?;
    Ok(state_dir)
}

fn resolve_state_dir(project_root: &Path, state_dir: Option<PathBuf>) -> PathBuf {
    match state_dir {
        Some(path) if path.is_absolute() => path,
        Some(path) => project_root.join(path),
        None => project_root.join(".move-fuzz"),
    }
}

fn stable_project_path(project_root: &Path, workdir: &Path, path: &Path) -> PathBuf {
    match path.strip_prefix(workdir) {
        Ok(relative_path) => project_root.join(relative_path),
        Err(_) => path.to_path_buf(),
    }
}

/// Configures the unit test validation hook to run the extended checker.
fn configure_extended_checks_for_unit_test() {
    fn validate(env: &GlobalEnv) {
        extended_checks::run_extended_checks(env);
    }
    test_validation::set_validation_hook(Box::new(validate));
}

#[cfg(test)]
mod tests {
    use super::{
        build_address_aliases, claim_dir, is_owned_dir, prepare_autogen_dir, prepare_state_dir,
        resolve_state_dir, split_on_char,
    };
    use anyhow::Result;
    use std::{collections::BTreeSet, fs};
    use tempfile::TempDir;

    fn aliases_to_sorted_vec(input: Vec<BTreeSet<String>>) -> Vec<Vec<String>> {
        let mut result: Vec<Vec<String>> = input
            .into_iter()
            .map(|set| set.into_iter().collect())
            .collect();
        result.sort();
        result
    }

    #[test]
    fn test_build_address_aliases_merges_transitively() -> Result<()> {
        let aliases = build_address_aliases(vec![
            "a=b".to_string(),
            "b=c".to_string(),
            "x=y".to_string(),
        ])?;
        assert_eq!(aliases_to_sorted_vec(aliases), vec![
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            vec!["x".to_string(), "y".to_string()],
        ]);
        Ok(())
    }

    #[test]
    fn test_build_address_aliases_ignores_redundant_pair() -> Result<()> {
        let aliases = build_address_aliases(vec![
            "a=b".to_string(),
            "b=c".to_string(),
            "a=c".to_string(),
        ])?;
        assert_eq!(aliases_to_sorted_vec(aliases), vec![vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string()
        ]]);
        Ok(())
    }

    #[test]
    fn test_prepare_autogen_dir_cleans_fuzzer_directory() -> Result<()> {
        let tmp = TempDir::new()?;
        let autogen = tmp.path().join("autogen");
        fs::create_dir_all(autogen.join("sources"))?;
        fs::write(
            autogen.join("Move.toml"),
            "[package]\nname = \"Autogen\"\nversion = \"1.0.0\"\n",
        )?;
        fs::write(
            autogen.join("sources").join("stale.move"),
            "module 0x1::M {}",
        )?;
        // only a directory move-fuzz claimed may be wiped
        claim_dir(&autogen)?;

        let prepared = prepare_autogen_dir(tmp.path())?;
        assert_eq!(prepared, autogen);
        assert!(prepared.exists());
        assert!(!prepared.join("sources").join("stale.move").exists());
        assert!(is_owned_dir(&prepared));
        Ok(())
    }

    /// A user package that happens to be named `Autogen` must not be deleted.
    /// The package name was the old provenance test; it is not evidence.
    #[test]
    fn test_prepare_autogen_dir_rejects_unmarked_package_named_autogen() -> Result<()> {
        let tmp = TempDir::new()?;
        let autogen = tmp.path().join("autogen");
        fs::create_dir_all(autogen.join("sources"))?;
        fs::write(
            autogen.join("Move.toml"),
            "[package]\nname = \"Autogen\"\nversion = \"1.0.0\"\n",
        )?;
        let precious = autogen.join("sources").join("mine.move");
        fs::write(&precious, "module 0x1::mine {}")?;

        assert!(prepare_autogen_dir(tmp.path()).is_err());
        assert!(precious.exists(), "user source must survive the refusal");
        Ok(())
    }

    #[test]
    fn test_split_on_char_rejects_empty_segments() {
        assert_eq!(split_on_char("a=b", '='), Some(("a", "b")));
        assert_eq!(split_on_char("=b", '='), None);
        assert_eq!(split_on_char("a=", '='), None);
        assert_eq!(split_on_char("a=b=c", '='), None);
    }

    #[test]
    fn test_build_address_aliases_rejects_empty_names() {
        assert!(build_address_aliases(vec!["=b".to_string()]).is_err());
        assert!(build_address_aliases(vec!["a=".to_string()]).is_err());
    }

    #[test]
    fn test_prepare_autogen_dir_rejects_foreign_directory() -> Result<()> {
        let tmp = TempDir::new()?;
        let autogen = tmp.path().join("autogen");
        fs::create_dir_all(&autogen)?;
        fs::write(
            autogen.join("Move.toml"),
            "[package]\nname = \"NotAutogen\"\nversion = \"1.0.0\"\n",
        )?;

        assert!(prepare_autogen_dir(tmp.path()).is_err());
        Ok(())
    }

    #[test]
    fn test_resolve_state_dir_defaults_under_project_root() -> Result<()> {
        let tmp = TempDir::new()?;
        assert_eq!(
            resolve_state_dir(tmp.path(), None),
            tmp.path().join(".move-fuzz")
        );
        Ok(())
    }

    #[test]
    fn test_prepare_state_dir_resets_existing_directory() -> Result<()> {
        let tmp = TempDir::new()?;
        let state_dir = tmp.path().join(".move-fuzz");
        fs::create_dir_all(&state_dir)?;
        fs::write(state_dir.join("stale.json"), "{}")?;
        claim_dir(&state_dir)?;

        let prepared = prepare_state_dir(tmp.path(), None, true)?;
        assert_eq!(prepared, state_dir);
        assert!(prepared.exists());
        assert!(!prepared.join("stale.json").exists());
        assert!(is_owned_dir(&prepared));
        Ok(())
    }

    /// `--state-dir <project> --reset-state` used to delete the project: the
    /// state directory was removed recursively with no ownership check.
    #[test]
    fn test_prepare_state_dir_refuses_to_reset_the_project() -> Result<()> {
        let tmp = TempDir::new()?;
        let project = tmp.path().join("project");
        fs::create_dir_all(project.join("sources"))?;
        let precious = project.join("sources").join("important.move");
        fs::write(&precious, "module 0x1::important {}")?;

        let err = prepare_state_dir(&project, Some(project.clone()), true).unwrap_err();
        assert!(
            err.to_string().contains("contains the project"),
            "unexpected error: {err}"
        );
        assert!(precious.exists(), "project source must survive the refusal");
        Ok(())
    }

    /// The same protection for an unrelated directory that simply already has
    /// content: without the marker move-fuzz cannot know it is not ours.
    #[test]
    fn test_prepare_state_dir_refuses_unmarked_nonempty_dir() -> Result<()> {
        let tmp = TempDir::new()?;
        let elsewhere = tmp.path().join("someones-data");
        fs::create_dir_all(&elsewhere)?;
        let precious = elsewhere.join("notes.txt");
        fs::write(&precious, "do not delete")?;

        assert!(prepare_state_dir(tmp.path(), Some(elsewhere.clone()), true).is_err());
        assert!(precious.exists());
        // and the same without --reset-state, since the campaign would write into it
        assert!(prepare_state_dir(tmp.path(), Some(elsewhere.clone()), false).is_err());
        assert!(precious.exists());
        Ok(())
    }

    /// An empty directory is safe to adopt: there is nothing to lose.
    #[test]
    fn test_prepare_state_dir_adopts_empty_dir() -> Result<()> {
        let tmp = TempDir::new()?;
        let state_dir = tmp.path().join("fresh");
        fs::create_dir_all(&state_dir)?;

        let prepared = prepare_state_dir(tmp.path(), Some(state_dir.clone()), false)?;
        assert_eq!(prepared, state_dir);
        assert!(is_owned_dir(&prepared));
        Ok(())
    }
}

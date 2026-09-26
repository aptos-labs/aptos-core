// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Test-only selection policy. Build and lint commands retain the legacy selector.

use crate::{
    common::{head_package_graph, legacy_packages, package_name},
    SelectedPackageArgs, TARGETED_UNIT_TEST_PACKAGES_TO_IGNORE,
};
use anyhow::{ensure, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use clap::{Args, ValueEnum};
use determinator::{
    rules::{DeterminatorMarkChanged, DeterminatorPostRule, DeterminatorRules, PathRule},
    Determinator,
};
use glob::{MatchOptions, Pattern};
use guppy::graph::{
    cargo::{CargoOptions, CargoResolverVersion},
    DependencyDirection, PackageGraph,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    str::FromStr,
    sync::LazyLock,
};

// Shared with the workflow consumer. Listing requires neither Git nor Cargo metadata.
#[derive(Debug, Deserialize, Serialize)]
pub struct E2eRunner {
    description: String,
    workflow: String,
    job: String,
    nightly_jobs: Vec<String>,
}

static E2E_REGISTRY: LazyLock<BTreeMap<String, E2eRunner>> = LazyLock::new(|| {
    serde_json::from_str(include_str!(
        "../../../.github/actions/e2e-test-determinator/registry.json"
    ))
    .expect("embedded E2E registry must be valid JSON")
});

pub fn list_e2e_tests(format: PlanFormat) -> Result<()> {
    match format {
        PlanFormat::Json => println!("{}", serde_json::to_string_pretty(&*E2E_REGISTRY)?),
        PlanFormat::Text => {
            for (name, runner) in E2E_REGISTRY.iter() {
                println!("{name}: {} ({})", runner.description, runner.workflow);
            }
        },
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, ValueEnum, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Legacy,
    Subsystem,
    Compare,
}

#[derive(Clone, Debug, Args)]
pub struct PlanArgs {
    #[arg(long, value_enum, default_value = "text")]
    pub format: PlanFormat,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum PlanFormat {
    Text,
    Json,
}

/// A repository-relative glob, validated when parsed.
#[derive(Debug, Deserialize)]
#[serde(try_from = "String")]
struct Glob(Pattern);

impl TryFrom<String> for Glob {
    type Error = anyhow::Error;

    fn try_from(pattern: String) -> Result<Self> {
        pattern.parse()
    }
}

impl FromStr for Glob {
    type Err = anyhow::Error;

    fn from_str(pattern: &str) -> Result<Self> {
        validate_path(pattern)?;
        Ok(Self(Pattern::new(pattern)?))
    }
}

impl Glob {
    fn matches(&self, path: &str) -> bool {
        self.0.matches_with(path, MatchOptions {
            case_sensitive: true,
            require_literal_separator: true,
            require_literal_leading_dot: false,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    version: u32,
    unmatched_changes: String,
    #[serde(default)]
    global_test_inputs: Vec<Glob>,
    #[serde(default)]
    ignored_paths: Vec<Glob>,
    subsystems: BTreeMap<String, Subsystem>,
    #[serde(default)]
    e2e_tests: BTreeMap<String, E2eTest>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Subsystem {
    roots: Vec<String>,
    #[serde(default)]
    ignored_paths: Vec<Glob>,
    selection: Selection,
    #[serde(default)]
    related_test_roots: Vec<String>,
    #[serde(default)]
    related_test_packages: Vec<String>,
    #[serde(default)]
    always_test_packages: Vec<String>,
    #[serde(default)]
    path_rules: Vec<InputRule>,
    #[serde(default)]
    e2e_tests: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct E2eTest {
    #[serde(default)]
    affected_packages: Vec<String>,
    #[serde(default)]
    input_paths: Vec<Glob>,
}

#[derive(Debug, Default)]
struct SelectionResult {
    packages: BTreeSet<String>,
    e2e_tests: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Selection {
    Affected,
    All,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InputRule {
    paths: Vec<Glob>,
    affects_packages: Vec<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct TestPlan {
    schema_version: u32,
    mode: Mode,
    base: String,
    head: String,
    /// Tracked changes between the merge base and the working tree, including
    /// staged/unstaged edits; untracked files are not analyzed.
    changed_paths: Vec<String>,
    explicit_packages: bool,
    pub packages: Vec<String>,
    #[serde(skip)]
    package_specs: BTreeMap<String, String>,
    e2e_tests: BTreeMap<String, Vec<String>>,
    e2e_legacy_only: Vec<String>,
    e2e_subsystem_only: Vec<String>,
    excluded_packages: Vec<String>,
    subsystems: BTreeMap<String, SubsystemPlan>,
    reasons: Vec<String>,
    comparison_error: Option<String>,
    legacy_only: Vec<String>,
    subsystem_only: Vec<String>,
}

#[derive(Debug, Default, Serialize)]
struct SubsystemPlan {
    changed_paths: Vec<String>,
    seeds: BTreeSet<String>,
    packages: BTreeSet<String>,
    e2e_tests: BTreeMap<String, Vec<String>>,
    reasons: Vec<String>,
}

impl TestPlan {
    pub fn execution_packages(&self) -> Vec<String> {
        self.packages
            .iter()
            .map(|name| self.package_specs.get(name).unwrap_or(name).clone())
            .collect()
    }

    pub fn print(&self, format: PlanFormat) -> Result<()> {
        match format {
            PlanFormat::Json => println!("{}", serde_json::to_string_pretty(self)?),
            PlanFormat::Text => {
                println!(
                    "Test selection: {:?}; base: {}; head: {}",
                    self.mode, self.base, self.head
                );
                for reason in &self.reasons {
                    println!("  {reason}");
                }
                for (name, subsystem) in &self.subsystems {
                    println!(
                        "  {name}: {} changed paths, {} packages",
                        subsystem.changed_paths.len(),
                        subsystem.packages.len()
                    );
                    for reason in &subsystem.reasons {
                        println!("    {reason}");
                    }
                }
                if let Some(error) = &self.comparison_error {
                    eprintln!("Subsystem comparison failed; executing legacy selection: {error}");
                }
                if self.mode == Mode::Compare {
                    println!("  Legacy only: {:?}", self.legacy_only);
                    println!("  Subsystem only: {:?}", self.subsystem_only);
                    println!("  E2E legacy only: {:?}", self.e2e_legacy_only);
                    println!("  E2E subsystem only: {:?}", self.e2e_subsystem_only);
                }
                for (name, reasons) in &self.e2e_tests {
                    println!("  E2E {name}: {}", reasons.join("; "));
                }
                println!("  Excluded by runner: {:?}", self.excluded_packages);
                println!(
                    "Test packages ({}): {}",
                    self.packages.len(),
                    self.packages.join(", ")
                );
            },
        }
        Ok(())
    }

    fn finish(&mut self, packages: BTreeSet<String>) {
        let (excluded, selected): (Vec<_>, Vec<_>) =
            packages.into_iter().partition(|name| excluded(name));
        self.excluded_packages = excluded;
        self.packages = selected;
    }
}

fn excluded(spec: &str) -> bool {
    TARGETED_UNIT_TEST_PACKAGES_TO_IGNORE.contains(&package_name(spec))
}

fn runnable(packages: &BTreeSet<String>) -> BTreeSet<String> {
    packages
        .iter()
        .filter(|name| !excluded(name))
        .cloned()
        .collect()
}

fn names(packages: Vec<String>) -> BTreeSet<String> {
    packages
        .iter()
        .map(|spec| package_name(spec).to_owned())
        .collect()
}

fn load_config(args: &SelectedPackageArgs, root: &Utf8Path) -> Result<Config> {
    let own_path: Glob = args.subsystem_config.parse()?;
    let path = root.join(&args.subsystem_config);
    let mut config: Config =
        toml::from_str(&fs::read_to_string(&path).with_context(|| format!("reading {path}"))?)?;
    // Policy changes cannot disable their own global coverage.
    config.global_test_inputs.push(own_path);
    Ok(config)
}

pub fn plan(args: &SelectedPackageArgs) -> Result<TestPlan> {
    let mode = args.determinator;
    let mut plan = TestPlan {
        schema_version: 1,
        mode,
        ..TestPlan::default()
    };
    if !args.package.is_empty() {
        plan.explicit_packages = true;
        plan.reasons
            .push("Explicit package selection overrides automatic determination".into());
        plan.finish(args.package.iter().cloned().collect());
        return Ok(plan);
    }

    plan.base = args.identify_merge_base()?;
    plan.head = args.git_rev_parse("HEAD")?;
    let paths = args.compute_changed_files(&plan.base)?;
    plan.changed_paths = paths.iter().map(|p| p.to_string()).collect();
    plan.changed_paths.sort();
    let head = head_package_graph()?;
    plan.package_specs = head
        .workspace()
        .iter()
        .map(|package| (package.name().to_owned(), package.id().repr().to_owned()))
        .collect();

    match mode {
        Mode::Legacy => {
            let base = args.base_package_graph(&plan.base)?;
            plan.reasons
                .push("Legacy dependency-based selection".into());
            plan.e2e_tests = legacy_e2e_tests();
            plan.finish(names(legacy_packages(&base, &head, &paths)?));
        },
        Mode::Subsystem => {
            let config = load_config(args, head.workspace().root())?;
            config.validate(&head, None)?;
            // Global inputs and an empty diff need no historical dependency graph.
            if matches_any(&config.global_test_inputs, &plan.changed_paths) {
                config.validate(&head, Some(&head))?;
                plan.reasons.push(GLOBAL_INPUT_REASON.into());
                let selected = global_selection(&head);
                plan.e2e_tests = selected.e2e_tests;
                plan.finish(selected.packages);
                return Ok(plan);
            }
            if plan.changed_paths.is_empty() {
                config.validate(&head, Some(&head))?;
                plan.reasons.push("No tracked changes".into());
                return Ok(plan);
            }
            let base = args.base_package_graph(&plan.base)?;
            let selected = select(
                &config,
                &base,
                &head,
                &plan.changed_paths,
                &mut plan.subsystems,
                &mut plan.reasons,
            )?;
            plan.e2e_tests = selected.e2e_tests;
            plan.finish(selected.packages);
        },
        Mode::Compare => {
            // A misspelled runner is a configuration mistake, even during rollout.
            // Other comparison failures still retain the legacy execution policy.
            let config = load_config(args, head.workspace().root());
            if let Ok(config) = &config {
                config.validate_e2e_names()?;
            }
            let base = args.base_package_graph(&plan.base)?;
            let legacy = names(legacy_packages(&base, &head, &paths)?);
            let compared = config.and_then(|config| {
                select(
                    &config,
                    &base,
                    &head,
                    &plan.changed_paths,
                    &mut plan.subsystems,
                    &mut plan.reasons,
                )
            });
            match compared {
                Ok(selected) => {
                    let left = runnable(&legacy);
                    let right = runnable(&selected.packages);
                    let legacy_e2e: BTreeSet<_> = legacy_e2e_tests().into_keys().collect();
                    let subsystem_e2e: BTreeSet<_> = selected.e2e_tests.into_keys().collect();
                    plan.e2e_legacy_only = legacy_e2e.difference(&subsystem_e2e).cloned().collect();
                    plan.e2e_subsystem_only =
                        subsystem_e2e.difference(&legacy_e2e).cloned().collect();
                    plan.legacy_only = left.difference(&right).cloned().collect();
                    plan.subsystem_only = right.difference(&left).cloned().collect();
                },
                Err(error) => plan.comparison_error = Some(format!("{error:#}")),
            }
            plan.reasons
                .push("Comparison mode executes the legacy selection".into());
            plan.e2e_tests = legacy_e2e_tests();
            plan.finish(legacy);
        },
    }
    Ok(plan)
}

const GLOBAL_INPUT_REASON: &str =
    "Global test input changed: selecting all workspace test packages";

fn global_selection(head: &PackageGraph) -> SelectionResult {
    SelectionResult {
        packages: workspace_packages(head).into_keys().collect(),
        e2e_tests: legacy_e2e_tests(),
    }
}

/// Package roots are relative to each graph's own workspace, including cached
/// base metadata produced on a different machine.
fn workspace_packages(graph: &PackageGraph) -> BTreeMap<String, Utf8PathBuf> {
    graph
        .workspace()
        .iter_by_path()
        .map(|(path, package)| (package.name().to_owned(), path.to_owned()))
        .collect()
}

fn validate_path(path: &str) -> Result<()> {
    ensure!(
        !path.is_empty()
            && !path.contains(['\\', ':'])
            && !path.starts_with('/')
            && path
                .split('/')
                .all(|c| !c.is_empty() && c != ".." && c != "."),
        "Invalid repository-relative path: {path}"
    );
    Ok(())
}

fn matches(patterns: &[Glob], path: &str) -> bool {
    patterns.iter().any(|pattern| pattern.matches(path))
}

fn matches_any(patterns: &[Glob], paths: &[String]) -> bool {
    paths.iter().any(|path| matches(patterns, path))
}

fn under(path: &str, roots: &[String]) -> bool {
    roots
        .iter()
        .any(|root| Utf8Path::new(path).starts_with(root))
}

fn legacy_e2e_tests() -> BTreeMap<String, Vec<String>> {
    E2E_REGISTRY
        .keys()
        .map(|name| {
            (name.clone(), vec![
                "Legacy E2E execution; workflow event gates still apply".into(),
            ])
        })
        .collect()
}

impl Config {
    fn validate_e2e_names(&self) -> Result<()> {
        for name in self.e2e_tests.keys() {
            ensure!(
                E2E_REGISTRY.contains_key(name),
                "Unknown E2E runner: {name}; use cargo x list-e2e-tests"
            );
        }
        for (subsystem, config) in &self.subsystems {
            for name in &config.e2e_tests {
                ensure!(E2E_REGISTRY.contains_key(name) && self.e2e_tests.contains_key(name), "Unknown E2E test in {subsystem}: {name}; use cargo x list-e2e-tests and define its dependencies");
            }
        }
        Ok(())
    }

    fn validate(&self, head: &PackageGraph, base: Option<&PackageGraph>) -> Result<()> {
        self.validate_e2e_names()?;
        ensure!(
            self.version == 1,
            "Unsupported subsystem schema version: {}",
            self.version
        );
        ensure!(
            self.unmatched_changes == "legacy",
            "unmatched_changes must be legacy"
        );
        ensure!(!self.subsystems.is_empty(), "No subsystems configured");
        let current = workspace_packages(head);
        let previous = base.map(workspace_packages).unwrap_or_default();
        for (name, test) in &self.e2e_tests {
            ensure!(
                !test.affected_packages.is_empty() || !test.input_paths.is_empty(),
                "E2E test {name} has no dependencies"
            );
            for package in &test.affected_packages {
                ensure!(
                    current.contains_key(package),
                    "Unknown workspace package in E2E test {name}: {package}"
                );
            }
        }
        for (name, subsystem) in &self.subsystems {
            ensure!(
                !name.is_empty() && !subsystem.roots.is_empty(),
                "Subsystem must have a name and roots"
            );
            for root in subsystem.roots.iter().chain(&subsystem.related_test_roots) {
                validate_path(root)?;
                ensure!(
                    !root.contains(['*', '?', '[', ']']),
                    "Roots cannot contain globs: {root}"
                );
                // Deletion is validated once base metadata is available. A root
                // can also contain standalone inputs rather than Cargo packages.
                if base.is_some() {
                    ensure!(
                        head.workspace().root().join(root).is_dir()
                            || current
                                .values()
                                .chain(previous.values())
                                .any(|p| p.starts_with(root)),
                        "Unknown subsystem root: {root}"
                    );
                }
            }
            for package in subsystem
                .related_test_packages
                .iter()
                .chain(&subsystem.always_test_packages)
                .chain(
                    subsystem
                        .path_rules
                        .iter()
                        .flat_map(|r| &r.affects_packages),
                )
            {
                ensure!(
                    current.contains_key(package),
                    "Unknown workspace package in {name}: {package}"
                );
            }
            for rule in &subsystem.path_rules {
                ensure!(
                    !rule.paths.is_empty() && !rule.affects_packages.is_empty(),
                    "Input mappings require paths and affected packages"
                );
            }
        }
        Ok(())
    }
}

/// The nearest workspace package containing `path`, relative to the graph's own root.
fn owner<'g>(path: &str, graph: &'g PackageGraph) -> Option<&'g str> {
    let workspace = graph.workspace();
    Utf8Path::new(path)
        .ancestors()
        .find_map(|dir| workspace.member_by_path(dir).ok())
        .map(|package| package.name())
}

fn select(
    config: &Config,
    base: &PackageGraph,
    head: &PackageGraph,
    changed: &[String],
    reports: &mut BTreeMap<String, SubsystemPlan>,
    reasons: &mut Vec<String>,
) -> Result<SelectionResult> {
    config.validate(head, Some(base))?;
    if matches_any(&config.global_test_inputs, changed) {
        reasons.push(GLOBAL_INPUT_REASON.into());
        return Ok(global_selection(head));
    }
    let current = workspace_packages(head);
    let mut selected = SelectionResult::default();
    let mut covered = BTreeSet::new();
    let mut ignored = BTreeSet::new();
    for path in changed {
        let mapped = config
            .subsystems
            .values()
            .any(|s| s.path_rules.iter().any(|r| matches(&r.paths, path)));
        let e2e_input = config
            .e2e_tests
            .values()
            .any(|test| matches(&test.input_paths, path));
        if !mapped && !e2e_input && matches(&config.ignored_paths, path) {
            ignored.insert(path.clone());
        }
    }
    for (name, subsystem) in &config.subsystems {
        let mut report = SubsystemPlan::default();
        let candidates: BTreeSet<_> = current
            .iter()
            .filter(|(_, root)| {
                under(root.as_str(), &subsystem.roots)
                    || under(root.as_str(), &subsystem.related_test_roots)
            })
            .map(|(name, _)| name.clone())
            .chain(subsystem.related_test_packages.iter().cloned())
            .chain(subsystem.always_test_packages.iter().cloned())
            .collect();
        let mut coarse = subsystem.selection == Selection::All;
        // Turn each input into one exact rule with all its seeds. This preserves
        // additive mappings and bypasses the engine's implicit path ignores.
        let mut rules = DeterminatorRules::parse("use-default-rules = false")?;
        for path in changed.iter().filter(|p| !ignored.contains(*p)) {
            let mappings: Vec<_> = subsystem
                .path_rules
                .iter()
                .filter(|r| matches(&r.paths, path))
                .collect();
            if !under(path, &subsystem.roots) && mappings.is_empty() {
                continue;
            }
            covered.insert(path.clone());
            let e2e_input = subsystem
                .e2e_tests
                .iter()
                .any(|test| matches(&config.e2e_tests[test].input_paths, path));
            // An ignore belongs only to this subsystem. Mark it covered so it
            // cannot trigger unmatched-path fallback; overlapping subsystems
            // still evaluate the path independently.
            if mappings.is_empty() && !e2e_input && matches(&subsystem.ignored_paths, path) {
                continue;
            }
            report.changed_paths.push(path.clone());
            let mut seeds = BTreeSet::new();
            let old_owner = owner(path, base);
            let new_owner = owner(path, head);
            for package in old_owner.into_iter().chain(new_owner) {
                report.seeds.insert(package.to_owned());
                if current.contains_key(package) {
                    seeds.insert(package.to_owned());
                }
            }
            for mapping in &mappings {
                seeds.extend(mapping.affects_packages.iter().cloned());
            }
            report.seeds.extend(seeds.iter().cloned());
            if old_owner.is_none() && new_owner.is_none() && mappings.is_empty() {
                coarse = true;
                report
                    .reasons
                    .push(format!("Unmapped input {path}: selecting all candidates"));
            }
            rules.path_rules.push(PathRule {
                globs: vec![Pattern::escape(path)],
                mark_changed: DeterminatorMarkChanged::Packages(seeds.into_iter().collect()),
                post_rule: DeterminatorPostRule::Skip,
            });
        }
        if report.changed_paths.is_empty() {
            continue;
        }
        if coarse {
            report.packages = candidates;
            for test in &subsystem.e2e_tests {
                report.e2e_tests.insert(test.clone(), vec![format!(
                    "{name}: all-candidate fallback"
                )]);
            }
            if subsystem.selection == Selection::All {
                report
                    .reasons
                    .push("Configured all-package selection".into());
            }
        } else {
            let mut engine = Determinator::new(base, head);
            let mut options = CargoOptions::new();
            options
                .set_resolver(CargoResolverVersion::V2)
                .set_include_dev(true);
            engine.set_cargo_options(&options);
            engine.set_rules(&rules)?;
            engine.add_changed_paths(&report.changed_paths);
            let affected: BTreeSet<_> = engine
                .compute()
                .affected_set
                .packages(DependencyDirection::Forward)
                .map(|p| p.name().to_owned())
                .collect();
            report.packages = affected.intersection(&candidates).cloned().collect();
            for test_name in &subsystem.e2e_tests {
                let test = &config.e2e_tests[test_name];
                let mut why = Vec::new();
                for package in &test.affected_packages {
                    if affected.contains(package) {
                        why.push(format!("{name}: affected package {package}"));
                    }
                }
                for path in changed.iter().filter(|p| !ignored.contains(*p)) {
                    if matches(&test.input_paths, path) {
                        why.push(format!("{name}: input {path}"));
                    }
                }
                if !why.is_empty() {
                    report.e2e_tests.insert(test_name.clone(), why);
                }
            }
            report
                .packages
                .extend(subsystem.always_test_packages.iter().cloned());
            report.reasons.push(
                "Dependency impact across base/head graphs, intersected with subsystem candidates"
                    .into(),
            );
        }
        selected.packages.extend(report.packages.iter().cloned());
        for (test, why) in &report.e2e_tests {
            selected
                .e2e_tests
                .entry(test.clone())
                .or_default()
                .extend(why.iter().cloned());
        }
        reports.insert(name.clone(), report);
    }
    let unmatched: Vec<_> = changed
        .iter()
        .filter(|p| !covered.contains(*p) && !ignored.contains(*p))
        .collect();
    if !unmatched.is_empty() {
        reasons.push(format!(
            "Unconfigured paths {unmatched:?}: union with legacy selection for the full change"
        ));
        selected
            .packages
            .extend(names(legacy_packages(base, head, changed)?));
        for (test, _) in legacy_e2e_tests() {
            selected
                .e2e_tests
                .entry(test)
                .or_default()
                .push("Unconfigured changes: retain legacy E2E coverage".into());
        }
    } else if covered.is_empty() {
        reasons.push("No relevant tracked changes".into());
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AptosCargoCli;
    use clap::Parser;
    use guppy::MetadataCommand;

    struct Fixture {
        dir: tempfile::TempDir,
    }

    impl Fixture {
        fn new() -> Self {
            let fixture = Self {
                dir: tempfile::tempdir().unwrap(),
            };
            fs::write(
                fixture.dir.path().join("Cargo.toml"),
                "[workspace]\nresolver = '2'\nmembers = ['move/*', 'outside/*', 'api/*']\n",
            )
            .unwrap();
            fixture.package("move/core", "core", &[]);
            fixture.package("outside/bridge", "bridge", &[("core", "../../move/core")]);
            fixture.package("move/consumer", "consumer", &[(
                "bridge",
                "../../outside/bridge",
            )]);
            fixture.package("api/related", "related", &[(
                "consumer",
                "../../move/consumer",
            )]);
            // The final consumer is test-only; intermediate edges are normal
            // dependencies so the transitive path changes their library builds.
            let related = fixture.dir.path().join("api/related/Cargo.toml");
            let manifest = fs::read_to_string(&related)
                .unwrap()
                .replace("[dependencies]", "[dev-dependencies]");
            fs::write(related, manifest).unwrap();
            fixture.package("api/unrelated", "unrelated", &[]);
            fixture
        }

        fn package(&self, path: &str, name: &str, deps: &[(&str, &str)]) {
            let dir = self.dir.path().join(path);
            fs::create_dir_all(dir.join("src")).unwrap();
            fs::write(dir.join("src/lib.rs"), "").unwrap();
            let mut manifest = format!(
                "[package]\nname = '{name}'\nversion = '0.1.0'\nedition = '2021'\n[dependencies]\n"
            );
            for (name, path) in deps {
                manifest.push_str(&format!("{name} = {{ path = '{path}' }}\n"));
            }
            fs::write(dir.join("Cargo.toml"), manifest).unwrap();
        }

        fn graph(&self) -> PackageGraph {
            let mut command = MetadataCommand::new();
            command.current_dir(self.dir.path());
            command.other_options(["--offline"]);
            command.exec().unwrap().build_graph().unwrap()
        }
    }

    fn config() -> Config {
        toml::from_str("version = 1\nunmatched_changes = 'legacy'\nglobal_test_inputs = ['Cargo.lock']\nignored_paths = ['**/*.md']\n[subsystems.move]\nroots = ['move']\nselection = 'affected'\nrelated_test_roots = ['api']\n").unwrap()
    }

    fn run(
        config: &Config,
        base: &PackageGraph,
        head: &PackageGraph,
        paths: &[&str],
    ) -> SelectionResult {
        select(
            config,
            base,
            head,
            &paths.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
            &mut BTreeMap::new(),
            &mut vec![],
        )
        .unwrap()
    }

    fn selection(
        config: &Config,
        base: &PackageGraph,
        head: &PackageGraph,
        paths: &[&str],
    ) -> BTreeSet<String> {
        run(config, base, head, paths).packages
    }

    fn globs(patterns: &[&str]) -> Vec<Glob> {
        patterns.iter().map(|p| p.parse().unwrap()).collect()
    }

    fn set(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|s| (*s).to_owned()).collect()
    }

    fn e2e_config() -> Config {
        let mut config = config();
        config.subsystems.get_mut("move").unwrap().e2e_tests = vec!["cli-e2e".into()];
        config.e2e_tests.insert("cli-e2e".into(), E2eTest {
            affected_packages: vec!["bridge".into()],
            input_paths: globs(&["move/consumer/fixture.md"]),
        });
        config
            .e2e_tests
            .insert("node-api-compatibility".into(), E2eTest {
                affected_packages: vec!["related".into()],
                input_paths: vec![],
            });
        config
    }

    fn e2e_selection(
        config: &Config,
        base: &PackageGraph,
        head: &PackageGraph,
        paths: &[&str],
    ) -> BTreeMap<String, Vec<String>> {
        run(config, base, head, paths).e2e_tests
    }

    #[test]
    fn e2e_requires_both_boundary_membership_and_dependency_impact() {
        let fixture = Fixture::new();
        let graph = fixture.graph();
        let config = e2e_config();
        let selected = e2e_selection(&config, &graph, &graph, &["move/core/src/lib.rs"]);
        // bridge is outside the Rust test boundary, but still an affected E2E dependency.
        assert_eq!(
            selected.keys().cloned().collect::<BTreeSet<_>>(),
            set(&["cli-e2e"])
        );
        assert!(selected["cli-e2e"]
            .iter()
            .any(|r| r.contains("affected package bridge")));
        // Both registered runners depend on core transitively, but only cli-e2e is listed.
        assert!(!selected.contains_key("node-api-compatibility"));
        // Changing a downstream consumer cannot affect bridge.
        assert!(e2e_selection(&config, &graph, &graph, &["move/consumer/src/lib.rs"]).is_empty());
    }

    #[test]
    fn e2e_file_dependencies_override_ignores_and_select_without_cargo_impact() {
        let fixture = Fixture::new();
        let graph = fixture.graph();
        let selected = e2e_selection(&e2e_config(), &graph, &graph, &["move/consumer/fixture.md"]);
        assert_eq!(
            selected.keys().cloned().collect::<BTreeSet<_>>(),
            set(&["cli-e2e"])
        );
        assert!(selected["cli-e2e"]
            .iter()
            .any(|r| r.contains("input move/consumer/fixture.md")));
    }

    #[test]
    fn e2e_fallbacks_preserve_coverage_and_empty_changes_select_nothing() {
        let fixture = Fixture::new();
        let graph = fixture.graph();
        let mut config = e2e_config();
        for paths in [vec!["python/test.py"], vec![
            "move/core/src/lib.rs",
            "outside/bridge/src/lib.rs",
        ]] {
            assert_eq!(
                e2e_selection(&config, &graph, &graph, &paths)
                    .keys()
                    .cloned()
                    .collect::<BTreeSet<_>>(),
                legacy_e2e_tests().into_keys().collect()
            );
        }
        assert!(e2e_selection(&config, &graph, &graph, &[]).is_empty());
        assert!(e2e_selection(&config, &graph, &graph, &["move/README.md"]).is_empty());
        // Coarse fallback stays within the subsystem's E2E boundary.
        assert_eq!(
            e2e_selection(&config, &graph, &graph, &["move/unknown/input.move"])
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>(),
            set(&["cli-e2e"])
        );
        config.subsystems.get_mut("move").unwrap().selection = Selection::All;
        assert_eq!(
            e2e_selection(&config, &graph, &graph, &["move/consumer/src/lib.rs"])
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>(),
            set(&["cli-e2e"])
        );
    }

    #[test]
    fn e2e_removed_dependency_edges_and_overlapping_boundaries() {
        let fixture = Fixture::new();
        let base = fixture.graph();
        fixture.package("outside/bridge", "bridge", &[]);
        fs::remove_dir_all(fixture.dir.path().join("move/core")).unwrap();
        let head = fixture.graph();
        let mut config = e2e_config();
        assert!(e2e_selection(&config, &base, &head, &[
            "move/core/src/lib.rs",
            "move/core/Cargo.toml"
        ])
        .contains_key("cli-e2e"));
        config.subsystems.insert("other".into(), Subsystem {
            ignored_paths: vec![],
            roots: vec!["move".into()],
            selection: Selection::Affected,
            related_test_roots: vec![],
            related_test_packages: vec![],
            always_test_packages: vec![],
            path_rules: vec![],
            e2e_tests: vec!["node-api-compatibility".into()],
        });
        assert_eq!(
            e2e_selection(&config, &base, &head, &[
                "move/core/src/lib.rs",
                "move/core/Cargo.toml"
            ])
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
            set(&["cli-e2e", "node-api-compatibility"])
        );
    }

    #[test]
    fn e2e_validation_rejects_unknown_runners_references_packages_and_bad_globs() {
        let fixture = Fixture::new();
        let graph = fixture.graph();
        let mut config = e2e_config();
        config
            .subsystems
            .get_mut("move")
            .unwrap()
            .e2e_tests
            .push("typo".into());
        assert!(config.validate(&graph, Some(&graph)).is_err());
        let mut config = e2e_config();
        config.e2e_tests.insert("typo".into(), E2eTest {
            affected_packages: vec!["core".into()],
            input_paths: vec![],
        });
        assert!(config.validate(&graph, Some(&graph)).is_err());
        let mut config = e2e_config();
        config
            .e2e_tests
            .get_mut("cli-e2e")
            .unwrap()
            .affected_packages = vec!["typo".into()];
        assert!(config.validate(&graph, Some(&graph)).is_err());
        for pattern in ["../fixture", "bad["] {
            assert!(pattern.parse::<Glob>().is_err(), "{pattern}");
        }
    }

    #[test]
    fn dependency_path_can_leave_and_reenter_subsystem() {
        let fixture = Fixture::new();
        let graph = fixture.graph();
        assert_eq!(
            selection(&config(), &graph, &graph, &["move/core/src/lib.rs"]),
            set(&["core", "consumer", "related"])
        );
        assert_eq!(
            selection(&config(), &graph, &graph, &["move/consumer/src/lib.rs"]),
            set(&["consumer", "related"])
        );
    }

    #[test]
    fn build_dependencies_propagate_through_the_subsystem_boundary() {
        let fixture = Fixture::new();
        let manifest = fixture.dir.path().join("outside/bridge/Cargo.toml");
        let contents = fs::read_to_string(&manifest)
            .unwrap()
            .replace("[dependencies]", "[build-dependencies]");
        fs::write(manifest, contents).unwrap();
        fs::write(
            fixture.dir.path().join("outside/bridge/build.rs"),
            "fn main() {}\n",
        )
        .unwrap();
        let graph = fixture.graph();
        assert_eq!(
            selection(&config(), &graph, &graph, &["move/core/src/lib.rs"]),
            set(&["core", "consumer", "related"])
        );
    }

    #[test]
    fn subsystem_ignores_skip_tests_without_legacy_or_coarse_fallback() {
        let fixture = Fixture::new();
        let graph = fixture.graph();
        let mut config = e2e_config();
        config.subsystems.get_mut("move").unwrap().ignored_paths =
            globs(&["move/core/doc/**", "move/documentation/**"]);
        let paths = ["move/core/doc/paper.tex", "move/documentation/example.move"];
        assert!(selection(&config, &graph, &graph, &paths).is_empty());
        assert!(e2e_selection(&config, &graph, &graph, &paths).is_empty());
        assert_eq!(
            selection(&config, &graph, &graph, &[
                paths[0],
                "move/consumer/src/lib.rs"
            ]),
            set(&["consumer", "related"])
        );
    }

    #[test]
    fn subsystem_ignores_do_not_suppress_other_subsystems_or_outside_changes() {
        let fixture = Fixture::new();
        let graph = fixture.graph();
        let mut config = e2e_config();
        config.subsystems.get_mut("move").unwrap().ignored_paths = globs(&["**"]);
        // An outside input still invokes full legacy fallback.
        assert!(
            selection(&config, &graph, &graph, &["outside/bridge/src/lib.rs"]).contains("bridge")
        );
        assert_eq!(
            e2e_selection(&config, &graph, &graph, &["outside/bridge/src/lib.rs"]).len(),
            E2E_REGISTRY.len()
        );
        config.subsystems.insert(
            "overlap".into(),
            toml::from_str(
                "roots = ['move/core']\nselection = 'affected'\nrelated_test_roots = ['outside']\n",
            )
            .unwrap(),
        );
        assert_eq!(
            selection(&config, &graph, &graph, &["move/core/src/lib.rs"]),
            set(&["core", "bridge"])
        );
    }

    #[test]
    fn subsystem_ignores_preserve_global_and_explicit_input_precedence() {
        let fixture = Fixture::new();
        let graph = fixture.graph();
        let mut config = e2e_config();
        config.subsystems.get_mut("move").unwrap().ignored_paths = globs(&["**"]);
        config
            .global_test_inputs
            .push("move/core/global.txt".parse().unwrap());
        assert_eq!(
            selection(&config, &graph, &graph, &["move/core/global.txt"]),
            set(&["core", "bridge", "consumer", "related", "unrelated"])
        );
        assert_eq!(
            e2e_selection(&config, &graph, &graph, &["move/consumer/fixture.md"])
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>(),
            set(&["cli-e2e"])
        );
        config
            .subsystems
            .get_mut("move")
            .unwrap()
            .path_rules
            .push(InputRule {
                paths: globs(&["move/input.txt"]),
                affects_packages: vec!["consumer".into()],
            });
        assert_eq!(
            selection(&config, &graph, &graph, &["move/input.txt"]),
            set(&["consumer", "related"])
        );
    }

    #[test]
    fn subsystem_ignores_validate_paths_and_globs() {
        for pattern in ["../docs/**", "docs/["] {
            assert!(
                toml::from_str::<Subsystem>(&format!(
                    "roots = ['move']\nselection = 'affected'\nignored_paths = ['{pattern}']\n"
                ))
                .is_err(),
                "{pattern}"
            );
        }
    }

    #[test]
    fn input_rules_are_additive_and_override_ignores() {
        let fixture = Fixture::new();
        let graph = fixture.graph();
        let mut config = config();
        let subsystem = config.subsystems.get_mut("move").unwrap();
        subsystem.path_rules = vec![
            InputRule {
                paths: globs(&["shared/input.md"]),
                affects_packages: vec!["consumer".into()],
            },
            InputRule {
                paths: globs(&["shared/input.md"]),
                affects_packages: vec!["unrelated".into()],
            },
        ];
        assert_eq!(
            selection(&config, &graph, &graph, &["shared/input.md"]),
            set(&["consumer", "related", "unrelated"])
        );
    }

    #[test]
    fn unmapped_inputs_and_all_mode_select_all_candidates() {
        let fixture = Fixture::new();
        let graph = fixture.graph();
        let mut config = config();
        let expected = set(&["core", "consumer", "related", "unrelated"]);
        assert_eq!(
            selection(&config, &graph, &graph, &["move/standalone/test.move"]),
            expected
        );
        config.subsystems.get_mut("move").unwrap().selection = Selection::All;
        assert_eq!(
            selection(&config, &graph, &graph, &["move/core/src/lib.rs"]),
            expected
        );
    }

    #[test]
    fn global_inputs_override_ignores_and_boundaries() {
        let fixture = Fixture::new();
        let graph = fixture.graph();
        let mut config = config();
        config.ignored_paths.push("Cargo.lock".parse().unwrap());
        assert_eq!(
            selection(&config, &graph, &graph, &["Cargo.lock"]),
            set(&["core", "consumer", "related", "unrelated", "bridge"])
        );
        assert!(selection(&config, &graph, &graph, &["move/core/README.md"]).is_empty());
        assert!(selection(&config, &graph, &graph, &[]).is_empty());
    }

    #[test]
    fn mixed_changes_union_full_legacy_selection() {
        let fixture = Fixture::new();
        let graph = fixture.graph();
        assert_eq!(
            selection(&config(), &graph, &graph, &[
                "move/core/src/lib.rs",
                "outside/bridge/src/lib.rs"
            ]),
            set(&["core", "consumer", "related", "bridge"])
        );
        // Related roots grant test eligibility but do not activate Move.
        assert_eq!(
            selection(&config(), &graph, &graph, &["api/unrelated/src/lib.rs"]),
            set(&["unrelated"])
        );
    }

    #[test]
    fn always_packages_and_overlapping_subsystems_are_unioned() {
        let fixture = Fixture::new();
        let graph = fixture.graph();
        let mut config = config();
        config
            .subsystems
            .get_mut("move")
            .unwrap()
            .always_test_packages
            .push("unrelated".into());
        config.subsystems.insert("bridge-tests".into(), Subsystem {
            roots: vec!["move/core".into()],
            ignored_paths: vec![],
            selection: Selection::Affected,
            related_test_roots: vec!["outside".into()],
            related_test_packages: vec![],
            always_test_packages: vec![],
            path_rules: vec![],
            e2e_tests: vec![],
        });
        assert_eq!(
            selection(&config, &graph, &graph, &["move/core/src/lib.rs"]),
            set(&["core", "consumer", "related", "unrelated", "bridge"])
        );
    }

    #[test]
    fn deleted_packages_and_removed_edges_affect_surviving_consumers() {
        let fixture = Fixture::new();
        let base = fixture.graph();
        fixture.package("outside/bridge", "bridge", &[]);
        fs::remove_dir_all(fixture.dir.path().join("move/core")).unwrap();
        let head = fixture.graph();
        assert_eq!(
            selection(&config(), &base, &head, &[
                "move/core/src/lib.rs",
                "move/core/Cargo.toml"
            ]),
            set(&["consumer", "related"])
        );
    }

    #[test]
    fn newly_added_packages_are_discovered() {
        let fixture = Fixture::new();
        let base = fixture.graph();
        fixture.package("move/new", "new", &[]);
        let head = fixture.graph();
        assert_eq!(
            selection(&config(), &base, &head, &[
                "move/new/src/lib.rs",
                "move/new/Cargo.toml"
            ]),
            set(&["new"])
        );
    }

    #[test]
    fn validation_rejects_invalid_config_and_paths() {
        let fixture = Fixture::new();
        let graph = fixture.graph();
        for path in [
            "",
            "../move",
            "/move",
            "move/../api",
            "C:\\move",
            "move//core",
        ] {
            assert!(validate_path(path).is_err(), "{path}");
        }
        let mut config = config();
        config.validate(&graph, Some(&graph)).unwrap();
        for version in [0, 2, u32::MAX] {
            config.version = version;
            let error = config.validate(&graph, Some(&graph)).unwrap_err();
            assert!(error
                .to_string()
                .contains("Unsupported subsystem schema version"));
        }
        config.version = 1;
        config
            .subsystems
            .get_mut("move")
            .unwrap()
            .related_test_packages
            .push("typo".into());
        assert!(config.validate(&graph, Some(&graph)).is_err());
        assert!(toml::from_str::<Config>("version=1\nunknown=true").is_err());
        assert!(!under("api-extra/lib.rs", &["api".into()]));
        assert!("move/**/*.move"
            .parse::<Glob>()
            .unwrap()
            .matches("move/main.move"));
    }

    #[test]
    fn plan_exclusions_and_cli_overrides() {
        let args = AptosCargoCli::try_parse_from([
            "cargo-x",
            "--determinator",
            "subsystem",
            "-p",
            "smoke-test",
            "-p",
            "move-core-types",
            "test-plan",
            "--format",
            "json",
        ])
        .unwrap();
        let plan = plan(&args.package_args).unwrap();
        assert_eq!(plan.packages, ["move-core-types"]);
        assert_eq!(plan.excluded_packages, ["smoke-test"]);
        assert!(plan.explicit_packages);
        assert_eq!(plan.mode, Mode::Subsystem);
        assert!(
            AptosCargoCli::try_parse_from(["cargo-x", "--determinator", "typo", "test-plan"])
                .is_err()
        );
    }
}

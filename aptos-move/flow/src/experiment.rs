// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native, deterministic helpers for the specification-inference experiment.
//!
//! Candidate enumeration is based on the compiler's model AST. Runtime-change
//! checking compares compiled modules after normalizing code for functions
//! whose executable ASTs differ only by specification blocks. The experiment
//! controller invokes these commands in a fresh process so it never trusts an
//! agent's claim that runtime code was preserved.

use crate::{
    candidate::{
        check_edit_scope, check_specification, CandidateCheckConfig, CandidateState,
        CandidateVerdict, ImplementationOutcome, PolicyReport, StageOutcome,
    },
    conditions::ConditionStatus,
    evaluation::sha256_hex,
    mcp::{
        package_data::{
            collect_diagnostics, inspect_diagnostics, render_diagnostics, DiagnosticRecord,
        },
        register_move_flow_package_hooks,
        tools::load_sanitized_prover_options,
    },
};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use codespan_reporting::term::termcolor::NoColor;
use move_model::{
    ast::{ConditionKind, Exp, ExpData, Operation, Value},
    metadata::{LanguageVersion, COMPILATION_METADATA_KEY},
    model::{FunId, FunctionEnv, GlobalEnv, ModuleEnv, QualifiedId, SpecFunId, VerificationScope},
    pragmas::{
        ABORTS_IF_IS_PARTIAL_PRAGMA, CONDITION_INFERRED_PROP, CONDITION_INFERRED_SATHARD,
        CONDITION_INFERRED_VACUOUS, VERIFY_PRAGMA,
    },
    ty::ReferenceKind,
};
use move_prover::inference::InferenceOutput;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// Which code a corpus may draw from, and what disqualifies a candidate.
///
/// This is one experiment's recipe rather than a property of Move, so it is an
/// input: changing the corpus is editing a file, not rebuilding this binary.
/// The structural exclusions -- a test-only function, a native without a body,
/// a trivial accessor -- stay in code, because they follow from the model
/// rather than from a choice about scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionPolicy {
    pub schema_version: u32,
    /// Packages to enumerate, in order.
    pub source_frames: Vec<SourceFrame>,
    /// A candidate whose path, module or function name contains one of these
    /// is excluded. Matched case-insensitively against
    /// `source_path::module::function`.
    pub safety_exclusion_terms: Vec<String>,
    /// Eligible-function count a module must fall within to be a target.
    pub module_function_count: CountRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFrame {
    /// Package directory, relative to the repository root.
    pub path: String,
    /// Substrings of which a source path must contain at least one. Empty
    /// admits the whole frame.
    #[serde(default)]
    pub include_paths: Vec<String>,
    /// Whether a candidate must already carry an upstream reference
    /// specification to be eligible.
    #[serde(default)]
    pub require_upstream_reference: bool,
    /// Reason recorded for a candidate this frame admits.
    pub eligible_reason: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CountRange {
    pub min: usize,
    pub max: usize,
}

impl CountRange {
    fn contains(&self, value: usize) -> bool {
        (self.min..=self.max).contains(&value)
    }
}

impl SelectionPolicy {
    fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("cannot read selection policy `{}`", path.display()))?;
        let policy: Self = serde_json::from_str(&text)
            .with_context(|| format!("invalid selection policy `{}`", path.display()))?;
        anyhow::ensure!(
            policy.schema_version == 1,
            "unsupported selection policy schema {}",
            policy.schema_version
        );
        anyhow::ensure!(
            !policy.source_frames.is_empty(),
            "selection policy names no source frame"
        );
        anyhow::ensure!(
            policy.module_function_count.min <= policy.module_function_count.max,
            "selection policy has an empty module function-count range"
        );
        Ok(policy)
    }

    /// The frame describing one source root.
    ///
    /// A root the policy does not name carries no frame restriction: corpus
    /// selection decides which code an experiment may draw from, and a package
    /// handed in directly has already been through that decision. The safety
    /// terms and the module band still apply.
    fn frame(&self, path: &str) -> SourceFrame {
        self.source_frames
            .iter()
            .find(|frame| frame.path == path)
            .cloned()
            .unwrap_or_else(|| SourceFrame {
                path: path.to_string(),
                include_paths: Vec::new(),
                require_upstream_reference: false,
                eligible_reason: "eligible_upstream_reference".to_string(),
            })
    }
}

#[derive(Parser, Debug, Clone)]
pub struct ExperimentArgs {
    #[command(subcommand)]
    command: ExperimentCommand,
}

#[derive(Subcommand, Debug, Clone)]
enum ExperimentCommand {
    /// Enumerate function and module candidates from compiler model ASTs.
    Inventory(InventoryArgs),
    /// Enumerate functions and exact dependency closures in an existing package.
    InventoryPackage(PackageInventoryArgs),
    /// Compare executable bytecode between pristine and edited packages.
    CompareImplementation(CompareImplementationArgs),
    /// Compile a package using Flow's compiler-model configuration.
    CheckPackage(PackageCheckArgs),
    /// Report function-contract coverage from the compiler model.
    ContractReport(PackageCheckArgs),
    /// Run treatment-blind WP compatibility.
    Infer(PackageTargetArgs),
    /// Run the prover using the same options as Flow's verification tool.
    Prove(PackageProveArgs),
    /// Run every agent-visible acceptance check for one candidate.
    CheckCandidate(CheckCandidateArgs),
}

#[derive(Parser, Debug, Clone)]
struct InventoryArgs {
    /// Root of the pinned aptos-core checkout.
    #[arg(long)]
    repo_root: PathBuf,
    /// Corpus selection policy: which packages and paths a candidate may come
    /// from, and which terms exclude one.
    #[arg(long)]
    selection_policy: PathBuf,
    /// Exact source commit recorded in the provenance manifest.
    #[arg(long)]
    source_commit: String,
    /// JSON output path.
    #[arg(long)]
    output: PathBuf,
}

#[derive(Parser, Debug, Clone)]
struct CompareImplementationArgs {
    /// Pristine task package.
    #[arg(long)]
    baseline: PathBuf,
    /// Agent-edited task package.
    #[arg(long)]
    candidate: PathBuf,
    /// JSON output path.
    #[arg(long)]
    output: PathBuf,
}

/// Enumerate one already selected package.
#[derive(Parser, Debug, Clone)]
struct PackageInventoryArgs {
    #[arg(long)]
    package: PathBuf,
    #[arg(long)]
    output: PathBuf,
    /// Corpus selection policy, for the safety terms and the module band.
    #[arg(long)]
    selection_policy: PathBuf,
}

#[derive(Parser, Debug, Clone)]
struct PackageCheckArgs {
    #[arg(long)]
    package: PathBuf,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Parser, Debug, Clone)]
struct PackageTargetArgs {
    #[arg(long)]
    package: PathBuf,
    #[arg(long)]
    target: String,
    #[arg(long)]
    output: PathBuf,
    /// Retain processor bytecode dumps beside the JSON report for diagnostics.
    #[arg(long)]
    dump_bytecode: bool,
    /// Write inferred `.spec.move` files beside their source modules.
    ///
    /// This mutates the package and is intended for disposable compatibility
    /// snapshots which are immediately passed to the prover.
    #[arg(long)]
    write_inferred_specs: bool,
}

#[derive(Parser, Debug, Clone)]
struct PackageProveArgs {
    #[arg(long)]
    package: PathBuf,
    /// Target to verify. Repeat to verify several against one built model.
    ///
    /// Building the model is the expensive part and does not depend on the
    /// target, so proving a package's contracts one process at a time pays for
    /// the same compile once per contract.
    #[arg(long, required = true)]
    target: Vec<String>,
    #[arg(long, default_value_t = 40)]
    timeout: usize,
    /// Generate an independent verification condition for each assertion.
    #[arg(long)]
    split_vcs_by_assert: bool,
    /// Ask whether the specifications are inconsistent instead of proving them.
    ///
    /// The prover asserts `false` at each function exit. An assertion that
    /// succeeds names a function whose assumptions are contradictory, so any
    /// postcondition holds there vacuously. Ordinary verification does not run
    /// in this mode.
    #[arg(long)]
    check_inconsistency: bool,
    #[arg(long)]
    output: PathBuf,
    /// Retain the generated Boogie program at this path for diagnostics.
    #[arg(long)]
    boogie_output: Option<PathBuf>,
}

#[derive(Parser, Debug, Clone)]
struct CheckCandidateArgs {
    /// Candidate check configuration written by the experiment controller.
    ///
    /// The task's baseline, target, editable paths, and required contract
    /// categories come from this file rather than from the checked workspace,
    /// so a candidate cannot relax the criteria it is judged by.
    #[arg(long)]
    config: PathBuf,
    #[arg(long)]
    output: PathBuf,
}

/// Offline corpus diagnostics may legitimately need more than the interactive
/// verifier's 60-second VC budget. Keep a finite guard against accidental
/// unbounded runs while allowing recorded long-proof evidence.
const MAX_EXPERIMENT_VC_TIMEOUT_SECS: usize = 300;

#[derive(Default)]
struct AstStats {
    nodes: usize,
    loops: usize,
    branches: usize,
    calls: usize,
    mutates_references: bool,
    higher_order: bool,
    global_state: bool,
    global_resource_mutation: bool,
    global_resource_types: BTreeSet<String>,
    arithmetic: bool,
    aborts: bool,
}

#[derive(Serialize)]
struct Inventory {
    schema_version: u32,
    source_commit: String,
    source_roots: Vec<String>,
    selection_policy: SelectionPolicy,
    candidates: Vec<Candidate>,
}

#[derive(Serialize)]
struct PackageInventory {
    schema_version: u32,
    package: String,
    candidates: Vec<Candidate>,
}

#[derive(Clone, Serialize)]
struct Candidate {
    source_root: String,
    source_path: String,
    package_module_target: String,
    granularity: String,
    module: String,
    function: Option<String>,
    target_functions: Vec<String>,
    source_sha256: String,
    source_loc: usize,
    function_count: usize,
    loops: usize,
    branches: usize,
    direct_callees: usize,
    dependency_depth: usize,
    /// Source-level closure, including inline functions and closure captures.
    /// This determines which source modules must be present in the package.
    transitive_function_dependencies: Vec<String>,
    /// Opaque/bodyless contract boundaries reachable through transparent
    /// implementations or through behavioral predicates in reached contracts.
    /// The prover executes transparent callees, but sees only specifications
    /// at these boundaries.
    called_function_dependencies: Vec<String>,
    /// Transitive spec functions referenced by the opaque/bodyless boundary
    /// contracts. This is a separate graph because `pragma opaque` stops only
    /// executable traversal, not calls made by specification expressions.
    spec_function_dependencies: Vec<String>,
    /// Diagnostic transitive call graph after inline expansion. This is useful
    /// for complexity measurements, but contracts below a direct modular call
    /// boundary are not requirements of the original target.
    transitive_called_function_dependencies: Vec<String>,
    transitive_module_dependencies: Vec<String>,
    mutable_references: bool,
    higher_order: bool,
    global_state: bool,
    /// Whether the executable body can directly mutate a global resource.
    /// This deliberately excludes read-only global operations such as
    /// `exists` and immutable `borrow_global`.
    global_resource_mutation: bool,
    /// Concrete global resource types directly mutated by this function's
    /// executable AST. The transitive executable closure is retained
    /// separately in the inventory for callers to combine these effects.
    global_resource_types: Vec<String>,
    arithmetic_or_abort: bool,
    reference_condition_count: usize,
    reference_paths: Vec<String>,
    feature_strata: Vec<String>,
    eligibility: String,
    decision_reason: String,
}

#[derive(Serialize)]
struct ImplementationComparison {
    schema_version: u32,
    equal: bool,
    baseline_modules: BTreeMap<String, String>,
    candidate_modules: BTreeMap<String, String>,
    added_modules: Vec<String>,
    removed_modules: Vec<String>,
    changed_modules: Vec<String>,
}

#[derive(Serialize)]
struct CompatibilityReport {
    schema_version: u32,
    stage: String,
    target: Option<String>,
    passed: bool,
    diagnostics: Vec<String>,
    /// The same diagnostics as data.
    ///
    /// `diagnostics` holds the human rendering, which exists to be read. A
    /// consumer that needs the file, line, column or the message a caret
    /// points at reads these instead of parsing the rendering back.
    ///
    /// Always written, empty included: its absence has to mean "produced by a
    /// flow that predates this field", not "nothing was reported".
    records: Vec<ReportedDiagnostic>,
}

/// One diagnostic, as position and text rather than as a rendered frame.
#[derive(Serialize)]
struct ReportedDiagnostic {
    headline: String,
    label: Option<String>,
    file: Option<String>,
    line: Option<usize>,
    column: Option<usize>,
    is_error: bool,
}

impl From<&DiagnosticRecord> for ReportedDiagnostic {
    fn from(record: &DiagnosticRecord) -> Self {
        Self {
            headline: record.headline.clone(),
            label: record.label.clone(),
            file: record.file.clone(),
            line: record.line,
            column: record.column,
            is_error: record.is_error,
        }
    }
}

#[derive(Serialize)]
struct ContractReport {
    schema_version: u32,
    package: String,
    function_count: usize,
    functions: Vec<FunctionContract>,
    spec_function_count: usize,
    spec_functions: Vec<SpecFunctionContract>,
}

#[derive(Serialize)]
struct FunctionContract {
    function: String,
    module: String,
    condition_count: usize,
    informative_condition_count: usize,
    untrusted_inferred_condition_count: usize,
    untrusted_inferred_condition_kinds: Vec<String>,
    condition_kinds: Vec<String>,
    specification_paths: Vec<String>,
    has_explicit_spec: bool,
    has_partial_aborts_if: bool,
    verification_disabled: bool,
    has_modifies_clause: bool,
    modifies_target_count: usize,
    modifies_all: bool,
    modifies_resource_types: Vec<String>,
    opaque: bool,
    native: bool,
    intrinsic: bool,
    inline: bool,
}

#[derive(Serialize)]
struct SpecFunctionContract {
    function: String,
    module: String,
    has_body: bool,
    uninterpreted: bool,
    native: bool,
    move_function_companion: bool,
}

pub fn run(args: &ExperimentArgs) -> Result<()> {
    register_move_flow_package_hooks();
    move_compiler_v2::logging::setup_logging(None);
    match &args.command {
        ExperimentCommand::Inventory(args) => inventory(args),
        ExperimentCommand::InventoryPackage(args) => inventory_package(args),
        ExperimentCommand::CompareImplementation(args) => compare_implementation(args),
        ExperimentCommand::CheckPackage(args) => check_package(args),
        ExperimentCommand::ContractReport(args) => contract_report(args),
        ExperimentCommand::Infer(args) => infer_package(args),
        ExperimentCommand::Prove(args) => prove_package(args),
        ExperimentCommand::CheckCandidate(args) => check_candidate(args),
    }
}

fn inventory_package(args: &PackageInventoryArgs) -> Result<()> {
    let package = args
        .package
        .canonicalize()
        .with_context(|| format!("cannot resolve `{}`", args.package.display()))?;
    let env = build_model(&package)
        .with_context(|| format!("failed to build package `{}`", package.display()))?;
    anyhow::ensure!(
        !env.has_errors(),
        "package `{}` has compiler errors",
        package.display()
    );
    let policy = SelectionPolicy::load(&args.selection_policy)?;
    let mut candidates = enumerate_package(&env, &package, "corpus-package", &policy);
    candidates.sort_by(|a, b| {
        (&a.granularity, &a.package_module_target).cmp(&(&b.granularity, &b.package_module_target))
    });
    write_json(&args.output, &PackageInventory {
        schema_version: 4,
        package: package.to_string_lossy().into_owned(),
        candidates,
    })
}

fn inventory(args: &InventoryArgs) -> Result<()> {
    anyhow::ensure!(
        args.source_commit.len() == 40 && args.source_commit.chars().all(|c| c.is_ascii_hexdigit()),
        "--source-commit must be a full 40-hex commit"
    );
    let repo_root = args
        .repo_root
        .canonicalize()
        .with_context(|| format!("cannot resolve `{}`", args.repo_root.display()))?;
    let policy = SelectionPolicy::load(&args.selection_policy)?;
    let mut candidates = Vec::new();
    for frame in &policy.source_frames {
        let source_root = frame.path.as_str();
        let package = repo_root.join(source_root);
        let env = build_model(&package)
            .with_context(|| format!("failed to build source frame `{source_root}`"))?;
        anyhow::ensure!(
            !env.has_errors(),
            "source frame `{source_root}` has compiler errors"
        );
        candidates.extend(enumerate_package(&env, &repo_root, source_root, &policy));
    }
    candidates.sort_by(|a, b| {
        (
            &a.source_root,
            &a.source_path,
            &a.granularity,
            &a.package_module_target,
        )
            .cmp(&(
                &b.source_root,
                &b.source_path,
                &b.granularity,
                &b.package_module_target,
            ))
    });
    write_json(&args.output, &Inventory {
        schema_version: 3,
        source_commit: args.source_commit.clone(),
        source_roots: policy
            .source_frames
            .iter()
            .map(|frame| frame.path.clone())
            .collect(),
        // An inventory states the recipe it was built with, so a corpus can be
        // read without its command line.
        selection_policy: policy.clone(),
        candidates,
    })
}

fn enumerate_package(
    env: &GlobalEnv,
    repo_root: &Path,
    source_root: &str,
    policy: &SelectionPolicy,
) -> Vec<Candidate> {
    let mut result = Vec::new();
    for module in env.get_primary_target_modules() {
        let transitive_module_dependencies = module_dependency_closure(env, &module);
        let source_path = relative_source_path(repo_root, module.get_source_path());
        if source_path.ends_with(".spec.move") {
            continue;
        }
        let module_source = fs::read(repo_root.join(&source_path)).unwrap_or_default();
        let mut functions = Vec::new();
        for function in module.get_functions() {
            if function.is_struct_api() || function.is_const_accessor() || function.is_lemma() {
                continue;
            }
            let candidate = function_candidate(
                env,
                policy,
                repo_root,
                &module,
                &function,
                source_root,
                &source_path,
                &transitive_module_dependencies,
            );
            functions.push(candidate.clone());
            result.push(candidate);
        }
        result.push(module_candidate(
            &module,
            policy,
            source_root,
            &source_path,
            &module_source,
            &functions,
            &transitive_module_dependencies,
        ));
    }
    result
}

fn function_candidate(
    env: &GlobalEnv,
    policy: &SelectionPolicy,
    repo_root: &Path,
    module: &ModuleEnv<'_>,
    function: &FunctionEnv<'_>,
    source_root: &str,
    source_path: &str,
    transitive_module_dependencies: &BTreeSet<String>,
) -> Candidate {
    // Shared across this candidate's three traversals, so a subgraph reachable
    // by several paths is walked once rather than once per path.
    let mut memo = TraversalMemo::default();
    let mut stats = AstStats::default();
    if let Some(def) = function.get_def() {
        def.visit_post_order(&mut |exp| {
            stats.nodes += 1;
            match exp {
                ExpData::Loop(..) => stats.loops += 1,
                ExpData::IfElse(..) => stats.branches += 1,
                ExpData::Match(_, _, arms) => stats.branches += arms.len(),
                ExpData::Mutate(..) => stats.mutates_references = true,
                ExpData::Invoke(..) | ExpData::Lambda(..) => stats.higher_order = true,
                ExpData::Call(node_id, op, arguments) => match op {
                    Operation::MoveFunction(..) => stats.calls += 1,
                    Operation::Closure(..) => stats.higher_order = true,
                    Operation::BorrowGlobal(kind) => {
                        stats.global_state = true;
                        if matches!(kind, ReferenceKind::Mutable) {
                            stats.global_resource_mutation = true;
                            stats.global_resource_types.insert(format!(
                                "{}",
                                env.get_node_type(*node_id)
                                    .skip_reference()
                                    .display(&env.get_type_display_ctx())
                            ));
                        }
                    },
                    Operation::MoveTo | Operation::MoveFrom => {
                        stats.global_state = true;
                        stats.global_resource_mutation = true;
                        let resource_type = if matches!(op, Operation::MoveTo) {
                            arguments.last().map(|argument| argument.node_id())
                        } else {
                            Some(*node_id)
                        };
                        if let Some(resource_type) = resource_type {
                            stats.global_resource_types.insert(format!(
                                "{}",
                                env.get_node_type(resource_type)
                                    .skip_reference()
                                    .display(&env.get_type_display_ctx())
                            ));
                        }
                    },
                    Operation::Exists(..) => stats.global_state = true,
                    Operation::Add
                    | Operation::Sub
                    | Operation::Mul
                    | Operation::Mod
                    | Operation::Div => stats.arithmetic = true,
                    Operation::Abort(..) => stats.aborts = true,
                    _ => {},
                },
                _ => {},
            }
            true
        });
    }
    stats.mutates_references |= function.is_mutating();
    stats.higher_order |= function.has_function_parameters();
    stats.global_state |= function
        .get_acquires_global_resources()
        .is_some_and(|resources| !resources.is_empty());

    let source = env.get_source(&function.get_loc()).unwrap_or_default();
    let spec = function.get_spec();
    let reference_paths: BTreeSet<String> = spec
        .conditions
        .iter()
        .map(|condition| relative_source_path(repo_root, env.get_file(condition.loc.file_id())))
        .collect();
    let reference_condition_count = spec.conditions.len();
    drop(spec);
    let has_explicit_reference = function.has_explicit_spec() && reference_condition_count > 0;

    let (eligibility, decision_reason) = eligibility(
        policy,
        source_root,
        source_path,
        module,
        function,
        has_explicit_reference,
        &stats,
    );
    let mut feature_strata = Vec::new();
    if stats.loops > 0 {
        feature_strata.push("loop".to_string());
    } else {
        feature_strata.push("straight-line".to_string());
    }
    if stats.higher_order {
        feature_strata.push("higher-order".to_string());
    }
    if stats.mutates_references {
        feature_strata.push("mutable-reference".to_string());
    }
    if stats.global_state {
        feature_strata.push("global-state".to_string());
    }
    if stats.calls > 1 {
        feature_strata.push("multiple-calls".to_string());
    }
    if stats.arithmetic || stats.aborts {
        feature_strata.push("arithmetic-abort".to_string());
    }
    let transitive_function_dependencies =
        function_dependency_closure(env, function, &mut BTreeSet::new(), &mut memo).0;
    let proof_dependencies = proof_dependencies(env, function);
    let transitive_called_function_dependencies =
        called_function_dependency_closure(env, function, &mut BTreeSet::new(), &mut memo).0;
    Candidate {
        source_root: source_root.to_string(),
        source_path: source_path.to_string(),
        package_module_target: function.get_full_name_with_address(),
        granularity: "function".to_string(),
        module: module.get_full_name_str(),
        function: Some(function.get_name_str()),
        target_functions: vec![function.get_name_str()],
        source_sha256: sha256_hex(source.as_bytes()),
        source_loc: source
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count(),
        function_count: 1,
        loops: stats.loops,
        branches: stats.branches,
        direct_callees: function.get_called_functions().map_or(0, BTreeSet::len),
        dependency_depth: call_depth(env, function, &mut BTreeSet::new(), &mut memo).0,
        transitive_function_dependencies: transitive_function_dependencies.into_iter().collect(),
        called_function_dependencies: proof_dependencies.contract_functions.into_iter().collect(),
        spec_function_dependencies: proof_dependencies.spec_functions.into_iter().collect(),
        transitive_called_function_dependencies: transitive_called_function_dependencies
            .into_iter()
            .collect(),
        transitive_module_dependencies: transitive_module_dependencies.iter().cloned().collect(),
        mutable_references: stats.mutates_references,
        higher_order: stats.higher_order,
        global_state: stats.global_state,
        global_resource_mutation: stats.global_resource_mutation,
        global_resource_types: stats.global_resource_types.into_iter().collect(),
        arithmetic_or_abort: stats.arithmetic || stats.aborts,
        reference_condition_count,
        reference_paths: reference_paths.into_iter().collect(),
        feature_strata,
        eligibility,
        decision_reason,
    }
}

fn module_candidate(
    module: &ModuleEnv<'_>,
    policy: &SelectionPolicy,
    source_root: &str,
    source_path: &str,
    source: &[u8],
    functions: &[Candidate],
    transitive_module_dependencies: &BTreeSet<String>,
) -> Candidate {
    let eligible: Vec<_> = functions
        .iter()
        .filter(|candidate| candidate.eligibility == "eligible")
        .collect();
    let function_count = eligible.len();
    let band = policy.module_function_count;
    let (eligibility, decision_reason) = if band.contains(function_count) {
        ("eligible".to_string(), "eligible_module".to_string())
    } else {
        (
            "excluded".to_string(),
            format!("module_function_count_outside_{}_{}", band.min, band.max),
        )
    };
    let feature_strata: BTreeSet<String> = eligible
        .iter()
        .flat_map(|candidate| candidate.feature_strata.iter().cloned())
        .collect();
    let reference_paths: BTreeSet<String> = eligible
        .iter()
        .flat_map(|candidate| candidate.reference_paths.iter().cloned())
        .collect();
    let target_functions = eligible
        .iter()
        .map(|candidate| {
            candidate
                .function
                .clone()
                .expect("function candidates have a function name")
        })
        .collect();
    let transitive_function_dependencies: BTreeSet<String> = eligible
        .iter()
        .flat_map(|candidate| candidate.transitive_function_dependencies.iter().cloned())
        .collect();
    let called_function_dependencies: BTreeSet<String> = eligible
        .iter()
        .flat_map(|candidate| candidate.called_function_dependencies.iter().cloned())
        .collect();
    let spec_function_dependencies: BTreeSet<String> = eligible
        .iter()
        .flat_map(|candidate| candidate.spec_function_dependencies.iter().cloned())
        .collect();
    let transitive_called_function_dependencies: BTreeSet<String> = eligible
        .iter()
        .flat_map(|candidate| {
            candidate
                .transitive_called_function_dependencies
                .iter()
                .cloned()
        })
        .collect();
    Candidate {
        source_root: source_root.to_string(),
        source_path: source_path.to_string(),
        package_module_target: module.get_full_name_str(),
        granularity: "module".to_string(),
        module: module.get_full_name_str(),
        function: None,
        target_functions,
        source_sha256: sha256_hex(source),
        source_loc: String::from_utf8_lossy(source)
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count(),
        function_count,
        loops: eligible.iter().map(|candidate| candidate.loops).sum(),
        branches: eligible.iter().map(|candidate| candidate.branches).sum(),
        direct_callees: eligible
            .iter()
            .map(|candidate| candidate.direct_callees)
            .sum(),
        dependency_depth: eligible
            .iter()
            .map(|candidate| candidate.dependency_depth)
            .max()
            .unwrap_or(0),
        transitive_function_dependencies: transitive_function_dependencies.into_iter().collect(),
        called_function_dependencies: called_function_dependencies.into_iter().collect(),
        spec_function_dependencies: spec_function_dependencies.into_iter().collect(),
        transitive_called_function_dependencies: transitive_called_function_dependencies
            .into_iter()
            .collect(),
        transitive_module_dependencies: transitive_module_dependencies.iter().cloned().collect(),
        mutable_references: eligible
            .iter()
            .any(|candidate| candidate.mutable_references),
        higher_order: eligible.iter().any(|candidate| candidate.higher_order),
        global_state: eligible.iter().any(|candidate| candidate.global_state),
        global_resource_mutation: eligible
            .iter()
            .any(|candidate| candidate.global_resource_mutation),
        global_resource_types: eligible
            .iter()
            .flat_map(|candidate| candidate.global_resource_types.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        arithmetic_or_abort: eligible
            .iter()
            .any(|candidate| candidate.arithmetic_or_abort),
        reference_condition_count: eligible
            .iter()
            .map(|candidate| candidate.reference_condition_count)
            .sum(),
        reference_paths: reference_paths.into_iter().collect(),
        feature_strata: feature_strata.into_iter().collect(),
        eligibility,
        decision_reason,
    }
}

fn eligibility(
    policy: &SelectionPolicy,
    source_root: &str,
    source_path: &str,
    module: &ModuleEnv<'_>,
    function: &FunctionEnv<'_>,
    has_explicit_reference: bool,
    stats: &AstStats,
) -> (String, String) {
    let frame = policy.frame(source_root);
    let lower = format!(
        "{}::{}::{}",
        source_path,
        module.get_full_name_str(),
        function.get_name_str()
    )
    .to_ascii_lowercase();
    let excluded = if source_path.contains("/tests/")
        || source_path
            .rsplit('/')
            .next()
            .is_some_and(|name| name.starts_with("test_"))
        || module.is_test_or_verify_only()
        || function.is_test_or_verify_only()
    {
        Some("test_or_verify_only")
    } else if !frame.include_paths.is_empty()
        && !frame
            .include_paths
            .iter()
            .any(|prefix| source_path.contains(prefix.as_str()))
    {
        Some("outside_candidate_frame")
    } else if policy
        .safety_exclusion_terms
        .iter()
        .any(|term| lower.contains(term.to_ascii_lowercase().as_str()))
    {
        Some("safety_exclusion")
    } else if function.is_native() || function.get_def().is_none() {
        Some("native_or_missing_body")
    } else if is_trivial_accessor(function.get_def(), stats) {
        Some("trivial_accessor")
    } else if frame.require_upstream_reference && !has_explicit_reference {
        Some("missing_upstream_reference")
    } else {
        None
    };
    match excluded {
        Some(reason) => ("excluded".to_string(), reason.to_string()),
        None => ("eligible".to_string(), frame.eligible_reason.clone()),
    }
}

fn is_trivial_accessor(def: Option<&Exp>, stats: &AstStats) -> bool {
    if stats.nodes > 8 || stats.loops > 0 || stats.branches > 0 || stats.calls > 0 {
        return false;
    }
    fn contains_select(exp: &Exp) -> bool {
        let mut found = false;
        exp.visit_post_order(&mut |item| {
            found |= matches!(item, ExpData::Call(_, Operation::Select(..), _));
            !found
        });
        found
    }
    def.is_some_and(contains_select)
}

/// Memoized results of a traversal, keyed by qualified function name.
///
/// Only an acyclic result is cached: one truncated by the cycle guard depends
/// on which ancestors were active, so it is not a property of the node.
#[derive(Default)]
struct TraversalMemo {
    depth: BTreeMap<String, usize>,
    used: BTreeMap<String, BTreeSet<String>>,
    called: BTreeMap<String, BTreeSet<String>>,
}

/// Longest call chain below `function`, with whether a cycle truncated it.
///
/// Without memoization a shared subgraph is recomputed once per path into it,
/// which is exponential on a Fibonacci-shaped call graph.
fn call_depth(
    env: &GlobalEnv,
    function: &FunctionEnv<'_>,
    active: &mut BTreeSet<String>,
    memo: &mut TraversalMemo,
) -> (usize, bool) {
    let name = function.get_full_name_with_address();
    if let Some(depth) = memo.depth.get(&name) {
        return (*depth, false);
    }
    if !active.insert(name.clone()) {
        return (0, true);
    }
    let mut depth = 0;
    let mut truncated = false;
    for callee in function.get_called_functions().into_iter().flatten() {
        let (below, cycle) = call_depth(env, &env.get_function(*callee), active, memo);
        truncated |= cycle;
        depth = depth.max(1 + below);
    }
    active.remove(&name);
    if !truncated {
        memo.depth.insert(name, depth);
    }
    (depth, truncated)
}

fn function_dependency_closure(
    env: &GlobalEnv,
    function: &FunctionEnv<'_>,
    active: &mut BTreeSet<String>,
    memo: &mut TraversalMemo,
) -> (BTreeSet<String>, bool) {
    let name = function.get_full_name_with_address();
    if let Some(cached) = memo.used.get(&name) {
        return (cached.clone(), false);
    }
    if !active.insert(name.clone()) {
        return (BTreeSet::new(), true);
    }
    let mut truncated = false;
    let mut result = BTreeSet::new();
    // `get_called_functions` describes calls which remain after expansion.  It
    // deliberately omits inline functions, but their source modules are still
    // required when the corpus is compiled from source.  Inventory source
    // dependencies from the pre-expansion usage graph instead.
    for callee in function.get_used_functions().into_iter().flatten() {
        let callee = env.get_function(*callee);
        result.insert(callee.get_full_name_with_address());
        let (below, cycle) = function_dependency_closure(env, &callee, active, memo);
        truncated |= cycle;
        result.extend(below);
    }
    active.remove(&name);
    if !truncated {
        memo.used.insert(name, result.clone());
    }
    (result, truncated)
}

/// Return the exact transitive closure of executable calls which remain after
/// compiler expansion. Unlike `function_dependency_closure`, this excludes
/// inline-only source dependencies and closure captures: those sources must be
/// present to compile the corpus, but they do not require callable contracts.
fn called_function_dependency_closure(
    env: &GlobalEnv,
    function: &FunctionEnv<'_>,
    active: &mut BTreeSet<String>,
    memo: &mut TraversalMemo,
) -> (BTreeSet<String>, bool) {
    let name = function.get_full_name_with_address();
    if let Some(cached) = memo.called.get(&name) {
        return (cached.clone(), false);
    }
    if !active.insert(name.clone()) {
        return (BTreeSet::new(), true);
    }
    let mut truncated = false;
    let mut result = BTreeSet::new();
    for callee in function.get_called_functions().into_iter().flatten() {
        let callee = env.get_function(*callee);
        let callee_name = callee.get_full_name_with_address();
        if callee_name != name {
            result.insert(callee_name);
        }
        let (below, cycle) = called_function_dependency_closure(env, &callee, active, memo);
        truncated |= cycle;
        result.extend(below);
    }
    active.remove(&name);
    result.remove(&name);
    if !truncated {
        memo.called.insert(name, result.clone());
    }
    (result, truncated)
}

#[derive(Default)]
struct ProofDependencies {
    contract_functions: BTreeSet<String>,
    spec_functions: BTreeSet<String>,
    visited_move_functions: BTreeSet<QualifiedId<FunId>>,
    visited_spec_functions: BTreeSet<QualifiedId<SpecFunId>>,
}

/// Compute everything whose specification is visible while proving `target`.
/// Transparent Move bodies are traversed. Opaque/native/intrinsic functions
/// become contract boundaries, but expressions in those contracts are then
/// traversed through both ordinary spec-function calls and Move behavioral
/// predicates (`result_of`, `ensures_of`, and friends).
fn proof_dependencies(env: &GlobalEnv, target: &FunctionEnv<'_>) -> ProofDependencies {
    let mut result = ProofDependencies::default();
    result
        .visited_move_functions
        .insert(target.get_qualified_id());
    for callee in target.get_called_functions().into_iter().flatten() {
        collect_move_dependency(env, *callee, &mut result);
    }
    result
}

fn collect_move_dependency(
    env: &GlobalEnv,
    dependency: QualifiedId<FunId>,
    result: &mut ProofDependencies,
) {
    if !result.visited_move_functions.insert(dependency) {
        return;
    }
    let function = env.get_function(dependency);
    if function.is_opaque() || function.is_native_or_intrinsic() {
        result
            .contract_functions
            .insert(function.get_full_name_with_address());
        collect_dependencies_from_contract(env, &function, result);
    } else {
        for callee in function.get_called_functions().into_iter().flatten() {
            collect_move_dependency(env, *callee, result);
        }
    }
}

fn collect_dependencies_from_contract(
    env: &GlobalEnv,
    function: &FunctionEnv<'_>,
    result: &mut ProofDependencies,
) {
    let spec = function.get_spec();
    for condition in &spec.conditions {
        collect_dependencies_from_spec_exp(env, &condition.exp, result);
        for exp in &condition.additional_exps {
            collect_dependencies_from_spec_exp(env, exp, result);
        }
    }
    if let Some(frame) = &spec.frame_spec {
        for exp in &frame.modifies_targets {
            collect_dependencies_from_spec_exp(env, exp, result);
        }
    }
}

fn collect_dependencies_from_spec_exp(env: &GlobalEnv, exp: &Exp, result: &mut ProofDependencies) {
    let mut move_dependencies = BTreeSet::new();
    exp.as_ref().visit_post_order(&mut |node| {
        if let ExpData::Call(_, operation, arguments) = node {
            match operation {
                Operation::MoveFunction(module, function) => {
                    move_dependencies.insert(module.qualified(*function));
                },
                Operation::Behavior(_, _) => {
                    if let Some(first) = arguments.first() {
                        if let ExpData::Call(_, Operation::Closure(module, function, _), _) =
                            first.as_ref()
                        {
                            move_dependencies.insert(module.qualified(*function));
                        }
                    }
                },
                _ => {},
            }
        }
        true
    });
    for dependency in move_dependencies {
        collect_move_dependency(env, dependency, result);
    }

    for dependency in exp.called_spec_funs(env) {
        let dependency = dependency.to_qualified_id();
        if !result.visited_spec_functions.insert(dependency) {
            continue;
        }
        let module = env.get_module(dependency.module_id);
        let declaration = env.get_spec_fun(dependency);
        result.spec_functions.insert(format!(
            "{}::{}",
            module.get_full_name_str(),
            declaration.name.display(env.symbol_pool())
        ));
        if let Some(body) = &declaration.body {
            collect_dependencies_from_spec_exp(env, body, result);
        }
        let declaration_spec = declaration.spec.borrow();
        for condition in &declaration_spec.conditions {
            collect_dependencies_from_spec_exp(env, &condition.exp, result);
            for exp in &condition.additional_exps {
                collect_dependencies_from_spec_exp(env, exp, result);
            }
        }
        if let Some(frame) = &declaration_spec.frame_spec {
            for exp in &frame.modifies_targets {
                collect_dependencies_from_spec_exp(env, exp, result);
            }
        }
    }
}

fn module_dependency_closure(env: &GlobalEnv, module: &ModuleEnv<'_>) -> BTreeSet<String> {
    fn visit(env: &GlobalEnv, module: &ModuleEnv<'_>, visited: &mut BTreeSet<String>) {
        // Include specification-only edges as well as executable imports. An
        // opaque Move function stops implementation traversal, but its contract
        // can call spec functions in another module, whose bodies can in turn
        // reference further modules.
        let mut dependencies = module.get_used_modules(true);
        // The model's module-use set is based on the expanded program.  Add
        // modules containing inline functions explicitly: those modules can
        // disappear from bytecode dependencies while remaining mandatory for
        // recompiling the Move sources (for example `sigma_protocol::verify`).
        for function in module.get_functions() {
            dependencies.extend(
                function
                    .get_used_functions_with_transitive_inline()
                    .into_iter()
                    .map(|function| function.module_id),
            );
        }
        // Inline expansion can also erase the only model-level edge to an
        // imported source module.  Source packages still need that module to
        // resolve the import before expansion, so retain declared `use` and
        // `friend` dependencies as well.
        let source = env.get_file_source(module.get_loc().file_id());
        dependencies.extend(source.lines().filter_map(|line| {
            let declaration = line
                .trim()
                .strip_prefix("use ")
                .or_else(|| line.trim().strip_prefix("friend "))?;
            let name = declaration.split("::").nth(1)?;
            let name = name
                .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
                .next()?;
            if name.is_empty() {
                return None;
            }
            env.find_module_by_name(env.symbol_pool().make(name))
                .map(|module| module.get_id())
        }));
        for dependency in dependencies {
            let dependency = env.get_module(dependency);
            let name = dependency.get_full_name_str();
            if visited.insert(name) {
                visit(env, &dependency, visited);
            }
        }
    }
    let mut result = BTreeSet::new();
    visit(env, module, &mut result);
    result.remove(&module.get_full_name_str());
    result
}

fn compare_implementation(args: &CompareImplementationArgs) -> Result<()> {
    let comparison = implementation_comparison(&args.baseline, &args.candidate)?;
    write_json(&args.output, &comparison)
}

/// Compare the executable bytecode of a pristine and an edited package.
/// What the baseline package assumes rather than verifies.
///
/// Built once more rather than threaded out of the implementation comparison:
/// the compile is seconds, and the check is dominated by the prover.
fn baseline_contracts(package: &Path) -> Result<crate::candidate::BaselineContracts> {
    let env = build_model_for_implementation_comparison(package)
        .with_context(|| format!("failed to build `{}`", package.display()))?;
    let mut contracts = crate::candidate::BaselineContracts::default();
    for module in env.get_modules() {
        for function in module.get_functions() {
            let qualified = format!(
                "{}::{}",
                module.get_name().display_full(&env),
                function.get_name().display(env.symbol_pool())
            );
            if function.is_opaque() || function.is_native() || function.is_intrinsic() {
                contracts.opaque.insert(
                    qualified.clone(),
                    crate::candidate::contract_fingerprint(&env, &function),
                );
            }
            if function.is_intrinsic() {
                contracts.intrinsic.insert(qualified);
            }
        }
    }
    contracts.spec_functions = crate::candidate::spec_function_definitions(&env);
    Ok(contracts)
}

fn implementation_comparison(
    baseline_package: &Path,
    candidate_package: &Path,
) -> Result<ImplementationComparison> {
    let baseline = build_model_for_implementation_comparison(baseline_package)
        .with_context(|| format!("failed to build baseline `{}`", baseline_package.display()))?;
    let candidate =
        build_model_for_implementation_comparison(candidate_package).with_context(|| {
            format!(
                "failed to build candidate `{}`",
                candidate_package.display()
            )
        })?;
    anyhow::ensure!(
        !baseline.has_errors(),
        "baseline package has compiler errors"
    );
    anyhow::ensure!(
        !candidate.has_errors(),
        "candidate package has compiler errors"
    );
    let (baseline_modules, candidate_modules) =
        implementation_module_hashes(&baseline, &candidate)?;
    let baseline_names: BTreeSet<_> = baseline_modules.keys().cloned().collect();
    let candidate_names: BTreeSet<_> = candidate_modules.keys().cloned().collect();
    let added_modules: Vec<_> = candidate_names
        .difference(&baseline_names)
        .cloned()
        .collect();
    let removed_modules: Vec<_> = baseline_names
        .difference(&candidate_names)
        .cloned()
        .collect();
    let changed_modules = baseline_names
        .intersection(&candidate_names)
        .filter(|name| baseline_modules.get(*name) != candidate_modules.get(*name))
        .cloned()
        .collect::<Vec<_>>();
    let equal =
        added_modules.is_empty() && removed_modules.is_empty() && changed_modules.is_empty();
    Ok(ImplementationComparison {
        schema_version: 1,
        equal,
        baseline_modules,
        candidate_modules,
        added_modules,
        removed_modules,
        changed_modules,
    })
}

fn check_package(args: &PackageCheckArgs) -> Result<()> {
    let env = build_model(&args.package)
        .with_context(|| format!("failed to build `{}`", args.package.display()))?;
    let passed = !env.has_errors();
    // One pass: rendering drains, so the structured form is taken from the
    // same records the rendering is made of.
    let collected = collect_diagnostics(&env);
    let records = collected.iter().map(ReportedDiagnostic::from).collect();
    let diagnostics = collected.into_iter().map(|record| record.text).collect();
    write_json(&args.output, &CompatibilityReport {
        schema_version: 2,
        stage: "compile".to_string(),
        target: None,
        passed,
        diagnostics,
        records,
    })?;
    anyhow::ensure!(passed, "package has compiler-model errors");
    Ok(())
}

fn contract_report(args: &PackageCheckArgs) -> Result<()> {
    let package = args
        .package
        .canonicalize()
        .with_context(|| format!("cannot resolve `{}`", args.package.display()))?;
    let env = build_model(&package)
        .with_context(|| format!("failed to build `{}`", package.display()))?;
    anyhow::ensure!(!env.has_errors(), "package has compiler-model errors");
    let mut functions = Vec::new();
    let mut spec_functions = Vec::new();
    for module in env.get_primary_target_modules() {
        for function in module.get_functions() {
            let spec = function.get_spec();
            let condition_kinds = spec
                .conditions
                .iter()
                .map(|condition| format!("{:?}", condition.kind))
                .collect();
            let informative_condition_count = spec
                .conditions
                .iter()
                .filter(|condition| is_informative_contract_condition(condition))
                .count();
            let untrusted_inferred_conditions = spec
                .conditions
                .iter()
                .filter(|condition| is_untrusted_inferred_contract_condition(&env, condition))
                .collect::<Vec<_>>();
            let specification_paths = spec
                .conditions
                .iter()
                .map(|condition| {
                    relative_source_path(&package, env.get_file(condition.loc.file_id()))
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let (has_modifies_clause, modifies_target_count, modifies_all, modifies_resource_types) =
                spec.frame_spec
                    .as_ref()
                    .map(|frame| {
                        (
                            !frame.modifies_targets.is_empty() || frame.modifies_all,
                            frame.modifies_targets.len(),
                            frame.modifies_all,
                            frame
                                .modifies_targets
                                .iter()
                                .map(|target| {
                                    format!(
                                        "{}",
                                        env.get_node_type(target.node_id())
                                            .skip_reference()
                                            .display(&env.get_type_display_ctx())
                                    )
                                })
                                .collect::<BTreeSet<_>>()
                                .into_iter()
                                .collect(),
                        )
                    })
                    .unwrap_or((false, 0, false, Vec::new()));
            functions.push(FunctionContract {
                function: function.get_full_name_with_address(),
                module: module.get_full_name_str(),
                condition_count: spec.conditions.len(),
                informative_condition_count,
                untrusted_inferred_condition_count: untrusted_inferred_conditions.len(),
                untrusted_inferred_condition_kinds: untrusted_inferred_conditions
                    .iter()
                    .map(|condition| format!("{:?}", condition.kind))
                    .collect(),
                condition_kinds,
                specification_paths,
                has_explicit_spec: function.has_explicit_spec(),
                has_partial_aborts_if: function
                    .is_pragma_true(ABORTS_IF_IS_PARTIAL_PRAGMA, || false),
                verification_disabled: function.is_pragma_false(VERIFY_PRAGMA),
                has_modifies_clause,
                modifies_target_count,
                modifies_all,
                modifies_resource_types,
                opaque: function.is_opaque(),
                native: function.is_native(),
                intrinsic: function.is_intrinsic(),
                inline: function.is_inline(),
            });
        }
        for (_, declaration) in module.get_spec_funs() {
            spec_functions.push(SpecFunctionContract {
                function: format!(
                    "{}::{}",
                    module.get_full_name_str(),
                    declaration.name.display(env.symbol_pool())
                ),
                module: module.get_full_name_str(),
                has_body: declaration.body.is_some(),
                uninterpreted: declaration.uninterpreted,
                native: declaration.is_native,
                move_function_companion: declaration.is_move_fun,
            });
        }
    }
    functions.sort_by(|left, right| left.function.cmp(&right.function));
    spec_functions.sort_by(|left, right| left.function.cmp(&right.function));
    write_json(&args.output, &ContractReport {
        schema_version: 7,
        package: package.to_string_lossy().into_owned(),
        function_count: functions.len(),
        functions,
        spec_function_count: spec_functions.len(),
        spec_functions,
    })
}

fn is_informative_contract_condition(condition: &move_model::ast::Condition) -> bool {
    match condition.kind {
        ConditionKind::LetPost(..) | ConditionKind::LetPre(..) => false,
        ConditionKind::Ensures | ConditionKind::Requires => {
            !matches!(condition.exp.as_ref(), ExpData::Value(_, Value::Bool(true)))
        },
        _ => true,
    }
}

/// WP labels clauses with `vacuous` and `sathard` when it has explicitly
/// identified them as unsuitable contract candidates.  They must not be used
/// to make an opaque dependency boundary appear complete: they need repair by
/// an agent (for example, a loop invariant) and a transparent proof first.
fn is_untrusted_inferred_contract_condition(
    env: &GlobalEnv,
    condition: &move_model::ast::Condition,
) -> bool {
    env.get_symbol_property(&condition.properties, CONDITION_INFERRED_PROP)
        .is_some_and(|value| {
            let value = env.symbol_pool().string(value);
            value.as_ref() == CONDITION_INFERRED_VACUOUS
                || value.as_ref() == CONDITION_INFERRED_SATHARD
        })
}

fn infer_package(args: &PackageTargetArgs) -> Result<()> {
    let filter = prover_filter(&args.target)?;
    let mut env = build_model(&args.package)
        .with_context(|| format!("failed to build `{}`", args.package.display()))?;
    let mut diagnostics = render_diagnostics(&env);
    if env.has_errors() {
        write_compatibility_report(
            &args.output,
            "wp_inference",
            &args.target,
            false,
            diagnostics,
        )?;
        anyhow::bail!("package has compiler-model errors");
    }
    let temporary = tempfile::tempdir()?;
    let dump_dir = args.output.with_extension("bytecode");
    if args.dump_bytecode {
        fs::create_dir_all(&dump_dir)?;
    }
    let mut options = load_sanitized_prover_options(&args.package).map_err(anyhow::Error::msg)?;
    options.prover.verify_scope = verification_scope(Some(&filter));
    aptos_framework::prover::configure_aptos_custom_natives(&mut options);
    options.inference.inference = true;
    options.inference.inference_output = InferenceOutput::File;
    options.inference.inference_output_dir = if args.write_inferred_specs {
        None
    } else {
        Some(temporary.path().to_string_lossy().into_owned())
    };
    options.prover.dump_bytecode = args.dump_bytecode;
    options.prover.dump_cfg = args.dump_bytecode;
    options.output_path = if args.dump_bytecode {
        dump_dir.join("output.bpl")
    } else {
        temporary.path().join("output.bpl")
    }
    .to_string_lossy()
    .into_owned();
    let mut error_writer = NoColor::new(Vec::new());
    let result = move_prover::inference::run_spec_inference_with_model(
        &mut env,
        &mut error_writer,
        options,
        Instant::now(),
    );
    diagnostics.extend(render_diagnostics(&env));
    let writer = String::from_utf8(error_writer.into_inner()).unwrap_or_default();
    if !writer.trim().is_empty() {
        diagnostics.push(writer);
    }
    if let Err(error) = &result {
        diagnostics.push(format!("{error:#}"));
    }
    write_compatibility_report(
        &args.output,
        "wp_inference",
        &args.target,
        result.is_ok(),
        diagnostics,
    )?;
    result
}

fn prove_package(args: &PackageProveArgs) -> Result<()> {
    anyhow::ensure!(
        (1..=MAX_EXPERIMENT_VC_TIMEOUT_SECS).contains(&args.timeout),
        "timeout must be between 1 and {} seconds",
        MAX_EXPERIMENT_VC_TIMEOUT_SECS
    );
    let filters = args
        .target
        .iter()
        .map(|target| prover_filter(target))
        .collect::<Result<Vec<_>>>()?;
    let joined = args.target.join(", ");
    let mut env = build_model(&args.package)
        .with_context(|| format!("failed to build `{}`", args.package.display()))?;
    let compile_diagnostics = render_diagnostics(&env);
    if env.has_errors() {
        write_compatibility_report(&args.output, "prover", &joined, false, compile_diagnostics)?;
        anyhow::bail!("package has compiler-model errors");
    }

    // One report per target, so a caller still learns which of them failed.
    let mut outcomes = Vec::new();
    for (target, filter) in args.target.iter().zip(&filters) {
        // A failed target leaves its errors on the model, and the prover
        // refuses to run against a model that carries errors. Each target is
        // proved against the state the package compiled to, not against what
        // the target before it left behind.
        env.clear_diag();
        let mut diagnostics = compile_diagnostics.clone();
        let result = run_prover_on_model(
            &mut env,
            &args.package,
            Some(filter),
            args.timeout,
            args.split_vcs_by_assert,
            args.check_inconsistency,
            args.boogie_output.as_deref(),
            &mut diagnostics,
            None,
        );
        outcomes.push((target.clone(), result.is_ok(), diagnostics));
    }
    let passed = outcomes.iter().all(|(_, ok, _)| *ok);
    match outcomes.as_slice() {
        [(target, ok, diagnostics)] => {
            write_compatibility_report(&args.output, "prover", target, *ok, diagnostics.clone())?;
        },
        many => {
            for (target, ok, diagnostics) in many {
                write_compatibility_report(
                    &target_report_path(&args.output, target),
                    "prover",
                    target,
                    *ok,
                    diagnostics.clone(),
                )?;
            }
            write_compatibility_report(&args.output, "prover", &joined, passed, Vec::new())?;
        },
    }
    anyhow::ensure!(passed, "one or more targets did not verify");
    Ok(())
}

/// Per-target report path beside the summary the caller asked for.
fn target_report_path(output: &Path, target: &str) -> PathBuf {
    let sanitized: String = target
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect();
    let stem = output
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    output.with_file_name(format!("{stem}.{sanitized}.json"))
}

/// Run the prover against an already built model, collecting its diagnostics.
///
/// The candidate check and the standalone prove command must agree on solver
/// options, so both drive verification through this one configuration.
fn run_prover_on_model(
    env: &mut GlobalEnv,
    package: &Path,
    filter: Option<&str>,
    timeout: usize,
    split_vcs_by_assert: bool,
    check_inconsistency: bool,
    boogie_output: Option<&Path>,
    diagnostics: &mut Vec<String>,
    records: Option<&mut Vec<DiagnosticRecord>>,
) -> Result<()> {
    run_prover_on_model_with_deadline(
        None,
        env,
        package,
        filter,
        timeout,
        split_vcs_by_assert,
        check_inconsistency,
        boogie_output,
        diagnostics,
        records,
    )
}

/// What is left of a deadline, in whole seconds, never less than one.
///
/// A caller's deadline covers the whole check, and the model build, baseline
/// comparison and policy checks run before the prover does; the prover gets
/// the remainder, not the whole.
fn remaining_deadline(deadline_seconds: Option<u64>, started: Instant) -> Option<u64> {
    deadline_seconds.map(|deadline| {
        Duration::from_secs(deadline)
            .saturating_sub(started.elapsed())
            .as_secs()
            .max(1)
    })
}

/// `run_prover_on_model` with a hard deadline for the prover process.
///
/// The prover's own watchdog is proportional to the solver budget with a
/// floor of minutes, so a caller that answers within a shorter deadline -- an
/// MCP tool -- has to bound the process itself, or the work outlives the
/// answer.
#[allow(clippy::too_many_arguments)]
fn run_prover_on_model_with_deadline(
    hard_timeout_secs: Option<u64>,
    env: &mut GlobalEnv,
    package: &Path,
    filter: Option<&str>,
    timeout: usize,
    split_vcs_by_assert: bool,
    check_inconsistency: bool,
    boogie_output: Option<&Path>,
    diagnostics: &mut Vec<String>,
    records: Option<&mut Vec<DiagnosticRecord>>,
) -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let mut options = load_sanitized_prover_options(package).map_err(anyhow::Error::msg)?;
    options.prover.verify_scope = verification_scope(filter);
    options.backend.vc_timeout = timeout;
    // `vc_timeout` covers solver time only. The watchdog has to cover Boogie
    // parsing, inlining, and VC construction *and* leave Z3 room to overshoot
    // its own soft timeout, which it does on nonlinear or heavily quantified
    // queries. A fixed margin kills the process before Z3 reports the VC
    // timeout, and a killed process carries no solver output -- which loses the
    // quantifier-instantiation analysis exactly where it is most wanted. Defer
    // to the backend's proportional watchdog instead of a constant grace.
    let process_timeout = options.backend.process_timeout_secs(timeout);
    let process_timeout = hard_timeout_secs.map_or(process_timeout, |deadline| deadline.max(1));
    options.backend.hard_timeout_secs = process_timeout;
    options.backend.split_vcs_by_assert = split_vcs_by_assert;
    options.prover.check_inconsistency = check_inconsistency;
    if split_vcs_by_assert {
        // A split root is verified as one sequential solver job per assertion.
        // The timeout is a per-VC solver budget, so an aggregate package
        // deadline of the same size can kill later shards before they run.
        // Each job still has `hard_timeout_secs`; leave the aggregate deadline
        // disabled for this diagnostic mode.
        options.backend.package_timeout_secs = 0;
        // Assertion splitting can create many solver instances. Running them
        // concurrently makes hard quantified VCs contend for memory and CPU,
        // defeating the diagnostic split (and can be slower than one core).
        options.backend.proc_cores = 1;
        // Explicit QI thresholds make some quantified assertion shards much
        // slower than Z3's own defaults. Keep the legacy thresholds for normal
        // verification, but use the solver defaults for diagnostic splitting.
        options.backend.use_solver_default_qi_thresholds = true;
    } else {
        options.backend.package_timeout_secs = process_timeout;
    }
    options.backend.package_error_limit = 20;
    aptos_framework::prover::configure_aptos_custom_natives(&mut options);
    options.output_path = boogie_output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| temporary.path().join("output.bpl"))
        .to_string_lossy()
        .into_owned();
    options.backend.keep_artifacts = boogie_output.is_some();
    let mut error_writer = NoColor::new(Vec::new());
    let result =
        move_prover::run_move_prover_with_model_v2(env, &mut error_writer, options, Instant::now());
    // The prover reports its verification failures to the error writer before
    // returning, so structured inspection must run before the rendering pass
    // drains what remains.
    if let Some(sink) = records {
        sink.extend(inspect_diagnostics(env));
    }
    diagnostics.extend(render_diagnostics(env));
    let writer = String::from_utf8(error_writer.into_inner()).unwrap_or_default();
    if !writer.trim().is_empty() {
        diagnostics.push(writer);
    }
    if let Err(error) = &result {
        diagnostics.push(format!("{error:#}"));
    }
    result
}

/// The prover's own wording for a solver budget exhausted on a condition.
const PROVER_TIMEOUT_DIAGNOSTIC: &str = "out of resources/timeout";

/// Wording for a whole prover process killed at its wall-clock limit. This
/// timeout names no condition at all, so attribution matters more here than for
/// a per-condition budget, not less.
const PROVER_PROCESS_TIMEOUT_DIAGNOSTIC: &str = "exceeded hard timeout";

/// Whether a diagnostic reports the prover running out of time.
pub fn is_prover_timeout(text: &str) -> bool {
    text.contains(PROVER_TIMEOUT_DIAGNOSTIC) || text.contains(PROVER_PROCESS_TIMEOUT_DIAGNOSTIC)
}

/// Wall-clock ceiling for locating a prover timeout. Local computation is
/// unmetered, but the session still has an end-to-end budget.
const TIMEOUT_ATTRIBUTION_BUDGET_SECS: u64 = 180;

/// Per-function attribution of a prover timeout.
///
/// A timeout reports only that some obligation exceeded the solver budget. The
/// caller otherwise has to rediscover which one by repeatedly narrowing the
/// filter, which costs model turns for work the machine can do locally.
#[derive(Debug, Default, Serialize)]
pub struct TimeoutAttribution {
    pub proved: Vec<String>,
    pub timed_out: Vec<String>,
    pub failed: Vec<String>,
    pub slowest: Option<String>,
    pub slowest_millis: u128,
    pub hard_assertions: Vec<String>,
    pub split_note: Option<String>,
    pub incomplete: bool,
}

impl TimeoutAttribution {
    /// Compact report for a model. Only the facts a repair decision needs.
    pub fn render(&self) -> String {
        let mut lines = vec!["TIMEOUT ATTRIBUTION".to_string()];
        if !self.timed_out.is_empty() {
            lines.push(format!("Timed out: {}", self.timed_out.join(", ")));
        }
        if !self.failed.is_empty() {
            lines.push(format!("Failed logically: {}", self.failed.join(", ")));
        }
        lines.push(format!(
            "Proved in isolation: {}/{}",
            self.proved.len(),
            self.proved.len() + self.timed_out.len() + self.failed.len()
        ));
        if let Some(slowest) = &self.slowest {
            lines.push(format!("Slowest: {} ({} ms)", slowest, self.slowest_millis));
        }
        for assertion in &self.hard_assertions {
            lines.push(format!("Hard assertion: {assertion}"));
        }
        if let Some(note) = &self.split_note {
            lines.push(note.clone());
        }
        if self.incomplete {
            lines.push("Not every function in scope was probed within the budget.".to_string());
        }
        if !self.timed_out.is_empty() {
            lines.push(
                "A hard assertion usually needs restructuring, not a larger timeout; \
                 see \"Toolchain capabilities and limits\" on recursion-aligned helpers \
                 and single recursion."
                    .to_string(),
            );
        }
        lines.join("\n")
    }
}

/// Names of the target functions a filter selects, for timeout attribution.
/// `filter` of `None` names every function in the package's target modules.
pub fn scoped_function_names(env: &GlobalEnv, filter: Option<&str>) -> Vec<String> {
    let mut scoped = Vec::new();
    for module in env.get_primary_target_modules() {
        for function in module.get_functions() {
            let name = function.get_name().display(env.symbol_pool()).to_string();
            let full = format!("{}::{name}", module.get_name().display_full(env));
            // The prover's scope carries no address.
            if filter.is_none_or(|filter| function_in_scope(&full, filter)) {
                scoped.push(format!("{}::{name}", module.get_name().display(env)));
            }
        }
    }
    scoped.sort();
    scoped
}

/// Probe each function in scope separately to locate a prover timeout.
///
/// Each probe rebuilds the model so the prover never observes annotations from
/// an earlier run. Probing stops when `budget` is exhausted and says so, rather
/// than reporting a partial result as complete.
///
/// `scoped` is supplied by the caller: naming the functions needs a model, and
/// every caller already holds one.
pub fn attribute_prover_timeout(
    package: &Path,
    scoped: Vec<String>,
    timeout: usize,
    budget: Duration,
) -> Result<TimeoutAttribution> {
    let started = Instant::now();

    let mut attribution = TimeoutAttribution::default();
    for qualified in scoped {
        if started.elapsed() >= budget {
            attribution.incomplete = true;
            break;
        }
        let mut diagnostics = Vec::new();
        let probe = Instant::now();
        let mut probe_env = build_model(package)?;
        let result = run_prover_on_model_with_deadline(
            Some(budget.saturating_sub(started.elapsed()).as_secs().max(1)),
            &mut probe_env,
            package,
            Some(&qualified),
            timeout,
            false,
            false,
            None,
            &mut diagnostics,
            None,
        );
        let elapsed = probe.elapsed().as_millis();
        let timed_out = diagnostics.iter().any(|line| is_prover_timeout(line));
        if result.is_ok() {
            attribution.proved.push(qualified.clone());
        } else if timed_out {
            attribution.timed_out.push(qualified.clone());
        } else {
            attribution.failed.push(qualified.clone());
        }
        if elapsed > attribution.slowest_millis {
            attribution.slowest_millis = elapsed;
            attribution.slowest = Some(qualified);
        }
    }

    // One split pass over the first timed-out function names the individual
    // assertion that exceeds the budget.
    if let Some(function) = attribution.timed_out.first().cloned() {
        if started.elapsed() < budget {
            let mut diagnostics = Vec::new();
            let mut records = Vec::new();
            let mut split_env = build_model(package)?;
            let _ = run_prover_on_model_with_deadline(
                Some(budget.saturating_sub(started.elapsed()).as_secs().max(1)),
                &mut split_env,
                package,
                Some(&function),
                timeout,
                true,
                false,
                None,
                &mut diagnostics,
                Some(&mut records),
            );
            // The prover reports each unproved assertion as a diagnostic, so
            // read the records it produced rather than the rendered text: a
            // reworded or re-wrapped message must not silently empty this.
            attribution.hard_assertions = records
                .iter()
                .filter(|record| record.is_error)
                .map(|record| match (&record.file, record.line) {
                    (Some(file), Some(line)) => {
                        format!("{file}:{line}: {}", record.headline)
                    },
                    _ => record.headline.clone(),
                })
                .take(3)
                .collect();
            if attribution.hard_assertions.is_empty() {
                // Every assertion of the function proved on its own, so the
                // budget is exceeded by their combination rather than by one
                // hard obligation. Splitting further will not isolate it.
                attribution.split_note = Some(
                    "Each assertion proved separately; the combined condition exceeds the budget."
                        .to_string(),
                );
            }
        } else {
            attribution.incomplete = true;
        }
    }
    Ok(attribution)
}

/// Pair every declared condition in scope with the prover's report about it.
///
/// A diagnostic is attached to a condition only when it points at that
/// condition's own source position. Anything else is returned separately rather
/// than guessed at, so an unattached failure is visible instead of silently
/// marking an unrelated obligation as failing.
fn condition_statuses(
    env: &GlobalEnv,
    package: &Path,
    filter: Option<&str>,
    diagnostics: &[DiagnosticRecord],
) -> (Vec<ConditionStatus>, Vec<String>) {
    let mut failures: BTreeMap<(String, usize), &DiagnosticRecord> = BTreeMap::new();
    for record in diagnostics.iter().filter(|record| record.is_error) {
        if let (Some(file), Some(line)) = (&record.file, record.line) {
            failures
                .entry((
                    relative_source_path(package, std::ffi::OsStr::new(file)),
                    line,
                ))
                .or_insert(record);
        }
    }
    let mut attached: BTreeSet<(String, usize)> = BTreeSet::new();
    let mut statuses = Vec::new();
    for module in env.get_primary_target_modules() {
        for function in module.get_functions() {
            let qualified = format!(
                "{}::{}",
                module.get_name().display_full(env),
                function.get_name().display(env.symbol_pool())
            );
            if !filter.is_none_or(|filter| function_in_scope(&qualified, filter)) {
                continue;
            }
            // Inline body spec blocks carry the loop invariants the
            // inference workflow edits, so a status is reported for them too.
            for condition in &crate::candidate::conditions_of(&function) {
                let Some((file, location)) = env.get_file_and_location(&condition.loc) else {
                    continue;
                };
                let file = relative_source_path(package, std::ffi::OsStr::new(&file));
                let line = location.line.to_usize() + 1;
                let failure = failures.get(&(file.clone(), line));
                if failure.is_some() {
                    attached.insert((file.clone(), line));
                }
                statuses.push(ConditionStatus {
                    function: qualified.clone(),
                    kind: format!("{:?}", condition.kind),
                    file,
                    line,
                    verified: failure.is_none(),
                    diagnostic: failure.map(|record| record.headline.clone()),
                });
            }
        }
    }
    let unattached = failures
        .iter()
        .filter(|(key, _)| !attached.contains(*key))
        .map(|((file, line), record)| format!("{file}:{line}: {}", record.headline))
        .collect();
    (statuses, unattached)
}

/// Whether `module::function` falls inside a prover filter.
/// A task target, `address::module[::function]`, as a scope filter.
///
/// The two grammars overlap at two parts: `a::b` is `address::module` in a
/// target and `module::function` in a filter. The target's shape is known
/// here, so it is resolved rather than guessed. A named address cannot be
/// compared numerically anyway, so it is dropped; a hex one is kept, and
/// three-part targets are already unambiguous.
pub(crate) fn target_scope(target: &str) -> String {
    match target.split("::").collect::<Vec<_>>()[..] {
        [address, module] if !address.starts_with("0x") => module.to_string(),
        _ => target.to_string(),
    }
}

/// Whether `qualified` -- `module::fun` or `address::module::fun` -- is
/// selected by `filter`, which may name a module or a function, with or
/// without an address.
///
/// A hex address in both is compared numerically. A named address cannot be
/// resolved here and matches by module alone, as before: the study's targets
/// use one, and the model renders addresses numerically.
pub(crate) fn function_in_scope(qualified: &str, filter: &str) -> bool {
    let qualified: Vec<&str> = qualified.split("::").collect();
    let (address, module, function) = match qualified[..] {
        [module, function] => (None, module, function),
        [address, module, function] => (Some(address), module, function),
        _ => return false,
    };
    let filter: Vec<&str> = filter.split("::").collect();
    let (wanted_address, wanted_module, wanted_function) = match filter[..] {
        [module] => (None, module, None),
        [candidate, module] if candidate.starts_with("0x") => (Some(candidate), module, None),
        [module, function] => (None, module, Some(function)),
        [candidate, module, function] => (Some(candidate), module, Some(function)),
        _ => return false,
    };
    if let (Some(wanted), Some(actual)) = (wanted_address, address) {
        if !same_address(wanted, actual) {
            return false;
        }
    }
    module == wanted_module && wanted_function.is_none_or(|wanted| wanted == function)
}

/// Two hex address literals compared numerically; anything else must match
/// textually, and a named address is left to the module comparison.
fn same_address(wanted: &str, actual: &str) -> bool {
    use move_core_types::account_address::AccountAddress;
    match (
        AccountAddress::from_hex_literal(wanted),
        AccountAddress::from_hex_literal(actual),
    ) {
        (Ok(wanted), Ok(actual)) => wanted == actual,
        (Err(_), _) => true,
        (Ok(_), Err(_)) => wanted == actual,
    }
}

/// Refuse a package whose manifests name a dependency that resolution would
/// fetch over the network.
///
/// The resolver follows every local dependency's own manifest, so this walks
/// them too: a root that names only a local dependency could otherwise reach
/// a `git` dependency one manifest down.
pub(crate) fn reject_remote_dependencies(package: &Path) -> Result<(), String> {
    let mut pending = vec![package.to_path_buf()];
    let mut seen = std::collections::BTreeSet::new();
    while let Some(dir) = pending.pop() {
        let dir = dir.canonicalize().unwrap_or(dir);
        if !seen.insert(dir.clone()) {
            continue;
        }
        let manifest_path = dir.join("Move.toml");
        let manifest =
            move_package::source_package::manifest_parser::parse_move_manifest_from_file(
                &manifest_path,
            )
            .map_err(|error| format!("cannot read `{}`: {error:#}", manifest_path.display()))?;
        for (name, dependency) in manifest
            .dependencies
            .iter()
            .chain(manifest.dev_dependencies.iter())
        {
            if dependency.git_info.is_some() || dependency.node_info.is_some() {
                return Err(format!(
                    "an evaluation session cannot fetch the remote dependency `{name}` named in \
                     `{}`; only local dependencies are permitted",
                    manifest_path.display()
                ));
            }
            pending.push(dir.join(&dependency.local));
        }
    }
    Ok(())
}

fn check_candidate(args: &CheckCandidateArgs) -> Result<()> {
    let config = CandidateCheckConfig::load(&args.config)?;
    let verdict = evaluate_candidate(&config)?;
    let accepted = verdict.accepted;
    write_json(&args.output, &verdict)?;
    anyhow::ensure!(accepted, "candidate rejected: {}", verdict.state);
    Ok(())
}

pub fn evaluate_candidate(config: &CandidateCheckConfig) -> Result<CandidateVerdict> {
    anyhow::ensure!(
        (1..=MAX_EXPERIMENT_VC_TIMEOUT_SECS).contains(&config.timeout_seconds),
        "timeout must be between 1 and {} seconds",
        MAX_EXPERIMENT_VC_TIMEOUT_SECS
    );
    let check_started = Instant::now();
    // A candidate compared against a baseline is untrusted, and the judge
    // builds it outside the session's guard: a manifest naming a dependency
    // the build would fetch is refused here, before any build, and as a
    // verdict rather than an error -- an error would read as an
    // infrastructure failure and earn the candidate a retry.
    if config.baseline.is_some() {
        if let Err(message) = reject_remote_dependencies(&config.package) {
            return Ok(CandidateVerdict::new(
                CandidateState::PolicyViolation,
                message,
                StageOutcome::default(),
                ImplementationOutcome::default(),
                CandidateVerdict::unchecked_policy(),
                StageOutcome::default(),
            ));
        }
    }
    // `None` means every target module. A task states a target and the filter is
    // derived from it; an ordinary session may supply neither, and then the
    // check covers whatever the package declares as a target.
    let filter: Option<String> = match &config.filter {
        Some(filter) => Some(filter.clone()),
        None if config.target.is_empty() => None,
        None => Some(prover_filter(&config.target)?),
    };
    // The prover's filter carries no address; scope checks keep the task's
    // full target so two same-named modules at different addresses are told
    // apart.
    let scope: Option<String> = match &config.filter {
        Some(filter) => Some(filter.clone()),
        None if config.target.is_empty() => None,
        None => Some(target_scope(&config.target)),
    };
    let mut env = build_model(&config.package)
        .with_context(|| format!("failed to build `{}`", config.package.display()))?;
    let compile_diagnostics = render_diagnostics(&env);
    let compile = StageOutcome {
        ran: true,
        passed: !env.has_errors(),
        timed_out: false,
        diagnostics: compile_diagnostics,
    };
    if !compile.passed {
        let diagnostics = compile.joined_diagnostics();
        return Ok(CandidateVerdict::new(
            CandidateState::CompileFailure,
            diagnostics,
            compile,
            ImplementationOutcome::default(),
            CandidateVerdict::unchecked_policy(),
            StageOutcome::default(),
        ));
    }

    // Edit scope and an unchanged implementation are properties of a change
    // rather than of the specification text, so both need the tree the change
    // started from. They are reported rather than enforced: a caller decides
    // what an out-of-scope edit means, and a session with no pristine copy of
    // the package gets neither report.
    let (implementation, changed, scope_violations, baseline_opaque) = match &config.baseline {
        Some(baseline) => {
            let comparison = implementation_comparison(baseline, &config.package)?;
            let (changed, violations) =
                check_edit_scope(baseline, &config.package, &config.allowed_edit_paths)?;
            (
                ImplementationOutcome {
                    ran: true,
                    equal: comparison.equal,
                    added_modules: comparison.added_modules,
                    removed_modules: comparison.removed_modules,
                    changed_modules: comparison.changed_modules,
                },
                changed,
                violations,
                Some(baseline_contracts(baseline)?),
            )
        },
        None => (
            ImplementationOutcome::default(),
            Vec::new(),
            Vec::new(),
            None,
        ),
    };
    // Only the target's own sources: a vendored dependency may legitimately
    // disable verification, and the agent did not write it.
    let mut policy = check_specification(
        &env,
        &config.package,
        scope.as_deref(),
        &config.required_contract_categories,
    )?;
    policy.changed_paths = changed;
    policy.scope_violations = scope_violations;
    // With a baseline the candidate's own additions are rejected below;
    // without one the check says what the acceptance leans on instead.
    if baseline_opaque.is_none() {
        policy.assumed_contracts = crate::candidate::opaque_outside_scope(&env, scope.as_deref());
    }
    if let Some(baseline_opaque) = &baseline_opaque {
        let added = crate::candidate::assumed_contract_violations(
            &env,
            &config.package,
            scope.as_deref(),
            baseline_opaque,
        );
        if !added.is_empty() {
            policy.violations.extend(added);
            policy.passed = false;
        }
    }
    let implementation_changed = implementation.ran && !implementation.equal;
    if config.enforce_edit_policy && (implementation_changed || !policy.scope_violations.is_empty())
    {
        let mut diagnostics = Vec::new();
        if implementation_changed {
            diagnostics.push(format!(
                "the implementation changed: {}",
                implementation.summary()
            ));
        }
        if !policy.scope_violations.is_empty() {
            diagnostics.push(PolicyReport::format_violations(&policy.scope_violations));
        }
        return Ok(CandidateVerdict::new(
            CandidateState::PolicyViolation,
            diagnostics.join("\n"),
            compile,
            implementation,
            policy,
            StageOutcome::default(),
        ));
    }
    if !policy.passed {
        let diagnostics = PolicyReport::format_violations(&policy.violations);
        return Ok(CandidateVerdict::new(
            CandidateState::ForbiddenWeakening,
            diagnostics,
            compile,
            implementation,
            policy,
            StageOutcome::default(),
        ));
    }

    let mut prover_diagnostics = Vec::new();
    let mut prover_records = Vec::new();
    let proved = run_prover_on_model_with_deadline(
        remaining_deadline(config.process_deadline_seconds, check_started),
        &mut env,
        &config.package,
        filter.as_deref(),
        config.timeout_seconds,
        false,
        // Acceptance means the contract says something. An unsatisfiable
        // precondition discharges every obligation in the body, so without this
        // a candidate could pass by assuming its way out of the proof. A
        // syntactic rule would only catch `requires false`; this also catches
        // `requires x != x` and a precondition contradicting an `aborts_if`.
        true,
        None,
        &mut prover_diagnostics,
        Some(&mut prover_records),
    );
    let (conditions, unattached) = if config.report_conditions {
        condition_statuses(&env, &config.package, scope.as_deref(), &prover_records)
    } else {
        (Vec::new(), Vec::new())
    };
    let timed_out = prover_diagnostics
        .iter()
        .any(|line| is_prover_timeout(line));
    let prover = StageOutcome {
        ran: true,
        passed: proved.is_ok(),
        timed_out,
        diagnostics: prover_diagnostics,
    };
    if !prover.passed {
        // A verification failure is reported on the model as an error. A
        // failure that left none there -- a solver that could not be started,
        // a tool version the backend refuses -- checked nothing, and must not
        // be read as the specification being wrong.
        let reported = prover_records.iter().any(|record| record.is_error);
        let state = if prover.timed_out {
            CandidateState::ProverTimeout
        } else if reported {
            CandidateState::ProverFailure
        } else {
            CandidateState::InfrastructureFailure
        };
        let mut diagnostics = prover.joined_diagnostics();
        if prover.timed_out && config.attribute_timeouts {
            // Locate the obligation that exceeded the budget locally rather
            // than leaving the caller to narrow the filter by hand.
            match attribute_prover_timeout(
                &config.package,
                scoped_function_names(&env, scope.as_deref()),
                config.timeout_seconds,
                // Attribution spends what is left of the process deadline,
                // when there is one; the standalone command has the default.
                config.process_deadline_seconds.map_or(
                    Duration::from_secs(TIMEOUT_ATTRIBUTION_BUDGET_SECS),
                    |deadline| {
                        Duration::from_secs(deadline)
                            .saturating_sub(check_started.elapsed())
                            .min(Duration::from_secs(TIMEOUT_ATTRIBUTION_BUDGET_SECS))
                    },
                ),
            ) {
                Ok(attribution) => {
                    diagnostics.push_str("\n\n");
                    diagnostics.push_str(&attribution.render());
                },
                Err(error) => {
                    diagnostics
                        .push_str(&format!("\n\ntimeout attribution unavailable: {error:#}"));
                },
            }
        }
        return Ok(CandidateVerdict::new(
            state,
            diagnostics,
            compile,
            implementation,
            policy,
            prover,
        )
        .with_conditions(conditions, unattached));
    }
    if !policy.contract_coverage.passed {
        // Coverage is a different failure from weakening: nothing here was
        // weakened and nothing was edited out of scope -- the contract simply
        // does not yet say what the task requires. Reporting it as a weakening
        // told the author to repair locations that do not exist, and made a
        // second incomplete attempt end the run as a repeated policy breach.
        let diagnostics = PolicyReport::format_violations(&policy.contract_coverage.violations);
        return Ok(CandidateVerdict::new(
            CandidateState::IncompleteContract,
            diagnostics,
            compile,
            implementation,
            policy,
            prover,
        )
        .with_conditions(conditions, unattached));
    }
    Ok(CandidateVerdict::new(
        CandidateState::Accepted,
        String::new(),
        compile,
        implementation,
        policy,
        prover,
    )
    .with_conditions(conditions, unattached))
}

#[cfg(test)]
mod candidate_tests {
    use super::*;

    #[test]
    fn module_and_function_filters_select_the_right_scope() {
        assert!(function_in_scope("probe::heavy", "probe"));
        assert!(function_in_scope("probe::heavy", "probe::heavy"));
        assert!(!function_in_scope("probe::heavy", "probe::easy"));
        assert!(!function_in_scope("other::heavy", "probe"));
    }

    #[test]
    fn attribution_names_the_timed_out_function_and_its_peers() {
        let attribution = TimeoutAttribution {
            proved: vec!["probe::easy".to_string()],
            timed_out: vec!["probe::heavy".to_string()],
            failed: vec![],
            slowest: Some("probe::heavy".to_string()),
            slowest_millis: 3942,
            hard_assertions: vec![],
            split_note: Some("Each assertion proved separately.".to_string()),
            incomplete: false,
        };
        let rendered = attribution.render();
        assert!(rendered.contains("Timed out: probe::heavy"));
        assert!(rendered.contains("Proved in isolation: 1/2"));
        assert!(rendered.contains("Slowest: probe::heavy (3942 ms)"));
        assert!(rendered.contains("Each assertion proved separately."));
        assert!(!rendered.contains("Failed logically"));
    }

    #[test]
    fn both_timeout_wordings_are_recognised() {
        assert!(is_prover_timeout(
            "error: verification out of resources/timeout (global timeout set to 5s)"
        ));
        assert!(is_prover_timeout(
            "error: Boogie execution exceeded hard timeout of 70s"
        ));
        assert!(is_prover_timeout(
            "prover task exceeded hard timeout of 70s"
        ));
        assert!(!is_prover_timeout("error: post-condition does not hold"));
    }

    #[test]
    fn an_exhausted_budget_is_reported_rather_than_hidden() {
        let attribution = TimeoutAttribution {
            incomplete: true,
            ..Default::default()
        };
        assert!(attribution
            .render()
            .contains("Not every function in scope was probed"));
    }
}

fn write_compatibility_report(
    output: &Path,
    stage: &str,
    target: &str,
    passed: bool,
    diagnostics: Vec<String>,
) -> Result<()> {
    write_json(output, &CompatibilityReport {
        schema_version: 2,
        stage: stage.to_string(),
        target: Some(target.to_string()),
        passed,
        diagnostics,
        records: Vec::new(),
    })
}

fn prover_filter(target: &str) -> Result<String> {
    let parts: Vec<_> = target.split("::").collect();
    anyhow::ensure!(
        (2..=3).contains(&parts.len()) && parts.iter().all(|part| !part.is_empty()),
        "target must be address::module or address::module::function"
    );
    Ok(parts[1..].join("::"))
}

/// `None` verifies every function the package declares as a target.
fn verification_scope(filter: Option<&str>) -> VerificationScope {
    let Some(filter) = filter else {
        return VerificationScope::All;
    };
    if filter.contains("::") {
        VerificationScope::Only(filter.to_string())
    } else {
        VerificationScope::OnlyModule(filter.to_string())
    }
}

/// Compare executable ASTs while treating inline specification blocks as
/// transparent. Specs can force different local allocation and move/copy
/// choices in generated bytecode even when the Move program is unchanged.
fn executable_exp_equivalent(
    left_env: &GlobalEnv,
    left: &Exp,
    right_env: &GlobalEnv,
    right: &Exp,
) -> bool {
    use ExpData::*;

    // A trailing semicolon after a unit-valued statement is represented as an
    // empty tuple in the source model. It has no runtime effect, just like an
    // inline specification block. In particular, agents commonly write
    // `while (...) { ... } spec { ... };` in a unit-returning function.
    let is_transparent = |exp: &Exp| {
        matches!(exp.as_ref(), SpecBlock(..))
            || matches!(exp.as_ref(), Value(_, move_model::ast::Value::Tuple(values)) if values.is_empty())
            || matches!(exp.as_ref(), Call(_, Operation::Tuple, arguments) if arguments.is_empty())
    };
    let equivalent_list = |left: &[Exp], right: &[Exp]| {
        let left = left
            .iter()
            .filter(|exp| !is_transparent(exp))
            .collect::<Vec<_>>();
        let right = right
            .iter()
            .filter(|exp| !is_transparent(exp))
            .collect::<Vec<_>>();
        left.len() == right.len()
            && left
                .iter()
                .zip(right)
                .all(|(left, right)| executable_exp_equivalent(left_env, left, right_env, right))
    };

    match (left.as_ref(), right.as_ref()) {
        (Sequence(_, left), Sequence(_, right)) => equivalent_list(left, right),
        (Sequence(_, items), _) => {
            let items = items
                .iter()
                .filter(|exp| !is_transparent(exp))
                .collect::<Vec<_>>();
            items.len() == 1 && executable_exp_equivalent(left_env, items[0], right_env, right)
        },
        (_, Sequence(_, items)) => {
            let items = items
                .iter()
                .filter(|exp| !is_transparent(exp))
                .collect::<Vec<_>>();
            items.len() == 1 && executable_exp_equivalent(left_env, left, right_env, items[0])
        },
        (Invalid(_), Invalid(_)) | (SpecBlock(..), SpecBlock(..)) => true,
        (Value(_, left), Value(_, right)) => left == right,
        (LocalVar(_, left), LocalVar(_, right)) => {
            left.display(left_env.symbol_pool()).to_string()
                == right.display(right_env.symbol_pool()).to_string()
        },
        (Temporary(_, left), Temporary(_, right)) => left == right,
        (Call(left_id, left_op, left), Call(right_id, right_op, right)) => {
            left_op.display(left_env, *left_id).to_string()
                == right_op.display(right_env, *right_id).to_string()
                && equivalent_list(left, right)
        },
        (Invoke(_, left_fun, left), Invoke(_, right_fun, right)) => {
            executable_exp_equivalent(left_env, left_fun, right_env, right_fun)
                && equivalent_list(left, right)
        },
        (
            Lambda(_, left_pat, left_body, left_capture, _),
            Lambda(_, right_pat, right_body, right_capture, _),
        ) => {
            left_pat.display(left_env).to_string() == right_pat.display(right_env).to_string()
                && left_capture == right_capture
                && executable_exp_equivalent(left_env, left_body, right_env, right_body)
        },
        (
            Quant(_, left_kind, left_ranges, left_triggers, left_where, left_body),
            Quant(_, right_kind, right_ranges, right_triggers, right_where, right_body),
        ) => {
            left_kind == right_kind
                && left_ranges.len() == right_ranges.len()
                && left_ranges.iter().zip(right_ranges).all(
                    |((left_pat, left), (right_pat, right))| {
                        left_pat.display(left_env).to_string()
                            == right_pat.display(right_env).to_string()
                            && executable_exp_equivalent(left_env, left, right_env, right)
                    },
                )
                && left_triggers.len() == right_triggers.len()
                && left_triggers
                    .iter()
                    .zip(right_triggers)
                    .all(|(left, right)| equivalent_list(left, right))
                && match (left_where, right_where) {
                    (Some(left), Some(right)) => {
                        executable_exp_equivalent(left_env, left, right_env, right)
                    },
                    (None, None) => true,
                    _ => false,
                }
                && executable_exp_equivalent(left_env, left_body, right_env, right_body)
        },
        (
            Block(_, left_pat, left_binding, left_body),
            Block(_, right_pat, right_binding, right_body),
        ) => {
            left_pat.display(left_env).to_string() == right_pat.display(right_env).to_string()
                && match (left_binding, right_binding) {
                    (Some(left), Some(right)) => {
                        executable_exp_equivalent(left_env, left, right_env, right)
                    },
                    (None, None) => true,
                    _ => false,
                }
                && executable_exp_equivalent(left_env, left_body, right_env, right_body)
        },
        (
            IfElse(_, left_cond, left_then, left_else),
            IfElse(_, right_cond, right_then, right_else),
        ) => {
            executable_exp_equivalent(left_env, left_cond, right_env, right_cond)
                && executable_exp_equivalent(left_env, left_then, right_env, right_then)
                && executable_exp_equivalent(left_env, left_else, right_env, right_else)
        },
        (Match(_, left_discr, left_arms), Match(_, right_discr, right_arms)) => {
            executable_exp_equivalent(left_env, left_discr, right_env, right_discr)
                && left_arms.len() == right_arms.len()
                && left_arms.iter().zip(right_arms).all(|(left, right)| {
                    left.pattern.display(left_env).to_string()
                        == right.pattern.display(right_env).to_string()
                        && match (&left.condition, &right.condition) {
                            (Some(left), Some(right)) => {
                                executable_exp_equivalent(left_env, left, right_env, right)
                            },
                            (None, None) => true,
                            _ => false,
                        }
                        && executable_exp_equivalent(left_env, &left.body, right_env, &right.body)
                })
        },
        (Loop(_, left), Loop(_, right)) | (Return(_, left), Return(_, right)) => {
            executable_exp_equivalent(left_env, left, right_env, right)
        },
        (LoopCont(_, left_nest, left_continue), LoopCont(_, right_nest, right_continue)) => {
            left_nest == right_nest && left_continue == right_continue
        },
        (Assign(_, left_pat, left), Assign(_, right_pat, right)) => {
            left_pat.display(left_env).to_string() == right_pat.display(right_env).to_string()
                && executable_exp_equivalent(left_env, left, right_env, right)
        },
        (Mutate(_, left_lhs, left_rhs), Mutate(_, right_lhs, right_rhs)) => {
            executable_exp_equivalent(left_env, left_lhs, right_env, right_lhs)
                && executable_exp_equivalent(left_env, left_rhs, right_env, right_rhs)
        },
        _ => false,
    }
}

fn implementation_module_hashes(
    baseline: &GlobalEnv,
    candidate: &GlobalEnv,
) -> Result<(BTreeMap<String, String>, BTreeMap<String, String>)> {
    let baseline_modules = baseline
        .get_primary_target_modules()
        .into_iter()
        .map(|module| (module.get_full_name_str(), module))
        .collect::<BTreeMap<_, _>>();
    let candidate_modules = candidate
        .get_primary_target_modules()
        .into_iter()
        .map(|module| (module.get_full_name_str(), module))
        .collect::<BTreeMap<_, _>>();
    let mut baseline_hashes = BTreeMap::new();
    let mut candidate_hashes = BTreeMap::new();

    for (name, module) in &baseline_modules {
        let Some(compiled) = module.get_verified_module() else {
            continue;
        };
        let mut compiled = compiled.clone();
        compiled
            .metadata
            .retain(|metadata| metadata.key != COMPILATION_METADATA_KEY);
        let mut bytes = Vec::new();
        compiled
            .serialize(&mut bytes)
            .with_context(|| format!("cannot serialize `{name}`"))?;
        baseline_hashes.insert(name.clone(), sha256_hex(&bytes));
    }
    for (name, module) in &candidate_modules {
        let Some(compiled) = module.get_verified_module() else {
            continue;
        };
        let mut compiled = compiled.clone();
        compiled
            .metadata
            .retain(|metadata| metadata.key != COMPILATION_METADATA_KEY);

        if let Some(baseline_module) = baseline_modules.get(name) {
            // The compiler retains some specification-only material in tables
            // outside `metadata`. If the executable module model is unchanged,
            // use the baseline's normalized executable image rather than
            // comparing those model-only tables byte-for-byte.
            if executable_module_equivalent(baseline, baseline_module, candidate, module) {
                candidate_hashes.insert(
                    name.clone(),
                    baseline_hashes
                        .get(name)
                        .expect("baseline hash exists for matching module")
                        .clone(),
                );
                continue;
            }
            let baseline_functions = baseline_module
                .get_functions()
                .map(|function| (function.get_name_str(), function))
                .collect::<BTreeMap<_, _>>();
            let candidate_functions = module
                .get_functions()
                .map(|function| (function.get_name_str(), function))
                .collect::<BTreeMap<_, _>>();
            let baseline_compiled = baseline_module
                .get_verified_module()
                .expect("verified baseline module");
            // Preserve bytecode for every executable body which is unchanged
            // modulo specs. A module-level or function-level specification can
            // add model-only declarations without changing other bodies.
            for (function_name, baseline_function) in baseline_functions {
                let Some(candidate_function) = candidate_functions.get(&function_name) else {
                    continue;
                };
                let bodies_equal = match (baseline_function.get_def(), candidate_function.get_def())
                {
                    (Some(left), Some(right)) => {
                        executable_exp_equivalent(baseline, left, candidate, right)
                    },
                    (None, None) => true,
                    _ => false,
                };
                if bodies_equal {
                    if let (Some(baseline_index), Some(candidate_index)) = (
                        baseline_function.get_def_idx(),
                        candidate_function.get_def_idx(),
                    ) {
                        compiled.function_defs[candidate_index.0 as usize].code = baseline_compiled
                            .function_defs[baseline_index.0 as usize]
                            .code
                            .clone();
                    }
                }
            }
        }
        let mut bytes = Vec::new();
        compiled
            .serialize(&mut bytes)
            .with_context(|| format!("cannot serialize `{name}`"))?;
        candidate_hashes.insert(name.clone(), sha256_hex(&bytes));
    }
    Ok((baseline_hashes, candidate_hashes))
}

/// Whether the runtime-relevant part of two same-named modules is unchanged.
///
/// Move compiler metadata does not contain all specification artifacts. In
/// particular, a trailing loop `spec` block can allocate model-only entries in
/// a compiled module's tables. Compare source-model runtime definitions first,
/// so those entries cannot be mistaken for an executable change.
fn executable_module_equivalent(
    baseline_env: &GlobalEnv,
    baseline: &ModuleEnv<'_>,
    candidate_env: &GlobalEnv,
    candidate: &ModuleEnv<'_>,
) -> bool {
    // A public constant is executable behaviour too: a cross-module read goes
    // through a synthetic accessor, so a changed value changes what runs.
    let constants = |env: &GlobalEnv, module: &ModuleEnv<'_>| {
        module
            .get_named_constants()
            .map(|constant| {
                (
                    constant.get_name().display(env.symbol_pool()).to_string(),
                    (constant.get_type(), constant.get_value()),
                )
            })
            .collect::<BTreeMap<_, _>>()
    };
    if constants(baseline_env, baseline) != constants(candidate_env, candidate) {
        return false;
    }
    let baseline_functions = baseline
        .get_functions()
        .filter(|function| !function.is_lemma())
        .map(|function| (function.get_name_str(), function))
        .collect::<BTreeMap<_, _>>();
    let candidate_functions = candidate
        .get_functions()
        .filter(|function| !function.is_lemma())
        .map(|function| (function.get_name_str(), function))
        .collect::<BTreeMap<_, _>>();
    if baseline_functions.len() != candidate_functions.len() {
        return false;
    }
    for (name, baseline_function) in baseline_functions {
        let Some(candidate_function) = candidate_functions.get(&name) else {
            return false;
        };
        if baseline_function.visibility() != candidate_function.visibility()
            || baseline_function.is_native() != candidate_function.is_native()
            || baseline_function.is_entry() != candidate_function.is_entry()
            || baseline_function.is_inline() != candidate_function.is_inline()
            || baseline_function.get_parameter_types() != candidate_function.get_parameter_types()
            || baseline_function.get_result_type() != candidate_function.get_result_type()
            || baseline_function.get_type_parameter_count()
                != candidate_function.get_type_parameter_count()
        {
            return false;
        }
        let bodies_equal = match (baseline_function.get_def(), candidate_function.get_def()) {
            (Some(left), Some(right)) => {
                executable_exp_equivalent(baseline_env, left, candidate_env, right)
            },
            (None, None) => true,
            _ => false,
        };
        if !bodies_equal {
            return false;
        }
    }

    let Some(baseline_compiled) = baseline.get_verified_module() else {
        return false;
    };
    let Some(candidate_compiled) = candidate.get_verified_module() else {
        return false;
    };
    // Struct layout and friendship are executable/module-interface state.
    // Specification-only ghost fields do not occur in `struct_defs`.
    baseline_compiled.struct_defs == candidate_compiled.struct_defs
        && baseline_compiled.friend_decls == candidate_compiled.friend_decls
}

pub(crate) fn build_model(path: &Path) -> Result<GlobalEnv> {
    build_model_filtered_with_experiments(path, None, vec![])
}

fn build_model_for_implementation_comparison(path: &Path) -> Result<GlobalEnv> {
    build_model_filtered_with_experiments(path, None, vec![format!(
        "{}=off",
        move_compiler_v2::Experiment::OPTIMIZE
    )])
}

fn build_model_filtered_with_experiments(
    path: &Path,
    target_filter: Option<String>,
    experiments: Vec<String>,
) -> Result<GlobalEnv> {
    aptos_framework::build_model(
        true,
        false,
        true,
        path,
        BTreeMap::new(),
        target_filter,
        None,
        None,
        Some(LanguageVersion::latest()),
        false,
        aptos_framework::extended_checks::get_all_attribute_names().clone(),
        experiments,
        true,
        false,
    )
}

pub(crate) fn relative_source_path(root: &Path, path: &std::ffi::OsStr) -> String {
    let path = Path::new(path);
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create `{}`", parent.display()))?;
    }
    let data = serde_json::to_vec_pretty(value)?;
    fs::write(path, data).with_context(|| format!("cannot write `{}`", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A policy over the framework frame, as a corpus would declare it.
    fn corpus_policy() -> SelectionPolicy {
        SelectionPolicy {
            schema_version: 1,
            source_frames: vec![SourceFrame {
                path: "aptos-move/framework/aptos-framework".to_string(),
                include_paths: Vec::new(),
                require_upstream_reference: true,
                eligible_reason: "eligible_upstream_reference".to_string(),
            }],
            safety_exclusion_terms: vec!["crypto".to_string(), "consensus".to_string()],
            module_function_count: CountRange { min: 3, max: 10 },
        }
    }

    fn framework_frame(policy: &SelectionPolicy) -> String {
        policy.source_frames[0].path.clone()
    }

    #[test]
    fn inventory_excludes_verify_only_functions() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("resolve repository root");
        let policy = corpus_policy();
        let framework = framework_frame(&policy);
        let package = repo_root.join(&framework);
        let env = build_model(&package).expect("build framework model");
        let candidates = enumerate_package(&env, &repo_root, &framework, &policy);
        let candidate = candidates
            .iter()
            .find(|candidate| {
                candidate.package_module_target
                    == "0x1::big_ordered_map::test_verify_early_exit_walk_symbolic"
            })
            .expect("verify-only regression target is inventoried for provenance");

        assert_eq!(candidate.eligibility, "excluded");
        assert_eq!(candidate.decision_reason, "test_or_verify_only");
    }

    #[test]
    fn source_dependency_closure_includes_inline_function_modules() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("resolve repository root");
        let package = repo_root.join(framework_frame(&corpus_policy()));
        let env = build_model(&package).expect("build framework model");
        let module = env
            .get_modules()
            .find(|module| module.get_full_name_str() == "0x1::sigma_protocol_key_rotation")
            .expect("find key rotation module");

        assert!(module_dependency_closure(&env, &module).contains("0x1::sigma_protocol"));
    }

    #[test]
    fn opaque_contract_dependency_closure_follows_cross_module_spec_functions() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("resolve repository root");
        let package = repo_root.join(framework_frame(&corpus_policy()));
        let env = build_model(&package).expect("build framework model");
        let account = env
            .get_modules()
            .find(|module| module.get_full_name_str() == "0x1::account")
            .expect("find account module");
        let target = account
            .get_functions()
            .find(|function| function.get_name_str() == "assert_account_resource_with_error")
            .expect("find target function");

        let dependencies = proof_dependencies(&env, &target);
        assert!(dependencies
            .contract_functions
            .contains("0x1::account::exists_at"));
        assert!(dependencies
            .spec_functions
            .contains("0x1::account::spec_exists_at"));
        assert!(dependencies
            .spec_functions
            .contains("0x1::features::spec_is_enabled"));
    }

    #[test]
    fn runtime_hash_ignores_specs_but_rejects_executable_changes() {
        fn write_package(root: &Path, source: &str) {
            fs::create_dir_all(root.join("sources")).expect("create sources");
            fs::write(
                root.join("Move.toml"),
                "[package]\nname = \"runtime_guard\"\nversion = \"0.0.0\"\n",
            )
            .expect("write manifest");
            fs::write(root.join("sources/guard.move"), source).expect("write source");
        }

        let temporary = tempfile::tempdir().expect("create temporary package root");
        let baseline = temporary.path().join("baseline");
        let spec_only = temporary.path().join("spec-only");
        let trailing_loop_spec = temporary.path().join("trailing-loop-spec");
        let trailing_loop_spec_with_semicolon =
            temporary.path().join("trailing-loop-spec-with-semicolon");
        let unit_baseline = temporary.path().join("unit-baseline");
        let unit_trailing_semicolon = temporary.path().join("unit-trailing-semicolon");
        let runtime_change = temporary.path().join("runtime-change");
        write_package(
            &baseline,
            concat!(
                "module 0x42::guard { public fun f(x: u64): u64 { ",
                "let acc = 0; let i = 0; let n = x; ",
                "while (i < n) { acc = acc + i; i = i + 1; }; acc } }",
            ),
        );
        write_package(
            &spec_only,
            concat!(
                "module 0x42::guard { public fun f(x: u64): u64 { ",
                "let acc = 0; let i = 0; let n = x; ",
                "while ({ spec { invariant i <= n; invariant n == x; }; i < n }) { ",
                "acc = acc + i; i = i + 1; }; acc } ",
                "spec f { pragma opaque = true; ensures result >= 0; } ",
                "spec module { fun summary(x: u64): u64 { x } } }",
            ),
        );
        write_package(
            &runtime_change,
            concat!(
                "module 0x42::guard { public fun f(x: u64): u64 { ",
                "let acc = 0; let i = 0; let n = x; ",
                "while (i < n) { acc = acc + i + 1; i = i + 1; }; acc } }",
            ),
        );
        write_package(
            &trailing_loop_spec,
            concat!(
                "module 0x42::guard { public fun f(x: u64): u64 { ",
                "let acc = 0; let i = 0; let n = x; ",
                "while (i < n) { acc = acc + i; i = i + 1; } ",
                "spec { invariant i <= n; invariant n == x; }; acc } ",
                "spec f { pragma opaque = true; ensures result >= 0; } ",
                "spec module { fun summary(x: u64): u64 { x } } ",
                "spec lemma reflexive(x: u64) { ensures x == x; } }",
            ),
        );
        write_package(
            &trailing_loop_spec_with_semicolon,
            concat!(
                "module 0x42::guard { public fun f(x: u64): u64 { ",
                "let acc = 0; let i = 0; let n = x; ",
                "while (i < n) { acc = acc + i; i = i + 1; } ",
                "spec { invariant i <= n; invariant n == x; }; acc } ",
                "spec f { pragma opaque = true; ensures result >= 0; } ",
                "spec module { fun summary(x: u64): u64 { x } } ",
                "spec lemma reflexive(x: u64) { ensures x == x; } }",
            ),
        );
        write_package(
            &unit_trailing_semicolon,
            concat!(
                "module 0x42::guard { public fun f(x: u64) { ",
                "let i = 0; while (i < x) { i = i + 1; } ",
                "spec { invariant i <= x; }; } ",
                "spec f { pragma opaque = true; } }",
            ),
        );
        write_package(
            &unit_baseline,
            concat!(
                "module 0x42::guard { public fun f(x: u64) { ",
                "let i = 0; while (i < x) { i = i + 1; } } }",
            ),
        );

        let baseline_env = build_model_for_implementation_comparison(&baseline).expect("baseline");
        let spec_only_env =
            build_model_for_implementation_comparison(&spec_only).expect("spec-only candidate");
        let runtime_change_env =
            build_model_for_implementation_comparison(&runtime_change).expect("runtime change");
        let trailing_loop_spec_env = build_model_for_implementation_comparison(&trailing_loop_spec)
            .expect("trailing-loop-spec candidate");
        let trailing_loop_spec_with_semicolon_env =
            build_model_for_implementation_comparison(&trailing_loop_spec_with_semicolon)
                .expect("trailing-loop-spec-with-semicolon candidate");
        let unit_baseline_env =
            build_model_for_implementation_comparison(&unit_baseline).expect("unit baseline");
        let unit_trailing_semicolon_env =
            build_model_for_implementation_comparison(&unit_trailing_semicolon)
                .expect("unit-trailing-semicolon candidate");
        let (baseline_hashes, spec_only_hashes) =
            implementation_module_hashes(&baseline_env, &spec_only_env).expect("compare spec-only");
        let (_, runtime_change_hashes) =
            implementation_module_hashes(&baseline_env, &runtime_change_env)
                .expect("compare runtime change");
        let (_, trailing_loop_spec_hashes) =
            implementation_module_hashes(&baseline_env, &trailing_loop_spec_env)
                .expect("compare trailing loop spec");
        let (_, trailing_loop_spec_with_semicolon_hashes) =
            implementation_module_hashes(&baseline_env, &trailing_loop_spec_with_semicolon_env)
                .expect("compare trailing loop spec with semicolon");
        let (unit_baseline_hashes, unit_trailing_semicolon_hashes) =
            implementation_module_hashes(&unit_baseline_env, &unit_trailing_semicolon_env)
                .expect("compare unit trailing loop spec with semicolon");
        assert_eq!(baseline_hashes, spec_only_hashes);
        assert_eq!(baseline_hashes, trailing_loop_spec_hashes);
        assert_eq!(baseline_hashes, trailing_loop_spec_with_semicolon_hashes);
        assert_eq!(unit_baseline_hashes, unit_trailing_semicolon_hashes);
        assert_ne!(baseline_hashes, runtime_change_hashes);
    }

    #[test]
    fn a_task_target_is_not_read_as_a_filter() {
        // `a::b` is `address::module` in a target and `module::function` in a
        // filter. A named address is dropped, a hex one kept, three parts left
        // alone.
        assert_eq!("m", target_scope("campaign::m"));
        assert_eq!("0x1::m", target_scope("0x1::m"));
        assert_eq!("campaign::m::f", target_scope("campaign::m::f"));
        // The whole point: the module is still selected.
        assert!(function_in_scope(
            "0x4e110::m::f",
            &target_scope("campaign::m")
        ));
    }

    #[test]
    fn scope_matching_compares_addresses_when_given() {
        assert!(function_in_scope("0xcafe::m::f", "0xCAFE::m::f"));
        assert!(!function_in_scope("0xcafe::m::f", "0xbeef::m::f"));
        assert!(function_in_scope("0xcafe::m::f", "m::f"));
        assert!(function_in_scope("0xcafe::m::f", "m"));
        assert!(function_in_scope("0xcafe::m::f", "0xCAFE::m"));
        assert!(!function_in_scope("0xcafe::m::f", "0xBEEF::m"));
        assert!(function_in_scope("m::f", "m::f"));
        // A named address is left to the module comparison.
        assert!(function_in_scope("0x4e110::m::f", "campaign::m::f"));
    }

    #[test]
    fn a_candidate_naming_a_remote_dependency_is_a_policy_violation() {
        // The judge builds a candidate outside the session's guard; a manifest
        // that would fetch is refused before the build, as a verdict.
        let temporary = tempfile::tempdir().expect("temporary root");
        let write = |root: &Path, manifest_tail: &str| {
            fs::create_dir_all(root.join("sources")).expect("sources");
            fs::write(
                root.join("Move.toml"),
                format!("[package]\nname = \"remote\"\nversion = \"0.0.0\"\n{manifest_tail}"),
            )
            .expect("manifest");
            fs::write(
                root.join("sources/guard.move"),
                "module 0x42::guard { public fun f(x: u64): u64 { x } }\n",
            )
            .expect("source");
        };
        let baseline = temporary.path().join("baseline");
        let candidate = temporary.path().join("candidate");
        write(&baseline, "");
        write(
            &candidate,
            "[dependencies]\nRemote = { git = \"https://example.invalid/r.git\", rev = \"main\", subdir = \".\" }\n",
        );
        let config = CandidateCheckConfig {
            schema_version: 1,
            baseline: Some(baseline),
            package: candidate,
            target: "0x42::guard::f".to_string(),
            allowed_edit_paths: vec!["sources/**".to_string()],
            required_contract_categories: vec!["normal-result".to_string()],
            timeout_seconds: 10,
            attribute_timeouts: false,
            report_conditions: false,
            filter: None,
            enforce_edit_policy: true,
            process_deadline_seconds: None,
        };
        let verdict = evaluate_candidate(&config).expect("a verdict, not an error");
        let rendered = verdict.render();
        assert!(
            !verdict.accepted
                && rendered.starts_with("CANDIDATE_REJECTED")
                && rendered.contains("remote dependency"),
            "unexpected verdict:\n{rendered}"
        );
    }
}

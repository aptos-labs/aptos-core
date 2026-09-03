// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Static policy checks for a candidate specification.
//!
//! These decide whether an inferred specification is admissible at all,
//! independently of whether it verifies: the edit stayed inside the task's
//! declared paths, it introduced no forbidden weakening, and it covers the
//! contract categories the task requires. The experiment judge and the
//! agent-visible candidate check share this implementation so that an accepted
//! candidate cannot be rejected later by a differently-worded rule.

use crate::{conditions::ConditionStatus, evaluation::sha256_hex};
use anyhow::{Context, Result};
use move_model::{
    ast::{
        Condition, ConditionKind, Exp, ExpData, Operation, Proof, PropertyValue, Spec, SpecFunDecl,
        Value,
    },
    model::{FunctionEnv, GlobalEnv, Loc, ModuleEnv},
    pragmas::{
        ABORTS_IF_IS_PARTIAL_PRAGMA, ABORTS_IF_IS_STRICT_PRAGMA,
        ADDITION_OVERFLOW_UNCHECKED_PRAGMA, ASSUME_NO_ABORT_FROM_HERE_PRAGMA,
        CONDITION_ABSTRACT_PROP, CONDITION_CONCRETE_PROP, CONDITION_INFERRED_PROP,
        CONDITION_INFERRED_VACUOUS, DELEGATE_INVARIANTS_TO_CALLER_PRAGMA,
        DISABLE_INVARIANTS_IN_BODY_PRAGMA, EMITS_IS_PARTIAL_PRAGMA, UNROLL_PRAGMA,
        VERIFY_DURATION_ESTIMATE_PRAGMA, VERIFY_PRAGMA,
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

/// Paths excluded from workspace comparison. They hold build output, version
/// control data, or agent-local state rather than authored source.
///
/// `.coverage_map.mvcov` is written into the package root by the Move test
/// runner, so a session that ran the tests would otherwise be reported as
/// having edited a file it never wrote.
const IGNORED_NAMES: [&str; 5] = [
    ".git",
    ".claude",
    "build",
    "__pycache__",
    ".coverage_map.mvcov",
];

/// Pragmas that suppress an obligation rather than discharge it.
///
/// Each makes the prover check less than the contract appears to promise, so a
/// specification that needs one is not the specification it looks like. Solver
/// knobs such as `timeout` and `seed` are deliberately absent: they change how
/// a proof is found, not what it proves. The wording of each reason is the
/// pragma's own documented effect.
const OBLIGATION_SUPPRESSING_PRAGMAS: [(&str, &str); 5] = [
    (
        ADDITION_OVERFLOW_UNCHECKED_PRAGMA,
        "`u64` and `u128` addition is not checked for overflow, so an abort the code really has may be missing from the contract",
    ),
    (
        EMITS_IS_PARTIAL_PRAGMA,
        "the `emits` clauses become a lower bound, so events the code really emits may be missing from the contract",
    ),
    (
        ASSUME_NO_ABORT_FROM_HERE_PRAGMA,
        "aborts are ignored from that point on rather than characterized",
    ),
    (
        DISABLE_INVARIANTS_IN_BODY_PRAGMA,
        "global invariants are not checked between entry and exit",
    ),
    (
        DELEGATE_INVARIANTS_TO_CALLER_PRAGMA,
        "the invariant obligation is moved to callers, which this check does not verify",
    ),
];

/// Contract categories a task may require of an inferred specification.
///
/// Coverage is decided from the conditions the model carries rather than from
/// the text that produced them, so a condition included from a schema counts
/// exactly like one written in place.
pub const CONTRACT_CATEGORIES: [&str; 5] = [
    "normal-result",
    "abort",
    "state-transition",
    "frame",
    "loop-invariant",
];

/// Task parameters for one candidate check.
///
/// The controller materializes this outside the agent's writable workspace so
/// that an agent cannot relax its own acceptance criteria.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateCheckConfig {
    pub schema_version: u32,
    /// Pristine package the candidate is compared against.
    ///
    /// Optional: edit scope and an unchanged implementation are properties of a
    /// change, so they can only be judged against the tree the change started
    /// from. A session with no pristine copy skips both and says so.
    #[serde(default)]
    pub baseline: Option<PathBuf>,
    /// Edited package under evaluation.
    pub package: PathBuf,
    /// `address::module` or `address::module::function` under specification.
    pub target: String,
    pub allowed_edit_paths: Vec<String>,
    pub required_contract_categories: Vec<String>,
    /// Solver timeout per verification condition, in seconds.
    pub timeout_seconds: usize,
    /// Locate a prover timeout by probing each function in scope.
    ///
    /// This spends local computation to save the caller from narrowing the
    /// filter by hand, so it is enabled for the agent-visible check and left
    /// off for the judge, whose verdict does not depend on it.
    #[serde(default)]
    pub attribute_timeouts: bool,
    /// Report the status of each declared condition alongside the verdict.
    #[serde(default)]
    pub report_conditions: bool,
    /// Prover filter (`module` or `module::function`) used verbatim.
    ///
    /// A task states its target as `address::module[::function]` and the filter
    /// is derived from it; an ordinary session passes the filter directly.
    #[serde(default)]
    pub filter: Option<String>,
    /// Reject, rather than only report, a changed implementation or an edit
    /// outside the declared scope.
    ///
    /// Off by default: an ordinary session wants to be told, not blocked. A
    /// task configuration turns it on, because a candidate that changed the
    /// code it was asked to specify has not specified that code.
    #[serde(default)]
    pub enforce_edit_policy: bool,
    /// Hard deadline for the prover process, in seconds, when the caller
    /// answers within one; the prover's own watchdog has a floor of minutes.
    #[serde(default)]
    pub process_deadline_seconds: Option<u64>,
}

impl CandidateCheckConfig {
    /// Criteria for an ordinary session: whatever the package declares, with no
    /// task-imposed contract categories.
    pub fn for_package(
        package: &std::path::Path,
        filter: Option<&str>,
        timeout_seconds: usize,
    ) -> Result<Self> {
        let package = package.to_path_buf();
        Ok(Self {
            schema_version: 1,
            baseline: None,
            package,
            target: filter.unwrap_or_default().to_string(),
            allowed_edit_paths: Vec::new(),
            required_contract_categories: Vec::new(),
            timeout_seconds,
            attribute_timeouts: true,
            report_conditions: false,
            // An omitted filter stays `None`: it means every target module, and
            // an empty string would instead match no module at all.
            filter: filter.map(str::to_string),
            enforce_edit_policy: false,
            process_deadline_seconds: None,
        })
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read candidate check config `{}`", path.display()))?;
        let config: Self = serde_json::from_str(&text)
            .with_context(|| format!("invalid candidate check config `{}`", path.display()))?;
        anyhow::ensure!(
            config.schema_version == 1,
            "unsupported candidate check config schema {}",
            config.schema_version
        );
        anyhow::ensure!(
            !config.required_contract_categories.is_empty(),
            "candidate check config requires at least one contract category"
        );
        for category in &config.required_contract_categories {
            anyhow::ensure!(
                CONTRACT_CATEGORIES.contains(&category.as_str()),
                "unknown contract category `{category}`"
            );
        }
        anyhow::ensure!(
            !config.allowed_edit_paths.is_empty(),
            "candidate check config requires at least one editable path"
        );
        Ok(config)
    }
}

/// Outcome of one stage of the candidate check.
#[derive(Debug, Clone, Default, Serialize)]
pub struct StageOutcome {
    pub ran: bool,
    pub passed: bool,
    pub timed_out: bool,
    pub diagnostics: Vec<String>,
}

impl StageOutcome {
    pub fn joined_diagnostics(&self) -> String {
        self.diagnostics
            .iter()
            .map(|line| line.trim_end())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Whether the candidate still compiles to the task's executable bytecode.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ImplementationOutcome {
    pub ran: bool,
    pub equal: bool,
    pub added_modules: Vec<String>,
    pub removed_modules: Vec<String>,
    pub changed_modules: Vec<String>,
}

impl ImplementationOutcome {
    /// Which modules differ, for a caller that only reports the difference.
    pub fn summary(&self) -> String {
        [
            ("changed", &self.changed_modules),
            ("added", &self.added_modules),
            ("removed", &self.removed_modules),
        ]
        .iter()
        .filter(|(_, modules)| !modules.is_empty())
        .map(|(label, modules)| format!("{label} {}", modules.join(", ")))
        .collect::<Vec<_>>()
        .join("; ")
    }
}

/// Terminal classification of a candidate check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateState {
    Accepted,
    CompileFailure,
    /// Edit scope or runtime code, not the specification.
    PolicyViolation,
    ForbiddenWeakening,
    /// The specification neither weakens nor edits out of scope; it is simply
    /// missing a contract category the task requires. The repair is to add a
    /// clause, which is the opposite of what a weakening diagnosis asks for.
    IncompleteContract,
    ProverFailure,
    ProverTimeout,
    /// The prover could not run at all: no obligation was checked, so the
    /// candidate is neither accepted nor rejected.
    InfrastructureFailure,
}

impl CandidateState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "candidate_accepted",
            Self::CompileFailure => "compile_failure",
            Self::PolicyViolation => "policy_violation",
            Self::ForbiddenWeakening => "forbidden_weakening",
            Self::IncompleteContract => "incomplete_contract",
            Self::ProverFailure => "prover_failure",
            Self::ProverTimeout => "prover_timeout",
            Self::InfrastructureFailure => "infrastructure_failure",
        }
    }
}

/// Result of every agent-visible acceptance check.
#[derive(Debug, Clone, Serialize)]
pub struct CandidateVerdict {
    pub schema_version: u32,
    pub state: String,
    pub accepted: bool,
    pub diagnostics: String,
    pub compile: StageOutcome,
    pub implementation: ImplementationOutcome,
    pub policy: PolicyReport,
    pub prover: StageOutcome,
    /// Status of every declared condition in scope, when requested.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<ConditionStatus>,
    /// Failures that point at no declared condition.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unattached_diagnostics: Vec<String>,
}

impl CandidateVerdict {
    pub fn new(
        state: CandidateState,
        diagnostics: String,
        compile: StageOutcome,
        implementation: ImplementationOutcome,
        policy: PolicyReport,
        prover: StageOutcome,
    ) -> Self {
        Self {
            schema_version: 1,
            state: state.as_str().to_string(),
            accepted: state == CandidateState::Accepted,
            diagnostics,
            compile,
            implementation,
            policy,
            prover,
            conditions: Vec::new(),
            unattached_diagnostics: Vec::new(),
        }
    }

    /// Attach the per-condition report produced by the same prover run.
    pub fn with_conditions(
        mut self,
        conditions: Vec<ConditionStatus>,
        unattached_diagnostics: Vec<String>,
    ) -> Self {
        self.conditions = conditions;
        self.unattached_diagnostics = unattached_diagnostics;
        self
    }

    /// Empty policy report for a candidate rejected before policy ran.
    pub fn unchecked_policy() -> PolicyReport {
        PolicyReport {
            passed: false,
            changed_paths: Vec::new(),
            violations: Vec::new(),
            scope_violations: Vec::new(),
            assumed_contracts: Vec::new(),
            contract_coverage: ContractCoverage {
                passed: false,
                violations: Vec::new(),
            },
        }
    }

    /// Message shown to the agent. Acceptance is stated unmistakably so the
    /// workflow can stop instead of probing its own contract for exactness.
    pub fn render(&self) -> String {
        if self.accepted {
            let mut lines = vec![
                "CANDIDATE_ACCEPTED".to_string(),
                "All target obligations verified.".to_string(),
            ];
            // Only claim what was actually compared. Without a baseline there
            // is nothing to compare the implementation against.
            if self.implementation.ran {
                lines.push(
                    if self.implementation.equal {
                        "Implementation unchanged.".to_string()
                    } else {
                        format!(
                            "WARNING: the implementation changed: {}",
                            self.implementation.summary()
                        )
                    },
                );
            }
            lines.push("Contract coverage complete.".to_string());
            lines.extend(self.assumption_warnings());
            lines.extend(self.scope_warnings());
            return lines.join("\n");
        }
        let headline = match self.state.as_str() {
            "compile_failure" => "CANDIDATE_REJECTED: the package does not compile",
            "policy_violation" => {
                "CANDIDATE_REJECTED: the implementation changed or an edit falls outside the task's editable scope"
            },
            "forbidden_weakening" => "CANDIDATE_REJECTED: the specification is not admissible",
            "prover_timeout" => "CANDIDATE_REJECTED: verification timed out",
            "infrastructure_failure" => {
                "CHECK_UNAVAILABLE: the prover could not run, so this is not a verdict on the specification"
            },
            _ => "CANDIDATE_REJECTED: the target does not verify",
        };
        let mut sections = vec![headline.to_string()];
        if !self.diagnostics.trim().is_empty() {
            sections.push(self.diagnostics.trim_end().to_string());
        }
        if self.implementation.ran && !self.implementation.equal {
            sections.push(format!(
                "WARNING: the implementation changed: {}",
                self.implementation.summary()
            ));
        }
        sections.extend(self.assumption_warnings());
        sections.extend(self.scope_warnings());
        sections.join("\n")
    }

    /// Contracts the proof used without proving, reported so that an
    /// acceptance is not read as more than it is.
    fn assumption_warnings(&self) -> Vec<String> {
        if self.policy.assumed_contracts.is_empty() {
            return Vec::new();
        }
        vec![format!(
            "WARNING: verified against contracts that were assumed, not proved \
             here -- opaque functions outside the checked scope:\n{}",
            self.policy
                .assumed_contracts
                .iter()
                .map(|name| format!("  {name}"))
                .collect::<Vec<_>>()
                .join("\n")
        )]
    }

    /// Out-of-scope edits, reported rather than enforced.
    fn scope_warnings(&self) -> Vec<String> {
        if self.policy.scope_violations.is_empty() {
            return Vec::new();
        }
        vec![format!(
            "WARNING: edits outside the declared scope:\n{}",
            PolicyReport::format_violations(&self.policy.scope_violations)
        )]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Violation {
    pub code: String,
    pub path: String,
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractCoverage {
    pub passed: bool,
    pub violations: Vec<Violation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyReport {
    pub passed: bool,
    pub changed_paths: Vec<String>,
    pub violations: Vec<Violation>,
    /// Files edited outside the declared scope.
    ///
    /// Held apart from `violations` because it is reported rather than
    /// enforced: the caller decides what an out-of-scope edit means, so it
    /// does not by itself reject a candidate.
    #[serde(default)]
    pub scope_violations: Vec<Violation>,
    /// Opaque functions outside the checked scope, by qualified name.
    ///
    /// Their contracts are assumed at the target's call sites and are not
    /// proved here. With a baseline the candidate's own additions are
    /// rejected; without one the check cannot tell what the candidate added,
    /// so it says what the acceptance rests on instead of staying silent.
    #[serde(default)]
    pub assumed_contracts: Vec<String>,
    pub contract_coverage: ContractCoverage,
}

impl PolicyReport {
    /// Render violations the way the experiment judge reports them.
    pub fn format_violations(violations: &[Violation]) -> String {
        violations
            .iter()
            .map(|violation| {
                format!(
                    "{}:{}: {}: {}",
                    violation.path, violation.line, violation.code, violation.message
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Forbidden weakening and required contract coverage in the candidate's own
/// specifications.
///
/// Read from the compiler model rather than from the source text. The two are
/// not the same question: a condition included from a schema is an obligation
/// the target carries but no line the target contains, and an `ensures` inside
/// a spec function's body is a line the target contains but no obligation it
/// carries.
///
/// Needs no baseline: a weakening construct is rejected wherever it appears,
/// and a required category has to be covered by the specification as it stands.
/// `filter` of `None` means every function in the package's target modules;
/// an empty filter string would match no module and is not a way to say "all".
pub fn check_specification(
    env: &GlobalEnv,
    package: &Path,
    filter: Option<&str>,
    required_contract_categories: &[String],
) -> Result<PolicyReport> {
    for category in required_contract_categories {
        anyhow::ensure!(
            CONTRACT_CATEGORIES.contains(&category.as_str()),
            "unknown contract category `{category}`"
        );
    }

    let mut violations = Vec::new();
    let mut covered: BTreeSet<&'static str> = BTreeSet::new();
    let mut any_target = false;
    let abstract_property = env.symbol_pool().make(CONDITION_ABSTRACT_PROP);
    let concrete_property = env.symbol_pool().make(CONDITION_CONCRETE_PROP);
    let inferred_property = env.symbol_pool().make(CONDITION_INFERRED_PROP);
    let vacuous_value = env.symbol_pool().make(CONDITION_INFERRED_VACUOUS);

    for module in env.get_primary_target_modules() {
        // An axiom applies to everything in the module, whatever the filter.
        for condition in module.get_spec().conditions.iter() {
            if matches!(condition.kind, ConditionKind::Axiom(_)) {
                violations.push(Violation {
                    code: "axiom".to_string(),
                    message: "`axiom` is forbidden: it is assumed, never proved".to_string(),
                    ..location_of(env, package, &condition.loc)
                });
            }
        }
        for function in module.get_functions() {
            let qualified = format!(
                "{}::{}",
                module.get_name().display_full(env),
                function.get_name().display(env.symbol_pool())
            );
            // A lemma is proved unless its verification is disabled, in which
            // case every `apply` of it assumes it. That holds whatever the
            // filter selects, so it is checked for the whole target module.
            if function.is_lemma() && function.is_pragma_false(VERIFY_PRAGMA) {
                violations.push(Violation {
                    code: "unproved_lemma".to_string(),
                    message: "a lemma with verification disabled is assumed, not proved"
                        .to_string(),
                    ..location_of(env, package, &function.get_loc())
                });
            }
            // An assumption constrains whatever proof reaches it. The prover
            // inlines a transparent callee, so an `assume` in a helper outside
            // the checked scope still constrains the target's proof; these are
            // therefore checked before the scope filter, like `axiom`.
            let conditions = conditions_of(&function);
            for loc in proof_assumptions(&function) {
                violations.push(Violation {
                    code: "unjustified_assumption".to_string(),
                    message: "`assume` in a proof block is forbidden: it is trusted, not proved"
                        .to_string(),
                    ..location_of(env, package, &loc)
                });
            }
            for condition in &conditions {
                match condition.kind {
                    ConditionKind::Assume => violations.push(Violation {
                        code: "unjustified_assumption".to_string(),
                        message: "`assume` is forbidden: it narrows what is verified without \
                                  declaring a precondition"
                            .to_string(),
                        ..location_of(env, package, &condition.loc)
                    }),
                    ConditionKind::Axiom(_) => violations.push(Violation {
                        code: "axiom".to_string(),
                        message: "`axiom` is forbidden: it is assumed, never proved".to_string(),
                        ..location_of(env, package, &condition.loc)
                    }),
                    _ => {},
                }
            }
            // `unroll` carries a depth rather than a flag, so it needs the
            // numeric accessor: `is_pragma_true` never sees `pragma unroll = N`.
            if let Some(depth) = function.get_num_pragma(UNROLL_PRAGMA) {
                violations.push(Violation {
                    code: "suppressed_obligation".to_string(),
                    message: format!(
                        "`pragma unroll = {depth}` is forbidden: a loop without an invariant is \
                         unrolled to that depth, so the proof covers only {depth} iterations"
                    ),
                    ..location_of(env, package, &function.get_loc())
                });
            }
            for (pragma, reason) in OBLIGATION_SUPPRESSING_PRAGMAS {
                if function.is_pragma_true(pragma, || false) {
                    violations.push(Violation {
                        code: "suppressed_obligation".to_string(),
                        message: format!("`pragma {pragma}` is forbidden: {reason}"),
                        ..location_of(env, package, &function.get_loc())
                    });
                }
            }
            if !filter.is_none_or(|filter| crate::experiment::function_in_scope(&qualified, filter))
            {
                continue;
            }
            // A lemma states an auxiliary fact for the proof, not the
            // behaviour of executable code, so it neither covers a required
            // category nor counts as a target. Its own admissibility --
            // `unproved_lemma`, and any assumption it makes -- is checked
            // above, before the scope filter.
            if function.is_lemma() {
                continue;
            }
            any_target = true;
            let spec = function.get_spec();
            let at = |loc: &Loc| location_of(env, package, loc);

            // Pragmas that suppress or qualify the proof obligation.
            if function.is_pragma_false(VERIFY_PRAGMA) {
                violations.push(Violation {
                    code: "verify_false".to_string(),
                    message: "verification cannot be disabled".to_string(),
                    ..at(&function.get_loc())
                });
            }
            if function.is_pragma_true(ABORTS_IF_IS_PARTIAL_PRAGMA, || false) {
                violations.push(Violation {
                    code: "partial_aborts".to_string(),
                    message: "the abort characterization is incomplete: \
                              `aborts_if_is_partial` makes the emitted `aborts_if` clauses a \
                              lower bound. Complete the abort behavior and then remove the \
                              pragma; removing it on its own turns an incomplete contract \
                              into a false claim of exactness"
                        .to_string(),
                    ..at(&function.get_loc())
                });
            }
            if has_property(env, &function.get_spec(), VERIFY_DURATION_ESTIMATE_PRAGMA)
                || has_property(env, &module.get_spec(), VERIFY_DURATION_ESTIMATE_PRAGMA)
            {
                violations.push(Violation {
                    code: "duration_skip".to_string(),
                    message: "duration-based verification skipping is forbidden".to_string(),
                    ..at(&function.get_loc())
                });
            }

            // A `modifies` clause is a frame condition, and a frame is also a
            // statement about what the call changes.
            if spec
                .frame_spec
                .as_ref()
                .is_some_and(|frame| frame.modifies_all || !frame.modifies_targets.is_empty())
            {
                covered.insert("frame");
                covered.insert("state-transition");
            }

            for condition in conditions {
                // The inference engine marks a condition it derived from
                // unconstrained havoc `vacuous`: by its own definition such a
                // clause carries no information, so it cannot cover a category.
                if matches!(
                    condition.properties.get(&inferred_property),
                    Some(PropertyValue::Symbol(value)) if *value == vacuous_value
                ) {
                    violations.push(Violation {
                        code: "vacuous_inferred_condition".to_string(),
                        message: "a condition marked `[inferred = vacuous]` carries no \
                                  information: derive a real one or remove it"
                            .to_string(),
                        ..at(&condition.loc)
                    });
                    continue;
                }
                // A `[concrete]` condition is proof-only: verified against the
                // body and never assumed at a call site, so it is admissible
                // but publishes nothing and cannot cover a category.
                if condition.properties.contains_key(&concrete_property) {
                    continue;
                }
                // An `[abstract]` condition is applied at call sites and never
                // verified against the body, so it must not count as a contract.
                if condition.properties.contains_key(&abstract_property) {
                    violations.push(Violation {
                        code: "abstract_condition".to_string(),
                        message:
                            "an `[abstract]` condition is assumed by callers and never verified"
                                .to_string(),
                        ..at(&condition.loc)
                    });
                    continue;
                }
                match condition.kind {
                    ConditionKind::Ensures => {
                        if is_literal_true(&condition.exp) {
                            violations.push(Violation {
                                code: "vacuous_ensures".to_string(),
                                message: "vacuous ensures true is forbidden".to_string(),
                                ..at(&condition.loc)
                            });
                        }
                        covered.insert("normal-result");
                        if mentions_prior_or_global_state(&condition.exp) {
                            covered.insert("state-transition");
                        }
                    },
                    ConditionKind::AbortsIf => {
                        if is_literal_true(&condition.exp) {
                            violations.push(Violation {
                                code: "unconditional_abort".to_string(),
                                message: "unconditional aborts_if true is forbidden".to_string(),
                                ..at(&condition.loc)
                            });
                        }
                        covered.insert("abort");
                    },
                    ConditionKind::AbortsWith => {
                        covered.insert("abort");
                    },
                    ConditionKind::Emits => {
                        covered.insert("state-transition");
                    },
                    ConditionKind::LoopInvariant => {
                        if is_literal_true(&condition.exp) {
                            violations.push(Violation {
                                code: "vacuous_invariant".to_string(),
                                message: "vacuous invariant true is forbidden: it constrains \
                                          nothing about the loop"
                                    .to_string(),
                                ..at(&condition.loc)
                            });
                        } else {
                            covered.insert("loop-invariant");
                        }
                    },
                    _ => {},
                }
            }
        }
    }

    anyhow::ensure!(any_target, "{}", match filter {
        Some(filter) =>
            format!("filter `{filter}` selects no function in the package's target modules"),
        None => "the package has no function in any target module".to_string(),
    });

    let coverage_violations = required_contract_categories
        .iter()
        .filter(|category| !covered.contains(category.as_str()))
        .map(|category| Violation {
            code: "missing_contract_category".to_string(),
            path: "<target-specification>".to_string(),
            line: 1,
            message: format!(
                "required contract category `{category}` is absent from the specification"
            ),
        })
        .collect::<Vec<_>>();

    Ok(PolicyReport {
        passed: violations.is_empty(),
        changed_paths: Vec::new(),
        violations,
        scope_violations: Vec::new(),
        assumed_contracts: Vec::new(),
        contract_coverage: ContractCoverage {
            passed: coverage_violations.is_empty(),
            violations: coverage_violations,
        },
    })
}

/// Every condition attached to a function, wherever it is written.
///
/// Loop invariants live in an inline spec block inside the body rather than on
/// the function's own spec, so walking only `get_spec()` misses them.
/// Locations of `assume` statements in a function's proof blocks, including
/// those of its inline spec blocks.
fn proof_assumptions(function: &FunctionEnv<'_>) -> Vec<Loc> {
    let mut locs = Vec::new();
    if let Some(proof) = &function.get_spec().proof {
        collect_assumptions(proof, &mut locs);
    }
    if let Some(def) = function.get_def() {
        def.visit_post_order(&mut |exp| {
            if let ExpData::SpecBlock(_, inline) = exp {
                if let Some(proof) = &inline.proof {
                    collect_assumptions(proof, &mut locs);
                }
            }
            true
        });
    }
    locs
}

fn collect_assumptions(proof: &Proof, out: &mut Vec<Loc>) {
    match proof {
        Proof::Assume(loc, _) => out.push(loc.clone()),
        Proof::IfElse(_, _, then, otherwise) => {
            collect_assumptions(then, out);
            if let Some(otherwise) = otherwise {
                collect_assumptions(otherwise, out);
            }
        },
        Proof::Block(_, steps) => steps.iter().for_each(|step| collect_assumptions(step, out)),
        Proof::Post(_, inner) => collect_assumptions(inner, out),
        Proof::Let(..)
        | Proof::Assert(..)
        | Proof::Apply(..)
        | Proof::ForallApply(..)
        | Proof::Calc(..)
        | Proof::Split(..) => {},
    }
}

/// What the baseline package already assumed rather than verified.
///
/// The candidate is compared against this: assumptions it adds are
/// weakenings, assumptions the package already made are the package's own.
#[derive(Debug, Default, Clone)]
pub struct BaselineContracts {
    /// Functions whose contract a caller is verified against rather than
    /// their body -- opaque and native alike -- by qualified name, with their
    /// contract fingerprint.
    pub opaque: BTreeMap<String, String>,
    /// Intrinsic functions, by qualified name.
    pub intrinsic: BTreeSet<String>,
    /// Spec functions, by qualified name, with their definition.
    pub spec_functions: BTreeMap<String, String>,
}

/// A stable rendering of a spec function's definition: arity and displayed
/// body. A contract that references the function means what the body says.
pub fn spec_function_definition(env: &GlobalEnv, decl: &SpecFunDecl) -> String {
    decl.body.as_ref().map_or_else(
        || "<uninterpreted>".to_string(),
        |body| body.display(env).to_string(),
    )
}

/// How a spec function is identified across two models of the same package.
///
/// Arity is part of the identity, not of the definition: a module may declare
/// overloads sharing a name, and keying by name alone would collide them and
/// report an unchanged package as redefined.
pub fn spec_function_key(env: &GlobalEnv, module: &ModuleEnv<'_>, decl: &SpecFunDecl) -> String {
    format!(
        "{}::{}/{}/{}",
        module.get_name().display_full(env),
        decl.name.display(env.symbol_pool()),
        decl.type_params.len(),
        decl.params.len()
    )
}

/// A symbol-pool-independent rendering of a condition property's value.
///
/// `Debug` would print interned ids, which differ between two models of the
/// same source and would make every fingerprint compare unequal.
fn property_value(env: &GlobalEnv, value: &PropertyValue) -> String {
    match value {
        PropertyValue::Value(value) => format!("{value:?}"),
        PropertyValue::Symbol(symbol) => symbol.display(env.symbol_pool()).to_string(),
        PropertyValue::QualifiedSymbol(symbol) => symbol.display(env).to_string(),
    }
}

/// Every spec function in the model, by qualified name.
pub fn spec_function_definitions(env: &GlobalEnv) -> BTreeMap<String, String> {
    let mut definitions = BTreeMap::new();
    for module in env.get_modules() {
        for (_, decl) in module.get_spec_funs() {
            definitions.insert(
                spec_function_key(env, &module, decl),
                spec_function_definition(env, decl),
            );
        }
    }
    definitions
}

/// A stable rendering of a function's contract, comparable across two models
/// of the same sources.
///
/// Conditions are keyed by variant name and displayed expression rather than
/// by `Debug`, which would embed locations and interned symbol ids that differ
/// between models even for identical source.
pub fn contract_fingerprint(env: &GlobalEnv, function: &FunctionEnv<'_>) -> String {
    let spec = function.get_spec();
    let mut parts: Vec<String> = spec
        .conditions
        .iter()
        .map(|condition| {
            let kind = format!("{:?}", condition.kind);
            let kind = kind
                .split(|c: char| !c.is_ascii_alphanumeric())
                .next()
                .unwrap_or("");
            // Properties change what a condition means to a caller --
            // `[concrete]` withdraws it from call sites -- so they belong in
            // the fingerprint alongside the expression.
            let mut properties: Vec<String> = condition
                .properties
                .iter()
                .map(|(name, value)| {
                    format!(
                        "[{}={}]",
                        name.display(env.symbol_pool()),
                        property_value(env, value)
                    )
                })
                .collect();
            properties.sort();
            // `aborts_if P with CODE` keeps `CODE` here, and `emits` its
            // condition and handle: everything a caller is promised.
            let additional: String = condition
                .additional_exps
                .iter()
                .map(|exp| format!(" with {}", exp.display(env)))
                .collect();
            format!(
                "{kind}{} {}{additional}",
                properties.join(""),
                condition.exp.display(env)
            )
        })
        .collect();
    // Pragmas that qualify the contract a caller sees.
    parts.push(format!(
        "aborts_if_is_partial {}",
        function.is_pragma_true(ABORTS_IF_IS_PARTIAL_PRAGMA, || false)
    ));
    parts.push(format!(
        "emits_is_partial {}",
        function.is_pragma_true(EMITS_IS_PARTIAL_PRAGMA, || false)
    ));
    parts.push(format!(
        "aborts_if_is_strict {}",
        function.is_pragma_true(ABORTS_IF_IS_STRICT_PRAGMA, || false)
    ));
    parts.push(format!(
        "verify_false {}",
        function.is_pragma_false(VERIFY_PRAGMA)
    ));
    if let Some(frame) = &spec.frame_spec {
        parts.push(format!("modifies_all {}", frame.modifies_all));
        parts.extend(
            frame
                .modifies_targets
                .iter()
                .map(|target| format!("modifies {}", target.display(env))),
        );
    }
    parts.join("\n")
}

/// Opaque functions outside the checked scope, by qualified name.
///
/// Their contracts stand in for their bodies at every call site, and this
/// check proves only what the filter selects, so an acceptance rests on them.
/// Without a baseline the check cannot tell which the candidate added, so it
/// names them rather than either rejecting a package for its own dependencies
/// or staying silent.
pub fn opaque_outside_scope(env: &GlobalEnv, filter: Option<&str>) -> Vec<String> {
    let mut names = Vec::new();
    for module in env.get_modules() {
        for function in module.get_functions() {
            if !function.is_opaque() && !function.is_native() && !function.is_intrinsic() {
                continue;
            }
            let qualified = format!(
                "{}::{}",
                module.get_name().display_full(env),
                function.get_name().display(env.symbol_pool())
            );
            let verified_here = module.is_target()
                && filter
                    .is_none_or(|filter| crate::experiment::function_in_scope(&qualified, filter));
            if !verified_here {
                names.push(qualified);
            }
        }
    }
    names.sort();
    names
}

/// Assumptions the candidate added or altered, relative to the baseline.
///
/// An opaque function's contract is assumed at its call sites, and the check
/// verifies only the functions in scope: a helper the candidate made opaque,
/// or whose existing opaque contract it rewrote, would lend the target a
/// contract nothing proved. An intrinsic function is never verified at all,
/// so making one intrinsic is a weakening in any scope.
pub fn assumed_contract_violations(
    env: &GlobalEnv,
    package: &Path,
    filter: Option<&str>,
    baseline: &BaselineContracts,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    // A spec function the package already defined gives meaning to every
    // contract that references it, so redefining it changes what an unchanged
    // contract says. Adding one is fine.
    for module in env.get_modules() {
        for (_, decl) in module.get_spec_funs() {
            let name = spec_function_key(env, &module, decl);
            if baseline
                .spec_functions
                .get(&name)
                .is_some_and(|before| *before != spec_function_definition(env, decl))
            {
                violations.push(Violation {
                    code: "changed_spec_function".to_string(),
                    message: format!(
                        "`{name}` was redefined: contracts that reference it no longer mean what they did"
                    ),
                    ..location_of(env, package, &decl.loc)
                });
            }
        }
    }
    for module in env.get_modules() {
        for function in module.get_functions() {
            let qualified = format!(
                "{}::{}",
                module.get_name().display_full(env),
                function.get_name().display(env.symbol_pool())
            );
            let at = || location_of(env, package, &function.get_loc());
            if function.is_intrinsic() && !baseline.intrinsic.contains(&qualified) {
                violations.push(Violation {
                    code: "added_intrinsic".to_string(),
                    message: format!(
                        "`{qualified}` was made intrinsic, which skips its verification"
                    ),
                    ..at()
                });
            }
            // A native function is never verified either, and an intrinsic one
            // is modelled by the prover rather than proved; the declared
            // conditions of both become call-site assumptions exactly as an
            // opaque function's do.
            let assumed_at_call_sites =
                function.is_opaque() || function.is_native() || function.is_intrinsic();
            // What this check proves: a function of a primary target module
            // that the filter selects. An omitted filter means every target
            // module -- not every module, so a dependency helper is still
            // assumed rather than proved.
            let verified_here = module.is_target()
                && filter
                    .is_none_or(|filter| crate::experiment::function_in_scope(&qualified, filter));
            if !assumed_at_call_sites || verified_here {
                continue;
            }
            match baseline.opaque.get(&qualified) {
                // A native function absent from the baseline is an added
                // native function, which is an implementation change and is
                // reported as drift rather than here.
                None if !function.is_opaque() => {},
                None => violations.push(Violation {
                    code: "added_opaque".to_string(),
                    message: format!(
                        "`{qualified}` was made opaque: only the functions in scope are proved, so its contract would be assumed at the target's call sites without ever being verified. Leave the helper transparent -- the prover then reads its body -- or bring it into the checked scope"
                    ),
                    ..at()
                }),
                Some(fingerprint) if *fingerprint != contract_fingerprint(env, &function) => {
                    violations.push(Violation {
                        code: "changed_opaque_contract".to_string(),
                        message: format!(
                            "the opaque contract of `{qualified}` was changed: it is assumed at the target's call sites and not verified here"
                        ),
                        ..at()
                    })
                },
                Some(_) => {},
            }
        }
    }
    violations
}

pub(crate) fn conditions_of(function: &FunctionEnv<'_>) -> Vec<Condition> {
    let mut conditions = function.get_spec().conditions.clone();
    if let Some(def) = function.get_def() {
        def.visit_post_order(&mut |exp| {
            if let ExpData::SpecBlock(_, inline) = exp {
                // Inlining a call injects `assume`d marker conditions at the
                // call site. They belong to the prover, not to the candidate:
                // counting them would report an `unjustified_assumption` in
                // whatever file the inlined callee lives in -- source the
                // candidate never wrote and usually never changed -- and would
                // let a marker stand in for real contract coverage.
                conditions.extend(
                    inline
                        .conditions
                        .iter()
                        .filter(|condition| !is_inline_marker(&condition.exp))
                        .cloned(),
                );
            }
            true
        });
    }
    conditions
}

/// Whether a condition is one of the inliner's synthesised markers.
fn is_inline_marker(exp: &Exp) -> bool {
    matches!(exp.as_ref(), ExpData::Call(_, operation, _) if operation.is_inline_marker())
}

/// A `Violation` carrying only the position, for struct-update syntax.
fn location_of(env: &GlobalEnv, package: &Path, loc: &Loc) -> Violation {
    let (path, line) = match env.get_file_and_location(loc) {
        Some((file, location)) => (
            crate::experiment::relative_source_path(package, std::ffi::OsStr::new(&file)),
            location.line.to_usize() + 1,
        ),
        None => ("<target-specification>".to_string(), 1),
    };
    Violation {
        code: String::new(),
        path,
        line,
        message: String::new(),
    }
}

fn has_property(env: &GlobalEnv, spec: &Spec, name: &str) -> bool {
    spec.properties.contains_key(&env.symbol_pool().make(name))
}

/// Whether an expression is the literal `true`.
fn is_literal_true(exp: &Exp) -> bool {
    matches!(exp.as_ref(), ExpData::Value(_, Value::Bool(true)))
}

/// Whether an expression reads pre-state or global memory.
///
/// These are what make a postcondition a statement about a transition rather
/// than about the returned value alone.
fn mentions_prior_or_global_state(exp: &Exp) -> bool {
    let mut found = false;
    exp.visit_post_order(&mut |node| {
        if let ExpData::Call(_, operation, _) = node {
            if matches!(
                operation,
                Operation::Old | Operation::Global(..) | Operation::Exists(..)
            ) {
                found = true;
            }
        }
        true
    });
    found
}

/// Files changed outside the paths a task declares editable.
///
/// Edit scope is a property of a change rather than of the specification text,
/// so it needs the tree the change started from. An empty pattern list places
/// no restriction.
pub fn check_edit_scope(
    baseline: &Path,
    candidate: &Path,
    allowed_edit_paths: &[String],
) -> Result<(Vec<String>, Vec<Violation>)> {
    let changed = changed_paths(baseline, candidate)?;
    if allowed_edit_paths.is_empty() {
        return Ok((changed, Vec::new()));
    }
    let patterns = allowed_edit_paths
        .iter()
        .map(|pattern| compile_edit_pattern(pattern))
        .collect::<Result<Vec<_>>>()?;
    let violations = changed
        .iter()
        .filter(|relative| {
            !patterns
                .iter()
                .any(|variants| variants.iter().any(|glob| glob.matches(relative)))
        })
        .map(|relative| Violation {
            code: "out_of_scope_path".to_string(),
            path: relative.clone(),
            line: 1,
            message: "file is outside the task's declared editable paths".to_string(),
        })
        .collect();
    Ok((changed, violations))
}

/// Relative paths whose content differs between the two trees.
pub fn changed_paths(baseline: &Path, candidate: &Path) -> Result<Vec<String>> {
    let before = tree_digests(baseline)?;
    let after = tree_digests(candidate)?;
    let mut names: BTreeSet<&String> = before.keys().collect();
    names.extend(after.keys());
    Ok(names
        .into_iter()
        .filter(|name| before.get(*name) != after.get(*name))
        .cloned()
        .collect())
}

/// Match a relative path against a run manifest's edit-scope pattern.
///

/// Compile one editable-path pattern into the globs that satisfy it.
///
/// `*` spans directory separators here, and a `**/` marker is expanded to its
/// zero-directory forms as well, so `sources/**/*.move` also admits
/// `sources/target.spec.move`.
fn compile_edit_pattern(pattern: &str) -> Result<Vec<glob::Pattern>> {
    let mut variants: BTreeSet<String> = BTreeSet::from([pattern.to_string()]);
    let mut pending = vec![pattern.to_string()];
    while let Some(variant) = pending.pop() {
        let Some(marker) = variant.find("**/") else {
            continue;
        };
        let without_marker = format!("{}{}", &variant[..marker], &variant[marker + 3..]);
        if variants.insert(without_marker.clone()) {
            pending.push(without_marker);
        }
    }
    variants
        .iter()
        .map(|variant| {
            glob::Pattern::new(variant)
                .with_context(|| format!("invalid editable path pattern `{pattern}`"))
        })
        .collect()
}

fn tree_digests(root: &Path) -> Result<BTreeMap<String, String>> {
    let mut digests = BTreeMap::new();
    for path in tree_files(root) {
        let relative = relative_posix(root, &path);
        digests.insert(relative, path_digest(&path)?);
    }
    Ok(digests)
}

fn tree_files(root: &Path) -> Vec<PathBuf> {
    if !root.exists() {
        return Vec::new();
    }
    let mut files: Vec<PathBuf> = WalkDir::new(root)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || !IGNORED_NAMES.contains(&entry.file_name().to_string_lossy().as_ref())
        })
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file() || entry.file_type().is_symlink())
        .map(|entry| entry.into_path())
        .collect();
    files.sort();
    files
}

fn path_digest(path: &Path) -> Result<String> {
    if path.is_symlink() {
        let target = std::fs::read_link(path)
            .with_context(|| format!("cannot read link `{}`", path.display()))?;
        return Ok(format!("link:{}", target.to_string_lossy()));
    }
    let bytes = std::fs::read(path).with_context(|| format!("cannot read `{}`", path.display()))?;
    Ok(sha256_hex(&bytes))
}

fn relative_posix(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(root: &Path, relative: &str, content: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    /// Check one module's specification against the real compiler model.
    fn check_module(source: &str, required: &[&str]) -> PolicyReport {
        let package = crate::tests::common::make_package("probe", &[("m", source)]);
        let env = crate::experiment::build_model(package.path()).expect("model");
        assert!(!env.has_errors(), "probe module does not compile");
        let required: Vec<String> = required.iter().map(|name| name.to_string()).collect();
        check_specification(&env, package.path(), Some("m"), &required).expect("check")
    }

    fn codes(report: &PolicyReport) -> Vec<String> {
        let mut codes: Vec<String> = report
            .violations
            .iter()
            .map(|violation| violation.code.clone())
            .collect();
        codes.sort();
        codes
    }

    #[test]
    fn a_weakening_construct_is_rejected_wherever_it_appears() {
        let report = check_module(
            "module 0xCAFE::m {
    fun f(x: u64): u64 { x }
    spec f {
        aborts_if true;
        ensures true;
    }
}",
            &[],
        );

        assert_eq!(
            vec!["unconditional_abort", "vacuous_ensures"],
            codes(&report)
        );
        assert!(!report.passed);
    }

    #[test]
    fn suppressing_pragmas_are_rejected() {
        let report = check_module(
            "module 0xCAFE::m {
    fun f(x: u64): u64 { x }
    spec f {
        pragma verify = false;
        pragma aborts_if_is_partial = true;
        ensures result == x;
    }
}",
            &[],
        );

        assert_eq!(vec!["partial_aborts", "verify_false"], codes(&report));
    }

    #[test]
    fn a_required_category_must_be_covered() {
        let report = check_module(
            "module 0xCAFE::m {
    fun f(x: u64): u64 { x }
    spec f {
        ensures result == x;
    }
}",
            &["normal-result", "abort"],
        );

        let missing: Vec<&str> = report
            .contract_coverage
            .violations
            .iter()
            .map(|violation| violation.message.as_str())
            .collect();
        assert_eq!(1, missing.len());
        assert!(missing[0].contains("abort"), "unexpected: {}", missing[0]);
    }

    #[test]
    fn a_loop_invariant_is_found_in_the_body_it_is_written_in() {
        // Loop invariants are not on the function's own spec; they live in an
        // inline spec block inside the body.
        let report = check_module(
            "module 0xCAFE::m {
    fun f(n: u64): u64 {
        let i = 0;
        while (i < n) {
            i = i + 1;
        } spec {
            invariant i <= n;
        };
        i
    }
}",
            &["loop-invariant"],
        );

        assert!(
            report.contract_coverage.passed,
            "loop invariant not seen: {:?}",
            report.contract_coverage.violations
        );
    }

    #[test]
    fn a_condition_included_from_a_schema_counts_as_coverage() {
        // The obligation is the target's, though no `aborts_if` is written in
        // the function's own spec block. Reading the text would miss it.
        let report = check_module(
            "module 0xCAFE::m {
    fun f(x: u64): u64 { x + 1 }
    spec schema NoOverflow {
        x: u64;
        aborts_if x + 1 > MAX_U64;
    }
    spec f {
        include NoOverflow;
        ensures result == x + 1;
    }
}",
            &["normal-result", "abort"],
        );

        assert!(
            report.contract_coverage.passed,
            "schema-included condition not counted: {:?}",
            report.contract_coverage.violations
        );
    }

    #[test]
    fn modifies_covers_the_frame_and_the_transition() {
        let report = check_module(
            "module 0xCAFE::m {
    struct R has key { v: u64 }
    fun f(a: address) acquires R {
        R[a].v = R[a].v + 1;
    }
    spec f {
        modifies global<R>(a);
        ensures global<R>(a).v == old(global<R>(a).v) + 1;
    }
}",
            &["frame", "state-transition"],
        );

        assert!(
            report.contract_coverage.passed,
            "frame or transition not seen: {:?}",
            report.contract_coverage.violations
        );
    }

    #[test]
    fn an_accepted_candidate_still_reports_what_it_only_observed() {
        // Edit scope and a changed implementation are reported, not enforced,
        // so acceptance stands -- but the caller has to be told.
        let verdict = CandidateVerdict::new(
            CandidateState::Accepted,
            String::new(),
            StageOutcome::default(),
            ImplementationOutcome {
                ran: true,
                equal: false,
                added_modules: Vec::new(),
                removed_modules: Vec::new(),
                changed_modules: vec!["0x42::m".to_string()],
            },
            PolicyReport {
                passed: true,
                changed_paths: vec!["sources/m.move".to_string()],
                violations: Vec::new(),
                scope_violations: vec![Violation {
                    code: "out_of_scope_path".to_string(),
                    path: "sources/m.move".to_string(),
                    line: 1,
                    message: "file is outside the task's declared editable paths".to_string(),
                }],
                assumed_contracts: Vec::new(),
                contract_coverage: ContractCoverage {
                    passed: true,
                    violations: Vec::new(),
                },
            },
            StageOutcome::default(),
        );

        let rendered = verdict.render();

        assert!(rendered.starts_with("CANDIDATE_ACCEPTED"));
        assert!(rendered.contains("WARNING: the implementation changed: changed 0x42::m"));
        assert!(rendered.contains("WARNING: edits outside the declared scope"));
        assert!(rendered.contains("sources/m.move"));
        // The unqualified claim is only made when there was something to compare.
        assert!(!rendered.contains("Implementation unchanged."));
    }

    #[test]
    fn without_a_baseline_no_implementation_claim_is_made() {
        let verdict = CandidateVerdict::new(
            CandidateState::Accepted,
            String::new(),
            StageOutcome::default(),
            ImplementationOutcome::default(),
            PolicyReport {
                passed: true,
                changed_paths: Vec::new(),
                violations: Vec::new(),
                scope_violations: Vec::new(),
                assumed_contracts: Vec::new(),
                contract_coverage: ContractCoverage {
                    passed: true,
                    violations: Vec::new(),
                },
            },
            StageOutcome::default(),
        );

        let rendered = verdict.render();

        assert!(rendered.starts_with("CANDIDATE_ACCEPTED"));
        assert!(!rendered.contains("Implementation"));
        assert!(!rendered.contains("WARNING"));
    }

    #[test]
    fn build_output_is_not_a_workspace_change() {
        let temporary = tempfile::tempdir().unwrap();
        let baseline = temporary.path().join("baseline");
        let candidate = temporary.path().join("candidate");
        write(&baseline, "sources/a.move", "module a {}\n");
        write(&candidate, "sources/a.move", "module a {}\n");
        write(&candidate, "build/artifact.mv", "binary\n");

        let (changed, violations) =
            check_edit_scope(&baseline, &candidate, &["sources/**".to_string()]).unwrap();

        assert!(changed.is_empty());
        assert!(violations.is_empty());
    }
}

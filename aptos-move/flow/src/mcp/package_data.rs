// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::McpArgs;
use codespan::Span;
use codespan_reporting::{
    diagnostic::{LabelStyle, Severity},
    term::{emit, termcolor::NoColor, Config},
};
use move_compiler_v2::Experiment;
use move_model::model::{CwdRelativeFiles, FunId, GlobalEnv, Loc, ModuleId, QualifiedId};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// Source stage that produced a set of diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DiagnosticSource {
    Compiler,
    Verifier,
    Inference,
}

/// What scope was last verified and whether it succeeded.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum VerifiedScope {
    Package,
    Module(ModuleId),
    Function(QualifiedId<FunId>),
}

impl VerifiedScope {
    /// Returns true if a cached verification at `self` scope covers `request`.
    ///
    /// This is only sound for **success**: if a wider scope verified successfully,
    /// every narrower scope within it is also correct. For failures the error may
    /// reside outside the requested scope, so [`entails_error`] must be used instead.
    pub(crate) fn entails_success(&self, request: &VerifiedScope) -> bool {
        match self {
            VerifiedScope::Package => true,
            VerifiedScope::Module(m) => match request {
                VerifiedScope::Module(m2) => m == m2,
                VerifiedScope::Function(qid) => qid.module_id == *m,
                _ => false,
            },
            VerifiedScope::Function(f) => match request {
                VerifiedScope::Function(f2) => f == f2,
                _ => false,
            },
        }
    }

    /// Returns true if a cached **error** at `self` scope is relevant for `request`.
    ///
    /// A failure is only reused when the scopes are identical; a wider-scope failure
    /// does not imply that a narrower scope also fails (the error may be elsewhere).
    /// A narrower scope may miss errors from other locations, so only equality matters.
    pub(crate) fn entails_error(&self, request: &VerifiedScope) -> bool {
        self == request
    }
}

// ================================================================================================

/// Compiled package data holding a `GlobalEnv` (the Move model).
pub(crate) struct PackageData {
    env: GlobalEnv,
    verified: Option<(VerifiedScope, bool, usize)>,
    /// Whether the initial compilation produced errors.
    has_compilation_errors: bool,
    /// Per-source diagnostic messages, keyed by the stage that produced them.
    diagnostics: BTreeMap<DiagnosticSource, Vec<String>>,
    /// Path used to build `env`; retained so tools can rebuild a fresh env
    /// with a per-call filter (matching CLI `aptos move prove -f` semantics).
    path: PathBuf,
    /// Args used to build `env`; retained for the same reason as `path`.
    args: McpArgs,
}

// SAFETY: `GlobalEnv` is `!Send` because it uses `Rc` and `NonNull` internally for its
// symbol pool and expression arena. However, all reference-counted pointers are fully
// contained within the `GlobalEnv` — no `Rc` clones escape — so moving the entire value
// to another thread is safe. Access is further guarded by a `Mutex` in `FlowSession`.
unsafe impl Send for PackageData {}

impl PackageData {
    /// Build the Move model for the package at `path`.
    ///
    /// Only fails on I/O errors or invalid package path. All compilation errors and
    /// warnings are stored in the returned `GlobalEnv`.
    pub(crate) fn init(path: &Path, args: &McpArgs) -> anyhow::Result<Self> {
        let env = Self::build_env(path, args, args.target_filter.clone())?;
        let compilation_diagnostics = render_diagnostics(&env);
        let has_compilation_errors = env.has_errors();
        log_diagnostics(&compilation_diagnostics, DiagnosticSource::Compiler);
        let mut diagnostics = BTreeMap::new();
        diagnostics.insert(DiagnosticSource::Compiler, compilation_diagnostics);
        Ok(Self {
            env,
            verified: None,
            has_compilation_errors,
            diagnostics,
            path: path.to_path_buf(),
            args: args.clone(),
        })
    }

    /// Build a fresh `GlobalEnv` whose primary-target set is narrowed by a
    /// positive `filter` and/or a list of module names to demote, composed on
    /// top of the session-level `target_filter` so per-call narrowing cannot
    /// widen the scope beyond what the session was started with. The cached
    /// env held by `PackageData` is unchanged. Errors when both knobs are empty.
    pub(crate) fn build_filtered_env(
        &self,
        filter: Option<&str>,
        excluded_modules: &[String],
    ) -> anyhow::Result<GlobalEnv> {
        if filter.is_none() && excluded_modules.is_empty() {
            anyhow::bail!("build_filtered_env called with no filter and no exclusions");
        }
        let mut args = self.args.clone();
        args.experiments
            .push(Experiment::UNSAFE_PACKAGE_VISIBILITY.to_string());
        let mut env = Self::build_env(&self.path, &args, args.target_filter.clone())?;
        let filter_module = filter.map(|f| super::tools::module_part_of(f).to_string());
        let exclude_names: std::collections::BTreeSet<&str> =
            excluded_modules.iter().map(|s| s.as_str()).collect();
        // Best-effort file-granular demote; on file-share conflicts, fall
        // back to verify_scope on the unchanged env.
        if filter_module.is_some() || !exclude_names.is_empty() {
            let demote_result = env.demote_modules_from_primary_targets(|m| {
                let filter_demotes = filter_module
                    .as_ref()
                    .is_some_and(|name| !m.matches_name(name));
                let exclude_demotes = exclude_names.iter().any(|n| m.matches_name(n));
                filter_demotes || exclude_demotes
            });
            if let Err(blocked) = demote_result {
                log::warn!(
                    "build_filtered_env: filter `{:?}` / exclude `{:?}` cannot \
                     narrow without splitting a source file (siblings: {:?}); \
                     falling back to verify_scope on the broader env",
                    filter_module,
                    excluded_modules,
                    blocked
                );
            }
        }
        Ok(env)
    }

    /// Lower-level helper that calls `aptos_framework::build_model` with the given
    /// `target_filter`. Shared between `init` (session-global filter) and
    /// `build_filtered_env` (per-call filter).
    fn build_env(
        path: &Path,
        args: &McpArgs,
        target_filter: Option<String>,
    ) -> anyhow::Result<GlobalEnv> {
        let named_addresses = args
            .named_addresses
            .iter()
            .map(|(name, addr)| (name.clone(), addr.into_inner()))
            .collect();
        aptos_framework::build_model(
            args.dev_mode,
            // test_mode off: test code is handled separately by run_move_unit_tests.
            // verify_mode on: #[verify_only] specs are needed by the prover.
            false, // test_mode
            true,  // verify_mode
            path,
            named_addresses,
            target_filter,
            args.bytecode_version,
            None,
            Some(args.language_version),
            false,
            aptos_framework::extended_checks::get_all_attribute_names().clone(),
            args.experiments.clone(),
            true,  // always build with bytecode
            false, // all_files_as_targets
        )
    }

    /// Access the compiled `GlobalEnv`.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn env(&self) -> &GlobalEnv {
        &self.env
    }

    /// Mutable access to the `GlobalEnv` (needed by the prover).
    pub(crate) fn env_mut(&mut self) -> &mut GlobalEnv {
        &mut self.env
    }

    /// Returns the cached verification result, if any.
    pub(crate) fn verified(&self) -> Option<(VerifiedScope, bool, usize)> {
        self.verified.clone()
    }

    /// Store a verification result together with the vc timeout used.
    pub(crate) fn set_verified(&mut self, scope: VerifiedScope, success: bool, timeout: usize) {
        self.verified = Some((scope, success, timeout));
    }

    /// Whether the initial compilation produced errors.
    pub(crate) fn has_compilation_errors(&self) -> bool {
        self.has_compilation_errors
    }

    /// Refusal message for tools that require a compiled package. Inlines the
    /// stored compiler diagnostics so the caller does not need a second tool
    /// call to see them.
    pub(crate) fn compilation_errors_report(&self) -> String {
        let messages = self.diagnostics(DiagnosticSource::Compiler);
        if messages.is_empty() {
            "package has compilation errors; run move_package_status for details".to_string()
        } else {
            format!("package has compilation errors:\n{}", messages.join("\n"))
        }
    }

    /// Returns stored diagnostic messages for the given source.
    pub(crate) fn diagnostics(&self, source: DiagnosticSource) -> &[String] {
        self.diagnostics
            .get(&source)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// Store diagnostics for the given source.
    pub(crate) fn set_diagnostics(&mut self, source: DiagnosticSource, diagnostics: Vec<String>) {
        log_diagnostics(&diagnostics, source);
        self.diagnostics.insert(source, diagnostics);
    }
}

/// Log stored diagnostics at INFO level.
fn log_diagnostics(diagnostics: &[String], source: DiagnosticSource) {
    if diagnostics.is_empty() {
        log::info!("stored diagnostics ({:?}): none", source);
    } else {
        log::info!(
            "stored diagnostics ({:?}): {} message(s):\n{}",
            source,
            diagnostics.len(),
            diagnostics.join("\n")
        );
    }
}

/// One diagnostic with its rendered text and its primary source position.
///
/// Reporting drains the environment's diagnostics, so text and position have to
/// be collected in the same pass.
#[derive(Debug, Clone)]
pub(crate) struct DiagnosticRecord {
    pub text: String,
    pub headline: String,
    pub file: Option<String>,
    pub line: Option<usize>,
    /// One-based column of the primary label.
    pub column: Option<usize>,
    /// The primary label's own message.
    ///
    /// `headline` is the diagnostic code's wording ("unused alias"); this is
    /// what the caret points at ("Unused 'use' of alias 'dep'"), which is
    /// where the specifics live.
    pub label: Option<String>,
    /// Notes attached below the diagnostic, as the producer wrote them.
    pub notes: Vec<String>,
    pub is_error: bool,
}

/// Inspect every recorded diagnostic at Warning level or above.
///
/// This reads diagnostics whether or not they have already been printed, so it
/// still sees the prover's own verification failures, which the prover reports
/// to its error writer before returning. Records carry no rendered `text`:
/// rendering is the reporting path's job and would print each diagnostic twice.
pub(crate) fn inspect_diagnostics(env: &GlobalEnv) -> Vec<DiagnosticRecord> {
    let mut records = Vec::new();
    env.inspect_diags(Severity::Warning, |diag| {
        let primary = diag
            .labels
            .iter()
            .find(|label| label.style == LabelStyle::Primary)
            .or_else(|| diag.labels.first());
        let (file, line, column) = match primary {
            Some(label) => {
                let loc = Loc::new(
                    label.file_id,
                    Span::new(label.range.start as u32, label.range.end as u32),
                );
                match env.get_file_and_location(&loc) {
                    Some((file, location)) => (
                        Some(file),
                        Some(location.line.to_usize() + 1),
                        Some(location.column.to_usize() + 1),
                    ),
                    None => (None, None, None),
                }
            },
            None => (None, None, None),
        };
        records.push(DiagnosticRecord {
            text: String::new(),
            headline: diag.message.clone(),
            file,
            line,
            column,
            label: primary.map(|label| label.message.clone()),
            notes: diag.notes.clone(),
            is_error: diag.severity >= Severity::Error,
        });
    });
    records
}

/// Collect all diagnostics at Warning level or above from a `GlobalEnv`.
pub(crate) fn collect_diagnostics(env: &GlobalEnv) -> Vec<DiagnosticRecord> {
    let mut records = Vec::new();
    env.report_diag_with_filter(
        |files, diag| {
            let mut buf = NoColor::new(Vec::new());
            let display_files = CwdRelativeFiles::new(files);
            emit(&mut buf, &Config::default(), &display_files, diag).expect("emit must not fail");
            let text = String::from_utf8(buf.into_inner()).unwrap_or_default();
            let primary = diag
                .labels
                .iter()
                .find(|label| label.style == LabelStyle::Primary)
                .or_else(|| diag.labels.first());
            let (file, line, column) = match primary {
                Some(label) => {
                    let location = files
                        .location(label.file_id, codespan::ByteIndex(label.range.start as u32))
                        .ok();
                    (
                        Some(files.name(label.file_id).to_string_lossy().to_string()),
                        location.map(|location| location.line.to_usize() + 1),
                        location.map(|location| location.column.to_usize() + 1),
                    )
                },
                None => (None, None, None),
            };
            records.push(DiagnosticRecord {
                text,
                headline: diag.message.clone(),
                column,
                label: primary.map(|label| label.message.clone()),
                notes: diag.notes.clone(),
                file,
                line,
                is_error: diag.severity >= Severity::Error,
            });
        },
        |d| d.severity >= Severity::Warning,
    );
    records
}

/// Render all diagnostics at Warning level or above from a `GlobalEnv`.
pub(crate) fn render_diagnostics(env: &GlobalEnv) -> Vec<String> {
    collect_diagnostics(env)
        .into_iter()
        .map(|record| record.text)
        .collect()
}

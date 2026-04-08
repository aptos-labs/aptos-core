// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::McpArgs;
use codespan_reporting::{
    diagnostic::Severity,
    term::{emit, termcolor::NoColor, Config},
};
use move_model::model::{FunId, GlobalEnv, ModuleId, QualifiedId};
use std::{collections::BTreeMap, path::Path};

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
        let named_addresses = args
            .named_addresses
            .iter()
            .map(|(name, addr)| (name.clone(), addr.into_inner()))
            .collect();
        let env = aptos_framework::build_model(
            args.dev_mode,
            // test_mode off: test code is handled separately by run_move_unit_tests.
            // verify_mode on: #[verify_only] specs are needed by the prover.
            false, // test_mode
            true,  // verify_mode
            path,
            named_addresses,
            args.target_filter.clone(),
            args.bytecode_version,
            None,
            Some(args.language_version),
            false,
            aptos_framework::extended_checks::get_all_attribute_names().clone(),
            args.experiments.clone(),
            true, // always build with bytecode
        )?;
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
        })
    }

    /// Access the compiled `GlobalEnv`.
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

/// Render all diagnostics at Warning level or above from a `GlobalEnv`.
pub(crate) fn render_diagnostics(env: &GlobalEnv) -> Vec<String> {
    let mut messages = Vec::new();
    env.report_diag_with_filter(
        |files, diag| {
            let mut buf = NoColor::new(Vec::new());
            emit(&mut buf, &Config::default(), files, diag).expect("emit must not fail");
            let text = String::from_utf8(buf.into_inner()).unwrap_or_default();
            messages.push(text);
        },
        |d| d.severity >= Severity::Warning,
    );
    messages
}

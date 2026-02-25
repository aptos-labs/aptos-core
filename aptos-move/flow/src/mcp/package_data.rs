// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::McpArgs;
use codespan_reporting::{
    diagnostic::Severity,
    term::{emit, termcolor::NoColor, Config},
};
use move_model::model::{FunId, GlobalEnv, ModuleId, QualifiedId};
use std::path::Path;

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

/// Compiled package data holding a `GlobalEnv` (the Move model).
pub(crate) struct PackageData {
    env: GlobalEnv,
    path: String,
    args: McpArgs,
    verified: Option<(VerifiedScope, bool, usize)>,
    /// Pre-rendered diagnostic messages from the last stage that produced them.
    diagnostics: Vec<String>,
    /// Label of the stage that produced the diagnostics (e.g. "checking", "compiling", "verifying").
    diagnostics_source: String,
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
            true,  // test_mode
            false, // verify_mode
            path,
            named_addresses,
            args.target_filter.clone(),
            args.bytecode_version,
            None,
            args.language_version,
            false,
            aptos_framework::extended_checks::get_all_attribute_names().clone(),
            args.experiments.clone(),
            false, // no bytecode needed for initial build
        )?;
        let diagnostics = render_diagnostics(&env);
        log_diagnostics(&diagnostics, "checking");
        Ok(Self {
            env,
            path: path.to_string_lossy().into_owned(),
            args: args.clone(),
            verified: None,
            diagnostics,
            diagnostics_source: "checking".to_string(),
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

    /// Returns true if any target module has compiled bytecode attached.
    pub(crate) fn has_bytecode(&self) -> bool {
        self.env
            .get_modules()
            .any(|m| m.is_target() && m.get_verified_module().is_some())
    }

    /// Rebuild the model with bytecode generation enabled (required by the prover).
    /// Resets the cached verification result.
    pub(crate) fn rebuild_with_bytecode(&mut self) -> anyhow::Result<()> {
        let named_addresses = self
            .args
            .named_addresses
            .iter()
            .map(|(name, addr)| (name.clone(), addr.into_inner()))
            .collect();
        self.env = aptos_framework::build_model(
            self.args.dev_mode,
            true,  // test_mode
            false, // verify_mode
            self.path.as_ref(),
            named_addresses,
            self.args.target_filter.clone(),
            self.args.bytecode_version,
            None,
            self.args.language_version,
            false,
            aptos_framework::extended_checks::get_all_attribute_names().clone(),
            self.args.experiments.clone(),
            true, // with bytecode for prover
        )?;
        self.verified = None;
        self.diagnostics = render_diagnostics(&self.env);
        log_diagnostics(&self.diagnostics, "compiling");
        self.diagnostics_source = "compiling".to_string();
        Ok(())
    }

    /// Returns the cached verification result, if any.
    pub(crate) fn verified(&self) -> Option<(VerifiedScope, bool, usize)> {
        self.verified.clone()
    }

    /// Store a verification result together with the vc timeout used.
    pub(crate) fn set_verified(&mut self, scope: VerifiedScope, success: bool, timeout: usize) {
        self.verified = Some((scope, success, timeout));
    }

    /// Returns stored diagnostic messages and the source stage that produced them.
    pub(crate) fn diagnostics(&self) -> (&[String], &str) {
        (&self.diagnostics, &self.diagnostics_source)
    }

    /// Override diagnostics with output from a later stage (e.g. verifying, inferring).
    pub(crate) fn set_diagnostics(&mut self, diagnostics: Vec<String>, source: &str) {
        log_diagnostics(&diagnostics, source);
        self.diagnostics = diagnostics;
        self.diagnostics_source = source.to_string();
    }
}

/// Log stored diagnostics at INFO level.
fn log_diagnostics(diagnostics: &[String], source: &str) {
    if diagnostics.is_empty() {
        log::info!("stored diagnostics ({}): none", source);
    } else {
        log::info!(
            "stored diagnostics ({}): {} message(s):\n{}",
            source,
            diagnostics.len(),
            diagnostics.join("\n")
        );
    }
}

/// Render all diagnostics at Warning level or above from a `GlobalEnv`.
fn render_diagnostics(env: &GlobalEnv) -> Vec<String> {
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

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod package_manifest;
mod package_query;
mod package_spec_infer;
mod package_status;
pub(crate) mod package_test;
mod package_verify;
pub(crate) mod replay_tracing;
mod replay_transaction;
mod spec_check;

use super::package_data::VerifiedScope;
use move_model::model::{GlobalEnv, VerificationScope};
use std::path::Path;

/// Load prover options for an MCP call.
///
/// When no `Prover.toml` is present, returns `Options::default()` — identical
/// to the pre-PR behavior.
///
/// When `Prover.toml` is present, parses it and copies a tiny allowlist of
/// fields onto a fresh default. Every other knob — executables, paths, output
/// flags, fan-out, coverage toggles — stays at its safe default regardless of
/// what the file says. Allowlist:
///
/// - `prover.borrow_natives`
///
/// These are the only fields the framework's own `Prover.toml` uses; adding
/// anything else here requires a deliberate review of the field's effect on
/// host resources and verification coverage.
pub(crate) fn load_sanitized_prover_options(
    package_path: &Path,
) -> Result<move_prover::cli::Options, String> {
    let mut opts = move_prover::cli::Options::default();
    // A timeout otherwise reports only that a budget was exhausted. The replay
    // costs a bounded solver run on the failure path and names the definitions
    // responsible, so Flow always asks for it. Sanitizing to the default
    // backend keeps the Z3 solver the analysis requires.
    opts.backend.timeout_analysis = true;
    let prover_toml = package_path.join("Prover.toml");
    if !prover_toml.exists() {
        return Ok(opts);
    }
    let from_toml = move_prover::cli::Options::create_from_toml_file(
        &prover_toml.to_string_lossy(),
    )
    .map_err(|e| {
        format!(
            "failed to parse Prover.toml at `{}`: {}",
            prover_toml.display(),
            e
        )
    })?;
    opts.prover.borrow_natives = from_toml.prover.borrow_natives;
    Ok(opts)
}

/// Extract the module-name portion of a filter/exclude entry.
///
/// The last `::` separates an optional `function` suffix from the module
/// (matches the prover's `-o` convention). A bare name is the module name
/// itself. Single source of truth — all `filter` / `exclude` parsing sites
/// must use this helper to stay in sync.
pub(crate) fn module_part_of(entry: &str) -> &str {
    entry.rfind("::").map_or(entry, |pos| &entry[..pos])
}

/// Resolve an optional list of exclusion strings into `Vec<VerificationScope>`.
///
/// Each entry follows the same format as a filter: `module_name::function_name`
/// becomes `VerificationScope::Only(...)`, and `module_name` becomes
/// `VerificationScope::OnlyModule(...)`.
pub(crate) fn resolve_excludes(excludes: Option<&[String]>) -> Vec<VerificationScope> {
    let excludes = match excludes {
        None | Some(&[]) => return vec![],
        Some(v) => v,
    };
    excludes
        .iter()
        .map(|entry| {
            if entry.contains("::") {
                VerificationScope::Only(entry.clone())
            } else {
                VerificationScope::OnlyModule(entry.clone())
            }
        })
        .collect()
}

/// Resolve an optional filter string into `(VerifiedScope, VerificationScope)`.
pub(crate) fn resolve_filter(
    env: &GlobalEnv,
    filter: Option<&str>,
) -> Result<(VerifiedScope, VerificationScope), rmcp::ErrorData> {
    let filter = match filter {
        None => return Ok((VerifiedScope::Package, VerificationScope::All)),
        Some(f) => f,
    };

    if let Some(pos) = filter.rfind("::") {
        // Function filter: "module::function"
        let module_part = module_part_of(filter);
        let func_part = &filter[pos + 2..];

        let module_sym = env.symbol_pool().make(module_part);
        let module = env
            .find_module_by_name(module_sym)
            .filter(|m| m.is_target())
            .ok_or_else(|| {
                rmcp::ErrorData::invalid_params(
                    format!(
                        "no module matching `{}` found in target modules",
                        module_part
                    ),
                    None,
                )
            })?;
        let func_sym = env.symbol_pool().make(func_part);
        let func = module.find_function(func_sym).ok_or_else(|| {
            rmcp::ErrorData::invalid_params(
                format!("no function matching `{}` found in target modules", filter),
                None,
            )
        })?;
        let qid = func.get_qualified_id();
        Ok((
            VerifiedScope::Function(qid),
            VerificationScope::Only(filter.to_string()),
        ))
    } else {
        // Module filter: "module_name"
        let module_sym = env.symbol_pool().make(filter);
        let module = env
            .find_module_by_name(module_sym)
            .filter(|m| m.is_target())
            .ok_or_else(|| {
                rmcp::ErrorData::invalid_params(
                    format!("no module matching `{}` found in target modules", filter),
                    None,
                )
            })?;
        Ok((
            VerifiedScope::Module(module.get_id()),
            VerificationScope::OnlyModule(filter.to_string()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A timeout otherwise reports only an exhausted budget, so every Flow
    /// prover invocation asks for the replay analysis.
    #[test]
    fn prover_options_request_timeout_analysis_without_a_prover_toml() {
        let package = tempfile::tempdir().unwrap();

        let options = load_sanitized_prover_options(package.path()).unwrap();

        assert!(options.backend.timeout_analysis);
    }

    #[test]
    fn prover_options_request_timeout_analysis_with_a_prover_toml() {
        let package = tempfile::tempdir().unwrap();
        fs::write(
            package.path().join("Prover.toml"),
            "[prover]\nborrow_natives = [\"borrow_mut\"]\n",
        )
        .unwrap();

        let options = load_sanitized_prover_options(package.path()).unwrap();

        assert!(options.backend.timeout_analysis);
        assert_eq!(
            vec!["borrow_mut".to_string()],
            options.prover.borrow_natives
        );
    }

    /// The analysis drives the solver directly and rejects a non-Z3 backend,
    /// so sanitization must not carry one over from the package.
    #[test]
    fn prover_options_keep_the_default_z3_backend() {
        let package = tempfile::tempdir().unwrap();
        fs::write(
            package.path().join("Prover.toml"),
            "[backend]\nuse_cvc5 = true\n",
        )
        .unwrap();

        let options = load_sanitized_prover_options(package.path()).unwrap();

        assert!(!options.backend.use_cvc5);
        assert!(options.backend.boogie_flags.is_empty());
    }
}

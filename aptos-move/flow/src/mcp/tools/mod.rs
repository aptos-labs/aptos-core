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

use super::package_data::VerifiedScope;
use move_model::model::{GlobalEnv, VerificationScope};
use std::path::Path;

/// Upper bound on package-supplied `shards`. Bounded so a malicious
/// `Prover.toml` can't expand a single MCP call into an unbounded shard fan-out.
const MAX_SHARDS: usize = 16;

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
/// - `backend.shards` (clamped to `[1, MAX_SHARDS]`)
/// - `prover.borrow_natives`
///
/// These are the only fields the framework's own `Prover.toml` uses; adding
/// anything else here requires a deliberate review of the field's effect on
/// host resources and verification coverage.
pub(crate) fn load_sanitized_prover_options(
    package_path: &Path,
) -> Result<move_prover::cli::Options, String> {
    let prover_toml = package_path.join("Prover.toml");
    if !prover_toml.exists() {
        return Ok(move_prover::cli::Options::default());
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
    let mut opts = move_prover::cli::Options::default();
    opts.backend.shards = from_toml.backend.shards.clamp(1, MAX_SHARDS);
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

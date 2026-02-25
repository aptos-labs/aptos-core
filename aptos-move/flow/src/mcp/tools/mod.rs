// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod package_manifest;
mod package_spec_infer;
mod package_status;
pub(crate) mod package_test;
mod package_verify;

use super::package_data::VerifiedScope;
use move_model::model::{GlobalEnv, VerificationScope};

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
        let module_part = &filter[..pos];
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

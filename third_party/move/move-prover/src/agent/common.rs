// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Shared helpers for AI-powered spec inference strategies.

use crate::{cli::Options, inference};
use codespan_reporting::term::termcolor::Buffer;
use std::time::Instant;

/// Check whether the source contains vacuous conditions (indicating missing loop invariants).
pub(crate) fn has_vacuous_conditions(source: &str) -> bool {
    source.contains("[inferred = vacuous]")
}

/// Validate Move source for banned patterns (text-only scan).
///
/// Checks:
/// - `pragma aborts_if_is_partial` (policy violation)
///
/// Returns `Ok(())` if clean, `Err(message)` with violation description.
pub(crate) fn validate_source(source: &str) -> Result<(), String> {
    if source.contains("aborts_if_is_partial") {
        Err("`pragma aborts_if_is_partial` is banned. \
             Abort specs must be complete — add the missing `aborts_if` conditions instead."
            .to_string())
    } else {
        Ok(())
    }
}

/// Check whether verification diagnostics indicate a timeout.
pub(crate) fn has_timeout(diagnostics: &str) -> bool {
    diagnostics.contains("out of resource")
        || diagnostics.contains("timed out")
        || diagnostics.contains("verification inconclusive")
}

/// Compile-check a Move source string. Returns `Ok(Ok(()))` on success,
/// or `Ok(Err(diagnostics))` on compilation failure.
pub(crate) fn compile_check(
    source: &str,
    original_options: &Options,
    experiments: &[String],
) -> anyhow::Result<Result<(), String>> {
    let temp_dir = tempfile::TempDir::new()?;
    let temp_source = temp_dir.path().join("compile_check.move");
    std::fs::write(&temp_source, source)?;

    let base_name = "compile_check.bpl";
    let output = temp_dir
        .path()
        .join(base_name)
        .to_str()
        .unwrap()
        .to_string();

    let mut check_options = Options::default();
    check_options.output_path = output;
    check_options.move_sources = vec![temp_source.to_string_lossy().to_string()];
    check_options.move_deps = original_options.move_deps.clone();
    check_options.move_named_address_values = original_options.move_named_address_values.clone();
    check_options.language_version = original_options.language_version;
    // Leave prover/backend at defaults — we only need compilation.

    let mut error_writer = Buffer::no_color();
    let result =
        crate::create_move_prover_v2_model(&mut error_writer, check_options, experiments.to_vec());
    let diags = String::from_utf8_lossy(&error_writer.into_inner()).to_string();
    match result {
        Ok(env) if !env.has_errors() && diags.is_empty() => Ok(Ok(())),
        Ok(_) => Ok(Err(diags)),
        Err(e) => Ok(Err(format!("{e}\n{diags}"))),
    }
}

/// Run WP-based spec inference and return the concatenated enriched source.
/// Returns `None` if no functions have inferred specs.
pub(crate) fn run_inference(
    options: Options,
    experiments: &[String],
) -> anyhow::Result<Option<String>> {
    let mut inf_options = options;
    inf_options.inference.inference = true;

    let now = Instant::now();
    let mut error_writer = Buffer::no_color();
    let env = crate::create_move_prover_v2_model(
        &mut error_writer,
        inf_options.clone(),
        experiments.to_vec(),
    );
    let mut env = match env {
        Ok(env) => env,
        Err(e) => {
            let diags = String::from_utf8_lossy(&error_writer.into_inner()).to_string();
            return Err(if diags.is_empty() {
                e
            } else {
                e.context(diags)
            });
        },
    };
    let pairs =
        match inference::run_inference_to_strings(&mut env, &mut error_writer, inf_options, now) {
            Ok(pairs) => pairs,
            Err(e) => {
                let diags = String::from_utf8_lossy(&error_writer.into_inner()).to_string();
                return Err(if diags.is_empty() {
                    e
                } else {
                    e.context(diags)
                });
            },
        };

    if pairs.is_empty() {
        return Ok(None);
    }
    Ok(Some(
        pairs
            .into_iter()
            .map(|(_, s)| s)
            .collect::<Vec<_>>()
            .join("\n"),
    ))
}

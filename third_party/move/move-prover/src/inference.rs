// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Inference driver for the Move Prover.
//!
//! This module runs the bytecode pipeline through `SpecInferenceProcessor` to infer
//! specifications (ensures, aborts_if, modifies) via weakest precondition analysis,
//! then prints the inferred specs to stdout or files. No Boogie backend is involved.
//!
//! # Command Line Usage
//!
//! Run inference instead of verification with the `-i` / `--inference` flag:
//!
//! ```shell
//! cargo run -p move-prover -- \
//!     -d third_party/move/move-stdlib/sources -a std=0x1 \
//!     -i <source-files>
//! ```
//!
//! Relevant options:
//!
//! | Flag | Description |
//! |------|-------------|
//! | `-i` / `--inference` | Enable inference mode (no Boogie/verification). |
//! | `--inference-output stdout` | Print enriched source to stdout (default). |
//! | `--inference-output file` | Write per-module `.inferred.move` files. |
//! | `--inference-output unified` | Emit a single unified output file. |
//! | `--inference-output-dir DIR` | Directory for `file`/`unified` output. |
//! | `--dump-bytecode` | Also dump the bytecode pipeline with WP annotations. |

use crate::cli::Options;
use codespan_reporting::term::termcolor::WriteColor;
#[allow(unused_imports)]
use log::{debug, info};
use move_model::{
    ast::SpecBlockTarget, emitln, model::GlobalEnv, pragmas::CONDITION_INFERRED_PROP,
    sourcifier::Sourcifier, symbol::Symbol,
};
use move_prover_bytecode_pipeline::pipeline_factory;
use move_stackless_bytecode::{
    function_target_pipeline::FunctionTargetsHolder, print_targets_with_annotations_for_test,
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Debug, Clone, Default, clap::Args)]
pub struct InferenceOptions {
    /// Run spec inference instead of verification.
    #[arg(short = 'i', long)]
    pub inference: bool,
    /// Output mode for inference results.
    #[arg(long, default_value_t, value_enum)]
    pub inference_output: InferenceOutput,
    /// Output directory for generated spec files (used with file output mode).
    #[arg(long)]
    pub inference_output_dir: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum InferenceOutput {
    #[default]
    Stdout,
    File,
    Unified,
    /// Suppress all output (used internally when the caller captures results directly).
    #[clap(skip)]
    None,
}

/// Run spec inference on the model and print inferred specs to stdout.
pub fn run_spec_inference_with_model<W: WriteColor>(
    env: &mut GlobalEnv,
    error_writer: &mut W,
    options: Options,
    start_time: Instant,
) -> anyhow::Result<()> {
    run_spec_inference_inner(env, error_writer, options, start_time, false)?;
    Ok(())
}

//e Like `run_spec_inference_with_model`, but also returns a bytecode dump
/// with WP annotations for debugging test baselines.
pub fn run_spec_inference_with_model_and_dump<W: WriteColor>(
    env: &mut GlobalEnv,
    error_writer: &mut W,
    options: Options,
    start_time: Instant,
) -> anyhow::Result<String> {
    Ok(run_spec_inference_inner(env, error_writer, options, start_time, true)?.unwrap_or_default())
}

/// Run the inference pipeline and return enriched source strings (for agent mode).
///
/// Unlike `run_spec_inference_with_model`, this suppresses stdout/file output
/// since the caller captures the result directly via `generate_unified_string`.
pub fn run_inference_to_strings<W: WriteColor>(
    env: &mut GlobalEnv,
    error_writer: &mut W,
    mut options: Options,
    start_time: Instant,
) -> anyhow::Result<Vec<(PathBuf, String)>> {
    options.inference.inference_output = InferenceOutput::None;
    run_spec_inference_inner(env, error_writer, options, start_time, false)?;
    Ok(generate_unified_string(env))
}

/// Inner function that optionally captures a bytecode dump.
fn run_spec_inference_inner<W: WriteColor>(
    env: &mut GlobalEnv,
    error_writer: &mut W,
    mut options: Options,
    start_time: Instant,
    capture_dump: bool,
) -> anyhow::Result<Option<String>> {
    let build_duration = start_time.elapsed();
    crate::check_errors(
        env,
        &options,
        error_writer,
        "exiting with model building errors",
    )?;

    // Enable inference in prover options and add as environment extension.
    options.prover.inference = true;
    env.set_extension(options.prover.clone());

    // Create function targets for all functions in the environment.
    let mut targets = FunctionTargetsHolder::default();
    let output_dir = Path::new(&options.output_path)
        .parent()
        .expect("expect the parent directory of the output path to exist");
    let output_prefix = options.move_sources.first().map_or("bytecode", |s| {
        Path::new(s).file_name().unwrap().to_str().unwrap()
    });

    for module_env in env.get_modules() {
        if module_env.is_target() {
            info!("preparing module {}", module_env.get_full_name_str());
        }
        for func_env in module_env.get_functions() {
            targets.add_target(&func_env)
        }
    }

    // Build and run the inference pipeline.
    let now = Instant::now();
    let pipeline = pipeline_factory::default_pipeline_with_options(&options.prover);

    if options.prover.dump_bytecode {
        let dump_file_base = output_dir
            .join(output_prefix)
            .into_os_string()
            .into_string()
            .unwrap();
        pipeline.run_with_dump(
            env,
            &mut targets,
            &dump_file_base,
            options.prover.dump_cfg,
            &|target| {
                target.register_annotation_formatters_for_test();
                target.register_annotation_formatter(Box::new(
                    move_prover_bytecode_pipeline::spec_inference::format_wp_annotation,
                ));
            },
            || true,
        )
    } else {
        pipeline.run(env, &mut targets);
    }

    // Capture bytecode dump if requested.
    let dump = if capture_dump {
        Some(print_targets_with_annotations_for_test(
            env,
            "inference pipeline",
            &targets,
            &|target| {
                target.register_annotation_formatters_for_test();
                target.register_annotation_formatter(Box::new(
                    move_prover_bytecode_pipeline::spec_inference::format_wp_annotation,
                ));
            },
            false,
        ))
    } else {
        None
    };

    let trafo_duration = now.elapsed();
    crate::check_errors(
        env,
        &options,
        error_writer,
        "exiting with bytecode transformation errors",
    )?;

    // Output inferred specs.
    match options.inference.inference_output {
        InferenceOutput::Stdout => {
            output_to_stdout(env);
        },
        InferenceOutput::File => {
            output_to_files(env, &options)?;
        },
        InferenceOutput::Unified => {
            output_unified(env, &options)?;
        },
        InferenceOutput::None => {},
    }

    info!(
        "{:.2}s build, {:.2}s inference, total {:.2}s",
        build_duration.as_secs_f64(),
        trafo_duration.as_secs_f64(),
        build_duration.as_secs_f64() + trafo_duration.as_secs_f64()
    );

    crate::check_errors(env, &options, error_writer, "exiting with inference errors")?;
    Ok(dump)
}

/// Output inferred specs to stdout (default mode).
fn output_to_stdout(env: &GlobalEnv) {
    let inferred_sym = env.symbol_pool().make(CONDITION_INFERRED_PROP);
    let sourcifier = Sourcifier::new(env, true);

    for module in env.get_modules() {
        if !module.is_target() {
            continue;
        }
        for fun in module.get_functions() {
            if fun.is_native() || fun.is_intrinsic() {
                continue;
            }
            let spec = fun.get_spec();
            let has_inferred = spec
                .conditions
                .iter()
                .any(|c| c.properties.contains_key(&inferred_sym));
            if has_inferred {
                sourcifier.print_fun(fun.get_qualified_id(), fun.get_def());
            }
        }
    }

    let result = sourcifier.result();
    if !result.is_empty() {
        println!("{}", result);
    }
}

/// Output inferred specs to per-module `.spec.move` files.
fn output_to_files(env: &GlobalEnv, options: &Options) -> anyhow::Result<()> {
    let inferred_sym = env.symbol_pool().make(CONDITION_INFERRED_PROP);
    for module in env.get_modules() {
        if !module.is_target() {
            continue;
        }

        // Check if this module has any functions with inferred specs.
        let has_any_inferred = module.get_functions().any(|fun| {
            if fun.is_native() || fun.is_intrinsic() {
                return false;
            }
            let spec = fun.get_spec();
            spec.conditions
                .iter()
                .any(|c| c.properties.contains_key(&inferred_sym))
        });
        if !has_any_inferred {
            continue;
        }

        // Determine output path.
        let source_path = PathBuf::from(module.get_source_path());
        let stem = source_path
            .file_stem()
            .expect("source file should have a stem");
        let output_path = if let Some(ref dir) = options.inference.inference_output_dir {
            PathBuf::from(dir).join(format!("{}.spec.move", stem.to_string_lossy()))
        } else {
            let source_dir = source_path
                .parent()
                .expect("source file should have a parent directory");
            source_dir.join(format!("{}.spec.move", stem.to_string_lossy()))
        };

        // Generate spec content for this module.
        let sourcifier = Sourcifier::new(env, true);
        emitln!(
            sourcifier.writer(),
            "spec {} {{",
            module.get_full_name_str()
        );
        sourcifier.writer().indent();

        for fun in module.get_functions() {
            if fun.is_native() || fun.is_intrinsic() {
                continue;
            }
            // Filter conditions to only inferred ones, print, then restore.
            let original_conditions = {
                let mut spec = fun.get_mut_spec();
                let original = std::mem::take(&mut spec.conditions);
                spec.conditions = original
                    .iter()
                    .filter(|c| c.properties.contains_key(&inferred_sym))
                    .cloned()
                    .collect();
                original
            };
            if !fun.get_spec().conditions.is_empty() {
                sourcifier.print_fun_spec(&fun);
            }
            // Restore original conditions.
            fun.get_mut_spec().conditions = original_conditions;
        }

        sourcifier.writer().unindent();
        emitln!(sourcifier.writer(), "}");

        let result = sourcifier.result();
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&output_path, &result)?;
        info!("wrote inferred specs to {}", output_path.display());
    }
    Ok(())
}

/// Generate enriched source strings: the original source with inferred spec blocks
/// injected inline after each function definition (or appended to existing spec blocks).
/// Returns a vec of (source_path, enriched_source) pairs.
pub fn generate_unified_string(env: &GlobalEnv) -> Vec<(PathBuf, String)> {
    let inferred_sym = env.symbol_pool().make(CONDITION_INFERRED_PROP);
    let mut results = Vec::new();

    for module in env.get_modules() {
        if !module.is_target() {
            continue;
        }

        // Check if this module has any functions with inferred specs.
        let has_any_inferred = module.get_functions().any(|fun| {
            if fun.is_native() || fun.is_intrinsic() {
                return false;
            }
            fun.get_spec()
                .conditions
                .iter()
                .any(|c| c.properties.contains_key(&inferred_sym))
        });
        if !has_any_inferred {
            continue;
        }

        // Read the original source.
        let file_id = module.get_loc().file_id();
        let source = env.get_file_source(file_id).to_string();

        // Collect insertions: (byte_offset, text_to_insert)
        let mut insertions: Vec<(usize, String)> = Vec::new();

        // Build a map of function id -> spec block info for existing standalone spec blocks.
        let spec_block_infos = module.get_spec_block_infos();
        let module_id = module.get_id();

        for fun in module.get_functions() {
            if fun.is_native() || fun.is_intrinsic() {
                continue;
            }
            // Check for inferred conditions without keeping the Ref alive,
            // since helper functions below need get_mut_spec().
            let has_inferred = fun
                .get_spec()
                .conditions
                .iter()
                .any(|c| c.properties.contains_key(&inferred_sym));
            if !has_inferred {
                continue;
            }

            let fun_id = fun.get_id();

            // Check if there's an existing standalone spec block for this function.
            let existing_spec_block = spec_block_infos.iter().find(|info| {
                matches!(&info.target, SpecBlockTarget::Function(mid, fid)
                    if *mid == module_id && *fid == fun_id)
            });

            if let Some(spec_info) = existing_spec_block {
                // Append inferred conditions inside the existing spec block,
                // just before the line containing the closing `}`.
                let block_end = spec_info.loc.span().end().to_usize();
                // Find the closing `}`, then find the start of its line
                // so we insert before the line with `}`.
                let brace_pos = source[..block_end]
                    .rfind('}')
                    .expect("spec block should have closing brace");
                // Insert before the newline preceding the `}` line's indentation.
                let insert_pos = source[..brace_pos]
                    .rfind('\n')
                    .map(|p| p + 1)
                    .unwrap_or(brace_pos);

                // Detect indentation from the spec block opening line.
                let block_start = spec_info.loc.span().start().to_usize();
                let indent = detect_indent(&source, block_start);
                let inner_indent = format!("{}    ", indent);

                // Collect the set of property keys from the original spec block
                // source text so we can distinguish user-written pragmas from
                // inferred ones.
                let original_prop_keys: std::collections::BTreeSet<Symbol> = {
                    let block_src = &source[spec_info.loc.span().start().to_usize()
                        ..spec_info.loc.span().end().to_usize()];
                    fun.get_spec()
                        .properties
                        .keys()
                        .filter(|k| {
                            let name = env.symbol_pool().string(**k);
                            block_src.contains(&format!("pragma {}", name.as_str()))
                        })
                        .copied()
                        .collect()
                };

                // Generate only the inferred condition lines.
                let cond_text =
                    generate_inferred_conditions(env, &fun, &inner_indent, &original_prop_keys);

                insertions.push((insert_pos, cond_text));
            } else {
                // No existing spec block: insert a full spec block after the function definition.
                let fun_end = fun.get_loc().span().end().to_usize();

                // Detect indentation from the function definition line.
                let fun_start = fun.get_loc().span().start().to_usize();
                let indent = detect_indent(&source, fun_start);

                // Generate a full spec block.
                let spec_text = generate_full_spec_block(env, &fun, &indent);

                insertions.push((fun_end, format!("\n{}", spec_text)));
            }
        }

        // Sort insertions by byte offset in reverse order so earlier offsets
        // aren't invalidated by prior insertions.
        insertions.sort_by(|a, b| b.0.cmp(&a.0));

        let mut result = source;
        for (offset, text) in &insertions {
            result.insert_str(*offset, text);
        }

        let source_path = PathBuf::from(module.get_source_path());
        results.push((source_path, result));
    }

    results
}

/// Output inferred specs as enriched source files: the original source with
/// inferred spec blocks injected inline after each function definition (or
/// appended to existing spec blocks).
fn output_unified(env: &GlobalEnv, options: &Options) -> anyhow::Result<()> {
    let results = generate_unified_string(env);
    for (source_path, result) in results {
        let stem = source_path
            .file_stem()
            .expect("source file should have a stem");
        let output_path = if let Some(ref dir) = options.inference.inference_output_dir {
            PathBuf::from(dir).join(format!("{}.enriched.move", stem.to_string_lossy()))
        } else {
            let source_dir = source_path
                .parent()
                .expect("source file should have a parent directory");
            source_dir.join(format!("{}.enriched.move", stem.to_string_lossy()))
        };

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&output_path, &result)?;
        info!("wrote enriched source to {}", output_path.display());
    }
    Ok(())
}

/// Detect the leading whitespace of the line containing the given byte offset.
fn detect_indent(source: &str, byte_offset: usize) -> String {
    // Find the start of the line containing this offset.
    let line_start = source[..byte_offset]
        .rfind('\n')
        .map(|p| p + 1)
        .unwrap_or(0);
    // Extract leading whitespace.
    let line = &source[line_start..];
    let indent_len = line.len() - line.trim_start().len();
    line[..indent_len].to_string()
}

/// Generate only the inferred condition lines (for appending inside an existing spec block).
/// Also includes any new pragmas added by inference (e.g. `pragma verify = false`).
fn generate_inferred_conditions(
    env: &GlobalEnv,
    fun: &move_model::model::FunctionEnv,
    indent: &str,
    original_property_keys: &std::collections::BTreeSet<Symbol>,
) -> String {
    let inferred_sym = env.symbol_pool().make(CONDITION_INFERRED_PROP);
    let sourcifier = Sourcifier::new(env, true);

    // Filter to only inferred conditions and new properties, print, then restore.
    let (original_conditions, original_properties) = {
        let mut spec = fun.get_mut_spec();
        let orig_conds = std::mem::take(&mut spec.conditions);
        spec.conditions = orig_conds
            .iter()
            .filter(|c| c.properties.contains_key(&inferred_sym))
            .cloned()
            .collect();
        // Keep only properties that were added by inference (not in original spec).
        let orig_props = std::mem::take(&mut spec.properties);
        spec.properties = orig_props
            .iter()
            .filter(|(k, _)| !original_property_keys.contains(k))
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        (orig_conds, orig_props)
    };

    sourcifier.print_fun_spec(fun);

    // Restore original conditions and properties.
    {
        let mut spec = fun.get_mut_spec();
        spec.conditions = original_conditions;
        spec.properties = original_properties;
    }

    let raw = sourcifier.result();

    // Extract the pragma and condition lines from the generated spec block.
    // The format is: "\nspec name(...) {\n    <lines>\n}\n"
    let mut condition_lines = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("spec ") || trimmed == "{" || trimmed == "}" {
            continue;
        }
        condition_lines.push(format!("{}{}", indent, trimmed));
    }

    if condition_lines.is_empty() {
        return String::new();
    }

    format!("{}\n", condition_lines.join("\n"))
}

/// Generate a full `spec fn_name(...) { ... }` block for a function.
fn generate_full_spec_block(
    env: &GlobalEnv,
    fun: &move_model::model::FunctionEnv,
    indent: &str,
) -> String {
    let inferred_sym = env.symbol_pool().make(CONDITION_INFERRED_PROP);
    let sourcifier = Sourcifier::new(env, true);

    // Filter to only inferred conditions, print, then restore.
    let original_conditions = {
        let mut spec = fun.get_mut_spec();
        let original = std::mem::take(&mut spec.conditions);
        spec.conditions = original
            .iter()
            .filter(|c| c.properties.contains_key(&inferred_sym))
            .cloned()
            .collect();
        original
    };

    sourcifier.print_fun_spec(fun);

    // Restore original conditions.
    fun.get_mut_spec().conditions = original_conditions;

    let raw = sourcifier.result();

    // Re-indent every non-blank line: the Sourcifier generates lines with its
    // own base indentation (none for spec header/footer, 4 spaces for body).
    // We prefix every non-blank line with the target indent to match the
    // surrounding function definition.
    let mut result_lines = Vec::new();
    for line in raw.lines() {
        if line.trim().is_empty() {
            continue;
        }
        result_lines.push(format!("{}{}", indent, line));
    }

    format!("{}\n", result_lines.join("\n"))
}

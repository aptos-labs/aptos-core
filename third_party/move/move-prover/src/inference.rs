// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
    ast::SpecBlockTarget, emitln, model::GlobalEnv, sourcifier::Sourcifier, symbol::Symbol,
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

#[derive(Debug, Clone, clap::Args)]
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
    /// File suffix for unified output mode (default: "enriched.move").
    #[arg(long, default_value = "enriched.move")]
    pub inference_unified_suffix: String,
}

impl Default for InferenceOptions {
    fn default() -> Self {
        use clap::{Command, FromArgMatches};
        let cmd = <Self as clap::Args>::augment_args(Command::new(""));
        let matches = cmd.get_matches_from(std::iter::empty::<String>());
        <Self as FromArgMatches>::from_arg_matches(&matches).unwrap()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum InferenceOutput {
    #[default]
    Stdout,
    File,
    Unified,
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
            if !func_env.is_test_only() {
                targets.add_target(&func_env)
            }
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
    let inferred_sym = env.symbol_pool().make("inferred");
    match options.inference.inference_output {
        InferenceOutput::Stdout => {
            output_to_stdout(env, inferred_sym);
        },
        InferenceOutput::File => {
            output_to_files(env, inferred_sym, &options)?;
        },
        InferenceOutput::Unified => {
            output_unified(env, inferred_sym, &options)?;
        },
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
fn output_to_stdout(env: &GlobalEnv, inferred_sym: Symbol) {
    let sourcifier = Sourcifier::new(env, true);

    for module in env.get_modules() {
        if !module.is_target() {
            continue;
        }
        for fun in module.get_functions() {
            if fun.is_native() || fun.is_intrinsic() || fun.is_test_only() {
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
///
/// If a `.spec.move` file already exists and was compiled as part of the module,
/// inferred conditions are merged into it (appended to existing spec blocks or
/// inserted as new blocks) — mirroring the merge logic in `output_unified`.
/// Otherwise, a fresh file is generated from scratch.
fn output_to_files(env: &GlobalEnv, inferred_sym: Symbol, options: &Options) -> anyhow::Result<()> {
    for module in env.get_modules() {
        if !module.is_target() {
            continue;
        }

        // Check if this module has any functions with inferred specs.
        let has_any_inferred = module.get_functions().any(|fun| {
            if fun.is_native() || fun.is_intrinsic() || fun.is_test_only() {
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

        // Check if an existing .spec.move was compiled as part of this module
        // by looking for its path among the env's source files.
        let spec_path_str = output_path.to_string_lossy();
        let spec_file_id = env
            .get_source_file_ids()
            .into_iter()
            .find(|&fid| env.get_file(fid).to_string_lossy() == spec_path_str);

        let result = if let Some(spec_fid) = spec_file_id {
            // Merge into the existing .spec.move file, same strategy as output_unified.
            let source = env.get_file_source(spec_fid).to_string();
            let spec_block_infos = module.get_spec_block_infos();
            let module_id = module.get_id();

            let mut insertions: Vec<(usize, String)> = Vec::new();

            // Find the position before the last `}` in the spec file — this is
            // the closing brace of the outer `spec module_name { }` block and
            // serves as the insertion point for new spec blocks.
            let module_close_insert_pos = source.rfind('}').and_then(|brace_pos| {
                source[..brace_pos]
                    .rfind('\n')
                    .map(|p| p + 1)
                    .or(Some(brace_pos))
            });

            for fun in module.get_functions() {
                if fun.is_native() || fun.is_intrinsic() || fun.is_test_only() {
                    continue;
                }
                let has_inferred = {
                    let spec = fun.get_spec();
                    spec.conditions
                        .iter()
                        .any(|c| c.properties.contains_key(&inferred_sym))
                };
                if !has_inferred {
                    continue;
                }

                let fun_id = fun.get_id();

                // Look for an existing spec block for this function in the spec file.
                let existing_spec_block = spec_block_infos.iter().find(|info| {
                    info.loc.file_id() == spec_fid
                        && matches!(&info.target, SpecBlockTarget::Function(mid, fid)
                            if *mid == module_id && *fid == fun_id)
                });

                if let Some(spec_info) = existing_spec_block {
                    // Append inferred conditions inside the existing spec block,
                    // same logic as output_unified.
                    let block_end = spec_info.loc.span().end().to_usize();
                    let block_start = spec_info.loc.span().start().to_usize();
                    let brace_pos = source[..block_end]
                        .rfind('}')
                        .expect("spec block should have closing brace");

                    let open_brace_pos = source[block_start..block_end]
                        .find('{')
                        .map(|p| block_start + p)
                        .expect("spec block should have opening brace");
                    let is_single_line = !source[open_brace_pos..brace_pos].contains('\n');

                    let insert_pos = if is_single_line {
                        brace_pos
                    } else {
                        source[..brace_pos]
                            .rfind('\n')
                            .map(|p| p + 1)
                            .unwrap_or(brace_pos)
                    };
                    let indent = detect_indent(&source, block_start);
                    let inner_indent = format!("{}    ", indent);

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

                    let (use_text, cond_text) = generate_inferred_conditions(
                        env,
                        &fun,
                        inferred_sym,
                        &inner_indent,
                        &original_prop_keys,
                    );

                    let final_text = if is_single_line && !cond_text.is_empty() {
                        format!("\n{}{}{}", use_text, cond_text, indent)
                    } else {
                        cond_text
                    };

                    insertions.push((insert_pos, final_text));

                    if !is_single_line && !use_text.is_empty() {
                        let use_insert_pos = source[open_brace_pos..]
                            .find('\n')
                            .map(|p| open_brace_pos + p + 1)
                            .unwrap_or(open_brace_pos + 1);
                        insertions.push((use_insert_pos, use_text));
                    }
                } else if let Some(insert_pos) = module_close_insert_pos {
                    // No existing spec block for this function — insert a full block
                    // before the closing `}` of the outer `spec module { }` block.
                    let indent = "    ";
                    let spec_text = generate_full_spec_block(env, &fun, inferred_sym, indent);
                    insertions.push((insert_pos, format!("{}\n", spec_text)));
                }
            }

            // Sort insertions in reverse byte-offset order so earlier offsets
            // aren't invalidated by prior insertions. For equal offsets (e.g.
            // multiple new spec blocks all targeting `module_close_insert_pos`),
            // stable sort preserves declaration order — concatenate them into a
            // single insertion so `insert_str` produces the correct order.
            insertions.sort_by(|a, b| b.0.cmp(&a.0));
            let mut merged = source;
            let mut i = 0;
            while i < insertions.len() {
                let offset = insertions[i].0;
                let mut combined = String::new();
                while i < insertions.len() && insertions[i].0 == offset {
                    combined.push_str(&insertions[i].1);
                    i += 1;
                }
                merged.insert_str(offset, &combined);
            }
            merged
        } else {
            // No existing spec file — generate from scratch.
            generate_fresh_spec_file(env, &module, inferred_sym)
        };

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&output_path, &result)?;
        info!("wrote inferred specs to {}", output_path.display());
    }
    Ok(())
}

/// Generate a fresh `.spec.move` file from scratch (no existing file to merge into).
fn generate_fresh_spec_file(
    env: &GlobalEnv,
    module: &move_model::model::ModuleEnv,
    inferred_sym: Symbol,
) -> String {
    let sourcifier = Sourcifier::new(env, true);
    emitln!(
        sourcifier.writer(),
        "spec {} {{",
        module.get_full_name_str()
    );
    sourcifier.writer().indent();

    for fun in module.get_functions() {
        if fun.is_native() || fun.is_intrinsic() || fun.is_test_only() {
            continue;
        }
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
        fun.get_mut_spec().conditions = original_conditions;
    }

    sourcifier.writer().unindent();
    emitln!(sourcifier.writer(), "}");
    sourcifier.result()
}

/// Output inferred specs as enriched source files: the original source with
/// inferred spec blocks injected inline after each function definition (or
/// appended to existing spec blocks).
fn output_unified(env: &GlobalEnv, inferred_sym: Symbol, options: &Options) -> anyhow::Result<()> {
    for module in env.get_modules() {
        if !module.is_target() {
            continue;
        }

        // Check if this module has any functions with inferred specs.
        let has_any_inferred = module.get_functions().any(|fun| {
            if fun.is_native() || fun.is_intrinsic() || fun.is_test_only() {
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

        // Read the original source.
        let file_id = module.get_loc().file_id();
        let source = env.get_file_source(file_id).to_string();

        // Collect insertions: (byte_offset, text_to_insert)
        let mut insertions: Vec<(usize, String)> = Vec::new();

        // Build a map of function id -> spec block info for existing standalone spec blocks.
        let spec_block_infos = module.get_spec_block_infos();
        let module_id = module.get_id();

        for fun in module.get_functions() {
            if fun.is_native() || fun.is_intrinsic() || fun.is_test_only() {
                continue;
            }
            // Check for inferred conditions/frame_spec without keeping the Ref alive,
            // since helper functions below need get_mut_spec().
            let has_inferred = {
                let spec = fun.get_spec();
                spec.conditions
                    .iter()
                    .any(|c| c.properties.contains_key(&inferred_sym))
            };
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
                let block_start = spec_info.loc.span().start().to_usize();
                // Find the closing `}`, then find the start of its line
                // so we insert before the line with `}`.
                let brace_pos = source[..block_end]
                    .rfind('}')
                    .expect("spec block should have closing brace");

                // Find the opening `{` to detect single-line blocks like `spec foo {}`.
                let open_brace_pos = source[block_start..block_end]
                    .find('{')
                    .map(|p| block_start + p)
                    .expect("spec block should have opening brace");
                let is_single_line = !source[open_brace_pos..brace_pos].contains('\n');

                let insert_pos = if is_single_line {
                    // For single-line/empty blocks, insert right before the `}`.
                    brace_pos
                } else {
                    // For multi-line blocks, insert before the line containing `}`.
                    source[..brace_pos]
                        .rfind('\n')
                        .map(|p| p + 1)
                        .unwrap_or(brace_pos)
                };
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
                // Returns (use_decls, conditions) separately because `use` must
                // appear at the top of a spec block, before existing conditions.
                let (use_text, cond_text) = generate_inferred_conditions(
                    env,
                    &fun,
                    inferred_sym,
                    &inner_indent,
                    &original_prop_keys,
                );
                // Filter out spec-level `use` declarations for modules already
                // imported at module level (e.g. the original source has
                // `use aptos_framework::object::{Self, …}` so we don't need
                // `use 0x1::object;` inside the spec block).
                let use_text = filter_redundant_uses(&source, &use_text);

                // For single-line blocks, wrap the conditions so the block expands:
                //   `spec foo {}` -> `spec foo {\n    ...\n}`
                let final_text = if is_single_line && !cond_text.is_empty() {
                    format!("\n{}{}{}", use_text, cond_text, indent)
                } else {
                    cond_text
                };

                insertions.push((insert_pos, final_text));

                // Insert `use` declarations right after the opening `{` so they
                // appear before existing user conditions.
                if !is_single_line && !use_text.is_empty() {
                    // Find the end of the line containing `{`.
                    let use_insert_pos = source[open_brace_pos..]
                        .find('\n')
                        .map(|p| open_brace_pos + p + 1)
                        .unwrap_or(open_brace_pos + 1);
                    insertions.push((use_insert_pos, use_text));
                }
            } else {
                // No existing spec block: insert a full spec block after the function definition.
                let fun_end = fun.get_loc().span().end().to_usize();

                // Detect indentation from the function definition line.
                let fun_start = fun.get_loc().span().start().to_usize();
                let indent = detect_indent(&source, fun_start);

                // Generate a full spec block.
                let spec_text = generate_full_spec_block(env, &fun, inferred_sym, &indent);

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

        // Determine output path.
        let source_path = PathBuf::from(module.get_source_path());
        let stem = source_path
            .file_stem()
            .expect("source file should have a stem");
        let suffix = &options.inference.inference_unified_suffix;
        let output_path = if let Some(ref dir) = options.inference.inference_output_dir {
            PathBuf::from(dir).join(format!("{}.{}", stem.to_string_lossy(), suffix))
        } else {
            let source_dir = source_path
                .parent()
                .expect("source file should have a parent directory");
            source_dir.join(format!("{}.{}", stem.to_string_lossy(), suffix))
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
/// Filter out `use <addr>::<module>;` lines from `use_text` when the module
/// name is already importable from a module-level `use` in the source.
/// Detects `use <anything>::<module>::{Self, …}`, `use <anything>::<module>;`,
/// and `use <anything>::<module> as <alias>;`.
fn filter_redundant_uses(source: &str, use_text: &str) -> String {
    if use_text.is_empty() {
        return String::new();
    }
    let filtered: Vec<&str> = use_text
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            // Extract module name from `use <addr>::<module>;`
            if let Some(rest) = trimmed.strip_prefix("use ") {
                if let Some(module_name) = rest.trim_end_matches(';').rsplit("::").next() {
                    // Check if the original source already imports this module
                    // (with `Self` or as a bare module).
                    let has_self_import = source.contains(&format!("::{}::{{", module_name))
                        || source.contains(&format!("::{};", module_name))
                        || source.contains(&format!("::{}  as ", module_name));
                    return !has_self_import;
                }
            }
            true
        })
        .collect();
    if filtered.is_empty() {
        String::new()
    } else {
        format!("{}\n", filtered.join("\n"))
    }
}

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
    inferred_sym: Symbol,
    indent: &str,
    original_property_keys: &std::collections::BTreeSet<Symbol>,
) -> (String, String) {
    let sourcifier = Sourcifier::new(env, true);

    // Filter to only inferred conditions, properties, and frame_spec; print; then restore.
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
    // Separate `use` declarations from other lines: `use` must appear at the
    // top of a spec block, so when appending to an existing block they need
    // to be inserted right after the opening `{`, not before the closing `}`.
    //
    // Preserve relative indentation: the sourcifier produces properly indented
    // output (e.g. block content indented inside `{ }`). Strip only the base
    // indent (the spec block's content indent level) so deeper lines keep their
    // extra indentation.
    let is_content_line = |trimmed: &str| {
        !trimmed.is_empty() && !trimmed.starts_with("spec ") && trimmed != "{" && trimmed != "}"
    };
    let base_indent_len = raw
        .lines()
        .filter(|line| is_content_line(line.trim()))
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);
    let mut use_lines = Vec::new();
    let mut condition_lines = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if !is_content_line(trimmed) {
            continue;
        }
        // Strip the base indent, preserving relative indentation for
        // continuation lines (e.g. inside `{ let ...; expr }` blocks).
        let stripped = if line.len() > base_indent_len {
            &line[base_indent_len..]
        } else {
            trimmed
        };
        if trimmed.starts_with("use ") {
            use_lines.push(format!("{}{}", indent, stripped));
        } else {
            condition_lines.push(format!("{}{}", indent, stripped));
        }
    }

    if use_lines.is_empty() && condition_lines.is_empty() {
        return (String::new(), String::new());
    }

    let uses = if use_lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", use_lines.join("\n"))
    };
    let conds = if condition_lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", condition_lines.join("\n"))
    };
    (uses, conds)
}

/// Generate a full `spec fn_name(...) { ... }` block for a function.
fn generate_full_spec_block(
    env: &GlobalEnv,
    fun: &move_model::model::FunctionEnv,
    inferred_sym: Symbol,
    indent: &str,
) -> String {
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

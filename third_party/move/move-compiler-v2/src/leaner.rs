// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lean source adapter for compiler-v2's generic XIR reader.
//!
//! A `.lean` source emits one versioned deployable XIR module. This module
//! owns source discovery and Lean process orchestration; schema validation,
//! model loading, and stackless translation live in [`crate::xir`].

pub(crate) use crate::xir::import_sources;
use crate::{xir::XirSource, Options};
use anyhow::{bail, Context, Result};
use codespan::Span;
use codespan_reporting::diagnostic::Severity;
use move_command_line_common::files::{extension_equals, find_filenames, FileHash, LEAN_EXTENSION};
use move_model::model::{GlobalEnv, Loc};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
    sync::{Condvar, Mutex},
};

const MAX_CONCURRENT_LEAN_ELABORATIONS: usize = 2;

// Lean opens enough library artifacts during startup that launching one
// process per parallel transactional test can exceed the host descriptor
// limit.
static ACTIVE_LEAN_ELABORATIONS: Mutex<usize> = Mutex::new(0);
static LEAN_ELABORATION_AVAILABLE: Condvar = Condvar::new();

struct LeanElaborationPermit;

/// A position emitted by `lean --json`.
///
/// Lean line numbers are one-based while columns are zero-based Unicode scalar
/// offsets. Move diagnostics use zero-based UTF-8 byte offsets, so positions
/// are translated against the exact source passed to Lean.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LeanPosition {
    line: usize,
    column: usize,
}

#[derive(Debug, Eq, PartialEq)]
struct LeanDiagnostic {
    message: String,
    severity: Severity,
    start: usize,
    end: usize,
}

struct SourceDiagnostics {
    path: PathBuf,
    text: String,
    is_target: bool,
    diagnostics: Vec<LeanDiagnostic>,
}

pub(crate) struct LeanElaboration {
    pub(crate) modules: Vec<XirSource>,
    source_diagnostics: Vec<SourceDiagnostics>,
}

impl LeanElaborationPermit {
    fn acquire() -> Self {
        let mut active = ACTIVE_LEAN_ELABORATIONS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        while *active >= MAX_CONCURRENT_LEAN_ELABORATIONS {
            active = LEAN_ELABORATION_AVAILABLE
                .wait(active)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        *active += 1;
        Self
    }
}

impl Drop for LeanElaborationPermit {
    fn drop(&mut self) {
        let mut active = ACTIVE_LEAN_ELABORATIONS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *active -= 1;
        drop(active);
        LEAN_ELABORATION_AVAILABLE.notify_one();
    }
}

fn parse_position(value: &Value) -> Option<LeanPosition> {
    Some(LeanPosition {
        line: value.get("line")?.as_u64()?.try_into().ok()?,
        column: value.get("column")?.as_u64()?.try_into().ok()?,
    })
}

fn position_to_byte_offset(source: &str, position: LeanPosition) -> Option<usize> {
    let line = position.line.checked_sub(1)?;
    let line_start = if line == 0 {
        0
    } else {
        source
            .match_indices('\n')
            .nth(line - 1)
            .map(|(offset, _)| offset + 1)?
    };
    let line_text = source[line_start..]
        .split_once('\n')
        .map_or_else(|| &source[line_start..], |(line_text, _)| line_text);
    let column_offset = if position.column == line_text.chars().count() {
        line_text.len()
    } else {
        line_text
            .char_indices()
            .nth(position.column)
            .map(|(offset, _)| offset)?
    };
    Some(line_start + column_offset)
}

fn source_mentions_name(source: &str, name: &str) -> bool {
    source
        .split(|character: char| {
            !(character.is_alphanumeric() || character == '_' || character == '\'')
        })
        .any(|word| word == name)
}

fn is_internal_lean_name(name: &str) -> bool {
    if name == "sorryAx" || name.starts_with("Lean.Name.anonymous.") {
        return true;
    }
    let final_component = name.rsplit('.').next().unwrap_or(name);
    final_component.starts_with("_move")
        || final_component.starts_with("_leaner")
        || final_component.starts_with('$')
}

/// Remove implementation-generated identifiers from diagnostics at the
/// common Leaner boundary. User-authored identifiers with the same spelling
/// remain printable when they occur in the source.
fn sanitize_lean_diagnostic_message(source: &str, message: &str) -> String {
    let message = message.replace("'sorryAx'", "an unresolved expression");
    let mut sanitized = String::with_capacity(message.len());
    let mut remaining = message.as_str();
    while let Some(open) = remaining.find('`') {
        sanitized.push_str(&remaining[..open]);
        let after_open = &remaining[open + 1..];
        let Some(close) = after_open.find('`') else {
            sanitized.push_str(&remaining[open..]);
            return sanitized;
        };
        let name = &after_open[..close];
        let after_name = &after_open[close + 1..];
        if is_internal_lean_name(name) && !source_mentions_name(source, name) {
            const CONFLICT_PREFIX: &str = " (conflicts with ";
            if sanitized.ends_with(CONFLICT_PREFIX) && after_name.starts_with(')') {
                sanitized.truncate(sanitized.len() - CONFLICT_PREFIX.len());
                remaining = &after_name[1..];
            } else {
                if sanitized.ends_with(' ') {
                    sanitized.pop();
                }
                remaining = after_name;
            }
        } else {
            sanitized.push('`');
            sanitized.push_str(name);
            sanitized.push('`');
            remaining = after_name;
        }
    }
    sanitized.push_str(remaining);
    sanitized
}

fn parse_lean_diagnostics(source: &str, output: &[u8]) -> Vec<LeanDiagnostic> {
    String::from_utf8_lossy(output)
        .lines()
        .filter_map(|line| {
            let value: Value = serde_json::from_str(line).ok()?;
            let severity = match value.get("severity")?.as_str()? {
                "error" => Severity::Error,
                "warning" => Severity::Warning,
                // Information messages such as `wrote Leaner XIR` are process
                // progress, not compiler diagnostics.
                _ => return None,
            };
            let start_position = parse_position(value.get("pos")?)?;
            let mut end_position = value
                .get("endPos")
                .and_then(parse_position)
                .unwrap_or(start_position);
            // Match Lean's language-server range policy. Unless explicitly
            // requested otherwise, a multi-line syntax node highlights only
            // its first line.
            if !value
                .get("keepFullRange")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                && end_position.line > start_position.line
            {
                end_position = LeanPosition {
                    line: start_position.line + 1,
                    column: 0,
                };
            }
            let start = position_to_byte_offset(source, start_position)?;
            let end = position_to_byte_offset(source, end_position)?;
            Some(LeanDiagnostic {
                message: sanitize_lean_diagnostic_message(source, value.get("data")?.as_str()?),
                severity,
                start,
                end: end.max(start),
            })
        })
        .collect()
}

pub(crate) struct LeanSource {
    path: String,
    is_target: bool,
}

pub(crate) fn extract_sources(options: &mut Options) -> Result<Vec<LeanSource>> {
    let mut lean_sources = BTreeMap::new();
    extract_from(&mut options.sources, &mut lean_sources, true)?;
    extract_from(&mut options.sources_deps, &mut lean_sources, false)?;
    Ok(lean_sources
        .into_iter()
        .map(|(path, is_target)| LeanSource { path, is_target })
        .collect())
}

fn add_source(
    lean_sources: &mut BTreeMap<String, bool>,
    source: String,
    is_target: bool,
) -> Result<()> {
    let source = fs::canonicalize(&source)
        .with_context(|| format!("unable to resolve Lean source `{source}`"))?
        .to_string_lossy()
        .into_owned();
    lean_sources
        .entry(source)
        .and_modify(|target| *target |= is_target)
        .or_insert(is_target);
    Ok(())
}

fn extract_from(
    inputs: &mut Vec<String>,
    lean_sources: &mut BTreeMap<String, bool>,
    is_target: bool,
) -> Result<()> {
    let mut move_inputs = vec![];
    for source in inputs.iter() {
        let path = Path::new(source);
        if path.is_file() && extension_equals(path, LEAN_EXTENSION) {
            add_source(lean_sources, source.clone(), is_target)?;
        } else {
            move_inputs.push(source.clone());
            if path.is_dir() {
                for source in
                    find_filenames(&[source], |path| extension_equals(path, LEAN_EXTENSION))?
                {
                    add_source(lean_sources, source, is_target)?;
                }
            }
        }
    }
    *inputs = move_inputs;
    Ok(())
}

pub(crate) fn elaborate_sources(sources: &[LeanSource]) -> Result<LeanElaboration> {
    let mut modules = vec![];
    let mut source_diagnostics = vec![];
    for source in sources {
        let (module, diagnostics) = elaborate_source(&source.path, source.is_target)?;
        if let Some(module) = module {
            modules.push(module);
        }
        if !diagnostics.diagnostics.is_empty() {
            source_diagnostics.push(diagnostics);
        }
    }
    Ok(LeanElaboration {
        modules,
        source_diagnostics,
    })
}

fn elaborate_source(
    source: &str,
    is_target: bool,
) -> Result<(Option<XirSource>, SourceDiagnostics)> {
    let source_path = fs::canonicalize(source)
        .with_context(|| format!("unable to resolve Lean source `{source}`"))?;
    let source_text = fs::read_to_string(&source_path)?;
    // Keep the output path in a private, randomly named directory. The Lean
    // exporter requires that its output does not already exist, and a unique
    // directory prevents another local user from replacing the emitted XIR
    // between elaboration and reading it here.
    let xir_dir = tempfile::Builder::new()
        .prefix("leaner-")
        .tempdir()
        .context("unable to create private XIR output directory")?;
    let xir_path = xir_dir.path().join("module.xir.json");
    let leaner_root = std::env::var_os("LEANER_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../lean/move"));
    let _elaboration_permit = LeanElaborationPermit::acquire();
    let output = Command::new("lake")
        .args(["env", "lean", "--json"])
        .arg(&source_path)
        .current_dir(&leaner_root)
        .env("LEANER_XIR_OUTPUT", &xir_path)
        .output()
        .with_context(|| {
            format!(
                "failed to launch Lean for `{}` (set LEANER_ROOT if the Lean project is elsewhere)",
                source_path.display()
            )
        })?;
    let diagnostics = SourceDiagnostics {
        path: source_path.clone(),
        text: source_text.clone(),
        is_target,
        diagnostics: parse_lean_diagnostics(&source_text, &output.stdout),
    };
    if !output.status.success() {
        if diagnostics
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
        {
            return Ok((None, diagnostics));
        }
        bail!(
            "Leaner elaboration failed for `{}` without a structured error diagnostic:\n{}{}",
            source_path.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let json = fs::read_to_string(&xir_path).with_context(|| {
        format!(
            "`{}` did not emit XIR; add a `move_module` declaration or `#emit_leaner_xir`",
            source_path.display()
        )
    });
    let module = crate::xir::parse_source_with_target(source_path, source_text, &json?, is_target)?;
    Ok((Some(module), diagnostics))
}

pub(crate) fn add_diagnostics(env: &mut GlobalEnv, elaboration: &LeanElaboration) {
    for source in &elaboration.source_diagnostics {
        let file_id = env.add_source(
            FileHash::new(&source.text),
            Rc::new(BTreeMap::new()),
            &source.path.to_string_lossy(),
            &source.text,
            source.is_target,
            source.is_target,
        );
        for diagnostic in &source.diagnostics {
            let start = u32::try_from(diagnostic.start).unwrap_or(u32::MAX);
            let end = u32::try_from(diagnostic.end).unwrap_or(u32::MAX);
            env.diag(
                diagnostic.severity,
                &Loc::new(file_id, Span::new(start, end)),
                &diagnostic.message,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_binary_format::access::ModuleAccess;
    use std::collections::BTreeSet;

    fn lake_available() -> bool {
        Command::new("lake")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    #[test]
    fn converts_lean_character_positions_to_byte_offsets() {
        let source = "first\n    let result ← *poisoned\n";
        assert_eq!(
            position_to_byte_offset(source, LeanPosition { line: 2, column: 4 }),
            Some(10)
        );
        assert_eq!(
            position_to_byte_offset(source, LeanPosition {
                line: 2,
                column: 26
            }),
            Some(34)
        );
    }

    #[test]
    fn parses_lean_json_diagnostics_with_source_ranges() {
        let source = "fun run := do\n  let result ← *poisoned\n";
        let output = br#"{"data":"borrow safety error","endPos":{"column":24,"line":2},"keepFullRange":false,"pos":{"column":2,"line":2},"severity":"error"}"#;
        assert_eq!(parse_lean_diagnostics(source, output), vec![
            LeanDiagnostic {
                message: "borrow safety error".to_owned(),
                severity: Severity::Error,
                start: 16,
                end: 40,
            }
        ]);
    }

    #[test]
    fn lean_diagnostics_hide_internal_names() {
        assert_eq!(
            sanitize_lean_diagnostic_message(
                "fun run (address : Address) := &Counter[address].value",
                "borrow safety error `_moveBorrowReturn0`: a returned reference must derive"
            ),
            "borrow safety error: a returned reference must derive"
        );
        assert_eq!(
            sanitize_lean_diagnostic_message(
                "fun run := pure 0",
                "borrow safety error `victim`: poisoned (conflicts with `_moveBorrowReturn1`)"
            ),
            "borrow safety error `victim`: poisoned"
        );
        assert_eq!(
            sanitize_lean_diagnostic_message(
                "fun run (_moveUser : U64) := _moveUser",
                "invalid local `_moveUser`"
            ),
            "invalid local `_moveUser`"
        );
        assert_eq!(
            sanitize_lean_diagnostic_message(
                "fun run (_moveUser' : U64) := _moveUser'",
                "invalid local `_moveUser'`"
            ),
            "invalid local `_moveUser'`"
        );
        assert_eq!(
            sanitize_lean_diagnostic_message(
                "fun run := pure 0",
                "invalid local `Move.Generated._leanerVectorRef0`"
            ),
            "invalid local"
        );
        assert_eq!(
            sanitize_lean_diagnostic_message(
                "fun run := pure 0",
                "cannot evaluate code because 'sorryAx' uses 'sorry' and/or contains errors"
            ),
            "cannot evaluate code because an unresolved expression uses 'sorry' and/or contains errors"
        );
    }

    #[test]
    fn extracts_lean_file() {
        let lean_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../lean/move");
        let mut options = Options {
            sources: vec![lean_root
                .join("Move/Tests/Verification/Account.lean")
                .to_string_lossy()
                .into_owned()],
            ..Default::default()
        };
        let sources = extract_sources(&mut options).unwrap();
        assert_eq!(sources.len(), 1);
        assert!(sources[0].is_target);
        assert!(options.sources.is_empty());
    }

    #[test]
    fn deduplicates_equivalent_lean_paths() {
        let lean_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../lean/move");
        let mut options = Options {
            sources: vec![lean_root
                .join("Move/Tests/Verification/Account.lean")
                .to_string_lossy()
                .into_owned()],
            sources_deps: vec![lean_root
                .join("Move/Tests/Verification/../Verification/Account.lean")
                .to_string_lossy()
                .into_owned()],
            ..Default::default()
        };
        let sources = extract_sources(&mut options).unwrap();
        assert_eq!(sources.len(), 1);
        assert!(sources[0].is_target);
    }

    #[test]
    fn lean_source_runs_complete_compiler_v2_pipeline() {
        if !lake_available() {
            eprintln!("skipping Leaner integration test: `lake` is unavailable");
            return;
        }
        let lean_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../lean/move");
        let options = Options {
            sources: vec![lean_root
                .join("Move/Tests/Verification/Account.lean")
                .to_string_lossy()
                .into_owned()],
            ..Default::default()
        };
        let (env, units) = crate::run_move_compiler_to_stderr(options).unwrap();
        assert!(!env.has_errors());
        assert_eq!(units.len(), 1);
        let legacy_move_compiler::compiled_unit::AnnotatedCompiledUnit::Module(module) = &units[0]
        else {
            panic!("expected module")
        };
        assert_eq!(
            module.named_module.module.self_id().name().as_str(),
            "Account"
        );
    }

    #[test]
    fn imported_lean_modules_compile_as_move_dependencies() {
        if !lake_available() {
            eprintln!("skipping Leaner integration test: `lake` is unavailable");
            return;
        }
        let lean_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../lean/move");
        // Intentionally put the client first: XIR loading must use declared
        // Move dependencies, not filesystem or command-line order.
        let options = Options {
            sources: [
                lean_root.join("Move/Tests/Compiler/MultipleModules.lean"),
                lean_root.join("Move/Tests/Compiler/Fixtures/Modules/Math.lean"),
            ]
            .map(|path| path.to_string_lossy().into_owned())
            .to_vec(),
            ..Default::default()
        };
        let (env, units) = crate::run_move_compiler_to_stderr(options).unwrap();
        assert!(!env.has_errors());
        let names = units
            .iter()
            .map(|unit| match unit {
                legacy_move_compiler::compiled_unit::AnnotatedCompiledUnit::Module(module) => {
                    module
                        .named_module
                        .module
                        .self_id()
                        .name()
                        .as_str()
                        .to_owned()
                },
                legacy_move_compiler::compiled_unit::AnnotatedCompiledUnit::Script(_) => {
                    panic!("expected only modules")
                },
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(
            names,
            BTreeSet::from(["Client".to_owned(), "Math".to_owned()])
        );
        let client = units
            .iter()
            .find_map(|unit| match unit {
                legacy_move_compiler::compiled_unit::AnnotatedCompiledUnit::Module(module)
                    if module.named_module.module.self_id().name().as_str() == "Client" =>
                {
                    Some(&module.named_module.module)
                },
                _ => None,
            })
            .expect("Client module was generated");
        assert!(client
            .immediate_dependencies()
            .iter()
            .any(|dependency| dependency.name().as_str() == "Math"));
    }

    #[test]
    fn lean_source_dependency_is_not_emitted_as_a_target() {
        if !lake_available() {
            eprintln!("skipping Leaner integration test: `lake` is unavailable");
            return;
        }
        let lean_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../lean/move");
        let options = Options {
            sources: vec![lean_root
                .join("Move/Tests/Compiler/MultipleModules.lean")
                .to_string_lossy()
                .into_owned()],
            sources_deps: vec![lean_root
                .join("Move/Tests/Compiler/Fixtures/Modules/Math.lean")
                .to_string_lossy()
                .into_owned()],
            ..Default::default()
        };
        let (env, units) = crate::run_move_compiler_to_stderr(options).unwrap();
        assert!(!env.has_errors());
        assert_eq!(units.len(), 1);
        let legacy_move_compiler::compiled_unit::AnnotatedCompiledUnit::Module(module) = &units[0]
        else {
            panic!("expected module")
        };
        assert_eq!(
            module.named_module.module.self_id().name().as_str(),
            "Client"
        );
        let math = env
            .get_modules()
            .find(|module| module.get_name().display(&env).to_string() == "Math")
            .expect("Math dependency was imported");
        assert!(!math.is_target());
    }
}

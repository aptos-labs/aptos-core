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
use move_command_line_common::files::{extension_equals, find_filenames, LEAN_EXTENSION};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Condvar, Mutex},
};

const MAX_CONCURRENT_LEAN_ELABORATIONS: usize = 2;

// Lean opens enough library artifacts during startup that launching one
// process per parallel transactional test can exceed the host descriptor
// limit.
static ACTIVE_LEAN_ELABORATIONS: Mutex<usize> = Mutex::new(0);
static LEAN_ELABORATION_AVAILABLE: Condvar = Condvar::new();

struct LeanElaborationPermit;

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

pub(crate) fn elaborate_sources(sources: &[LeanSource]) -> Result<Vec<XirSource>> {
    sources
        .iter()
        .map(|source| elaborate_source(&source.path, source.is_target))
        .collect()
}

fn elaborate_source(source: &str, is_target: bool) -> Result<XirSource> {
    let source_path = fs::canonicalize(source)
        .with_context(|| format!("unable to resolve Lean source `{source}`"))?;
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
        .args(["env", "lean"])
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
    if !output.status.success() {
        bail!(
            "Leaner elaboration failed for `{}`:\n{}{}",
            source_path.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }
    let json = fs::read_to_string(&xir_path).with_context(|| {
        format!(
            "`{}` did not emit XIR; add a `move_module` declaration or `#emit_leaner_xir`",
            source_path.display()
        )
    });
    let text = fs::read_to_string(&source_path)?;
    crate::xir::parse_source_with_target(source_path, text, &json?, is_target)
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
    fn extracts_lean_file() {
        let lean_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../lean/move");
        let mut options = Options {
            sources: vec![lean_root
                .join("Move/Tests/Account.lean")
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
                .join("Move/Tests/Account.lean")
                .to_string_lossy()
                .into_owned()],
            sources_deps: vec![lean_root
                .join("Move/Tests/../Tests/Account.lean")
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
                .join("Move/Tests/Account.lean")
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
                lean_root.join("Move/Tests/MultipleModules.lean"),
                lean_root.join("Move/Tests/Modules/Math.lean"),
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
                .join("Move/Tests/MultipleModules.lean")
                .to_string_lossy()
                .into_owned()],
            sources_deps: vec![lean_root
                .join("Move/Tests/Modules/Math.lean")
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

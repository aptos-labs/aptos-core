// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

pub mod ast;
pub mod comments;
pub(crate) mod filter;
pub mod keywords;
pub mod lexer;
pub(crate) mod merge_spec_modules;
pub mod syntax;

use crate::{
    diagnostics::{codes::Severity, Diagnostics, FilesSourceText},
    parser::{self, ast::PackageDefinition, syntax::parse_file_string},
    shared::{CompilationEnv, IndexedPackagePath, NamedAddressMapIndex, NamedAddressMaps},
};
use anyhow::anyhow;
use comments::*;
use move_command_line_common::files::{find_move_filenames, FileHash};
use move_symbol_pool::Symbol;
use std::{
    collections::{BTreeSet, HashMap},
    fs::File,
    io::Read,
};

/// Note that all directory paths must be restricted so that all
/// Move files under the are suitable for use: e.g., rather than
/// pointing to a package's Move.toml's directory, they point
/// to `.../source`, `.../scripts`, and/or `../tests` as appropriate.
pub(crate) fn parse_program(
    compilation_env: &mut CompilationEnv,
    named_address_maps: NamedAddressMaps,
    targets: Vec<IndexedPackagePath>,
    deps: Vec<IndexedPackagePath>,
) -> anyhow::Result<(
    FilesSourceText,
    Result<(parser::ast::Program, CommentMap), Diagnostics>,
)> {
    fn find_move_filenames_with_address_mapping(
        paths_with_mapping: Vec<IndexedPackagePath>,
    ) -> anyhow::Result<Vec<IndexedPackagePath>> {
        let mut res = vec![];
        for IndexedPackagePath {
            package,
            path,
            named_address_map: named_address_mapping,
        } in paths_with_mapping
        {
            res.extend(
                find_move_filenames(&[path.as_str()], true)?
                    .into_iter()
                    .map(|s| IndexedPackagePath {
                        package,
                        path: Symbol::from(s),
                        named_address_map: named_address_mapping,
                    }),
            );
        }
        // sort the filenames so errors about redefinitions, or other inter-file conflicts, are
        // deterministic
        res.sort_by(|p1, p2| p1.path.cmp(&p2.path));
        Ok(res)
    }

    let targets = find_move_filenames_with_address_mapping(targets)?;
    let mut deps = find_move_filenames_with_address_mapping(deps)?;
    ensure_targets_deps_dont_intersect(compilation_env, &targets, &mut deps)?;
    let mut files: FilesSourceText = HashMap::new();
    let mut source_definitions = Vec::new();
    let mut source_comments = CommentMap::new();
    let mut lib_definitions = Vec::new();
    let mut diags: Diagnostics = Diagnostics::new();

    for IndexedPackagePath {
        package,
        path,
        named_address_map,
    } in targets
    {
        let (defs, comments, ds, file_hash) = parse_file(compilation_env, &mut files, path)?;
        source_definitions.extend(defs.into_iter().map(|def| PackageDefinition {
            package,
            named_address_map,
            def,
        }));
        source_comments.insert(file_hash, comments);
        diags.extend(ds);
    }

    for IndexedPackagePath {
        package,
        path,
        named_address_map,
    } in deps
    {
        let (defs, _, ds, _) = parse_file(compilation_env, &mut files, path)?;
        lib_definitions.extend(defs.into_iter().map(|def| PackageDefinition {
            package,
            named_address_map,
            def,
        }));
        diags.extend(ds);
    }

    // TODO fix this so it works likes other passes and the handling of errors is done outside of
    // this function
    let env_result = compilation_env.check_diags_at_or_above_severity(Severity::BlockingError);
    if let Err(env_diags) = env_result {
        diags.extend(env_diags)
    }

    let res = if diags.is_empty() {
        let pprog = parser::ast::Program {
            named_address_maps,
            source_definitions,
            lib_definitions,
        };
        Ok((pprog, source_comments))
    } else {
        Err(diags)
    };
    Ok((files, res))
}

fn ensure_targets_deps_dont_intersect(
    compilation_env: &CompilationEnv,
    targets: &[IndexedPackagePath],
    deps: &mut Vec<IndexedPackagePath>,
) -> anyhow::Result<()> {
    /// Canonicalize a file path.
    fn canonicalize(path: &Symbol) -> String {
        let p = path.as_str();
        match std::fs::canonicalize(p) {
            Ok(s) => s.to_string_lossy().to_string(),
            Err(_) => p.to_owned(),
        }
    }
    let target_set = targets
        .iter()
        .map(|p| canonicalize(&p.path))
        .collect::<BTreeSet<_>>();
    let dep_set = deps
        .iter()
        .map(|p| canonicalize(&p.path))
        .collect::<BTreeSet<_>>();
    let intersection = target_set.intersection(&dep_set).collect::<Vec<_>>();
    if intersection.is_empty() {
        return Ok(());
    }
    if compilation_env.flags().sources_shadow_deps() {
        deps.retain(|p| !intersection.contains(&&canonicalize(&p.path)));
        return Ok(());
    }
    let all_files = intersection
        .into_iter()
        .map(|s| format!("    {}", s))
        .collect::<Vec<_>>()
        .join("\n");
    Err(anyhow!(
        "The following files were marked as both targets and dependencies:\n{}",
        all_files
    ))
}

fn parse_file(
    compilation_env: &mut CompilationEnv,
    files: &mut FilesSourceText,
    fname: Symbol,
) -> anyhow::Result<(
    Vec<parser::ast::Definition>,
    MatchedFileCommentMap,
    Diagnostics,
    FileHash,
)> {
    let mut diags = Diagnostics::new();
    let mut f = File::open(fname.as_str())
        .map_err(|err| std::io::Error::new(err.kind(), format!("{}: {}", err, fname)))?;
    let mut source_buffer = String::new();
    f.read_to_string(&mut source_buffer)?;
    let file_hash = FileHash::new(&source_buffer);
    let (defs, comments) = match parse_file_string(compilation_env, file_hash, &source_buffer) {
        Ok(defs_and_comments) => defs_and_comments,
        Err(ds) => {
            diags.extend(ds);
            (vec![], MatchedFileCommentMap::new())
        },
    };
    files.insert(file_hash, (fname, source_buffer));
    Ok((defs, comments, diags, file_hash))
}

/// Parse Move source files from in-memory strings (no filesystem access)
///
/// This is the source-based equivalent of `parse_program()`. Instead of
/// discovering and reading files from the filesystem, it uses pre-loaded
/// source content provided in `FilesSourceText` format.
///
/// # Arguments
/// * `compilation_env` - Compilation environment for error reporting
/// * `named_address_maps` - Named address mappings for all packages
/// * `targets` - Target source files to compile (keyed by content hash)
/// * `deps` - Dependency source files (not compiled as targets)
///
/// # Returns
/// Tuple of:
/// - FilesSourceText: Combined map of all file hashes to (name, content)
/// - Result of parsed program or diagnostics
pub fn parse_program_from_sources(
    compilation_env: &mut CompilationEnv,
    named_address_maps: NamedAddressMaps,
    targets: FilesSourceText,
    named_address_map_index: NamedAddressMapIndex,
    deps: FilesSourceText,
) -> anyhow::Result<(
    FilesSourceText,
    Result<(parser::ast::Program, CommentMap), Diagnostics>,
)> {
    // Combine targets and deps into single source map for lookups
    let mut all_files = targets.clone();
    all_files.extend(deps.clone());

    // Check for duplicates between targets and deps (by content hash)
    ensure_targets_deps_dont_intersect_sources(&targets, &deps)?;

    let mut source_definitions = Vec::new();
    let mut source_comments = CommentMap::new();
    let mut lib_definitions = Vec::new();
    let mut diags: Diagnostics = Diagnostics::new();

    // Parse target files
    for (file_hash, (_file_name, source_content)) in targets.iter() {
        match parse_file_string(compilation_env, *file_hash, source_content) {
            Ok((defs, comments)) => {
                // All files use the same named address map (index 0)
                // In a real multi-package scenario, this would be more complex
                source_definitions.extend(defs.into_iter().map(|def| PackageDefinition {
                    package: None,
                    named_address_map: named_address_map_index,
                    def,
                }));
                source_comments.insert(*file_hash, comments);
            }
            Err(ds) => {
                diags.extend(ds);
            }
        }
    }

    // Parse dependency files
    for (file_hash, (_file_name, source_content)) in deps.iter() {
        match parse_file_string(compilation_env, *file_hash, source_content) {
            Ok((defs, _comments)) => {
                lib_definitions.extend(defs.into_iter().map(|def| PackageDefinition {
                    package: None,
                    named_address_map: named_address_map_index,
                    def,
                }));
            }
            Err(ds) => {
                diags.extend(ds);
            }
        }
    }

    // Check for compilation environment errors
    let env_result = compilation_env.check_diags_at_or_above_severity(Severity::BlockingError);
    if let Err(env_diags) = env_result {
        diags.extend(env_diags)
    }

    // Build result
    let res = if diags.is_empty() {
        let pprog = parser::ast::Program {
            named_address_maps,
            source_definitions,
            lib_definitions,
        };
        Ok((pprog, source_comments))
    } else {
        Err(diags)
    };

    Ok((all_files, res))
}

/// Check that targets and deps don't intersect (source-based version)
///
/// Unlike the file-based version which canonicalizes paths, this version
/// checks by content hash. If the same content appears in both targets
/// and deps, it's an error.
fn ensure_targets_deps_dont_intersect_sources(
    targets: &FilesSourceText,
    deps: &FilesSourceText,
) -> anyhow::Result<()> {
    use std::collections::BTreeSet;

    let target_hashes: BTreeSet<_> = targets.keys().collect();
    let dep_hashes: BTreeSet<_> = deps.keys().collect();

    let intersection: Vec<_> = target_hashes
        .intersection(&dep_hashes)
        .collect();

    if !intersection.is_empty() {
        let duplicate_files: Vec<_> = intersection
            .iter()
            .map(|hash| {
                let (name, _) = targets.get(hash).or_else(|| deps.get(hash)).unwrap();
                name.as_str()
            })
            .collect();

        anyhow::bail!(
            "The following files were marked as both targets and dependencies:\n{}",
            duplicate_files.join("\n")
        );
    }

    Ok(())
}

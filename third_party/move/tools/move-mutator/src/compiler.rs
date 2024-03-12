use crate::configuration::Configuration;
use either::Either;
use itertools::Itertools;
use move_command_line_common::{address::NumericalAddress, parser::NumberFormat};
use move_compiler::{attr_derivation, shared::Flags};
use move_compiler_v2::run_checker;
use move_model::model::GlobalEnv;
use move_package::{
    compilation::compiled_package::make_source_and_deps_for_compiler,
    resolution::resolution_graph::ResolvedTable,
    source_package::{layout::SourcePackageLayout, manifest_parser},
    BuildConfig,
};
use move_symbol_pool::Symbol;
use std::{collections::BTreeMap, fs, io, path::Path};

/// Generate the AST from the Move sources.
///
/// Generation of the AST is done by the Move model package.
///
/// Generated AST contains all the information for all the Move files provided in the package or Move sources vector
/// present in the mutator configuration.
/// For packages, this functions searches for all the needed files (like manifest) and dependencies. In case of
/// any error, that error is returned.
/// For single Move sources, this function uses the Move compiler to compile the given sources without checking
/// for dependencies or performing name resolution.
///
/// # Arguments
///
/// * `mutator_config` - configuration of the mutator tool.
/// * `config` - contains the actual build configuration.
/// * `package_path` - the path to the Move package.
///
/// # Errors
///
/// * If any error occurs during the generation, the string with the cause is returned.
///
/// # Panics
///
/// This function panics if the source path contains invalid characters.
///
/// # Returns
///
/// * `Result<GlobalEnv, anyhow::Error>` - `GlobalEnv` if successful, or an error if any error occurs.
pub fn generate_ast(
    mutator_config: &Configuration,
    config: &BuildConfig,
    package_path: &Path,
) -> Result<GlobalEnv, anyhow::Error> {
    trace!("Generating AST for package: {package_path:?} and config: {config:?}");

    let source_files = mutator_config
        .project
        .move_sources
        .iter()
        .map(|p| p.to_str().expect("source path contains invalid characters"))
        .collect::<Vec<_>>();

    let is_package = source_files.is_empty();

    // If the `-m` option is specified, we should use only `move_sources`. Using Move source means we won't
    // check for deps or resolve names as there might be no standard package layout. That means we can mutate
    // only quite simple files.
    let options = if is_package {
        prepare_compiler_for_package(config, package_path)?
    } else {
        prepare_compiler_for_files(config, source_files.as_slice())
    };

    let env = run_checker(options.clone())?;

    trace!("Sources parsed successfully, AST generated");

    Ok(env)
}

/// Prepare the compiler for the given package.
/// This function prepares the compiler for the given package - it resolves all names and dependencies reading them
/// from the manifest file present at the package root.
///
/// This function is mostly copy of the code present in the `move_package` crate (`build_all`).
///
/// # Arguments
///
/// * `config` - the build configuration.
/// * `package_path` - the path to the package.
///
/// # Errors
///
/// * If any error occurs during the preparation, the appropriate error is returned using anyhow.
///
/// # Returns
///
/// * `Result<Compiler<'a>, anyhow::Error>` - the prepared compiler if successful, or an error if any error occurs.
fn prepare_compiler_for_package(
    config: &BuildConfig,
    package_path: &Path,
) -> Result<move_compiler_v2::Options, anyhow::Error> {
    let mut compilation_msg = vec![];
    let resolved_graph = config
        .clone()
        .resolution_graph_for_package(package_path, &mut compilation_msg)?;
    let root_package =
        resolved_graph.package_table[&resolved_graph.root_package.package.name].clone();

    let immediate_dependencies_names = root_package.immediate_dependencies(&resolved_graph);

    let transitive_dependencies: Vec<(Symbol, bool, Vec<Symbol>, &ResolvedTable, bool)> =
        root_package
            .transitive_dependencies(&resolved_graph)
            .into_iter()
            .map(|package_name| {
                let dep_package = resolved_graph.package_table.get(&package_name).unwrap();
                let mut dep_source_paths = dep_package
                    .get_sources(&resolved_graph.build_options)
                    .unwrap();
                let mut source_available = true;
                // If source is empty, search bytecode(mv) files
                if dep_source_paths.is_empty() {
                    dep_source_paths = dep_package.get_bytecodes().unwrap();
                    source_available = false;
                }
                (
                    package_name,
                    immediate_dependencies_names.contains(&package_name),
                    dep_source_paths,
                    &dep_package.resolution_table,
                    source_available,
                )
            })
            .collect();

    let transitive_dependencies = transitive_dependencies
        .into_iter()
        .map(
            |(name, _is_immediate, source_paths, address_mapping, src_flag)| {
                (name, source_paths, address_mapping, src_flag)
            },
        )
        .collect::<Vec<_>>();
    let mut source_package_map: BTreeMap<String, Symbol> = BTreeMap::new();
    for (dep_package_name, source_paths, _, _) in &transitive_dependencies {
        for dep_path in source_paths.clone() {
            source_package_map.insert(dep_path.as_str().to_string(), *dep_package_name);
        }
    }
    let root_package_name = root_package.source_package.package.name;

    // gather source/dep files with their address mappings
    let (sources_package_paths, deps_package_paths) =
        make_source_and_deps_for_compiler(&resolved_graph, &root_package, transitive_dependencies)?;
    for source_path in &sources_package_paths.paths {
        source_package_map.insert(source_path.as_str().to_string(), root_package_name);
    }

    let mut flags = if config.test_mode {
        Flags::testing()
    } else {
        Flags::empty()
    };
    flags = flags.set_skip_attribute_checks(config.compiler_config.skip_attribute_checks);
    let mut known_attributes = config.compiler_config.known_attributes.clone();
    attr_derivation::add_attributes_for_flavor(&flags, &mut known_attributes);

    // Partition deps_package according whether src is available
    let (src_deps, bytecode_deps): (Vec<_>, Vec<_>) = deps_package_paths
        .clone()
        .into_iter()
        .partition_map(|(p, b)| if b { Either::Left(p) } else { Either::Right(p) });

    let mut paths = src_deps;
    paths.push(sources_package_paths.clone());

    let to_str_vec = |ps: &[Symbol]| {
        ps.iter()
            .map(move |s| s.as_str().to_owned())
            .collect::<Vec<_>>()
    };
    let mut global_address_map = BTreeMap::new();
    for pack in paths.iter().chain(bytecode_deps.iter()) {
        for (name, val) in &pack.named_address_map {
            let Some(_) = global_address_map.insert(name.as_str().to_owned(), *val) else {
                continue;
            };
        }
    }

    let options = move_compiler_v2::Options {
        sources: paths.iter().flat_map(|x| to_str_vec(&x.paths)).collect(),
        dependencies: bytecode_deps
            .iter()
            .flat_map(|x| to_str_vec(&x.paths))
            .collect(),
        named_address_mapping: global_address_map
            .into_iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect(),
        skip_attribute_checks: config.compiler_config.skip_attribute_checks,
        known_attributes: known_attributes.clone(),
        ..Default::default()
    };

    Ok(options)
}

/// Prepare the compiler for the given source files.
///
/// # Arguments
///
/// * `config` - the build configuration.
/// * `source_files` - vector of the source files.
///
/// # Errors
///
/// * If any error occurs during the preparation, the appropriate error is returned using anyhow.
///
/// # Returns
///
/// * `Result<Compiler<'a>, anyhow::Error>` - the prepared compiler if successful, or an error if any error occurs.
fn prepare_compiler_for_files(
    config: &BuildConfig,
    source_files: &[&str],
) -> move_compiler_v2::Options {
    debug!("Source files and folders: {source_files:?}");

    let named_addr_map = config
        .additional_named_addresses
        .clone()
        .into_iter()
        .map(|(name, addr)| {
            (
                name,
                NumericalAddress::new(addr.into_bytes(), NumberFormat::Decimal),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let known_attributes = config.compiler_config.known_attributes.clone();

    move_compiler_v2::Options {
        sources: source_files
            .iter()
            .map(std::string::ToString::to_string)
            .collect(),
        dependencies: vec![],
        named_address_mapping: named_addr_map
            .into_iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect(),
        skip_attribute_checks: config.compiler_config.skip_attribute_checks,
        known_attributes: known_attributes.clone(),
        ..Default::default()
    }
}

/// Verify the mutant.
/// This function compiles the mutated source and checks if the compilation is successful.
/// If the compilation is successful, the mutant is valid.
///
/// This function uses the Move compiler to compile the mutated source. To do so, it copies the whole package
/// to a temporary directory and replaces the original file with the mutated source. It may introduce problems
/// with dependencies that are specified as relative paths to the package root.
///
/// # Arguments
///
/// * `config` - the build configuration.
/// * `mutated_source` - the mutated source code as a string.
/// * `original_file` - the path to the original file.
///
/// # Errors
///
/// * If any error occurs during the verification, the string with the cause is returned.
///
/// # Returns
///
/// * `Result<(), anyhow::Error>` - Ok if the mutant is valid, or an error if any error occurs.
pub fn verify_mutant(
    config: &BuildConfig,
    mutated_source: &str,
    original_file: &Path,
) -> Result<(), anyhow::Error> {
    // Find the root for the package.
    let root = SourcePackageLayout::try_find_root(&original_file.canonicalize()?)?;

    debug!("Package path found: {root:?}");

    // Get the relative path to the original file.
    let relative_path = original_file.canonicalize()?;
    let relative_path = relative_path.strip_prefix(&root)?;

    debug!("Relative path: {relative_path:?}");

    let tempdir = tempfile::tempdir()?;

    debug!("Temporary directory: {:?}", tempdir.path());

    // Copy the whole package to the tempdir.
    // We need to copy the whole package because the Move compiler needs to find the Move.toml file and all the dependencies
    // as we don't know which files are needed for the compilation.
    copy_dir_all(&root, tempdir.path())?;

    // Write the mutated source to the tempdir in place of the original file.
    std::fs::write(tempdir.path().join(relative_path), mutated_source)?;

    // Rewrite the manifest file to use absolute paths
    rewrite_manifest_for_mutant(&root, tempdir.path())?;

    debug!(
        "Mutated source written to {:?}",
        tempdir.path().join(relative_path)
    );

    let mut compilation_msg = vec![];

    // Create a working config, making sure that the test mode is disabled.
    // We want just check if the compilation is successful.
    let mut working_config = config.clone();
    working_config.test_mode = false;

    // Compile the package.
    //TODO: It might be better to use the different compiler stage to speed up the whole
    // process. For the verification purposes it might be suffcient some earlier stage,
    // e.g. type-checking.
    working_config.compile_package_no_exit(tempdir.path(), &mut compilation_msg)?;

    info!(
        "Compilation status: {}",
        String::from_utf8(compilation_msg)
            .unwrap_or("Internal error: can't convert compilation error to UTF8".to_string())
    );

    Ok(())
}

/// Rewrite the manifest file to use absolute paths.
///
/// # Arguments
///
/// * `root` - the path to the package root.
/// * `tempdir` - the path to the temporary directory.
///
/// # Errors
///
/// * If any error occurs during the rewrite, the appropriate error is returned using anyhow.
///
/// # Panics
///
/// This function panics if dependency paths contain no Unicode characters.
///
/// # Returns
///
/// * `Result<(), anyhow::Error>` - Ok if the rewrite is successful, or an error if any error occurs.
pub fn rewrite_manifest_for_mutant(root: &Path, tempdir: &Path) -> Result<(), anyhow::Error> {
    let mut manifest_string = fs::read_to_string(root.join(SourcePackageLayout::Manifest.path()))?;
    let manifest = manifest_parser::parse_move_manifest_string(manifest_string.clone())?;
    let manifest = manifest_parser::parse_source_manifest(manifest)?;
    let curdir = std::env::current_dir()?;

    // We need to switch to package dir as paths in manifest are relative to package dir.
    std::env::set_current_dir(root)?;

    manifest
        .dependencies
        .values()
        .chain(manifest.dev_dependencies.values())
        .for_each(|dep| {
            let dep_canon = dep.local.canonicalize();
            if let Ok(dep_canon) = dep_canon {
                manifest_string = manifest_string
                    .replace(dep.local.to_str().unwrap(), dep_canon.to_str().unwrap());
            }
        });

    // Switch back to the original dir.
    std::env::set_current_dir(curdir)?;
    fs::write(
        tempdir.join(SourcePackageLayout::Manifest.path()),
        manifest_string,
    )?;
    Ok(())
}

/// Copies all files and directories from the source directory to the destination directory.
///
/// # Arguments
///
/// * `src` - the source directory.
/// * `dst` - the destination directory.
///
/// # Errors
/// * If any error occurs during the copy, the appropriate IO error is returned.
///
/// # Returns
///
/// * `io::Result<()>` - Ok if the copy is successful, or an error if any error occurs.
pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    if !dst.as_ref().exists() {
        fs::create_dir_all(dst.as_ref())?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn copy_dir_all_copies_all_files_and_directories() {
        let temp_dir = tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        let dst_dir = temp_dir.path().join("dst");

        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("file.txt"), "Hello, world!").unwrap();

        let result = copy_dir_all(&src_dir, &dst_dir);
        assert!(result.is_ok());
        assert!(dst_dir.join("file.txt").exists());
    }

    #[test]
    fn copy_dir_all_errors_if_source_does_not_exist() {
        let temp_dir = tempdir().unwrap();
        let src_dir = temp_dir.path().join("non_existent_src");
        let dst_dir = temp_dir.path().join("dst");

        let result = copy_dir_all(src_dir, dst_dir);
        assert!(result.is_err());
    }
}

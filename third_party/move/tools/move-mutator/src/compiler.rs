use either::Either;
use move_command_line_common::address::NumericalAddress;
use move_command_line_common::parser::NumberFormat;
use std::collections::BTreeMap;
use std::path::Path;
use std::{fs, io};

use crate::configuration::Configuration;
use itertools::Itertools;
use move_compiler::diagnostics::FilesSourceText;
use move_compiler::{
    attr_derivation,
    command_line::compiler::{Compiler, PASS_TYPING},
    diagnostics::unwrap_or_report_diagnostics,
    shared::Flags,
};
use move_package::compilation::compiled_package::make_source_and_deps_for_compiler;
use move_package::resolution::resolution_graph::ResolvedTable;
use move_package::source_package::layout::SourcePackageLayout;
use move_package::BuildConfig;
use move_symbol_pool::Symbol;

/// Generate the AST from the Move sources.
///
/// Generation of the AST is done by the Move compiler. Move compiler is stepped compiler, which means that
/// it is possible to get the intermediate results of the compilation. This function uses it to get the AST
/// right after the parsing phase.
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
/// # Returns
///
/// * `Result<(FilesSourceText, move_compiler::typing::ast::Program), anyhow::Error>` - tuple of `FilesSourceText` and Program if successful, or an error if any error occurs.
pub fn generate_ast(
    mutator_config: &Configuration,
    config: &BuildConfig,
    package_path: &Path,
) -> Result<(FilesSourceText, move_compiler::typing::ast::Program), anyhow::Error> {
    trace!("Generating AST for package: {package_path:?} and config: {config:?}");

    let source_files = mutator_config
        .project
        .move_sources
        .iter()
        .map(|p| p.to_str().expect("source path contains invalid characters"))
        .collect::<Vec<_>>();

    // If the `-m` option is specified, we should use only `move_sources`. Using Move source means we won't
    // check for deps or resolve names as there might be no standard package layout. That means we can mutate
    // only quite simple files.
    let compiler = if source_files.is_empty() {
        prepare_compiler_for_package(config, package_path)?
    } else {
        prepare_compiler_for_files(config, source_files)
    };

    let (files, res) = compiler.run::<PASS_TYPING>()?;

    let (_, stepped) = unwrap_or_report_diagnostics(&files, res);
    let (_, ast) = stepped.into_ast();

    trace!("Sources parsed successfully, AST generated");

    Ok((files, ast))
}

/// Prepare the compiler for the given package.
/// This function prepares the compiler for the given package - it resolves all names and dependencies reading them
/// from the manifest file present at the package root.
///
/// This function is mostly copy of the code present in the `move_package` crate (build_all).
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
fn prepare_compiler_for_package<'a>(
    config: &BuildConfig,
    package_path: &Path,
) -> Result<Compiler<'a>, anyhow::Error> {
    let mut compilation_msg = vec![];
    let resolved_graph = config
        .clone()
        .resolution_graph_for_package(package_path, &mut compilation_msg)?;
    let root_package =
        resolved_graph.package_table[&resolved_graph.root_package.package.name].clone();

    let immediate_dependencies_names = root_package.immediate_dependencies(&resolved_graph);

    let transitive_dependencies: Vec<(
        /* name */ Symbol,
        /* is immediate */ bool,
        /* source paths */ Vec<Symbol>,
        /* address mapping */ &ResolvedTable,
        /* whether source is available */ bool,
    )> = root_package
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

    // invoke the compiler
    let mut paths = src_deps;
    paths.push(sources_package_paths.clone());

    Ok(Compiler::from_package_paths(
        paths,
        bytecode_deps,
        flags,
        &known_attributes,
    ))
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
fn prepare_compiler_for_files<'a>(config: &BuildConfig, source_files: Vec<&str>) -> Compiler<'a> {
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

    let flags = Flags::empty();

    Compiler::from_files(
        source_files,
        vec![],
        named_addr_map,
        flags,
        &config.compiler_config.known_attributes,
    )
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
    working_config.compile_package(tempdir.path(), &mut compilation_msg)?;

    info!(
        "Compilation status: {}",
        String::from_utf8(compilation_msg)
            .unwrap_or("Internal error: can't convert compilation error to UTF8".to_string())
    );

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

        let result = copy_dir_all(&src_dir, &dst_dir);
        assert!(result.is_err());
    }
}

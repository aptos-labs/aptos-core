use move_command_line_common::address::NumericalAddress;
use move_command_line_common::parser::NumberFormat;
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::configuration::Configuration;
use move_compiler::diagnostics::FilesSourceText;
use move_compiler::{
    command_line::compiler::*, diagnostics::unwrap_or_report_diagnostics, shared::Flags,
};
use move_package::source_package::layout::SourcePackageLayout;
use move_package::BuildConfig;

/// Generate the AST from the Move sources.
///
/// Generation of the AST is done by the Move compiler. Move compiler is stepped compiler, which means that
/// it is possible to get the intermediate results of the compilation. This function uses it to get the AST
/// right after the parsing phase.
///
/// Generated AST contains all the information for all the Move files provided in the `source_files` vector.
/// Compiler searches automatically for all the needed files (like manifest) and dependencies. In case of
/// any error, that error is returned.
///
/// # Arguments
///
/// * `source_files` - vector of strings representing the Move source files paths.
/// * `config` - contains the actual build configuration.
/// * `package_path` - the path to the Move package.
///
/// # Returns
///
/// * `Result<(FilesSourceText, move_compiler::parser::ast::Program), anyhow::Error>` - tuple of FilesSourceText and Program if successful, or an error if any error occurs.
pub fn generate_ast(
    mutator_config: &Configuration,
    config: BuildConfig,
    package_path: PathBuf,
) -> Result<(FilesSourceText, move_compiler::parser::ast::Program), anyhow::Error> {
    let source_files = mutator_config.project.move_sources.clone();

    let named_addr_map = config
        .additional_named_addresses
        .into_iter()
        .map(|(name, addr)| {
            (
                name,
                NumericalAddress::new(addr.into_bytes(), NumberFormat::Decimal),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let out_dir = "mutator_build";
    let interface_files_dir = format!("{}/generated_interface_files", out_dir);
    let flags = Flags::empty();

    //TODO(asmie): check if root is not found (we cannot parse then Move.toml and get the address resolution)
    // Maybe we should then allow only source files with numerical addresses
    let _rooted_path = SourcePackageLayout::try_find_root(&package_path.canonicalize()?);

    let (files, res) = Compiler::from_files(
        source_files,
        vec![],
        named_addr_map,
        flags,
        &config.compiler_config.known_attributes,
    )
    .set_interface_files_dir(interface_files_dir)
    .run::<PASS_PARSER>()?;

    let (_, stepped) = unwrap_or_report_diagnostics(&files, res);
    let (_, ast) = stepped.into_ast();

    Ok((files, ast))
}

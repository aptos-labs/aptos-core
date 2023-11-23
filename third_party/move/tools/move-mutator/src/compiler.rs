use move_command_line_common::address::NumericalAddress;
use move_command_line_common::parser::NumberFormat;
use std::collections::BTreeMap;
use std::path::PathBuf;

use move_compiler::diagnostics::FilesSourceText;
use move_compiler::{
    command_line::compiler::*, diagnostics::unwrap_or_report_diagnostics, shared::Flags,
};
use move_package::source_package::layout::SourcePackageLayout;
use move_package::BuildConfig;

/// Generate the AST from the Move sources.
pub fn generate_ast(
    source_files: Vec<String>,
    config: BuildConfig,
    package_path: PathBuf,
) -> Result<(FilesSourceText, move_compiler::parser::ast::Program), anyhow::Error> {
    let mut named_addr_map = BTreeMap::new();

    for (name, addr) in config.additional_named_addresses {
        named_addr_map.insert(
            name,
            NumericalAddress::new(addr.into_bytes(), NumberFormat::Decimal),
        );
    }

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

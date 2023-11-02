use std::{ path::PathBuf, collections::BTreeMap };
use anyhow::Result;
use move_command_line_common::address::NumericalAddress;
use move_compiler::{
    FullyCompiledProgram,
    construct_pre_compiled_lib,
    shared::{ known_attributes::KnownAttribute, PackagePaths },
    Flags,
    diagnostics,
};

pub fn build_ast(path: &PathBuf) -> Result<FullyCompiledProgram> {
    let targets: Vec<String> = vec![path.as_path().to_str().unwrap().to_owned()];
    let paths = vec![PackagePaths {
        name: None,
        paths: targets,
        named_address_map: move_stdlib_named_addresses(),
    }];
    let fully_compiled_program = match
        construct_pre_compiled_lib(paths, None, Flags::empty(), KnownAttribute::get_all_attribute_names())?
    {
        Ok(p) => p,
        Err((files, diags)) => {
            diagnostics::report_diagnostics(&files, diags);
        }
    };
    Ok(fully_compiled_program)
}

fn move_stdlib_named_addresses() -> BTreeMap<String, NumericalAddress> {
    let mapping = [
        ("std", "0x1"),
        ("NamedAddr", "0xCAFE"),
    ];
    mapping
        .iter()
        .map(|(name, addr)| (name.to_string(), NumericalAddress::parse_str(addr).unwrap()))
        .collect()
}

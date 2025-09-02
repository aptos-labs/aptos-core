// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use anyhow::Context;
use clap::Parser;
use move_binary_format::{
    errors::VMError,
    file_format::{CompiledModule, CompiledScript},
};
use move_bytecode_verifier::{
    dependencies, verify_module, verify_module_with_config, verify_script_with_config,
    VerifierConfig,
};
use move_command_line_common::files::{
    MOVE_COMPILED_EXTENSION, MOVE_IR_EXTENSION, SOURCE_MAP_EXTENSION,
};
use move_ir_compiler::util;
use move_ir_to_bytecode::parser::{parse_module, parse_script};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Parser)]
#[clap(author, version, about)]
struct Args {
    /// Treat input file as a module (default is to treat file as a script)
    #[clap(short = 'm', long = "module")]
    pub module_input: bool,
    /// Do not automatically run the bytecode verifier
    #[clap(long = "no-verify")]
    pub no_verify: bool,
    /// Path to the Move IR source to compile
    #[clap(value_parser)]
    pub source_path: PathBuf,
    /// Instead of compiling the source, emit a dependency list of the compiled source
    #[clap(short = 'l', long = "list-dependencies")]
    pub list_dependencies: bool,
    /// Path to the list of modules that we want to link with
    #[clap(short = 'd', long = "deps")]
    pub deps_path: Option<String>,

    #[clap(long = "src-map")]
    pub output_source_maps: bool,
}

fn print_error_and_exit(verification_error: &VMError) -> ! {
    println!("Verification failed:");
    println!("{:?}", verification_error);
    std::process::exit(1);
}

fn do_verify_module(module: &CompiledModule, dependencies: &[CompiledModule]) {
    let config = VerifierConfig::default();
    verify_module_with_config(&config, module).unwrap_or_else(|err| print_error_and_exit(&err));
    if let Err(err) = dependencies::verify_module(&config, module, dependencies) {
        print_error_and_exit(&err);
    }
}

fn do_verify_script(script: &CompiledScript, dependencies: &[CompiledModule]) {
    let config = VerifierConfig::default();
    verify_script_with_config(&config, script).unwrap_or_else(|err| print_error_and_exit(&err));
    if let Err(err) = dependencies::verify_script(&config, script, dependencies) {
        print_error_and_exit(&err);
    }
}

fn write_output(path: &Path, buf: &[u8]) {
    let mut f = fs::File::create(path)
        .with_context(|| format!("Unable to open output file {:?}", path))
        .unwrap();
    f.write_all(buf)
        .with_context(|| format!("Unable to write to output file {:?}", path))
        .unwrap();
}

fn main() {
    let args = Args::parse();

    let source_path = Path::new(&args.source_path);
    let mvir_extension = MOVE_IR_EXTENSION;
    let mv_extension = MOVE_COMPILED_EXTENSION;
    let source_map_extension = SOURCE_MAP_EXTENSION;
    let extension = source_path
        .extension()
        .expect("Missing file extension for input source file");
    if extension != mvir_extension {
        println!(
            "Bad source file extension {:?}; expected {}",
            extension, mvir_extension
        );
        std::process::exit(1);
    }

    if args.list_dependencies {
        let source = fs::read_to_string(args.source_path.clone()).expect("Unable to read file");
        let dependency_list = if args.module_input {
            let module = parse_module(&source).expect("Unable to parse module");
            module.get_external_deps()
        } else {
            let script = parse_script(&source).expect("Unable to parse module");
            script.get_external_deps()
        };
        println!(
            "{}",
            serde_json::to_string(&dependency_list).expect("Unable to serialize dependencies")
        );
        return;
    }

    let deps_owned = {
        if let Some(path) = args.deps_path {
            let deps = fs::read_to_string(path).expect("Unable to read dependency file");
            let deps_list: Vec<Vec<u8>> =
                serde_json::from_str(deps.as_str()).expect("Unable to parse dependency file");
            deps_list
                .into_iter()
                .map(|module_bytes| {
                    let module = CompiledModule::deserialize(module_bytes.as_slice())
                        .expect("Downloaded module blob can't be deserialized");
                    verify_module(&module).expect("Downloaded module blob failed verifier");
                    module
                })
                .collect()
        } else {
            vec![]
        }
    };

    if args.module_input {
        let (compiled_module, source_map) = util::do_compile_module(&args.source_path, &deps_owned);
        if !args.no_verify {
            do_verify_module(&compiled_module, &deps_owned);
        }

        if args.output_source_maps {
            let source_map_bytes =
                bcs::to_bytes(&source_map).expect("Unable to serialize source maps for module");
            write_output(
                &source_path.with_extension(source_map_extension),
                &source_map_bytes,
            );
        }

        let mut module = vec![];
        compiled_module
            .serialize(&mut module)
            .expect("Unable to serialize module");
        write_output(&source_path.with_extension(mv_extension), &module);
    } else {
        let (compiled_script, source_map) = util::do_compile_script(&args.source_path, &deps_owned);
        if !args.no_verify {
            do_verify_script(&compiled_script, &deps_owned);
        }

        if args.output_source_maps {
            let source_map_bytes =
                bcs::to_bytes(&source_map).expect("Unable to serialize source maps for script");
            write_output(
                &source_path.with_extension(source_map_extension),
                &source_map_bytes,
            );
        }

        let mut script = vec![];
        compiled_script
            .serialize(&mut script)
            .expect("Unable to serialize script");
        write_output(&source_path.with_extension(mv_extension), &script);
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}

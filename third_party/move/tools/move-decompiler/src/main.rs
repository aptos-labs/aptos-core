// Revela decompiler. Copyright (c) Verichains, 2023-2024

#![forbid(unsafe_code)]

use std::fs;

use clap::Parser;

use move_binary_format::{
    binary_views::BinaryIndexedView,
    file_format::{CompiledModule, CompiledScript},
};
use move_decompiler::decompiler::{Decompiler, OptimizerSettings};
#[derive(Debug, Parser)]
#[clap(author, version, about)]
struct Args {
    /// Treat input file as a script (default is to treat file as a module)
    #[clap(short = 's', long = "script")]
    pub is_script: bool,

    // Input files
    #[clap(short = 'b', long = "bytecode")]
    pub files: Vec<String>,

    #[clap(
        long = "disable-variable-declaration-optimization",
        default_value = "false"
    )]
    pub disable_variable_declaration_optimization: bool,
}

enum CompiledBinary {
    Script(CompiledScript),
    Module(CompiledModule),
}

fn main() {
    let args = Args::parse();

    let binaries_store: Vec<_> = args
        .files
        .iter()
        .map(|file| {
            let bytecode_bytes = fs::read(file).unwrap_or_else(|err| {
                panic!("Error: failed to read file {}: {}", file.to_string(), err);
            });

            if args.is_script {
                CompiledBinary::Script(CompiledScript::deserialize(&bytecode_bytes).unwrap_or_else(
                    |err| {
                        panic!("Error: failed to deserialize script blob: {}", err);
                    },
                ))
            } else {
                CompiledBinary::Module(CompiledModule::deserialize(&bytecode_bytes).unwrap_or_else(
                    |err| {
                        panic!("Error: failed to deserialize module blob: {}", err);
                    },
                ))
            }
        })
        .collect();

    let binaries: Vec<_> = binaries_store
        .iter()
        .map(|binary| match binary {
            CompiledBinary::Script(script) => BinaryIndexedView::Script(script),
            CompiledBinary::Module(module) => BinaryIndexedView::Module(module),
        })
        .collect();

    let mut decompiler = Decompiler::new(
        binaries,
        OptimizerSettings {
            disable_optimize_variables_declaration: args.disable_variable_declaration_optimization,
        },
    );
    let output = decompiler.decompile().expect("Error: unable to decompile");
    println!("{}", output);
}

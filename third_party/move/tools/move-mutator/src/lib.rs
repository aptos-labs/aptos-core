pub mod cli;
mod compiler;

mod mutate;

mod operator;

mod mutant;
use crate::compiler::generate_ast;

use move_package::BuildConfig;
use std::path::PathBuf;

/// Runs the Move mutator tool.
/// Entry point for the Move mutator tool both for the CLI and the Rust API.
pub fn run_move_mutator(
    options: cli::Options,
    config: BuildConfig,
    package_path: PathBuf,
) -> anyhow::Result<()> {
    println!(
        "Executed move-mutator with the following options: {:?} \n config: {:?} \n package path: {:?}",
        options, config, package_path
    );

    let (_files, ast) = generate_ast(options.move_sources, config, package_path)?;

    let mutants = mutate::mutate(ast)?;

    println!("Mutants:");
    for mutant in mutants {
        println!("{}", mutant);
    }

    Ok(())
}

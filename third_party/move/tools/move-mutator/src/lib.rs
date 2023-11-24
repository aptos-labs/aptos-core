pub mod cli;
mod compiler;

mod mutate;

mod operator;

mod mutant;

use crate::compiler::generate_ast;
use std::path::Path;

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

    let (files, ast) = generate_ast(options.move_sources, config, package_path)?;

    let mutants = mutate::mutate(ast)?;

    let _ = std::fs::remove_dir_all("mutants_output");
    std::fs::create_dir("mutants_output")?;

    for (hash, file) in files {
        let (filename, source) = file;

        let path = Path::new(filename.as_str());
        let file_name = path.file_stem().unwrap().to_str().unwrap();

        let mut i = 0;
        for mutant in mutants.iter().filter(|m| m.get_file_hash() == hash) {
            let mutated_sources = mutant.apply(&source);
            for source in mutated_sources {
                let mut_path = format!("mutants_output/{}_{}.move", file_name, i);
                println!("{} written to {}", mutant, &mut_path);
                std::fs::write(mut_path, source)?;
                i += 1;
            }
        }
    }

    Ok(())
}

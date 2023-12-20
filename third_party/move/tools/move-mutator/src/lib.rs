pub mod cli;
mod compiler;

mod mutate;

mod mutant;
mod operator;
mod report;

use crate::compiler::generate_ast;
use std::path::Path;

use crate::report::Report;
use move_package::BuildConfig;
use std::path::PathBuf;

const OUTPUT_DIR: &str = "mutants_output";

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

    let _ = std::fs::remove_dir_all(OUTPUT_DIR);
    std::fs::create_dir(OUTPUT_DIR)?;

    let mut report: Report = Report::new();

    for (hash, (filename, source)) in files {
        let path = Path::new(filename.as_str());
        let file_name = path.file_stem().unwrap().to_str().unwrap();

        let mut i = 0;
        for mutant in mutants.iter().filter(|m| m.get_file_hash() == hash) {
            let mutated_sources = mutant.apply(&source);
            for mutated in mutated_sources {
                let mutant_path = PathBuf::from(format!("mutants_output/{}_{}.move", file_name, i));
                println!(
                    "{} written to {}",
                    mutant,
                    mutant_path.to_str().unwrap_or("")
                );
                std::fs::write(&mutant_path, &mutated.mutated_source)?;
                let mut entry = report::MutationReport::new(mutant_path.as_path(), path, &mutated.mutated_source, &source);
                entry.add_modification(mutated.mutation);
                report.add_entry(entry);
                i += 1;
            }
        }
    }

    report.save_to_json_file(Path::new("mutants_output/report.json"))?;
    report.save_to_text_file(Path::new("mutants_output/report.txt"))?;

    Ok(())
}

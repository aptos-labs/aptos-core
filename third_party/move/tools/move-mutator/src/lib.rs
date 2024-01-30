extern crate pretty_env_logger;
#[macro_use]
extern crate log;

pub mod cli;
pub mod compiler;

mod mutate;

pub mod configuration;
mod mutant;
mod operator;
mod operators;
mod output;
pub mod report;

use crate::compiler::{generate_ast, verify_mutant};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::fs;
use std::path::Path;

use crate::configuration::Configuration;
use crate::report::Report;
use move_package::BuildConfig;
use std::path::PathBuf;
use move_compiler::parser::ast::ModuleName;

/// Runs the Move mutator tool.
/// Entry point for the Move mutator tool both for the CLI and the Rust API.
///
/// # Arguments
///
/// * `options` - Command line options passed to the Move mutator tool.
/// * `config` - The build configuration for the Move package.
/// * `package_path` - The path to the Move package.
///
/// # Errors
/// Any error that occurs during the mutation process will be returned as an `anyhow::Error` with a description of the error.
///
/// # Returns
///
/// * `anyhow::Result<()>` - Returns `Ok(())` if the mutation process completes successfully, or an error if any error occurs.
pub fn run_move_mutator(
    options: cli::CLIOptions,
    config: &BuildConfig,
    package_path: &PathBuf,
) -> anyhow::Result<()> {
    // We need to initialize logger using try_init() as it might be already initialized in some other tool
    // (e.g. spec-test). If we use init() instead, we will get an abort.
    let _ = pretty_env_logger::try_init();

    info!(
        "Executed move-mutator with the following options: {options:?} \n config: {config:?} \n package path: {package_path:?}"
    );

    // Load configuration from file or create a new one.
    let mutator_configuration = match options.configuration_file {
        Some(path) => Configuration::from_file(path.as_path())?,
        None => Configuration::new(options, Some(package_path.clone())),
    };

    trace!("Mutator configuration: {mutator_configuration:?}");

    let (files, ast) = generate_ast(&mutator_configuration, config, package_path)?;

    trace!("Generated AST: {ast:?}");

    let mutants = mutate::mutate(ast, &mutator_configuration, &files)?;
    let output_dir = output::setup_output_dir(&mutator_configuration)?;
    let mut report: Report = Report::new();

    for (hash, (filename, source)) in files {
        let path = Path::new(filename.as_str());

        trace!("Processing file: {path:?}");

        // This `i` must be here as we must iterate over all mutants for a given file (ext and internal loop).
        let mut mutant_file_idx = 0u64;
        for mutant in mutants.iter().filter(|m| m.get_file_hash() == hash) {
            let mut mutated_sources = mutant.apply(&source);

            // If the downsample ratio is set, we need to downsample the mutants.
            if let Some(percentage) = mutator_configuration.project.downsampling_ratio_percentage {
                let to_remove = (mutated_sources.len() as f64 * percentage as f64 / 100.0) as usize;

                // Delete randomly elements from the vector.
                let mut rng = thread_rng();
                let chosen_elements: Vec<_> = mutated_sources
                    .choose_multiple(&mut rng, to_remove)
                    .cloned()
                    .collect();

                // Remove the chosen elements from the original vector
                for element in &chosen_elements {
                    mutated_sources.retain(|x| x != element);
                }
            }

            for mutated in mutated_sources {
                if mutator_configuration.project.verify_mutants {
                    let res = verify_mutant(config, &mutated.mutated_source, path);

                    // In case the mutant is not a valid Move file, skip the mutant (do not save it).
                    if res.is_err() {
                        warn!(
                            "Mutant {mutant} is not valid and will not be generated. Error: {res:?}"
                        );
                        continue;
                    }
                }

                let mutant_path = output::setup_mutant_path(&output_dir, path, mutant_file_idx)?;
                mutant_file_idx += 1;

                fs::write(&mutant_path, &mutated.mutated_source)?;

                info!("{} written to {}", mutant, mutant_path.display());

                let mod_name = if let Some(module) = mutant.get_module_name() {
                    let ModuleName(name) = module;
                    name.value.to_string()
                } else {
                    "script".to_owned()     // if there is no module name, it is a script
                };

                let mut entry = report::MutationReport::new(
                    mutant_path.as_path(),
                    path,
                    mod_name.as_str(),
                    &mutated.mutated_source,
                    &source,
                );

                entry.add_modification(mutated.mutation);
                report.add_entry(entry);
            }
        }
    }

    trace!("Saving reports to: {output_dir:?}");
    report.save_to_json_file(output_dir.join(Path::new("report.json")).as_path())?;
    report.save_to_text_file(output_dir.join(Path::new("report.txt")).as_path())?;

    trace!("Mutator tool is done here...");
    Ok(())
}

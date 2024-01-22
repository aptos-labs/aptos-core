extern crate pretty_env_logger;
#[macro_use]
extern crate log;

pub mod cli;
pub mod compiler;

mod mutate;

pub mod configuration;
mod mutant;
mod operator;
mod output;
pub mod report;

use crate::compiler::{generate_ast, verify_mutant};
use std::fs;
use std::path::Path;

use crate::configuration::Configuration;
use crate::report::Report;
use move_package::BuildConfig;
use std::path::PathBuf;

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
    options: cli::Options,
    config: &BuildConfig,
    package_path: &PathBuf,
) -> anyhow::Result<()> {
    // We need to initialize logger using try_init() as it might be already initialized in some other tool
    // (e.g. spec-test). If we use init() instead, we will get an abort.
    let _ = pretty_env_logger::try_init();

    info!(
        "Executed move-mutator with the following options: {:?} \n config: {:?} \n package path: {:?}",
        options, config, package_path
    );

    // Load configuration from file or create a new one.
    let mutator_configuration = match options.configuration_file {
        Some(path) => Configuration::from_file(path.as_path())?,
        None => Configuration::new(options, Some(package_path.clone())),
    };

    trace!("Mutator configuration: {mutator_configuration:?}");

    let (files, ast) = generate_ast(&mutator_configuration, config, package_path)?;
    let mutants = mutate::mutate(ast)?;
    let output_dir = output::setup_output_dir(&mutator_configuration)?;
    let mut report: Report = Report::new();

    for (hash, (filename, source)) in files {
        let path = Path::new(filename.as_str());

        trace!("Processing file: {path:?}");

        // Check if file is not excluded from mutant generation.
        //TODO(asmie): refactor this when proper filtering will be introduced in the M3
        if let Some(excluded) = mutator_configuration.project.exclude_files.as_ref() {
            if excluded.contains(&path.to_path_buf()) {
                continue;
            }
        }

        // Check if file is explicitly included in mutant generation (if include_only_files is set).
        if let Some(included) = mutator_configuration.project.include_only_files.as_ref() {
            if !included.contains(&path.to_path_buf()) {
                continue;
            }
        }

        let mut i = 0;
        for mutant in mutants.iter().filter(|m| m.get_file_hash() == hash) {
            let mutated_sources = mutant.apply(&source);
            for mutated in mutated_sources {
                if mutator_configuration.project.verify_mutants {
                    let res = verify_mutant(config, &mutated.mutated_source, path);

                    // In case the mutant is not a valid Move file, skip the mutant (do not save it).
                    if res.is_err() {
                        warn!(
                            "Mutant {} is not valid and will not be generated. Error: {:?}",
                            mutant, res
                        );
                        continue;
                    }
                }

                let mutant_path = output::setup_mutant_path(&output_dir, path, i)?;
                fs::write(&mutant_path, &mutated.mutated_source)?;

                info!("{} written to {}", mutant, mutant_path.display());

                let mut entry = report::MutationReport::new(
                    mutant_path.as_path(),
                    path,
                    &mutated.mutated_source,
                    &source,
                );

                entry.add_modification(mutated.mutation);
                report.add_entry(entry);
                i += 1;
            }
        }
    }

    trace!("Saving reports to: {output_dir:?}");
    report.save_to_json_file(output_dir.join(Path::new("report.json")).as_path())?;
    report.save_to_text_file(output_dir.join(Path::new("report.txt")).as_path())?;

    trace!("Mutator tool is done here...");
    Ok(())
}

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
/// # Panics
///
/// The function will panic if `downsampling_ratio_percentage` is not in the range 0..=100.
///
/// # Returns
///
/// * `anyhow::Result<()>` - Returns `Ok(())` if the mutation process completes successfully, or an error if any error occurs.
pub fn run_move_mutator(
    options: cli::CLIOptions,
    config: &BuildConfig,
    package_path: &Path,
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
        None => Configuration::new(options, Some(package_path.to_owned())),
    };

    trace!("Mutator configuration: {mutator_configuration:?}");

    let env = generate_ast(
        &mutator_configuration,
        config,
        mutator_configuration
            .project_path
            .as_ref()
            .unwrap_or(&package_path.to_owned())
            .as_path(),
    )?;

    trace!("Generated AST.");

    let mutants = mutate::mutate(&env, &mutator_configuration)?;
    let output_dir = output::setup_output_dir(&mutator_configuration)?;
    let mut report: Report = Report::new();

    for mutant in &mutants {
        let file_id = &mutant.get_file_id();
        let source = env.get_file_source(*file_id);
        let filename = env.get_file(*file_id);
        let path = Path::new(filename);

        trace!("Processing file: {path:?}");

        let mut mutated_sources = mutant.apply(source);

        // If the downsample ratio is set, we need to downsample the mutants.
        //TODO: currently we are downsampling the mutants after they are generated. This is not
        // ideal as we are generating all mutants and then removing some of them.
        if let Some(percentage) = mutator_configuration.project.downsampling_ratio_percentage {
            let no_of_mutants_to_keep = mutated_sources
                .len()
                .saturating_sub((mutated_sources.len() * percentage).div_ceil(100));
            assert!(
                no_of_mutants_to_keep <= mutated_sources.len(),
                "Invalid downsampling ratio"
            );

            // Delete randomly elements from the vector.
            let mut rng = thread_rng();
            let chosen_elements: Vec<_> = mutated_sources
                .choose_multiple(&mut rng, no_of_mutants_to_keep)
                .cloned()
                .collect();

            mutated_sources = chosen_elements;
        }

        for mutated in mutated_sources {
            if let Some(mutation_conf) = &mutator_configuration.mutation {
                if !mutation_conf.operators.is_empty()
                    && !mutation_conf
                        .operators
                        .contains(&mutated.mutation.get_operator_name().to_owned())
                {
                    continue;
                }
            }

            if mutator_configuration.project.verify_mutants {
                let res = verify_mutant(config, &mutated.mutated_source, path);

                // In case the mutant is not a valid Move file, skip the mutant (do not save it).
                if res.is_err() {
                    warn!("Mutant {mutant} is not valid and will not be generated. Error: {res:?}");
                    continue;
                }
            }

            let mutant_path = output::setup_mutant_path(&output_dir, path)?;

            fs::write(&mutant_path, &mutated.mutated_source)?;

            info!("{} written to {}", mutant, mutant_path.display());

            let mod_name = if let Some(name) = mutant.get_module_name() {
                name
            } else {
                "script".to_owned() // if there is no module name, it is a script
            };

            let mut entry = report::MutationReport::new(
                mutant_path.as_path(),
                path,
                mod_name.as_str(),
                mutant
                    .get_function_name()
                    .map_or_else(String::new, |f| f.to_string())
                    .as_str(),
                &mutated.mutated_source,
                source,
            );

            entry.add_modification(mutated.mutation);
            report.add_entry(entry);
        }
    }

    trace!("Saving reports to: {output_dir:?}");
    report.save_to_json_file(output_dir.join(Path::new("report.json")).as_path())?;
    report.save_to_text_file(output_dir.join(Path::new("report.txt")).as_path())?;

    trace!("Mutator tool is done here...");
    Ok(())
}

pub mod cli;
mod compiler;

mod mutate;

mod configuration;
mod mutant;
mod operator;
mod report;

use crate::compiler::generate_ast;
use std::path::Path;

use crate::configuration::Configuration;
use crate::report::Report;
use move_package::BuildConfig;
use std::path::PathBuf;

const DEFAULT_OUTPUT_DIR: &str = "mutants_output";

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

    let mutator_configuration = match options.configuration_file {
        Some(path) => configuration::Configuration::from_file(path.as_path())?,
        None => configuration::Configuration::new(options, Some(package_path.clone())),
    };

    let (files, ast) = generate_ast(&mutator_configuration, config, package_path)?;

    let mutants = mutate::mutate(ast)?;

    let output_dir = setup_output_dir(&mutator_configuration)?;

    let mut report: Report = Report::new();

    for (hash, (filename, source)) in files {
        let path = Path::new(filename.as_str());
        let file_name = path.file_stem().unwrap().to_str().unwrap();

        // Check if file is not excluded from mutant generation
        //TODO(asmie): refactor this when proper filtering will be introduced in the M3
        if let Some(excluded) = mutator_configuration.project.exclude_files.as_ref() {
            if excluded.contains(&path.to_path_buf()) {
                continue;
            }
        }

        // Check if file is explicitly included in mutant generation (if include_only_files is set)
        if let Some(included) = mutator_configuration.project.include_only_files.as_ref() {
            if !included.contains(&path.to_path_buf()) {
                continue;
            }
        }

        let mut i = 0;
        for mutant in mutants.iter().filter(|m| m.get_file_hash() == hash) {
            let mutated_sources = mutant.apply(&source);
            for mutated in mutated_sources {
                let mutant_path = setup_mutant_path(&output_dir, file_name, i);

                println!(
                    "{} written to {}",
                    mutant,
                    mutant_path.to_str().unwrap_or("")
                );
                std::fs::write(&mutant_path, &mutated.mutated_source)?;
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

    let report_path = PathBuf::from(output_dir);

    report.save_to_json_file(report_path.join(Path::new("report.json")).as_path())?;
    report.save_to_text_file(report_path.join(Path::new("report.txt")).as_path())?;

    Ok(())
}

/// Sets up the path for the mutant.
#[inline]
fn setup_mutant_path(output_dir: &Path, filename: &str, index: u64) -> PathBuf {
    PathBuf::from(format!(
        "{}/{}_{}.move",
        &output_dir.to_str().unwrap_or(DEFAULT_OUTPUT_DIR),
        filename,
        index
    ))
}

/// Sets up the output directory for the mutants.
fn setup_output_dir(mutator_configuration: &Configuration) -> anyhow::Result<PathBuf> {
    let output_dir = mutator_configuration
        .project
        .output_dir
        .clone()
        .unwrap_or(PathBuf::from(DEFAULT_OUTPUT_DIR));

    // Check if output directory exists and if it should be overwritten
    if output_dir.exists() && mutator_configuration.project.no_overwrite.unwrap_or(false) {
        return Err(anyhow::anyhow!(
            "Output directory already exists. Use --no-overwrite=false to overwrite."
        ));
    }

    let _ = std::fs::remove_dir_all(&output_dir);
    std::fs::create_dir(&output_dir)?;

    Ok(output_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn setup_mutant_path_creates_correct_path() {
        let output_dir = Path::new("/path/to/output");
        let filename = "test";
        let index = 1;
        let result = setup_mutant_path(output_dir, filename, index);
        assert_eq!(result, PathBuf::from("/path/to/output/test_1.move"));
    }

    #[test]
    fn setup_mutant_path_handles_empty_output_dir() {
        let output_dir = Path::new("");
        let filename = "test";
        let index = 1;
        let result = setup_mutant_path(output_dir, filename, index);
        assert_eq!(result, PathBuf::from("/test_1.move"));
    }

    #[test]
    fn setup_mutant_path_handles_empty_filename() {
        let output_dir = Path::new("/path/to/output");
        let filename = "";
        let index = 1;
        let result = setup_mutant_path(output_dir, filename, index);
        assert_eq!(result, PathBuf::from("/path/to/output/_1.move"));
    }

    #[test]
    fn setup_output_dir_creates_directory_if_not_exists() {
        let temp_dir = tempdir().unwrap();
        let output_dir = temp_dir.path().join("output");
        let options = cli::Options {
            output_dir: Some(output_dir.clone()),
            no_overwrite: Some(false),
            ..Default::default()
        };
        let config = Configuration::new(options, None);
        assert!(setup_output_dir(&config).is_ok());
        assert!(output_dir.exists());
    }

    #[test]
    fn setup_output_dir_overwrites_directory_if_exists_and_no_overwrite_is_false() {
        let temp_dir = tempdir().unwrap();
        let output_dir = temp_dir.path().join("output");
        fs::create_dir(&output_dir).unwrap();
        let options = cli::Options {
            output_dir: Some(output_dir.clone()),
            no_overwrite: Some(false),
            ..Default::default()
        };
        let config = Configuration::new(options, None);
        assert!(setup_output_dir(&config).is_ok());
        assert!(output_dir.exists());
    }

    #[test]
    fn setup_output_dir_errors_if_directory_exists_and_no_overwrite_is_true() {
        let temp_dir = tempdir().unwrap();
        let output_dir = temp_dir.path().join("output");
        fs::create_dir(&output_dir).unwrap();
        let options = cli::Options {
            output_dir: Some(output_dir.clone()),
            no_overwrite: Some(true),
            ..Default::default()
        };
        let config = Configuration::new(options, None);
        assert!(setup_output_dir(&config).is_err());
    }
}

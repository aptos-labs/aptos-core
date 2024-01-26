use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Command line options for specification test tool.
#[derive(Parser, Default, Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct CLIOptions {
    /// The paths to the Move sources.
    #[clap(long, short, value_parser)]
    pub move_sources: Vec<PathBuf>,
    /// Work only over specified modules.
    #[clap(long, short)]
    pub include_modules: Option<Vec<String>>,
    /// Optional configuration file for mutator tool.
    #[clap(long, value_parser)]
    pub mutator_conf: Option<PathBuf>,
    /// Optional configuration file for prover tool.
    #[clap(long, value_parser)]
    pub prover_conf: Option<PathBuf>,
    /// Optional configuration file for prover tool.
    #[clap(short, long, value_parser)]
    pub output: Option<PathBuf>,
    /// Extra arguments to pass to the prover.
    #[clap(long, value_parser)]
    pub extra_prover_args: Option<Vec<String>>,
}

/// This function creates a mutator CLI options from the given spec-test options.
#[must_use]
pub fn create_mutator_options(options: &CLIOptions) -> move_mutator::cli::CLIOptions {
    move_mutator::cli::CLIOptions {
        move_sources: options.move_sources.clone(),
        mutate_modules: options.include_modules.clone(),
        configuration_file: options.mutator_conf.clone(),
        ..Default::default()
    }
}

/// This function generates a prover CLI options from the given spec-test options.
///
/// # Errors
/// Errors are returned as `anyhow::Result`.
pub fn generate_prover_options(options: &CLIOptions) -> anyhow::Result<move_prover::cli::Options> {
    let prover_conf = if let Some(conf) = &options.prover_conf {
        move_prover::cli::Options::create_from_toml_file(conf.to_str().unwrap_or(""))?
    } else if let Some(args) = &options.extra_prover_args {
        move_prover::cli::Options::create_from_args(args)?
    } else {
        move_prover::cli::Options::default()
    };

    Ok(prover_conf)
}

/// This function checks if the mutator output path is provided in the configuration file.
/// We don't need to check if the mutator output path is provided in the options as they were created
/// from the spec-test options which does not allow setting it.
#[must_use]
pub fn check_mutator_output_path(options: &move_mutator::cli::CLIOptions) -> Option<PathBuf> {
    if let Some(conf) = &options.configuration_file {
        let c = move_mutator::configuration::Configuration::from_file(conf);
        if let Ok(c) = c {
            return c.project.out_mutant_dir;
        }
    };

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn cli_options_starts_empty() {
        let options = CLIOptions::default();
        assert!(options.move_sources.is_empty());
        assert!(options.include_modules.is_none());
        assert!(options.mutator_conf.is_none());
        assert!(options.prover_conf.is_none());
        assert!(options.output.is_none());
        assert!(options.extra_prover_args.is_none());
    }

    #[test]
    fn create_mutator_options_copies_fields() {
        let mut options = CLIOptions::default();
        options.move_sources.push(PathBuf::from("path/to/file"));
        options.include_modules = Some(vec!["test1".to_string(), "test2".to_string()]);
        options.mutator_conf = Some(PathBuf::from("path/to/mutator/conf"));

        let mutator_options = create_mutator_options(&options);

        assert_eq!(mutator_options.move_sources, options.move_sources);
        assert_eq!(mutator_options.mutate_modules, options.include_modules);
        assert_eq!(mutator_options.configuration_file, options.mutator_conf);
    }

    #[test]
    fn check_mutator_output_path_returns_none_when_no_conf() {
        let options = move_mutator::cli::CLIOptions::default();
        assert!(check_mutator_output_path(&options).is_none());
    }

    #[test]
    fn check_mutator_output_path_returns_path_when_conf_exists() {
        let json_content = r#"
            {
                "project": {
                    "move_sources": ["/path/to/move/source"],
                    "out_mutant_dir": "path/to/out_mutant_dir"
                },
                "project_path": "/path/to/project"
            }
        "#;

        fs::write("test_mutator_conf.json", json_content).unwrap();

        let mut options = move_mutator::cli::CLIOptions::default();
        options.configuration_file = Some(PathBuf::from("test_mutator_conf.json"));

        let path = check_mutator_output_path(&options);
        fs::remove_file("test_mutator_conf.json").unwrap();

        assert!(path.is_some());
        assert_eq!(path.unwrap(), PathBuf::from("path/to/out_mutant_dir"));
    }
}

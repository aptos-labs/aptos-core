use clap::{Arg, Command};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Command line options for mutator
#[derive(Default, Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Options {
    /// The paths to the Move sources.
    pub move_sources: Vec<String>,
    /// The paths to the Move sources to include.
    pub include_only_files: Option<Vec<PathBuf>>,
    /// The paths to the Move sources to exclude.
    pub exclude_files: Option<Vec<PathBuf>>,
    /// The path where to put the output files.
    pub output_dir: Option<PathBuf>,
    /// Indicates if mutants should be verified and made sure mutants can compile.
    pub verify_mutants: Option<bool>,
    /// Indicates if the output files should be overwritten.
    pub no_overwrite: Option<bool>,
    /// Name of the filter to use for down sampling.
    pub downsample_filter: Option<String>,
    /// Optional configuration file. If provided, it will override the default configuration.
    pub configuration_file: Option<PathBuf>,
    /// Indicates if the output should be verbose.
    pub verbose: Option<bool>,
}

impl Options {
    /// Creates Options struct from command line arguments.
    pub fn create_from_args(args: &[String]) -> anyhow::Result<Options> {
        //TODO: this code need to be updated to use clap parser directly
        let cli = Command::new("mutate")
            .version("0.1.0")
            .about("The Move Mutator")
            .author("Eiger Team")
            .arg(
                Arg::new("sources")
                    .num_args(1..)
                    .value_name("PATH_TO_SOURCE_FILE")
                    .help("the source files to mutate"),
            )
            .after_help("See `move-mutator/doc` and `README.md` for documentation.");

        // Parse the arguments. This will abort the program on parsing errors and print help.
        // It will also accept options like --help.
        let matches = cli.get_matches_from(args);

        // Initialize options.
        let get_vec = |s: &str| -> Vec<String> {
            match matches.get_many::<String>(s) {
                Some(vs) => vs.map(|v| v.to_string()).collect(),
                _ => vec![],
            }
        };

        let mut options = Options::default();

        if matches
            .get_many::<String>("sources")
            .unwrap_or_default()
            .len()
            > 0
        {
            options.move_sources = get_vec("sources");
        }

        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_from_args() {
        let args = vec![
            String::from("mutate"),
            String::from("src/main.rs"),
            String::from("src/lib.rs"),
        ];

        let options = Options::create_from_args(&args).unwrap();
        assert_eq!(options.move_sources, vec!["src/main.rs", "src/lib.rs"]);
    }
}

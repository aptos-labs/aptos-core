use clap::{Arg, Command};
use serde::{Deserialize, Serialize};

/// Move mutator options
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Options {
    /// The paths to the Move sources.
    pub move_sources: Vec<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            move_sources: vec![],
        }
    }
}

impl Options {
    /// Creates options from toml configuration source.
    pub fn create_from_toml(toml_source: &str) -> anyhow::Result<Options> {
        Ok(toml::from_str(toml_source)?)
    }

    /// Creates options from toml configuration file.
    pub fn create_from_toml_file(toml_file: &str) -> anyhow::Result<Options> {
        Self::create_from_toml(&std::fs::read_to_string(toml_file)?)
    }

    /// Creates Options struct from command line arguments.
    pub fn create_from_args(args: &[String]) -> anyhow::Result<Options> {
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

/// Runs the Move mutator tool.
/// Entry point for the Move mutator tool both for the CLI and the Rust API.
pub fn run_move_mutator(options: Options) -> anyhow::Result<()> {
    println!(
        "Executed move-mutator with the following options: {:?}",
        options
    );

    Ok(())
}

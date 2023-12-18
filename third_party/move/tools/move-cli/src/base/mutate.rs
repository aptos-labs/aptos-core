use clap::*;
use move_package::BuildConfig;
use std::path::PathBuf;

/// Move mutator-specific options.
#[derive(Parser, Debug)]
pub enum MutatorOptions {
    // Pass through unknown commands to the mutator Clap parser
    #[clap(
        external_subcommand,
        takes_value(true),
        multiple_values(true),
        multiple_occurrences(true)
    )]
    Options(Vec<String>),
}

/// Mutate the Move files or package
#[derive(Parser)]
#[clap(name = "mutate")]
pub struct Mutate {
    /// Any options passed to the move-mutator
    #[clap(subcommand)]
    pub options: Option<MutatorOptions>,
}

impl Mutate {
    /// Executes the mutate command which produces mutants from the Move files or package using
    /// the provided configuration.
    /// If no path is provided, the current directory is used.
    pub fn execute(self, path: Option<PathBuf>, config: BuildConfig) -> anyhow::Result<()> {
        let path = path.unwrap_or_else(|| PathBuf::from("."));

        let Self { options } = self;

        let opts = match options {
            Some(MutatorOptions::Options(opts)) => opts,
            _ => vec![],
        };

        let options = move_mutator::cli::Options::create_from_args(&opts)?;
        move_mutator::run_move_mutator(options, config, path)
    }
}

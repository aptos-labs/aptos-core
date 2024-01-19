use clap::*;
use move_package::BuildConfig;
use std::path::PathBuf;

/// Mutate the Move files or package
#[derive(Parser)]
#[clap(name = "mutate")]
pub struct Mutate {
    /// Any options passed to the move-mutator
    #[clap(flatten)]
    pub options: Option<move_mutator::cli::Options>,
}

impl Mutate {
    /// Executes the mutate command which produces mutants from the Move files or package using
    /// the provided configuration.
    /// If no path is provided, the current directory is used.
    pub fn execute(self, path: Option<PathBuf>, config: BuildConfig) -> anyhow::Result<()> {
        let path = path.unwrap_or_else(|| PathBuf::from("."));

        let Self { options } = self;

        let options = options.unwrap_or_default();

        move_mutator::run_move_mutator(options, &config, &path)
    }
}

use clap::*;
use move_package::BuildConfig;
use std::path::PathBuf;

/// Test the Move specification using the Move Mutator and Move Prover
#[derive(Parser)]
#[clap(name = "spec-test")]
pub struct SpecTest {
    /// Any options passed to the move-spec-test
    #[clap(flatten)]
    pub options: Option<move_spec_test::cli::Options>,
}

impl SpecTest {
    /// Executes the spec-test command which produces mutants from the Move files or package using
    /// the provided configuration. Then it passes the mutants to the Move prover to check if the
    /// mutants are killed by the prover.
    /// If no path is provided, the current directory is used.
    pub fn execute(self, path: Option<PathBuf>, config: BuildConfig) -> anyhow::Result<()> {
        let path = path.unwrap_or_else(|| PathBuf::from("."));

        let Self { options } = self;

        let options = options.unwrap_or_default();

        move_spec_test::run_spec_test(options, config, path)
    }
}

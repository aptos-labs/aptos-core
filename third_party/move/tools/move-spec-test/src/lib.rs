pub mod cli;

extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use move_package::BuildConfig;
use std::path::PathBuf;

/// This function runs the specification testing, which is a combination of the
/// mutator tool and the prover tool
/// It takes the CLI options and constructs appropriate options for the
/// Move Mutator tool and Move Prover tool. Then it mutates the code storing
/// results in a temporary directory. Then it runs the prover on the mutated
/// code and remember the results, using them to generate the report at the end.
///
/// # Arguments
///
/// * `options` - A `cli::Options` representing the options for the spec test.
/// * `config` - A `BuildConfig` representing the build configuration.
/// * `package_path` - A `PathBuf` representing the path to the package.
///
/// # Returns
///
/// * `anyhow::Result<()>` - The result of the spec test.
pub fn run_spec_test(
    _options: cli::Options,
    _config: BuildConfig,
    _package_path: PathBuf,
) -> anyhow::Result<()> {
    pretty_env_logger::init();

    info!("Running spec test");

    // TODO: implement

    Ok(())
}

use move_package::{BuildConfig, ModelConfig};
use std::path::Path;
use std::time::Instant;
use termcolor::WriteColor;

/// The `prove` function is responsible for proving the package.
///
/// # Arguments
///
/// * `config` - A `BuildConfig` representing the build configuration.
/// * `package_path` - A `Path` to the package.
/// * `prover_conf` - `move_prover::cli::Options` the options for the prover.
/// * `error_writer` - `&mut dyn std::io::Write` the error writer.
///
/// # Returns
///
/// * `anyhow::Result<()>` - The result of the proving process.
pub(crate) fn prove<W: WriteColor>(
    config: &BuildConfig,
    package_path: &Path,
    prover_conf: &move_prover::cli::Options,
    mut error_writer: &mut W,
) -> anyhow::Result<()> {
    let model = config.clone().move_model_for_package(
        package_path,
        ModelConfig {
            all_files_as_targets: true,
            target_filter: None,
        },
    )?;

    let mut prover_conf = prover_conf.clone();
    prover_conf.output_path = package_path
        .to_path_buf()
        .join("output.bpl")
        .to_str()
        .unwrap_or("")
        .to_string();

    let now = Instant::now();

    move_prover::run_move_prover_with_model(&model, &mut error_writer, prover_conf, Some(now))
}

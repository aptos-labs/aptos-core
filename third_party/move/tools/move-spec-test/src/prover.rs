use anyhow::anyhow;
use move_package::{BuildConfig, ModelConfig};
use std::fs;
use std::path::Path;
use std::time::Instant;
use termcolor::WriteColor;

/// The `prove_mutant` function is responsible for setting up the output
/// directory and calling function proving the mutant.
///
/// # Arguments
///
/// * `config` - A `BuildConfig` representing the build configuration.
/// * `mutant_file` - `Path` the path to the mutant file.
/// * `original_file` - `Path` the path to the original file.
/// * `package_path` - `Path` the path to the package.
/// * `prover_conf` - `move_prover::cli::Options` the options for the prover.
/// * `outdir_prove` - `Path` the path to the output directory for proving.
/// * `error_writer` - `&mut dyn std::io::Write` representing the error writer.
///
/// # Returns
///
/// * `anyhow::Result<()>` - The result of the proving process.
pub(crate) fn prove_mutant<W: WriteColor>(
    config: &BuildConfig,
    mutant_file: &Path,
    original_file: &Path,
    package_path: &Path,
    prover_conf: &move_prover::cli::Options,
    outdir_prove: &Path,
    mut error_writer: &mut W,
) -> anyhow::Result<()> {
    debug!("Original file: {:?}", original_file);
    debug!("Mutant file: {:?}", mutant_file);

    let _ = fs::remove_dir_all(&outdir_prove);
    move_mutator::compiler::copy_dir_all(&package_path, &outdir_prove)?;

    trace!(
        "Copying mutant file {:?} to the package directory {:?}",
        mutant_file,
        outdir_prove.join(original_file)
    );

    if let Err(res) = fs::copy(mutant_file, outdir_prove.join(original_file)) {
        let msg = format!("Can't copy mutant file to the package directory: {:?}", res);
        warn!("{msg}");
        return Err(anyhow!(msg));
    }

    prove(&config, &outdir_prove, &prover_conf, &mut error_writer)
}

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

    let now = Instant::now();

    move_prover::run_move_prover_with_model(
        &model,
        &mut error_writer,
        prover_conf.clone(),
        Some(now),
    )
}

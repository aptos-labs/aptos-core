mod benchmark;
pub mod cli;
mod prover;
mod report;

extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use crate::benchmark::{Benchmark, Benchmarks};
use crate::prover::prove;
use anyhow::anyhow;
use move_package::source_package::layout::SourcePackageLayout;
use move_package::BuildConfig;
use std::fs;
use std::path::{Path, PathBuf};

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
/// # Errors
///
/// Errors are returned as `anyhow::Result`.
///
/// # Returns
///
/// * `anyhow::Result<()>` - The result of the spec test.
pub fn run_spec_test(
    options: &cli::CLIOptions,
    config: &BuildConfig,
    package_path: &Path,
) -> anyhow::Result<()> {
    // We need to initialize logger using try_init() as it might be already initialized in some other tool
    // (e.g. spec-test). If we use init() instead, we will get an abort.
    let _ = pretty_env_logger::try_init();

    // Check if package is correctly structured.
    let package_path = SourcePackageLayout::try_find_root(&package_path.canonicalize()?)?;

    info!("Running specification tester with the following options: {options:?} and package path: {package_path:?}");

    // Always create and use benchmarks.
    // Benchmarks call only time getting functions, so it's safe to use them in any case and
    // they are not expensive to create (won't hit the performance).
    let mut benchmarks = Benchmarks::new();
    benchmarks.spec_test.start();

    let prover_conf = cli::generate_prover_options(options)?;

    // Setup temporary directory structure.
    let outdir = tempfile::tempdir()?.into_path();
    let outdir_original = outdir.join("base");

    fs::create_dir_all(&outdir_original)?;

    let outdir_mutant = if let Some(mutant_path) = &options.use_generated_mutants {
        mutant_path.clone()
    } else {
        benchmarks.mutator.start();
        let outdir_mutant = run_mutator(options, config, &package_path, &outdir)?;
        benchmarks.mutator.stop();
        outdir_mutant
    };

    let report =
        move_mutator::report::Report::load_from_json_file(&outdir_mutant.join("report.json"))?;

    // Proving part.
    move_mutator::compiler::copy_dir_all(&package_path, &outdir_original)?;

    let mut error_writer = termcolor::StandardStream::stderr(termcolor::ColorChoice::Auto);

    let result = prove(config, &package_path, &prover_conf, &mut error_writer);

    if let Err(e) = result {
        let msg = format!("Original code verification failed! Prover failed with error: {e}");
        error!("{msg}");
        return Err(anyhow!(msg));
    }

    let mut spec_report = report::Report::new();

    let mut proving_benchmarks = vec![Benchmark::new(); report.get_mutants().len()];
    benchmarks.prover.start();
    for (elem, benchmark) in report
        .get_mutants()
        .iter()
        .zip(proving_benchmarks.iter_mut())
    {
        let mutant_file = elem.mutant_path();
        // Strip prefix to get the path relative to the package directory (or take that path if it's already relative).
        let original_file = elem
            .original_file_path()
            .strip_prefix(&package_path)
            .unwrap_or(elem.original_file_path());
        let outdir_prove = outdir.join("prove");

        spec_report.increment_mutants_tested(original_file, elem.get_module_name());

        let _ = fs::remove_dir_all(&outdir_prove);
        move_mutator::compiler::copy_dir_all(&package_path, &outdir_prove)?;

        trace!(
            "Copying mutant file {:?} to the package directory {:?}",
            mutant_file,
            outdir_prove.join(original_file)
        );

        if let Err(res) = fs::copy(mutant_file, outdir_prove.join(original_file)) {
            return Err(anyhow!(
                "Can't copy mutant file to the package directory: {res:?}"
            ));
        }

        benchmark.start();
        let result = prove(config, &outdir_prove, &prover_conf, &mut error_writer);
        benchmark.stop();

        if let Err(e) = result {
            trace!("Mutant killed! Prover failed with error: {e}");
            spec_report.increment_mutants_killed(original_file, elem.get_module_name());
        } else {
            trace!("Mutant hasn't been killed!");
            spec_report.add_mutants_alive_diff(
                original_file,
                elem.get_module_name(),
                elem.get_diff(),
            );
        }
    }

    benchmarks.prover.stop();
    benchmarks.prover_results = proving_benchmarks;

    if let Some(outfile) = &options.output {
        spec_report.save_to_json_file(outfile)?;
    }

    println!("\nTotal mutants tested: {}", spec_report.mutants_tested());
    println!("Total mutants killed: {}\n", spec_report.mutants_killed());
    spec_report.print_table();

    benchmarks.spec_test.stop();
    benchmarks.display();

    Ok(())
}

/// This function runs the Move Mutator tool.
fn run_mutator(
    options: &cli::CLIOptions,
    config: &BuildConfig,
    package_path: &Path,
    outdir: &Path,
) -> anyhow::Result<PathBuf> {
    debug!("Running the move mutator tool");
    let mut mutator_conf = cli::create_mutator_options(options);

    let outdir_mutant = if let Some(path) = cli::check_mutator_output_path(&mutator_conf) {
        path
    } else {
        mutator_conf.out_mutant_dir = Some(outdir.join("mutants"));
        mutator_conf.out_mutant_dir.clone().unwrap()
    };

    fs::create_dir_all(&outdir_mutant)?;
    move_mutator::run_move_mutator(mutator_conf, config, package_path)?;

    Ok(outdir_mutant)
}

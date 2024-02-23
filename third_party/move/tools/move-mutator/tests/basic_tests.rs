use move_mutator::cli::{CLIOptions, ModuleFilter};
use move_package::BuildConfig;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

const PACKAGE_PATHS: &[&str] = &[
    "tests/move-assets/breakcontinue",
    "tests/move-assets/poor_spec",
    "tests/move-assets/basic_coin",
    "tests/move-assets/relative_dep/p2",
    "tests/move-assets/same_names",
    "tests/move-assets/simple",
];

// Check if the mutator works correctly on the basic packages.
// It should generate a report with mutants.
#[test]
fn check_mutator_works_correctly() {
    let outdir = tempdir().unwrap().into_path();

    let options = CLIOptions {
        move_sources: vec![],
        mutate_modules: ModuleFilter::All,
        out_mutant_dir: Some(outdir.clone()),
        verify_mutants: false,
        no_overwrite: false,
        downsample_filter: None,
        downsampling_ratio_percentage: None,
        configuration_file: None,
    };

    let config = BuildConfig::default();

    for package_path in PACKAGE_PATHS {
        let package_path = Path::new(package_path);

        let result = move_mutator::run_move_mutator(options.clone(), &config, package_path);
        assert!(result.is_ok());

        let report_path = outdir.join("report.json");
        assert!(report_path.exists());

        let report = move_mutator::report::Report::load_from_json_file(&report_path).unwrap();
        assert!(!report.get_mutants().is_empty());
    }
}

#[test]
fn check_mutator_verify_mutants_correctly() {
    let outdir = tempdir().unwrap().into_path();

    let options = CLIOptions {
        move_sources: vec![],
        mutate_modules: ModuleFilter::All,
        out_mutant_dir: Some(outdir.clone()),
        verify_mutants: true,
        no_overwrite: false,
        downsample_filter: None,
        downsampling_ratio_percentage: None,
        configuration_file: None,
    };

    let config = BuildConfig::default();

    let package_path = Path::new(PACKAGE_PATHS[1]);

    let result = move_mutator::run_move_mutator(options.clone(), &config, package_path);
    assert!(result.is_ok());

    let report_path = outdir.join("report.json");
    assert!(report_path.exists());

    let report = move_mutator::report::Report::load_from_json_file(&report_path).unwrap();
    assert!(!report.get_mutants().is_empty());
}

// Check if the mutator fails on non-existing input path.
#[test]
fn check_mutator_fails_on_non_existing_path() {
    let outdir = tempdir().unwrap().into_path();

    let options = CLIOptions {
        move_sources: vec![],
        mutate_modules: ModuleFilter::All,
        out_mutant_dir: Some(outdir.clone()),
        verify_mutants: false,
        no_overwrite: false,
        downsample_filter: None,
        downsampling_ratio_percentage: None,
        configuration_file: None,
    };

    let config = BuildConfig::default();

    let package_path = PathBuf::from("/very/random/path");

    let result = move_mutator::run_move_mutator(options.clone(), &config, &package_path);
    assert!(result.is_err());
}

// Check if the mutator fails on non-existing output path.
#[test]
fn check_mutator_fails_on_non_existing_output_path() {
    let options = CLIOptions {
        move_sources: vec![],
        mutate_modules: ModuleFilter::All,
        out_mutant_dir: Some("/very/bad/path".into()),
        verify_mutants: false,
        no_overwrite: false,
        downsample_filter: None,
        downsampling_ratio_percentage: None,
        configuration_file: None,
    };

    let config = BuildConfig::default();

    let package_path = Path::new(PACKAGE_PATHS[0]);

    let result = move_mutator::run_move_mutator(options.clone(), &config, &package_path);
    assert!(result.is_err());
}

// Check if the mutator works with single files.
#[test]
fn check_mutator_works_with_single_files() {
    let outdir = tempdir().unwrap().into_path();

    let options = CLIOptions {
        move_sources: vec!["tests/move-assets/file_without_package/Sub.move".into()],
        mutate_modules: ModuleFilter::All,
        out_mutant_dir: Some(outdir.clone()),
        verify_mutants: false,
        no_overwrite: false,
        downsample_filter: None,
        downsampling_ratio_percentage: None,
        configuration_file: None,
    };

    let config = BuildConfig::default();

    let package_path = Path::new(".");

    let result = move_mutator::run_move_mutator(options.clone(), &config, &package_path);
    assert!(result.is_ok());

    let report_path = outdir.join("report.json");
    assert!(report_path.exists());

    let report = move_mutator::report::Report::load_from_json_file(&report_path).unwrap();
    assert!(!report.get_mutants().is_empty());
}

// Check if the mutator produce zero mutants if verification is enabled for
// files without any package (we're unable to verify such files successfully).
#[test]
fn check_mutator_fails_verify_file_without_package() {
    let outdir = tempdir().unwrap().into_path();

    let options = CLIOptions {
        move_sources: vec!["tests/move-assets/file_without_package/Sub.move".into()],
        mutate_modules: ModuleFilter::All,
        out_mutant_dir: Some(outdir.clone()),
        verify_mutants: true,
        no_overwrite: false,
        downsample_filter: None,
        downsampling_ratio_percentage: None,
        configuration_file: None,
    };

    let config = BuildConfig::default();

    let package_path = Path::new(".");

    let result = move_mutator::run_move_mutator(options.clone(), &config, &package_path);
    assert!(result.is_ok());

    let report_path = outdir.join("report.json");
    assert!(report_path.exists());

    let report = move_mutator::report::Report::load_from_json_file(&report_path).unwrap();
    assert!(report.get_mutants().is_empty());
}

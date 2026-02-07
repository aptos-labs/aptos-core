// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    common::types::{CliCommand, CliError, CliResult, CliTypedResult, MovePackageOptions},
    move_tool::fix_bytecode_version,
};
use aptos_framework::extended_checks;
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use legacy_move_compiler::compiled_unit::{CompiledUnit, NamedCompiledModule};
use move_coverage::{
    coverage_map::CoverageMap,
    format_csv_summary, format_human_summary,
    source_coverage::{ColorChoice, SourceCoverageBuilder, TextIndicator},
    summary::summarize_inst_cov,
};
use move_disassembler::disassembler::Disassembler;
use move_model::metadata::{CompilerVersion, LanguageVersion};
use move_package::{compilation::compiled_package::CompiledPackage, BuildConfig, CompilerConfig};
use std::path::PathBuf;

/// Options common to all coverage commands
#[derive(Debug, Parser, Default)]
pub struct CoverageCommon {
    /// Path to extra Move coverage files (`.mvcov`) to include, in addition to, or instead
    /// of the file produced by unit tests run with `--coverage` and stored at
    /// `./.coverage_map.mvcov`.
    #[arg(long, num_args = 0..)]
    extra_coverage: Vec<PathBuf>,
}

/// Display a coverage summary for all modules in a package
///
#[derive(Debug, Parser)]
pub struct SummaryCoverage {
    /// Display function coverage summaries
    ///
    /// When provided, it will include coverage on a function level
    #[clap(long)]
    pub summarize_functions: bool,
    /// Output CSV data of coverage
    #[clap(long = "csv")]
    pub output_csv: bool,
    /// A filter string to determine which unit tests to compute coverage on
    #[clap(long, short)]
    pub filter: Option<String>,
    #[clap(flatten)]
    pub common: CoverageCommon,
    #[clap(flatten)]
    pub move_options: MovePackageOptions,
}

impl SummaryCoverage {
    pub fn coverage(self) -> CliTypedResult<()> {
        let (coverage_map, package) = compile_coverage(self.common, self.move_options)?;
        let modules: Vec<_> = package
            .root_modules()
            .filter_map(|unit| {
                let mut retain = true;
                if let Some(filter_str) = &self.filter {
                    if !&unit.unit.name().as_str().contains(filter_str.as_str()) {
                        retain = false;
                    }
                }
                match &unit.unit {
                    CompiledUnit::Module(NamedCompiledModule { module, .. }) if retain => {
                        Some(module.clone())
                    },
                    _ => None,
                }
            })
            .collect();
        let coverage_map = coverage_map.to_unified_exec_map();
        if self.output_csv {
            format_csv_summary(
                modules.as_slice(),
                &coverage_map,
                summarize_inst_cov,
                &mut std::io::stdout(),
            )
        } else {
            format_human_summary(
                modules.as_slice(),
                &coverage_map,
                summarize_inst_cov,
                &mut std::io::stdout(),
                self.summarize_functions,
            )
        }
        Ok(())
    }
}

#[async_trait]
impl CliCommand<()> for SummaryCoverage {
    fn command_name(&self) -> &'static str {
        "SummaryCoverage"
    }

    async fn execute(self) -> CliTypedResult<()> {
        self.coverage()
    }
}

/// Display coverage information about the module against source code
#[derive(Debug, Parser)]
pub struct SourceCoverage {
    /// Show coverage for the given module
    #[clap(long = "module")]
    pub module_name: String,

    /// Colorize output based on coverage
    #[clap(long, default_value_t = ColorChoice::Default)]
    pub color: ColorChoice,

    /// Tag each line with a textual indication of coverage
    #[clap(long, default_value_t = TextIndicator::Explicit)]
    pub tag: TextIndicator,

    #[clap(flatten)]
    pub common: CoverageCommon,

    #[clap(flatten)]
    pub move_options: MovePackageOptions,
}

#[async_trait]
impl CliCommand<()> for SourceCoverage {
    fn command_name(&self) -> &'static str {
        "SourceCoverage"
    }

    async fn execute(self) -> CliTypedResult<()> {
        let (coverage_map, package) = compile_coverage(self.common, self.move_options)?;
        let unit = package.get_module_by_name_from_root(&self.module_name)?;
        let source_path = &unit.source_path;
        let source_map = match &unit.unit {
            CompiledUnit::Module(NamedCompiledModule { source_map, .. }) => source_map,
            _ => panic!("Should all be modules"),
        };
        let root_modules: Vec<_> = package
            .root_modules()
            .map(|unit| match &unit.unit {
                CompiledUnit::Module(NamedCompiledModule {
                    module, source_map, ..
                }) => (module, source_map),
                _ => unreachable!("Should all be modules"),
            })
            .collect();
        let source_coverage = SourceCoverageBuilder::new(&coverage_map, source_map, root_modules);
        let source_coverage = source_coverage.compute_source_coverage(source_path);
        let output_result =
            source_coverage.output_source_coverage(&mut std::io::stdout(), self.color, self.tag);
        output_result
            .map_err(|err| CliError::UnexpectedError(format!("Failed to get coverage {}", err)))
    }
}

/// Display coverage information about the module against disassembled bytecode
#[derive(Debug, Parser)]
pub struct BytecodeCoverage {
    #[clap(long = "module")]
    pub module_name: String,

    #[clap(flatten)]
    pub common: CoverageCommon,

    #[clap(flatten)]
    pub move_options: MovePackageOptions,
}

#[async_trait]
impl CliCommand<()> for BytecodeCoverage {
    fn command_name(&self) -> &'static str {
        "BytecodeCoverage"
    }

    async fn execute(self) -> CliTypedResult<()> {
        let (coverage_map, package) = compile_coverage(self.common, self.move_options)?;
        let unit = package.get_module_by_name_from_root(&self.module_name)?;
        let mut disassembler = Disassembler::from_unit(&unit.unit);
        disassembler.add_coverage_map(coverage_map.to_unified_exec_map());
        println!("{}", disassembler.disassemble()?);
        Ok(())
    }
}

fn compile_coverage(
    coverage_common: CoverageCommon,
    move_options: MovePackageOptions,
) -> CliTypedResult<(CoverageMap, CompiledPackage)> {
    let config = BuildConfig {
        dev_mode: move_options.dev,
        additional_named_addresses: move_options.named_addresses(),
        test_mode: false,
        full_model_generation: !move_options.skip_checks_on_test_code,
        install_dir: move_options.output_dir.clone(),
        skip_fetch_latest_git_deps: move_options.skip_fetch_latest_git_deps,
        compiler_config: CompilerConfig {
            known_attributes: extended_checks::get_all_attribute_names().clone(),
            skip_attribute_checks: move_options.skip_attribute_checks,
            bytecode_version: fix_bytecode_version(
                move_options.bytecode_version,
                move_options.language_version,
            ),
            compiler_version: move_options
                .compiler_version
                .or_else(|| Some(CompilerVersion::latest_stable())),
            language_version: move_options
                .language_version
                .or_else(|| Some(LanguageVersion::latest_stable())),
            experiments: move_options.compute_experiments(),
            print_errors: true,
        },
        ..Default::default()
    };

    let read_cov_file = |path: &PathBuf| {
        CoverageMap::from_binary_file(path).map_err(|err| {
            CliError::UnexpectedError(format!("Failed to retrieve coverage map {}", err))
        })
    };
    let path = move_options.get_package_path()?;
    let unit_cov_file = path.join(".coverage_map.mvcov");
    let mut cov_files = if unit_cov_file.exists() {
        vec![&unit_cov_file]
    } else {
        vec![]
    };
    cov_files.extend(coverage_common.extra_coverage.iter());
    if cov_files.is_empty() {
        return Err(CliError::CommandArgumentError(
            "expected previous run of \
        `aptos move test --coverage` to have stored coverage map at \
        `<package>/.coverage_map.mvcov`, or alternatively coverage maps provided via \
        `--extra-coverage`"
                .to_owned(),
        ));
    }
    let mut cov_map = read_cov_file(cov_files[0])?;
    for file in cov_files.into_iter().skip(1) {
        cov_map.merge(read_cov_file(file)?);
    }
    let package = config
        .compile_package(path.as_path(), &mut Vec::new())
        .map_err(|err| CliError::MoveCompilationError(err.to_string()))?;

    Ok((cov_map, package))
}

/// Computes coverage for a package
///
/// Computes coverage on a previous unit test run for a package.  Coverage input must
/// first be built with `aptos move test --coverage`
#[derive(Subcommand)]
pub enum CoveragePackage {
    Summary(SummaryCoverage),
    Source(SourceCoverage),
    Bytecode(BytecodeCoverage),
}

impl CoveragePackage {
    pub async fn execute(self) -> CliResult {
        match self {
            Self::Summary(tool) => tool.execute_serialized_success().await,
            Self::Source(tool) => tool.execute_serialized_success().await,
            Self::Bytecode(tool) => tool.execute_serialized_success().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use move_core_types::{account_address::AccountAddress, identifier::Identifier};
    use move_coverage::{
        coverage_map::{CoverageMap, ModuleCoverageMap},
        summary::{FunctionSummary, ModuleSummary},
    };
    use std::collections::BTreeMap;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Verify CLI argument structure for SummaryCoverage
    #[test]
    fn verify_summary_coverage_cli() {
        SummaryCoverage::command().debug_assert();
    }

    /// Verify CLI argument structure for SourceCoverage
    #[test]
    fn verify_source_coverage_cli() {
        SourceCoverage::command().debug_assert();
    }

    /// Verify CLI argument structure for BytecodeCoverage
    #[test]
    fn verify_bytecode_coverage_cli() {
        BytecodeCoverage::command().debug_assert();
    }

    /// Test that format_human_summary produces expected output
    #[test]
    fn test_format_human_summary_output() {
        use legacy_move_compiler::compiled_unit::{CompiledUnit, NamedCompiledModule};
        use move_binary_format::file_format;

        // Create a simple test module
        let mut module = file_format::empty_module();
        module.identifiers[0] = Identifier::new("TestModule").unwrap();

        // Add a function to the module
        module.function_handles.push(file_format::FunctionHandle {
            module: file_format::ModuleHandleIndex(0),
            name: file_format::IdentifierIndex(module.identifiers.len() as u16),
            parameters: file_format::SignatureIndex(0),
            return_: file_format::SignatureIndex(0),
            type_parameters: vec![],
            access_specifiers: None,
            attributes: vec![],
        });
        module
            .identifiers
            .push(Identifier::new("test_func").unwrap());

        module.function_defs.push(file_format::FunctionDefinition {
            function: file_format::FunctionHandleIndex(0),
            visibility: file_format::Visibility::Private,
            is_entry: false,
            acquires_global_resources: vec![],
            code: Some(file_format::CodeUnit {
                locals: file_format::SignatureIndex(0),
                code: vec![
                    file_format::Bytecode::LdU64(0),
                    file_format::Bytecode::Pop,
                    file_format::Bytecode::Ret,
                ],
            }),
        });

        let modules = vec![module.clone()];

        // Create a coverage map with partial coverage
        let mut coverage_map = CoverageMap::default();
        let addr = AccountAddress::ZERO;
        let module_name = Identifier::new("TestModule").unwrap();
        let func_name = Identifier::new("test_func").unwrap();

        // Cover 2 of 3 instructions
        coverage_map.insert("exec", addr, module_name.clone(), func_name.clone(), 0);
        coverage_map.insert("exec", addr, module_name.clone(), func_name.clone(), 1);

        let unified_map = coverage_map.to_unified_exec_map();

        // Format the summary
        let mut output = Vec::new();
        format_human_summary(
            modules.as_slice(),
            &unified_map,
            summarize_inst_cov,
            &mut output,
            true, // summarize_functions
        );

        let output_str = String::from_utf8(output).unwrap();

        // Verify output contains expected elements
        assert!(
            output_str.contains("Move Coverage Summary"),
            "Output should contain header"
        );
        assert!(
            output_str.contains("TestModule"),
            "Output should contain module name"
        );
        assert!(
            output_str.contains("test_func"),
            "Output should contain function name"
        );
        assert!(
            output_str.contains("total: 3"),
            "Output should show 3 total instructions"
        );
        assert!(
            output_str.contains("covered: 2"),
            "Output should show 2 covered instructions"
        );
    }

    /// Test that format_csv_summary produces valid CSV
    #[test]
    fn test_format_csv_summary_output() {
        use move_binary_format::file_format;

        // Create a simple test module
        let mut module = file_format::empty_module();
        module.identifiers[0] = Identifier::new("TestModule").unwrap();

        module.function_handles.push(file_format::FunctionHandle {
            module: file_format::ModuleHandleIndex(0),
            name: file_format::IdentifierIndex(module.identifiers.len() as u16),
            parameters: file_format::SignatureIndex(0),
            return_: file_format::SignatureIndex(0),
            type_parameters: vec![],
            access_specifiers: None,
            attributes: vec![],
        });
        module.identifiers.push(Identifier::new("my_func").unwrap());

        module.function_defs.push(file_format::FunctionDefinition {
            function: file_format::FunctionHandleIndex(0),
            visibility: file_format::Visibility::Private,
            is_entry: false,
            acquires_global_resources: vec![],
            code: Some(file_format::CodeUnit {
                locals: file_format::SignatureIndex(0),
                code: vec![file_format::Bytecode::Ret],
            }),
        });

        let modules = vec![module.clone()];

        // Create empty coverage map
        let coverage_map = CoverageMap::default();
        let unified_map = coverage_map.to_unified_exec_map();

        // Format as CSV
        let mut output = Vec::new();
        format_csv_summary(
            modules.as_slice(),
            &unified_map,
            summarize_inst_cov,
            &mut output,
        );

        let output_str = String::from_utf8(output).unwrap();

        // Verify CSV header
        assert!(
            output_str.starts_with("ModuleName,FunctionName,Covered,Uncovered"),
            "CSV should have proper header"
        );

        // Verify CSV contains data row
        let lines: Vec<&str> = output_str.lines().collect();
        assert!(
            lines.len() >= 2,
            "Should have header and at least one data row"
        );

        // Verify the data row has correct format (4 comma-separated values)
        if lines.len() > 1 {
            let data_row = lines[1];
            let columns: Vec<&str> = data_row.split(',').collect();
            assert_eq!(columns.len(), 4, "Each CSV row should have 4 columns");
        }
    }

    /// Test CoverageMap loading from binary file
    #[test]
    fn test_coverage_map_binary_roundtrip() {
        let mut coverage_map = CoverageMap::default();
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();
        let module_name = Identifier::new("TestModule").unwrap();
        let func_name = Identifier::new("test_func").unwrap();

        coverage_map.insert("exec1", addr, module_name.clone(), func_name.clone(), 0);
        coverage_map.insert("exec1", addr, module_name.clone(), func_name.clone(), 1);
        coverage_map.insert("exec1", addr, module_name.clone(), func_name.clone(), 0);

        // Serialize to temp file
        let temp_file = NamedTempFile::new().unwrap();
        let bytes = bcs::to_bytes(&coverage_map).unwrap();
        std::fs::write(temp_file.path(), &bytes).unwrap();

        // Load back
        let loaded = CoverageMap::from_binary_file(&temp_file.path()).unwrap();

        // Verify contents
        let unified = loaded.to_unified_exec_map();
        let module_map = unified.module_maps.get(&(addr, module_name)).unwrap();
        let func_cov = module_map.function_maps.get(&func_name).unwrap();

        assert_eq!(func_cov.get(&0), Some(&2), "PC 0 should be hit twice");
        assert_eq!(func_cov.get(&1), Some(&1), "PC 1 should be hit once");
    }

    /// Test merging multiple coverage maps
    #[test]
    fn test_coverage_map_merge() {
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();
        let module_name = Identifier::new("TestModule").unwrap();
        let func_name = Identifier::new("test_func").unwrap();

        let mut map1 = CoverageMap::default();
        map1.insert("exec1", addr, module_name.clone(), func_name.clone(), 0);
        map1.insert("exec1", addr, module_name.clone(), func_name.clone(), 1);

        let mut map2 = CoverageMap::default();
        map2.insert("exec1", addr, module_name.clone(), func_name.clone(), 0);
        map2.insert("exec1", addr, module_name.clone(), func_name.clone(), 2);

        map1.merge(map2);

        let unified = map1.to_unified_exec_map();
        let module_map = unified.module_maps.get(&(addr, module_name)).unwrap();
        let func_cov = module_map.function_maps.get(&func_name).unwrap();

        assert_eq!(func_cov.get(&0), Some(&2), "PC 0 should be merged");
        assert_eq!(func_cov.get(&1), Some(&1), "PC 1 from map1");
        assert_eq!(func_cov.get(&2), Some(&1), "PC 2 from map2");
    }

    /// Test ColorChoice enum parsing
    #[test]
    fn test_color_choice_values() {
        use std::str::FromStr;

        assert!(ColorChoice::from_str("none").is_ok());
        assert!(ColorChoice::from_str("default").is_ok());
        assert!(ColorChoice::from_str("always").is_ok());
        assert!(ColorChoice::from_str("invalid").is_err());
    }

    /// Test TextIndicator enum parsing
    #[test]
    fn test_text_indicator_values() {
        use std::str::FromStr;

        assert!(TextIndicator::from_str("none").is_ok());
        assert!(TextIndicator::from_str("explicit").is_ok());
        assert!(TextIndicator::from_str("on").is_ok());
        assert!(TextIndicator::from_str("invalid").is_err());
    }

    /// Test that CoverageCommon default has empty extra_coverage
    #[test]
    fn test_coverage_common_default() {
        let common = CoverageCommon::default();
        assert!(
            common.extra_coverage.is_empty(),
            "Default should have no extra coverage files"
        );
    }
}

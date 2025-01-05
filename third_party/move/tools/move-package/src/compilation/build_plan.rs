// Copyright (c) Aptos Foundation
// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use super::package_layout::CompiledPackageLayout;
use crate::{
    compilation::compiled_package::{
        build_and_report_no_exit_v2_driver, build_and_report_v2_driver, CompiledPackage,
    },
    resolution::resolution_graph::ResolvedGraph,
    source_package::parsed_manifest::PackageName,
    CompilerConfig,
};
use anyhow::{Context, Result};
use move_compiler::{
    compiled_unit::AnnotatedCompiledUnit,
    diagnostics::{report_diagnostics_to_color_buffer, report_warnings, FilesSourceText},
    Compiler,
};
use move_compiler_v2::external_checks::ExternalChecks;
use move_model::model;
use petgraph::algo::toposort;
use std::{collections::BTreeSet, io::Write, path::Path, sync::Arc};
#[cfg(feature = "evm-backend")]
use {
    colored::Colorize,
    move_to_yul::{options::Options as MoveToYulOptions, run_to_yul},
    std::{fs, io},
    termcolor::Buffer,
    walkdir::WalkDir,
};

#[derive(Debug, Clone)]
pub struct BuildPlan {
    root: PackageName,
    sorted_deps: Vec<PackageName>,
    resolution_graph: ResolvedGraph,
}

/// A container for compiler results from either V1 or V2,
/// with all info needed for building various artifacts.
pub type CompilerDriverResult = anyhow::Result<(
    // The names and contents of all source files.
    FilesSourceText,
    // The compilation artifacts, including V1 intermediate ASTs.
    Vec<AnnotatedCompiledUnit>,
    // For compilation with V2, compiled program model.
    Option<model::GlobalEnv>,
)>;

#[cfg(feature = "evm-backend")]
fn should_recompile(
    source_paths: impl IntoIterator<Item = impl AsRef<Path>>,
    output_paths: impl IntoIterator<Item = impl AsRef<Path>>,
) -> Result<bool> {
    let mut earliest_output_mod_time = None;
    for output_path in output_paths.into_iter() {
        match fs::metadata(output_path) {
            Ok(meta) => {
                let mod_time = meta
                    .modified()
                    .expect("failed to get file modification time");

                match &mut earliest_output_mod_time {
                    None => earliest_output_mod_time = Some(mod_time),
                    Some(earliest_mod_time) => *earliest_mod_time = mod_time,
                }
            },
            Err(err) => {
                if let io::ErrorKind::NotFound = err.kind() {
                    return Ok(true);
                }
                return Err(err.into());
            },
        }
    }

    let earliest_output_mod_time = match earliest_output_mod_time {
        Some(mod_time) => mod_time,
        None => panic!("no output files given -- this should not happen"),
    };

    for source_path in source_paths.into_iter() {
        for entry in WalkDir::new(source_path) {
            let entry = entry?;

            let mod_time = entry
                .metadata()?
                .modified()
                .expect("failed to get file modification time");

            if mod_time > earliest_output_mod_time {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

impl BuildPlan {
    pub fn create(resolution_graph: ResolvedGraph) -> Result<Self> {
        let mut sorted_deps = match toposort(&resolution_graph.graph, None) {
            Ok(nodes) => nodes,
            Err(err) => {
                // Is a DAG after resolution otherwise an error should be raised from that.
                anyhow::bail!("IPE: Cyclic dependency found after resolution {:?}", err)
            },
        };

        sorted_deps.reverse();

        Ok(Self {
            root: resolution_graph.root_package.package.name,
            sorted_deps,
            resolution_graph,
        })
    }

    /// Compilation results in the process exit upon warning/failure
    pub fn compile<W: Write>(
        &self,
        config: &CompilerConfig,
        writer: &mut W,
    ) -> Result<CompiledPackage> {
        self.compile_with_driver(
            writer,
            config,
            vec![],
            |compiler| {
                let (files, units) = compiler.build_and_report()?;
                Ok((files, units, None))
            },
            build_and_report_v2_driver,
        )
        .map(|(package, _)| package)
    }

    /// Compilation process does not exit even if warnings/failures are encountered.
    /// External checks on Move code can be provided via `external_checks`, these checks
    /// are only run when using the compiler v2.
    pub fn compile_no_exit<W: Write>(
        &self,
        config: &CompilerConfig,
        external_checks: Vec<Arc<dyn ExternalChecks>>,
        writer: &mut W,
    ) -> Result<(CompiledPackage, Option<model::GlobalEnv>)> {
        self.compile_with_driver(
            writer,
            config,
            external_checks,
            |compiler| {
                let (files, units_res) = compiler.build()?;
                match units_res {
                    Ok((units, warning_diags)) => {
                        report_warnings(&files, warning_diags);
                        Ok((files, units, None))
                    },
                    Err(error_diags) => {
                        assert!(!error_diags.is_empty());
                        let diags_buf = report_diagnostics_to_color_buffer(&files, error_diags);
                        if let Err(err) = std::io::stdout().write_all(&diags_buf) {
                            anyhow::bail!("Cannot output compiler diagnostics: {}", err);
                        }
                        anyhow::bail!("Compilation error");
                    },
                }
            },
            build_and_report_no_exit_v2_driver,
        )
    }

    pub fn compile_with_driver<W: Write>(
        &self,
        writer: &mut W,
        config: &CompilerConfig,
        external_checks: Vec<Arc<dyn ExternalChecks>>,
        compiler_driver_v1: impl FnMut(Compiler) -> CompilerDriverResult,
        compiler_driver_v2: impl FnMut(move_compiler_v2::Options) -> CompilerDriverResult,
    ) -> Result<(CompiledPackage, Option<model::GlobalEnv>)> {
        let root_package = &self.resolution_graph.package_table[&self.root];
        let project_root = match &self.resolution_graph.build_options.install_dir {
            Some(under_path) => under_path.clone(),
            None => self.resolution_graph.root_package_path.clone(),
        };
        let immediate_dependencies_names =
            root_package.immediate_dependencies(&self.resolution_graph);
        let transitive_dependencies = root_package
            .transitive_dependencies(&self.resolution_graph)
            .into_iter()
            .map(|package_name| {
                let dep_package = self
                    .resolution_graph
                    .package_table
                    .get(&package_name)
                    .unwrap();
                let mut dep_source_paths = dep_package
                    .get_sources(&self.resolution_graph.build_options)
                    .unwrap();
                let mut source_available = true;
                // If source is empty, search bytecode(mv) files
                if dep_source_paths.is_empty() {
                    dep_source_paths = dep_package.get_bytecodes().unwrap();
                    source_available = false;
                }
                (
                    package_name,
                    immediate_dependencies_names.contains(&package_name),
                    dep_source_paths,
                    &dep_package.resolution_table,
                    source_available,
                )
            })
            .collect();

        let (compiled, model) = CompiledPackage::build_all(
            writer,
            &project_root,
            root_package.clone(),
            transitive_dependencies,
            config,
            external_checks,
            &self.resolution_graph,
            compiler_driver_v1,
            compiler_driver_v2,
        )?;

        Self::clean(
            &project_root.join(CompiledPackageLayout::Root.path()),
            self.sorted_deps.iter().copied().collect(),
        )?;
        Ok((compiled, model))
    }

    #[cfg(feature = "evm-backend")]
    pub fn compile_evm<W: Write>(&self, writer: &mut W) -> Result<()> {
        let root_package = &self.resolution_graph.package_table[&self.root];
        let project_root = match &self.resolution_graph.build_options.install_dir {
            Some(under_path) => under_path.clone(),
            None => self.resolution_graph.root_package_path.clone(),
        };
        let build_root_path = project_root
            .join(CompiledPackageLayout::Root.path())
            .join("evm");

        // Step 1: Compile Move into Yul
        //   Step 1a: Gather command line arguments for move-to-yul
        let dependencies = self
            .resolution_graph
            .package_table
            .iter()
            .filter_map(|(name, package)| {
                if name == &root_package.source_package.package.name {
                    None
                } else {
                    Some(format!(
                        "{}/sources",
                        package.package_path.to_string_lossy()
                    ))
                }
            })
            .collect::<Vec<_>>();

        let sources = vec![format!(
            "{}/sources",
            root_package.package_path.to_string_lossy()
        )];

        let bytecode_output = format!(
            "{}/{}.bin",
            build_root_path.to_string_lossy(),
            root_package.source_package.package.name
        );

        let yul_output = format!(
            "{}/{}.yul",
            build_root_path.to_string_lossy(),
            root_package.source_package.package.name
        );
        let abi_output = format!(
            "{}/{}.abi.json",
            build_root_path.to_string_lossy(),
            root_package.source_package.package.name
        );

        let output_paths = [&bytecode_output, &yul_output, &abi_output];

        let package_names = self
            .resolution_graph
            .package_table
            .iter()
            .map(|(name, _)| name.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let named_address_mapping = self
            .resolution_graph
            .extract_named_address_mapping()
            .map(|(name, addr)| format!("{}={}", name.as_str(), addr))
            .collect();

        //   Step 1b: Check if a fresh compilation is really needed. Only recompile if either
        //              a) Some of the output artifacts are missing
        //              b) Any source files have been modified since last compile
        let manifests = self
            .resolution_graph
            .package_table
            .iter()
            .map(|(_name, package)| format!("{}/Move.toml", package.package_path.to_string_lossy()))
            .collect::<Vec<_>>();

        let all_sources = manifests
            .iter()
            .chain(sources.iter())
            .chain(dependencies.iter());

        if !should_recompile(all_sources, output_paths)? {
            writeln!(writer, "{} {}", "CACHED".bold().green(), package_names)?;
            return Ok(());
        }

        //   Step 1c: Call move-to-yul
        writeln!(
            writer,
            "{} {} to Yul",
            "COMPILING".bold().green(),
            package_names
        )?;

        if let Err(err) = std::fs::remove_dir_all(&build_root_path) {
            match err.kind() {
                io::ErrorKind::NotFound => (),
                _ => {
                    writeln!(
                        writer,
                        "{} Failed to remove build dir {}: {}",
                        "ERROR".bold().red(),
                        build_root_path.to_string_lossy(),
                        err,
                    )?;

                    return Err(err.into());
                },
            }
        }
        if let Err(err) = std::fs::create_dir_all(&build_root_path) {
            writeln!(
                writer,
                "{} Failed to create build dir {}",
                "ERROR".bold().red(),
                build_root_path.to_string_lossy(),
            )?;

            return Err(err.into());
        }

        // TODO: should inherit color settings from current shell
        let mut error_buffer = Buffer::ansi();
        if let Err(err) = run_to_yul(&mut error_buffer, MoveToYulOptions {
            dependencies,
            named_address_mapping,
            sources,
            output: yul_output.clone(),
            abi_output,

            ..MoveToYulOptions::default()
        }) {
            writeln!(
                writer,
                "{} Failed to compile Move into Yul {}",
                err,
                "ERROR".bold().red()
            )?;

            writeln!(
                writer,
                "{}",
                std::str::from_utf8(error_buffer.as_slice()).unwrap()
            )?;

            let mut source = err.source();
            while let Some(s) = source {
                writeln!(writer, "{}", s)?;
                source = s.source();
            }

            return Err(err);
        }

        // Step 2: Compile Yul into bytecode using solc

        let yul_source = match std::fs::read_to_string(&yul_output) {
            Ok(yul_source) => yul_source,
            Err(err) => {
                writeln!(
                    writer,
                    "{} Failed to read from {}",
                    "ERROR".bold().red(),
                    yul_output,
                )?;

                return Err(err.into());
            },
        };

        writeln!(
            writer,
            "{} EVM bytecote from Yul",
            "GENERATING".bold().green(),
        )?;

        match evm_exec_utils::compile::solc_yul(&yul_source, false) {
            Ok((bytecode, _)) => {
                let mut bytecode_file = match std::fs::File::create(&bytecode_output) {
                    Ok(file) => file,
                    Err(err) => {
                        writeln!(
                            writer,
                            "{} Failed to create bytecode output {}",
                            "ERROR".bold().red(),
                            bytecode_output,
                        )?;

                        return Err(err.into());
                    },
                };

                if let Err(err) = bytecode_file.write_all(hex::encode(&bytecode).as_bytes()) {
                    writeln!(
                        writer,
                        "{} Failed to write bytecode to file {}",
                        "ERROR".bold().red(),
                        bytecode_output,
                    )?;

                    return Err(err.into());
                }
            },
            Err(err) => {
                writeln!(
                    writer,
                    "{} Failed to generate EVM bytecote",
                    "ERROR".bold().red()
                )?;

                let mut source = err.source();
                while let Some(s) = source {
                    writeln!(writer, "{}", s)?;
                    source = s.source();
                }

                return Err(err);
            },
        }

        Ok(())
    }

    // Clean out old packages that are no longer used, or no longer used under the current
    // compilation flags
    fn clean(build_root: &Path, keep_paths: BTreeSet<PackageName>) -> Result<()> {
        for dir in std::fs::read_dir(build_root)? {
            let path = dir
                .with_context(|| {
                    format!(
                        "Cleaning subdirectories of build root {}",
                        build_root.to_string_lossy()
                    )
                })?
                .path();
            if path.is_dir() && !keep_paths.iter().any(|name| path.ends_with(name.as_str())) {
                std::fs::remove_dir_all(&path).with_context(|| {
                    format!("When deleting directory {}", path.to_string_lossy())
                })?;
            }
        }
        Ok(())
    }
}

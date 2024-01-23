// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

use super::utils::*;
use crate::{analyzer_handler::*, project::Project};
use anyhow::{Ok, Result};
use codespan_reporting::diagnostic::Severity;
use move_compiler::shared::{NumericalAddress, PackagePaths};
use move_core_types::account_address::*;
use move_model::{options::ModelBuilderOptions, run_model_builder_with_options};
use move_package::source_package::{layout::SourcePackageLayout, manifest_parser::*};
use num_bigint::BigUint;
use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, HashMap},
    path::{Path, PathBuf},
    rc::Rc,
};
use tempfile::tempdir;
use walkdir::WalkDir;

// Determines the base of the number literal, depending on the prefix
pub(crate) fn determine_num_text_and_base(s: &str) -> (&str, move_compiler::shared::NumberFormat) {
    for c in s.chars() {
        if c.is_alphabetic() {
            return (s, move_compiler::shared::NumberFormat::Hex);
        }
    }
    (s, move_compiler::shared::NumberFormat::Decimal)
}

// Parse an address from a decimal or hex encoding
pub fn parse_addr_str_to_number(
    s: &str,
) -> Option<(
    [u8; AccountAddress::LENGTH],
    move_compiler::shared::NumberFormat,
)> {
    let (txt, base) = determine_num_text_and_base(s);

    let parsed = match base {
        move_compiler::shared::NumberFormat::Hex => BigUint::parse_bytes(txt[2..].as_bytes(), 16),
        move_compiler::shared::NumberFormat::Decimal => BigUint::parse_bytes(txt.as_bytes(), 10),
    }?;

    let bytes = parsed.to_bytes_be();
    if bytes.len() > AccountAddress::LENGTH {
        return None;
    }
    let mut result = [0u8; AccountAddress::LENGTH];
    result[(AccountAddress::LENGTH - bytes.len())..].clone_from_slice(&bytes);
    Some((result, base))
}

pub fn parse_addr_str(s: &str) -> Option<NumericalAddress> {
    parse_addr_str_to_number(s).map(|(n, format)| NumericalAddress::new(n, format))
}

pub fn parse_named_address_item(s: &str) -> anyhow::Result<(String, NumericalAddress)> {
    let before_after = s.split('=').collect::<Vec<_>>();

    if before_after.len() != 2 {
        anyhow::bail!(
            "Invalid named address assignment. Must be of the form <address_name>=<address>, but \
             found '{}'",
            s
        );
    }
    let name = before_after[0].parse()?;
    if let Some(addr) = parse_addr_str(before_after[1]) {
        Ok((name, addr))
    } else {
        Ok((
            name,
            NumericalAddress::new(
                AccountAddress::from_hex_literal("0x0")
                    .unwrap()
                    .into_bytes(),
                move_compiler::shared::NumberFormat::Hex,
            ),
        ))
    }
}

pub fn parse_addresses_from_options(
    named_addr_strings: Vec<String>,
) -> anyhow::Result<BTreeMap<String, NumericalAddress>> {
    named_addr_strings
        .iter()
        .map(|x| parse_named_address_item(x))
        .collect()
}

impl Project {
    pub(crate) fn mk_multi_project_key(&self) -> im::HashSet<PathBuf> {
        use im::HashSet;
        let mut v = HashSet::default();
        for x in self.manifest_paths.iter() {
            v.insert(x.clone());
        }
        v
    }

    pub fn load_ok(&self) -> bool {
        self.manifest_not_exists.is_empty() && self.manifest_load_failures.is_empty()
    }

    pub fn new(
        root_dir: impl Into<PathBuf>,
        report_err: impl FnMut(String) + Clone,
    ) -> Result<Self> {
        let working_dir = root_dir.into();
        log::info!("scan modules at {:?}", &working_dir);
        let mut new_project = Self {
            modules: Default::default(),
            manifests: Default::default(),
            hash_file: Rc::new(RefCell::new(PathBufHashMap::new())),
            file_line_mapping: Rc::new(RefCell::new(FileLineMapping::new())),
            manifest_paths: Default::default(),
            manifest_not_exists: Default::default(),
            manifest_load_failures: Default::default(),
            manifest_mod_time: Default::default(),
            global_env: Default::default(),
            current_modifing_file_content: Default::default(),
            targets: vec![],
            dependents: vec![],
            addrname_2_addrnum: Default::default(),
            err_diags: String::default(),
        };

        let mut targets_paths: Vec<PathBuf> = Vec::new();
        let mut dependents_paths: Vec<PathBuf> = Vec::new();
        new_project.load_project(
            &working_dir,
            report_err,
            true,
            &mut targets_paths,
            &mut dependents_paths,
        )?;
        log::info!("targets_paths.len() = {:?}", targets_paths.len());
        log::info!("dependents_paths.len() = {:?}", dependents_paths.len());

        let build_config = move_package::BuildConfig {
            test_mode: true,
            install_dir: Some(tempdir().unwrap().path().to_path_buf()),
            skip_fetch_latest_git_deps: true,
            ..Default::default()
        };
        let resolution_graph =
            build_config.resolution_graph_for_package(&working_dir, &mut Vec::new())?;
        let named_address_mapping: Vec<_> = resolution_graph
            .extract_named_address_mapping()
            .map(|(name, addr)| format!("{}={}", name.as_str(), addr))
            .collect();
        let addrs = parse_addresses_from_options(named_address_mapping)?;

        let targets = vec![PackagePaths {
            name: None,
            paths: targets_paths
                .into_iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect::<Vec<_>>(),
            named_address_map: addrs.clone(),
        }];

        let dependents = vec![PackagePaths {
            name: None,
            paths: dependents_paths
                .into_iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect::<Vec<_>>(),
            named_address_map: addrs.clone(),
        }];

        let attributes: BTreeSet<String> = Default::default();
        new_project.targets = targets.clone();
        new_project.dependents = dependents.clone();
        new_project.global_env = run_model_builder_with_options(
            targets,
            dependents,
            ModelBuilderOptions {
                compile_via_model: true,
                ..Default::default()
            },
            false,
            &attributes,
        )
        .expect("Failed to create GlobalEnv!");
        log::info!(
            "env.get_module_count() = {:?}",
            &new_project.global_env.get_module_count()
        );
        use codespan_reporting::term::termcolor::Buffer;
        let mut error_writer = Buffer::no_color();

        let mut helper = HashMap::new();
        for (addr_name, addr_num) in addrs.iter() {
            helper.insert(addr_name.clone(), addr_num.to_string());
        }

        new_project.addrname_2_addrnum = helper;
        new_project
            .global_env
            .report_diag(&mut error_writer, Severity::Error);
        new_project.err_diags = String::from_utf8_lossy(&error_writer.into_inner()).to_string();
        Ok(new_project)
    }

    pub fn update_defs(&mut self, file_path: &PathBuf, content: String) {
        use std::result::Result::Ok;
        log::info!("update_defs for file:{:?}", file_path);
        let root_dir = match super::utils::discover_manifest_and_kind(file_path) {
            Some((x, _)) => x,
            None => {
                log::error!("not move project.");
                return;
            },
        };

        let new_project = match Project::new(root_dir, |msg| log::info!("{}", msg)) {
            Ok(x) => x,
            Err(_) => {
                log::error!("reload project failed");
                return;
            },
        };

        self.current_modifing_file_content = content;
        self.targets = new_project.targets.clone();
        self.dependents = new_project.dependents.clone();
        self.global_env = new_project.global_env;

        log::info!(
            "env.get_module_count() = {:?}",
            &self.global_env.get_module_count()
        );
        use codespan_reporting::term::termcolor::Buffer;
        let mut error_writer = Buffer::no_color();
        self.global_env
            .report_diag(&mut error_writer, Severity::Error);
        self.err_diags = String::from_utf8_lossy(&error_writer.into_inner()).to_string();
    }

    /// Load a Move.toml project.
    pub(crate) fn load_project(
        &mut self,
        manifest_path: &Path,
        mut report_err: impl FnMut(String) + Clone,
        is_main_source: bool,
        targets_paths: &mut Vec<PathBuf>,
        dependents_paths: &mut Vec<PathBuf>,
    ) -> Result<()> {
        let manifest_path = normal_path(manifest_path);
        if self.modules.get(&manifest_path).is_some() {
            log::trace!("manifest '{:?}' loaded before skipped.", &manifest_path);
            return Ok(());
        }
        if self.manifest_paths.contains(&manifest_path) {
            log::trace!("manifest '{:?}' loaded before skipped.", &manifest_path);
            return Ok(());
        }

        self.manifest_paths.push(manifest_path.clone());
        log::trace!("load manifest file at {:?}", &manifest_path);

        let source_paths1 =
            self.load_layout_files_v2(&manifest_path, SourcePackageLayout::Sources)?;
        let source_paths2 =
            self.load_layout_files_v2(&manifest_path, SourcePackageLayout::Tests)?;
        let source_paths3 =
            self.load_layout_files_v2(&manifest_path, SourcePackageLayout::Scripts)?;
        if is_main_source {
            targets_paths.extend(source_paths1);
            targets_paths.extend(source_paths2);
            targets_paths.extend(source_paths3);
        } else {
            dependents_paths.extend(source_paths1);
            dependents_paths.extend(source_paths2);
            dependents_paths.extend(source_paths3);
        }

        if !manifest_path.exists() {
            self.manifest_not_exists.insert(manifest_path);
            return anyhow::Result::Ok(());
        }
        {
            let mut file = manifest_path.clone();
            file.push(PROJECT_FILE_NAME);

            self.manifest_mod_time
                .insert(file.clone(), file_modify_time(file.as_path()));
        }

        let manifest = match parse_move_manifest_from_file(&manifest_path) {
            std::result::Result::Ok(x) => x,
            std::result::Result::Err(err) => {
                report_err(format!(
                    "parse manifest '{:?} 'failed.\n addr must exactly 32 length or start with '0x' like '0x2'\n{:?}",
                    manifest_path,
                    err
                ));
                log::error!("parse_move_manifest_from_file failed,err:{:?}", err);
                self.manifest_load_failures.insert(manifest_path.clone());
                return anyhow::Result::Ok(());
            },
        };
        self.manifests.push(manifest.clone());
        // load depends.
        for (dep_name, de) in manifest
            .dependencies
            .iter()
            .chain(manifest.dev_dependencies.iter())
        {
            let de_path = de.local.clone();
            let p = path_concat(manifest_path.as_path(), &de_path);
            log::debug!(
                "load dependency for p '{:?}' manifest_path '{:?}' dep_name '{}'",
                &p,
                &manifest_path,
                dep_name
            );
            self.load_project(
                &p,
                report_err.clone(),
                false,
                targets_paths,
                dependents_paths,
            )?;
        }
        Ok(())
    }

    pub(crate) fn load_layout_files_v2(
        &mut self,
        manifest_path: &Path,
        kind: SourcePackageLayout,
    ) -> Result<Vec<PathBuf>> {
        let mut ret_paths = Vec::new();
        let mut p = manifest_path.to_path_buf();
        p.push(kind.location_str());
        for item in WalkDir::new(&p) {
            let file = match item {
                std::result::Result::Err(_e) => continue,
                std::result::Result::Ok(x) => x,
            };
            if file.file_type().is_file()
                && match file.file_name().to_str() {
                    Some(s) => s.ends_with(".move"),
                    None => continue,
                }
            {
                if file
                    .file_name()
                    .to_str()
                    .map(|x| x.starts_with('.'))
                    .unwrap_or(false)
                {
                    continue;
                }
                log::debug!("load source file {:?}", file.path());
                ret_paths.push(file.path().to_path_buf());
            }
        }
        Ok(ret_paths)
    }

    pub(crate) fn manifest_beed_modified(&self) -> bool {
        self.manifest_mod_time.iter().any(|(k, v)| {
            if file_modify_time(k.as_path()).cmp(v) != Ordering::Equal {
                log::info!(
                    "going to reload project becasue of modify of '{:?}' {:?} {:?}",
                    k.as_path(),
                    file_modify_time(k.as_path()),
                    v
                );
                true
            } else {
                false
            }
        })
    }

    pub fn run_visitor_for_file(
        &self,
        visitor: &mut dyn ItemOrAccessHandler,
        filepath: &Path,
        source_str: String,
    ) {
        visitor.handle_project_env(self, &self.global_env, filepath, source_str);
    }
}

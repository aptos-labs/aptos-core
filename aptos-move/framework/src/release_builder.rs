// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    built_package::{BuildOptions, BuiltPackage},
    path_relative_to_crate,
    release_bundle::{ReleaseBundle, ReleasePackage},
};
use anyhow::{anyhow, Context};
use aptos_sdk_builder::rust;
use aptos_types::transaction::EntryABI;
use clap::Parser;
use std::path::{Path, PathBuf};

pub const RELEASE_BUNDLE_EXTENSION: &str = "mrb";

/// Options to configure the generation of a release.
#[derive(Debug, Clone, Parser)]
#[clap(name = "Aptos Releasing", author, version)]
pub struct ReleaseOptions {
    #[clap(flatten)]
    pub build_options: BuildOptions,
    /// The path to the Move packages for which to create a release.
    #[clap(long, value_parser, num_args(1..))]
    pub packages: Vec<PathBuf>,
    /// The path where to place generated Rust bindings for this module, in order for
    /// each package. If the value is empty (`""`) for a particular package, no bindings are
    /// generated.
    #[clap(long)]
    pub rust_bindings: Vec<String>,

    /// For each package, whether it should be built with using latest language features.
    /// Generally packages being deployed to testnet/mainnet need to use default features,
    /// while those that don't (like aptos-experimental) can use latest language features.
    #[clap(long)]
    pub package_use_latest_language: Vec<bool>,

    /// The path to the file where to place the release bundle.
    #[clap(long, default_value = "head.mrb", value_parser)]
    pub output: PathBuf,
}

impl ReleaseOptions {
    /// Creates a release bundle from the specified options and saves it to disk. As a side
    /// effect, also generates rust bindings.
    pub fn create_release(self) -> anyhow::Result<()> {
        let ReleaseOptions {
            build_options,
            packages,
            rust_bindings,
            package_use_latest_language,
            output,
        } = self;
        let mut released_packages = vec![];
        let mut source_paths = vec![];
        for ((package_path, rust_binding_path), use_latest_language) in packages
            .into_iter()
            .zip(rust_bindings.into_iter())
            .zip(package_use_latest_language.into_iter())
        {
            let cur_build_options = if use_latest_language {
                build_options.clone().set_latest_language()
            } else {
                build_options.clone()
            };
            let built = BuiltPackage::build(package_path.clone(), cur_build_options).with_context(
                || {
                    format!(
                        "Failed to build package at path: {}",
                        package_path.display()
                    )
                },
            )?;
            if !rust_binding_path.is_empty() {
                let abis = built
                    .extract_abis()
                    .ok_or_else(|| anyhow!("ABIs not available, can't generate sdk"))?;
                let binding_path = rust_binding_path.clone();
                Self::generate_rust_bindings(&abis, &PathBuf::from(rust_binding_path))
                    .with_context(|| {
                        format!(
                            "Failed to generate Rust bindings for {} at binding path {}",
                            package_path.display(),
                            binding_path
                        )
                    })?;
            }
            let released = ReleasePackage::new(built)?;
            let size = bcs::to_bytes(&released)?.len();
            println!(
                "Including package `{}` size {}k",
                released.name(),
                size / 1000,
            );
            released_packages.push(released);
            let relative_path = path_relative_to_crate(package_path.join("sources"));
            source_paths.push(relative_path.display().to_string());
        }
        let bundle = ReleaseBundle::new(released_packages, source_paths);
        let parent = output.parent().expect("Failed to get parent directory");
        std::fs::create_dir_all(parent).context("Failed to create dirs")?;
        std::fs::write(&output, bcs::to_bytes(&bundle)?).context("Failed to write output")?;
        Ok(())
    }

    fn generate_rust_bindings(abis: &[EntryABI], path: &Path) -> anyhow::Result<()> {
        {
            let mut file = std::fs::File::create(path)
                .with_context(|| format!("Failed to create {}", path.display()))?;
            rust::output(&mut file, abis, true)
                .with_context(|| format!("Failed to output rust bindings to {}", path.display()))?;
        }
        std::process::Command::new("rustfmt")
            .arg("--config")
            .arg("imports_granularity=crate")
            .arg(path)
            .output()
            .with_context(|| {
                format!(
                    "Failed to run rustfmt on {}, is rustfmt installed?",
                    path.display()
                )
            })?;
        Ok(())
    }
}

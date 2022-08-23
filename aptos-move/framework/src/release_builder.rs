// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::built_package::{BuildOptions, BuiltPackage};
use crate::path_relative_to_crate;
use crate::release_bundle::{ReleaseBundle, ReleasePackage};
use anyhow::anyhow;
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
    #[clap(long, parse(from_os_str))]
    pub packages: Vec<PathBuf>,
    /// The path where to place generated Rust bindings for this module, in order for
    /// each package. If the value is empty (`""`) for a particular package, no bindings are
    /// generated.
    #[clap(long)]
    pub rust_bindings: Vec<String>,
    /// The path to the file where to place the release bundle.
    #[clap(long, default_value = "head.mrb", parse(from_os_str))]
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
            output,
        } = self;
        let mut released_packages = vec![];
        let mut source_paths = vec![];
        for (package_path, rust_binding_path) in packages.into_iter().zip(rust_bindings.into_iter())
        {
            let built = BuiltPackage::build(package_path.clone(), build_options.clone())?;
            if !rust_binding_path.is_empty() {
                let abis = built
                    .extract_abis()
                    .ok_or_else(|| anyhow!("abis not available, can't generate sdk"))?;
                Self::generate_rust_bindings(&abis, &PathBuf::from(rust_binding_path))?;
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
        std::fs::create_dir_all(&output.parent().unwrap())?;
        std::fs::write(&output, bcs::to_bytes(&bundle)?)?;
        Ok(())
    }

    fn generate_rust_bindings(abis: &[EntryABI], path: &Path) -> anyhow::Result<()> {
        {
            let mut file = std::fs::File::create(path)?;
            rust::output(&mut file, abis, true)?;
        }
        std::process::Command::new("rustfmt")
            .arg("--config")
            .arg("imports_granularity=crate")
            .arg(path)
            .status()?;
        Ok(())
    }
}

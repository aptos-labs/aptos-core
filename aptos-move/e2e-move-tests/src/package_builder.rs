// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use itertools::Itertools;
use move_deps::move_command_line_common::files::MOVE_EXTENSION;
use move_deps::move_package::compilation::package_layout::CompiledPackageLayout;
use std::path::PathBuf;
use tempfile::{tempdir, TempDir};

/// A helper for building Move packages on-the-fly for testing.
#[derive(Debug, Clone)]
pub struct PackageBuilder {
    name: String,
    deps: Vec<String>,
    aliases: Vec<(String, String)>,
    sources: Vec<(String, String)>,
}

impl PackageBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            deps: vec![],
            aliases: vec![],
            sources: vec![],
        }
    }

    pub fn add_dep(&mut self, dep: &str) {
        self.deps.push(dep.to_string())
    }

    pub fn add_alias(&mut self, name: &str, addr: &str) {
        self.aliases.push((name.to_string(), addr.to_string()))
    }

    pub fn add_source(&mut self, name: &str, src: &str) {
        self.sources.push((name.to_string(), src.to_string()))
    }

    pub fn write_to_disk(self, path: PathBuf) -> anyhow::Result<()> {
        let sources_path = path.join(CompiledPackageLayout::Sources.path());
        std::fs::create_dir_all(&sources_path)?;
        std::fs::write(
            path.join("Move.toml"),
            format!(
                "\
[package]
name = \"{}\"
version = \"0.0.0\"
[addresses]
{}
[dependencies]
{}",
                self.name,
                self.aliases
                    .into_iter()
                    .map(|(k, v)| format!("{} = \"{}\"", k, v))
                    .join("\n"),
                self.deps.into_iter().join("\n")
            ),
        )?;
        for (name, src) in self.sources {
            std::fs::write(sources_path.join(name).with_extension(MOVE_EXTENSION), src)?
        }
        Ok(())
    }

    pub fn write_to_temp(self) -> anyhow::Result<TempDir> {
        let dir = tempdir()?;
        self.write_to_disk(dir.path().to_path_buf())?;
        Ok(dir)
    }
}

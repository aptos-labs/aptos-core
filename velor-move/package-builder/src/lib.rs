// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_framework::natives::code::UpgradePolicy;
use itertools::Itertools;
use move_command_line_common::files::MOVE_EXTENSION;
use move_package::compilation::package_layout::CompiledPackageLayout;
use std::path::Path;
use tempfile::{tempdir, TempDir};

/// A helper for building Move packages on-the-fly for testing.
#[derive(Debug, Clone)]
pub struct PackageBuilder {
    name: String,
    policy: UpgradePolicy,
    deps: Vec<(String, String)>,
    aliases: Vec<(String, String)>,
    sources: Vec<(String, String)>,
}

impl PackageBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            policy: UpgradePolicy::compat(),
            deps: vec![],
            aliases: vec![],
            sources: vec![],
        }
    }

    pub fn with_policy(self, policy: UpgradePolicy) -> Self {
        Self { policy, ..self }
    }

    pub fn add_local_dep(&mut self, name: &str, path: &str) {
        self.deps.push((name.to_string(), path.to_string()))
    }

    pub fn add_alias(&mut self, name: &str, addr: &str) {
        self.aliases.push((name.to_string(), addr.to_string()))
    }

    pub fn add_source(&mut self, name: &str, src: &str) {
        self.sources.push((name.to_string(), src.to_string()))
    }

    pub fn write_to_disk(self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let path = path.as_ref();

        let sources_path = path.join(CompiledPackageLayout::Sources.path());
        std::fs::create_dir_all(&sources_path)?;
        std::fs::write(
            path.join("Move.toml"),
            format!(
                "\
[package]
name = \"{}\"
version = \"0.0.0\"
upgrade_policy = \"{}\"
[addresses]
{}
[dependencies]
{}",
                self.name,
                self.policy,
                self.aliases
                    .into_iter()
                    .map(|(k, v)| format!("{} = \"{}\"", k, v))
                    .join("\n"),
                self.deps
                    .into_iter()
                    .map(|(name, dep_path)| format!("{} = {{ local = \"{}\" }}", name, dep_path))
                    .join("\n")
            ),
        )?;
        for (name, src) in self.sources {
            std::fs::write(sources_path.join(name).with_extension(MOVE_EXTENSION), src)?
        }
        Ok(())
    }

    pub fn write_to_temp(self) -> anyhow::Result<TempDir> {
        let dir = tempdir()?;
        self.write_to_disk(dir.path())?;
        Ok(dir)
    }
}

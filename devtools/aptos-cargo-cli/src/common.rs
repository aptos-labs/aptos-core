// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use clap::Args;
use determinator::{Determinator, Utf8Paths0};
use guppy::{graph::DependencyDirection, CargoMetadata, MetadataCommand};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};
use url::Url;

fn workspace_dir() -> PathBuf {
    let output = Command::new("cargo")
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .unwrap()
        .stdout;
    let cargo_path = Path::new(std::str::from_utf8(&output).unwrap().trim());
    cargo_path.parent().unwrap().to_path_buf()
}

#[derive(Args, Debug, Clone)]
pub struct SelectedPackageArgs {
    #[arg(short, long, global = true)]
    pub package: Vec<String>,
    // TODO: add changed_since
}

impl SelectedPackageArgs {
    fn compute_changed_files(&self, merge_base: &str) -> anyhow::Result<Utf8Paths0> {
        let mut command = Command::new("git");
        command.args(["diff", "-z", "--name-only"]);
        command.arg(merge_base);

        let output = command.output().map_err(|err| anyhow!("error: {}", err))?;
        if !output.status.success() {
            return Err(anyhow!("error"));
        }

        Utf8Paths0::from_bytes(output.stdout).map_err(|(_path, err)| anyhow!("{}", err))
    }

    fn git_rev_parse(&self, merge_base: &str) -> String {
        let output = Command::new("git")
            .arg("rev-parse")
            .arg(merge_base)
            .output()
            .expect("failed to execute git rev-parse");

        String::from_utf8(output.stdout)
            .expect("invalid UTF-8")
            .trim()
            .to_owned()
    }

    fn fetch_remote_metadata(&self, merge_base: &str) -> anyhow::Result<CargoMetadata> {
        let base_sha = self.git_rev_parse(merge_base);
        let file_name = format!("metadata-{}.json", base_sha);
        let dir_path = format!(
            "{}/target/aptos-x-tool",
            workspace_dir().to_str().expect("invalid UTF-8")
        );
        let file_path = format!("{}/{}", dir_path, file_name);
        let mut contents = String::new();

        // Check if the file exists in the local directory
        if let Ok(file) = fs::File::open(&file_path) {
            let mut buf_reader = std::io::BufReader::new(file);
            buf_reader.read_to_string(&mut contents)?;
        } else {
            // Make an HTTP call to the GCS bucket to get the file contents
            let url = format!(
                "https://storage.googleapis.com/aptos-core-cargo-metadata-public/{}",
                file_name
            );
            let response = reqwest::blocking::get(url)?.error_for_status()?;
            let response = response.text()?;
            contents = response.clone();

            // Write the contents of the file to the local directory
            fs::create_dir_all("target/aptos-x-tool")?;
            fs::write(file_path, response)?;
        }

        // Return the contents of the file
        Ok(CargoMetadata::parse_json(&contents)?)
    }

    pub fn compute_packages(&self) -> anyhow::Result<Vec<String>> {
        if !self.package.is_empty() {
            return Ok(self.package.clone());
        }

        // Determine merge base
        // TODO: support different merge bases
        let merge_base = "origin/main";

        // Download merge base metadata
        let base_metadata = self.fetch_remote_metadata(merge_base)?;
        let base_package_graph = base_metadata.build_graph().unwrap();

        // Compute head metadata
        let head_metadata = MetadataCommand::new()
            .exec()
            .map_err(|e| anyhow!("{}", e))?;
        let head_package_graph = head_metadata.build_graph().unwrap();

        // Compute changed files
        let changed_files = self.compute_changed_files(merge_base)?;

        // Run target determinator
        let mut determinator = Determinator::new(&base_package_graph, &head_package_graph);
        // The determinator expects a list of changed files to be passed in.
        determinator.add_changed_paths(&changed_files);

        let determinator_set = determinator.compute();
        let package_set = determinator_set
            .affected_set
            .packages(DependencyDirection::Forward)
            .map(|package| {
                let manifest_path = package.manifest_path();
                let parent_path = manifest_path.parent().expect("must exist");
                let mut url = Url::from_directory_path(parent_path)
                    .expect("must be a valid directory path")
                    .to_string();
                if url.ends_with('/') {
                    url.pop();
                }
                format!("{}#{}", url, package.name())
            })
            .collect();

        Ok(package_set)
    }
}

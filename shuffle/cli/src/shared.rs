// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use diem_sdk::client::BlockingClient;
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use serde_generate as serdegen;
use serde_generate::SourceInstaller;
use serde_reflection::Registry;
use std::{
    fs,
    path::{Path, PathBuf},
};
use transaction_builder_generator as buildgen;
use transaction_builder_generator::SourceInstaller as BuildgenSourceInstaller;

pub const MAIN_PKG_PATH: &str = "main";

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub(crate) blockchain: String,
}

pub fn read_config(project_path: &Path) -> Result<Config> {
    let config_string =
        fs::read_to_string(project_path.join("Shuffle").with_extension("toml")).unwrap();
    let read_config: Config = toml::from_str(config_string.as_str())?;
    Ok(read_config)
}

/// Send a transaction to the blockchain through the blocking client.
pub fn send(client: &BlockingClient, tx: diem_types::transaction::SignedTransaction) -> Result<()> {
    use diem_json_rpc_types::views::VMStatusView;

    client.submit(&tx)?;
    assert_eq!(
        client
            .wait_for_signed_transaction(&tx, Some(std::time::Duration::from_secs(60)), None)?
            .into_inner()
            .vm_status,
        VMStatusView::Executed,
    );
    Ok(())
}

// returns ~/.shuffle
pub fn get_shuffle_dir() -> PathBuf {
    BaseDirs::new().unwrap().home_dir().join(".shuffle")
}

/// Generates the typescript bindings for the main Move package based on the embedded
/// diem types and Move stdlib. Mimics much of the transaction_builder_generator's CLI
/// except with typescript defaults and embedded content, as opposed to repo directory paths.
pub fn generate_typescript_libraries(project_path: &Path) -> Result<()> {
    let pkg_path = project_path.join(MAIN_PKG_PATH);
    let target_dir = pkg_path.join("generated");
    let installer = serdegen::typescript::Installer::new(target_dir.clone());
    generate_runtime(&installer)?;
    generate_transaction_builders(&pkg_path, &target_dir)?;
    Ok(())
}

fn generate_runtime(installer: &serdegen::typescript::Installer) -> Result<()> {
    installer
        .install_serde_runtime()
        .expect("unable to install Serde runtime");
    installer
        .install_bcs_runtime()
        .expect("unable to install BCS runtime");

    // diem types
    let diem_types_content = String::from_utf8_lossy(include_bytes!(
        "../../../testsuite/generate-format/tests/staged/diem.yaml"
    ));
    let mut registry = serde_yaml::from_str::<Registry>(diem_types_content.as_ref())?;
    buildgen::typescript::replace_keywords(&mut registry);

    let config = serdegen::CodeGeneratorConfig::new("diemTypes".to_string())
        .with_encodings(vec![serdegen::Encoding::Bcs]);
    installer
        .install_module(&config, &registry)
        .expect("unable to install typescript diem types");
    Ok(())
}

fn generate_transaction_builders(pkg_path: &Path, target_dir: &Path) -> Result<()> {
    let module_name = "diemStdlib";
    let abi_directory = pkg_path;
    let abis =
        buildgen::read_abis(&[abi_directory]).expect("failed to read ABI from Move package main");

    let installer: buildgen::typescript::Installer =
        buildgen::typescript::Installer::new(PathBuf::from(target_dir));
    installer
        .install_transaction_builders(module_name, abis.as_slice())
        .expect("unable to install transaction builders for package");
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::{new, shared::get_shuffle_dir};
    use directories::BaseDirs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    use super::generate_typescript_libraries;

    #[test]
    fn test_get_shuffle_dir() {
        let path = BaseDirs::new()
            .unwrap()
            .home_dir()
            .to_str()
            .unwrap()
            .to_string()
            + "/.shuffle";
        let correct_dir = PathBuf::from(path);
        assert_eq!(get_shuffle_dir(), correct_dir);
    }

    #[test]
    #[ignore]
    // Tests if the generated typesript libraries can actually be run by deno runtime.
    // `ignore`d tests are still run on CI via codegen-unit-test, but help keep
    // the local testsuite fast for devs.
    fn test_generate_typescript_libraries() {
        let tmpdir = tempdir().unwrap();
        let dir_path = tmpdir.path();
        new::write_example_move_packages(dir_path).expect("unable to create move main pkg");
        generate_typescript_libraries(dir_path).expect("unable to generate TS libraries");

        let output = std::process::Command::new("deno")
            .args([
                "run",
                dir_path
                    .join("main/generated/diemStdlib/mod.ts")
                    .to_string_lossy()
                    .as_ref(),
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
    }
}

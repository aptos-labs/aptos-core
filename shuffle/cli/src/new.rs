// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::shared;
use anyhow::Result;
use diem_genesis_tool::validator_builder::ValidatorConfig;
use diem_types::{account_address::AccountAddress, on_chain_config::VMPublishingOption};
use include_dir::{include_dir, Dir};
use move_cli::{
    package::cli as pkgcli,
    sandbox::utils::{on_disk_state_view::OnDiskStateView, Mode, ModeType},
};
use move_lang::shared::NumericalAddress;
use move_package::source_package::layout::SourcePackageLayout;
use once_cell::sync::Lazy;
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

/// Default blockchain configuration
pub const DEFAULT_BLOCKCHAIN: &str = "goodday";

/// Directory of generated transaction builders for helloblockchain.
const EXAMPLES_DIR: Dir = include_dir!("../move/examples/Message");
pub const MESSAGE_EXAMPLE_PATH: &str = "Message";

pub fn handle(blockchain: String, pathbuf: PathBuf) -> Result<()> {
    let project_path = pathbuf.as_path();
    println!("Creating shuffle project in {}", project_path.display());
    fs::create_dir_all(project_path)?;

    // Shuffle projects aspire to be on a Diem Core Framework that
    // does not include the DPN. ModeType::Diem is not quite there but is a step
    // towards that goal. Bare and Stdlib do not work as expected but this is a WIP
    let mode = Mode::new(ModeType::Diem);
    let build_path = project_path.join(MESSAGE_EXAMPLE_PATH).join("build");
    let storage_path = project_path.join(MESSAGE_EXAMPLE_PATH).join("storage");
    let state = mode.prepare_state(build_path.as_path(), storage_path.as_path())?;

    let config = shared::Config {
        blockchain,
        named_addresses: fetch_named_addresses(&state)?,
    };
    write_project_config(project_path, &config)?;
    write_example_move_packages(project_path)?;
    generate_validator_config(project_path)?;
    Ok(())
}

// Fetches the named addresses for a particular project or genesis.
// Uses a BTreeMap over HashMap because upstream NumericalAddress does as well,
// probably because order matters.
fn fetch_named_addresses(state: &OnDiskStateView) -> Result<BTreeMap<String, AccountAddress>> {
    let address_bytes_map = state.get_named_addresses(BTreeMap::new())?;
    Ok(map_address_bytes_to_account_address(address_bytes_map))
}

// map_address_bytes_to_account_address converts BTreeMap<String,NumericalAddress>
// to BTreeMap<String, AccountAddress> because NumericalAddress is not serializable.
fn map_address_bytes_to_account_address(
    original: BTreeMap<String, NumericalAddress>,
) -> BTreeMap<String, AccountAddress> {
    original
        .into_iter()
        .map(|(name, addr)| (name, addr.into_inner()))
        .collect()
}

fn write_project_config(path: &Path, config: &shared::Config) -> Result<()> {
    let toml_path = PathBuf::from(path).join("Shuffle").with_extension("toml");
    let toml_string = toml::to_string(config)?;
    fs::write(toml_path, toml_string)?;
    Ok(())
}

static EXAMPLE_BLOCKLIST: Lazy<HashSet<&'static str>> = Lazy::new(|| [].iter().cloned().collect());

// Writes the move packages for a new project
fn write_example_move_packages(root_path: &Path) -> Result<()> {
    let creation_path = Path::new(&root_path).join(MESSAGE_EXAMPLE_PATH);
    pkgcli::create_move_package("helloblockchain", &creation_path)?;

    println!("Copying Examples...");
    let pkgdir = root_path.join(MESSAGE_EXAMPLE_PATH);
    for entry in EXAMPLES_DIR.find("**/*").unwrap() {
        match entry {
            include_dir::DirEntry::Dir(d) => {
                fs::create_dir_all(pkgdir.join(d.path()))?;
            }
            include_dir::DirEntry::File(f) => {
                let filename = file_entry_to_string(&f)?;
                if EXAMPLE_BLOCKLIST.contains(filename.as_str()) {
                    continue;
                }
                let dst = pkgdir.join(f.path());
                fs::write(dst.as_path(), f.contents())?;
            }
        }
    }
    Ok(())
}

fn file_entry_to_string(f: &include_dir::File) -> Result<String> {
    Ok(f.path()
        .file_name()
        .ok_or_else(|| anyhow::format_err!("embedded example filename unavailable"))?
        .to_string_lossy()
        .to_string())
}

fn generate_validator_config(project_path: &Path) -> Result<ValidatorConfig> {
    let publishing_option = VMPublishingOption::open();
    shuffle_custom_node::generate_validator_config(
        project_path.join("nodeconfig").as_path(),
        diem_framework_releases::current_module_blobs().to_vec(),
        publishing_option,
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use diem_config::config::NodeConfig;
    use shared::Config;
    use tempfile::tempdir;

    #[test]
    fn test_write_project_config() {
        let dir = tempdir().unwrap();
        let config = Config {
            blockchain: String::from(DEFAULT_BLOCKCHAIN),
            named_addresses: map_address_bytes_to_account_address(
                diem_framework::diem_framework_named_addresses(),
            ),
        };

        write_project_config(dir.path(), &config).unwrap();

        let config_string =
            fs::read_to_string(dir.path().join("Shuffle").with_extension("toml")).unwrap();
        let read_config: Config = toml::from_str(config_string.as_str()).unwrap();
        assert_eq!(config, read_config);
        let actual_std_address = read_config.named_addresses["Std"].short_str_lossless();
        assert_eq!(actual_std_address, "1");
    }

    #[test]
    fn test_handle_e2e() {
        let dir = tempdir().unwrap();
        handle(String::from(DEFAULT_BLOCKCHAIN), PathBuf::from(dir.path())).unwrap();

        // spot check move starter files
        let expected_example_content = String::from_utf8_lossy(include_bytes!(
            "../../move/examples/Message/sources/Message.move"
        ));
        let actual_example_content =
            fs::read_to_string(dir.path().join("Message/sources/Message.move")).unwrap();
        assert_eq!(expected_example_content, actual_example_content);

        // check if we can load generated node.yaml config file
        let _node_config =
            NodeConfig::load(dir.path().join("nodeconfig/0/node.yaml").as_path()).unwrap();
    }
}

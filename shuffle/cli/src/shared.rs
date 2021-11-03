// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0
use crate::new::DEFAULT_NETWORK;
use anyhow::Result;
use diem_crypto::ed25519::{Ed25519PrivateKey, Ed25519PublicKey};
use diem_sdk::client::BlockingClient;
use diem_types::transaction::authenticator::AuthenticationKey;
use directories::BaseDirs;
use move_package::compilation::compiled_package::CompiledPackage;
use serde::{Deserialize, Serialize};
use serde_generate as serdegen;
use serde_generate::SourceInstaller;
use serde_reflection::Registry;
use std::{
    fs,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};
use transaction_builder_generator as buildgen;
use transaction_builder_generator::SourceInstaller as BuildgenSourceInstaller;

pub const MAIN_PKG_PATH: &str = "main";
const NEW_KEY_FILE_CONTENT: &[u8] = include_bytes!("../new_account.key");

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    blockchain: String,
    network: String,
}

impl Config {
    pub fn new(config_blockchain: String, config_network: Option<String>) -> Self {
        let normalized_network = match config_network {
            Some(network) => network,
            None => String::from(DEFAULT_NETWORK),
        };
        Self {
            blockchain: config_blockchain,
            network: normalized_network,
        }
    }

    pub fn get_network(&self) -> &str {
        &self.network
    }
}

pub fn get_home_path() -> PathBuf {
    BaseDirs::new()
        .expect("Unable to deduce base directory for OS")
        .home_dir()
        .to_path_buf()
}

pub fn read_config(project_path: &Path) -> Result<Config> {
    let config_string = fs::read_to_string(project_path.join("Shuffle").with_extension("toml"))?;
    let read_config: Config = toml::from_str(config_string.as_str())?;
    Ok(read_config)
}

/// Send a transaction to the blockchain through the blocking client.
pub fn send_transaction(
    client: &BlockingClient,
    tx: diem_types::transaction::SignedTransaction,
) -> Result<()> {
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

/// Checks the current directory, and then parent directories for a Shuffle.toml
/// file to indicate the base project directory.
pub fn get_shuffle_project_path(cwd: &Path) -> Result<PathBuf> {
    let mut path: PathBuf = PathBuf::from(cwd);
    let project_file = Path::new("Shuffle.toml");

    loop {
        path.push(project_file);

        if path.is_file() {
            path.pop();
            return Ok(path);
        }

        if !(path.pop() && path.pop()) {
            return Err(anyhow::anyhow!(
                "unable to find Shuffle.toml; are you in a Shuffle project?"
            ));
        }
    }
}

// returns ~/.shuffle
pub fn get_shuffle_dir() -> PathBuf {
    BaseDirs::new().unwrap().home_dir().join(".shuffle")
}

// Contains all the commonly used paths in shuffle/cli
pub struct Home {
    shuffle_path: PathBuf,
    root_key_path: PathBuf,
    node_config_path: PathBuf,
    account_path: PathBuf,
    latest_path: PathBuf,
    latest_key_path: PathBuf,
    latest_address_path: PathBuf,
    test_path: PathBuf,
    test_key_path: PathBuf,
    test_key_address_path: PathBuf,
}

impl Home {
    pub fn new(home_path: &Path) -> Result<Self> {
        Ok(Self {
            shuffle_path: home_path.join(".shuffle"),
            root_key_path: home_path.join(".shuffle/nodeconfig/mint.key"),
            node_config_path: home_path.join(".shuffle/nodeconfig/0/node.yaml"),
            account_path: home_path.join(".shuffle/accounts"),
            latest_path: home_path.join(".shuffle/accounts/latest"),
            latest_key_path: home_path.join(".shuffle/accounts/latest/dev.key"),
            latest_address_path: home_path.join(".shuffle/accounts/latest/address"),
            test_path: home_path.join(".shuffle/accounts/test"),
            test_key_path: home_path.join(".shuffle/accounts/test/dev.key"),
            test_key_address_path: home_path.join(".shuffle/accounts/test/address"),
        })
    }

    pub fn get_shuffle_path(&self) -> &Path {
        &self.shuffle_path
    }

    pub fn get_root_key_path(&self) -> &Path {
        &self.root_key_path
    }

    pub fn get_node_config_path(&self) -> &Path {
        &self.node_config_path
    }

    pub fn get_latest_path(&self) -> &Path {
        &self.latest_path
    }

    pub fn get_latest_key_path(&self) -> &Path {
        &self.latest_key_path
    }

    pub fn get_latest_address_path(&self) -> &Path {
        &self.latest_address_path
    }

    pub fn get_account_path(&self) -> &Path {
        &self.account_path
    }

    pub fn get_test_key_path(&self) -> &Path {
        &self.test_key_path
    }

    pub fn create_archive_dir(&self, time: Duration) -> Result<PathBuf> {
        let archived_dir = self.account_path.join(time.as_secs().to_string());
        fs::create_dir(&archived_dir)?;
        Ok(archived_dir)
    }

    pub fn archive_old_key(&self, archived_dir: &Path) -> Result<()> {
        let old_key_path = self.latest_key_path.as_path();
        let archived_key_path = archived_dir.join("dev.key");
        fs::copy(old_key_path, archived_key_path)?;
        Ok(())
    }

    pub fn archive_old_address(&self, archived_dir: &Path) -> Result<()> {
        let old_address_path = self.latest_address_path.as_path();
        let archived_address_path = archived_dir.join("address");
        fs::copy(old_address_path, archived_address_path)?;
        Ok(())
    }

    pub fn generate_shuffle_accounts_path(&self) -> Result<()> {
        if !self.account_path.is_dir() {
            fs::create_dir(self.account_path.as_path())?;
        }
        Ok(())
    }

    pub fn generate_shuffle_latest_path(&self) -> Result<()> {
        if !self.latest_path.is_dir() {
            fs::create_dir(self.latest_path.as_path())?;
        }
        Ok(())
    }

    pub fn generate_key_file(&self) -> Result<Ed25519PrivateKey> {
        // Using NEW_KEY_FILE for now due to hard coded address in
        // /diem/shuffle/move/examples/main/sources/move.toml
        fs::write(self.latest_key_path.as_path(), NEW_KEY_FILE_CONTENT)?;
        Ok(generate_key::load_key(self.latest_key_path.as_path()))
    }

    pub fn generate_address_file(&self, public_key: &Ed25519PublicKey) -> Result<()> {
        let address = AuthenticationKey::ed25519(public_key).derived_address();
        let address_filepath = self.latest_address_path.as_path();
        let mut file = File::create(address_filepath)?;
        file.write_all(address.to_string().as_ref())?;
        Ok(())
    }

    pub fn generate_shuffle_test_path(&self) -> Result<()> {
        if !self.test_path.is_dir() {
            fs::create_dir(self.test_path.as_path())?;
        }
        Ok(())
    }

    pub fn generate_testkey_file(&self) -> Result<Ed25519PrivateKey> {
        Ok(generate_key::generate_and_save_key(
            self.test_key_path.as_path(),
        ))
    }

    pub fn generate_testkey_address_file(&self, public_key: &Ed25519PublicKey) -> Result<()> {
        let address = AuthenticationKey::ed25519(public_key).derived_address();
        let address_filepath = self.test_key_address_path.as_path();
        let mut file = File::create(address_filepath)?;
        file.write_all(address.to_string().as_ref())?;
        Ok(())
    }

    pub fn save_root_key(&self, root_key_path: &Path) -> Result<()> {
        let new_root_key = generate_key::load_key(root_key_path);
        generate_key::save_key(new_root_key, self.latest_key_path.as_path());
        Ok(())
    }
}

/// Generates the typescript bindings for the main Move package based on the embedded
/// diem types and Move stdlib. Mimics much of the transaction_builder_generator's CLI
/// except with typescript defaults and embedded content, as opposed to repo directory paths.
pub fn generate_typescript_libraries(project_path: &Path) -> Result<()> {
    let _compiled_package = build_move_packages(project_path)?;

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
        .map_err(|e| anyhow::anyhow!("unable to install Serde runtime: {:?}", e))?;
    installer
        .install_bcs_runtime()
        .map_err(|e| anyhow::anyhow!("unable to install BCS runtime: {:?}", e))?;

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
        .map_err(|e| anyhow::anyhow!("unable to install typescript diem types: {:?}", e))?;
    Ok(())
}

/// Builds the packages in the shuffle project using the move package system.
pub fn build_move_packages(project_path: &Path) -> Result<CompiledPackage> {
    println!("Building Examples...");
    let pkgdir = project_path.join(MAIN_PKG_PATH);
    let config = move_package::BuildConfig {
        dev_mode: true,
        test_mode: false,
        generate_docs: false,
        generate_abis: true,
        install_dir: None,
    };

    config.compile_package(pkgdir.as_path(), &mut std::io::stdout())
}

fn generate_transaction_builders(pkg_path: &Path, target_dir: &Path) -> Result<()> {
    let module_name = "diemStdlib";
    let abi_directory = pkg_path;
    let abis = buildgen::read_abis(&[abi_directory])?;

    let installer: buildgen::typescript::Installer =
        buildgen::typescript::Installer::new(PathBuf::from(target_dir));
    installer
        .install_transaction_builders(module_name, abis.as_slice())
        .map_err(|e| anyhow::anyhow!("unable to install transaction builders: {:?}", e))?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::{generate_typescript_libraries, get_shuffle_project_path};
    use crate::{
        new,
        shared::{Config, Home},
    };
    use diem_crypto::PrivateKey;
    use diem_infallible::duration_since_epoch;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_get_shuffle_project_path() {
        let tmpdir = tempdir().unwrap();
        let dir_path = tmpdir.path();

        std::fs::create_dir_all(dir_path.join("nested")).unwrap();
        std::fs::write(dir_path.join("Shuffle.toml"), "goodday").unwrap();

        let actual = get_shuffle_project_path(dir_path.join("nested").as_path()).unwrap();
        let expectation = dir_path;
        assert_eq!(&actual, expectation);
    }

    #[test]
    fn test_home_create_archive_dir() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".shuffle/accounts")).unwrap();

        let home = Home::new(dir.path()).unwrap();
        let time = duration_since_epoch();
        home.create_archive_dir(time).unwrap();
        let test_archive_dir = dir
            .path()
            .join(".shuffle/accounts")
            .join(time.as_secs().to_string());
        assert_eq!(test_archive_dir.is_dir(), true);
    }

    #[test]
    fn test_home_archive_old_key() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".shuffle/accounts/latest")).unwrap();

        let home = Home::new(dir.path()).unwrap();
        let private_key = home.generate_key_file().unwrap();

        let time = duration_since_epoch();
        let archived_dir = home.create_archive_dir(time).unwrap();
        home.archive_old_key(&archived_dir).unwrap();
        let test_archive_key_path = dir
            .path()
            .join(".shuffle/accounts")
            .join(time.as_secs().to_string())
            .join("dev.key");
        let archived_key = generate_key::load_key(test_archive_key_path);

        assert_eq!(private_key, archived_key);
    }

    #[test]
    fn test_home_archive_old_address() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".shuffle/accounts/latest")).unwrap();

        let home = Home::new(dir.path()).unwrap();
        let private_key = home.generate_key_file().unwrap();
        home.generate_address_file(&private_key.public_key())
            .unwrap();
        let address_path = dir.path().join(".shuffle/accounts/latest/address");

        let time = duration_since_epoch();
        let archived_dir = home.create_archive_dir(time).unwrap();
        home.archive_old_address(&archived_dir).unwrap();
        let test_archive_address_path = dir
            .path()
            .join(".shuffle/accounts")
            .join(time.as_secs().to_string())
            .join("address");

        let old_address = fs::read_to_string(address_path).unwrap();
        let archived_address = fs::read_to_string(test_archive_address_path).unwrap();

        assert_eq!(old_address, archived_address);
    }

    #[test]
    fn test_home_generate_shuffle_accounts_path() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".shuffle")).unwrap();

        let home = Home::new(dir.path()).unwrap();
        home.generate_shuffle_accounts_path().unwrap();
        assert_eq!(dir.path().join(".shuffle/accounts").is_dir(), true);
    }

    #[test]
    fn test_home_generate_shuffle_latest_path() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".shuffle/accounts")).unwrap();

        let home = Home::new(dir.path()).unwrap();
        home.generate_shuffle_latest_path().unwrap();
        assert_eq!(dir.path().join(".shuffle/accounts/latest").is_dir(), true);
    }

    #[test]
    fn test_home_generate_key_file() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".shuffle/accounts/latest")).unwrap();

        let home = Home::new(dir.path()).unwrap();
        home.generate_key_file().unwrap();
        assert_eq!(
            dir.path().join(".shuffle/accounts/latest/dev.key").exists(),
            true
        );
    }

    #[test]
    fn test_home_generate_address_file() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".shuffle/accounts/latest")).unwrap();

        let home = Home::new(dir.path()).unwrap();
        let public_key = home.generate_key_file().unwrap().public_key();
        home.generate_address_file(&public_key).unwrap();
        assert_eq!(
            dir.path().join(".shuffle/accounts/latest/address").exists(),
            true
        );
    }

    #[test]
    fn test_home_get_shuffle_path() {
        let dir = tempdir().unwrap();
        let home = Home::new(dir.path()).unwrap();
        let correct_dir = dir.path().join(".shuffle");
        assert_eq!(correct_dir, home.get_shuffle_path());
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

        let script_path = dir_path.join("main/generated/diemStdlib/mod.ts");
        let output = std::process::Command::new("deno")
            .args(["run", script_path.to_string_lossy().as_ref()])
            .output()
            .unwrap();
        assert!(output.status.success());

        let script_contents = std::fs::read(script_path.to_string_lossy().as_ref()).unwrap();
        assert!(String::from_utf8_lossy(script_contents.as_ref())
            .contains("static encodeSetMessageScript(message: Uint8Array): DiemTypes.Script"));
    }

    #[test]
    fn test_home_get_latest_path() {
        let dir = tempdir().unwrap();
        let home = Home::new(dir.path()).unwrap();
        let correct_dir = dir.path().join(".shuffle/accounts/latest");
        assert_eq!(correct_dir, home.get_latest_path());
    }

    #[test]
    fn test_home_get_account_path() {
        let dir = tempdir().unwrap();
        let home = Home::new(dir.path()).unwrap();
        let correct_dir = dir.path().join(".shuffle/accounts");
        assert_eq!(correct_dir, home.get_account_path());
    }

    #[test]
    fn test_home_get_nodeconfig_path() {
        let dir = tempdir().unwrap();
        let home = Home::new(dir.path()).unwrap();
        let correct_dir = dir.path().join(".shuffle/nodeconfig/0/node.yaml");
        assert_eq!(correct_dir, home.get_node_config_path());
    }

    #[test]
    fn test_home_get_root_key_path() {
        let dir = tempdir().unwrap();
        let home = Home::new(dir.path()).unwrap();
        let correct_dir = dir.path().join(".shuffle/nodeconfig/mint.key");
        assert_eq!(correct_dir, home.get_root_key_path());
    }

    #[test]
    fn test_home_get_latest_key_path() {
        let dir = tempdir().unwrap();
        let home = Home::new(dir.path()).unwrap();
        let correct_dir = dir.path().join(".shuffle/accounts/latest/dev.key");
        assert_eq!(correct_dir, home.get_latest_key_path());
    }

    #[test]
    fn test_get_network() {
        let temp_config = Config::new(
            String::from("goodday"),
            Option::Some(String::from("127.0.0.1:8081")),
        );
        assert_eq!(temp_config.get_network(), "127.0.0.1:8081");
    }

    #[test]
    fn test_save_root_key() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".shuffle/accounts/latest")).unwrap();
        let user_root_key_path = dir.path().join("root.key");
        let user_root_key = generate_key::generate_and_save_key(&user_root_key_path);

        let home = Home::new(dir.path()).unwrap();
        home.save_root_key(&user_root_key_path).unwrap();
        let new_root_key = generate_key::load_key(home.latest_key_path);

        assert_eq!(new_root_key, user_root_key);
    }
}

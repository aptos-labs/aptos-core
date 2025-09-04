// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{FullNode, HealthCheckError, LocalVersion, Node, NodeExt, Validator, Version};
use anyhow::{anyhow, ensure, Context, Result};
use velor_config::{
    config::{NodeConfig, SECURE_STORAGE_FILENAME},
    keys::ConfigKey,
};
use velor_db::{
    common::{LEDGER_DB_NAME, STATE_MERKLE_DB_NAME},
    fast_sync_storage_wrapper::SECONDARY_DB_DIR,
};
use velor_logger::{debug, info};
use velor_sdk::{
    crypto::ed25519::Ed25519PrivateKey,
    types::{account_address::AccountAddress, PeerId},
};
use velor_state_sync_driver::metadata_storage::STATE_SYNC_DB_NAME;
use std::{
    env,
    fs::{self, OpenOptions},
    path::PathBuf,
    process::{Child, Command},
    str::FromStr,
};
use url::Url;

#[derive(Debug)]
struct Process(Child);

impl Drop for Process {
    // When the Process struct goes out of scope we need to kill the child process
    fn drop(&mut self) {
        // check if the process has already been terminated
        match self.0.try_wait() {
            // The child process has already terminated, perhaps due to a crash
            Ok(Some(_)) => {},

            // The process is still running so we need to attempt to kill it
            _ => {
                self.0.kill().expect("Process wasn't running");
                self.0.wait().unwrap();
            },
        }
    }
}

#[derive(Debug)]
pub struct LocalNode {
    version: LocalVersion,
    process: std::sync::Mutex<Option<Process>>,
    name: String,
    index: usize,
    account_private_key: Option<ConfigKey<Ed25519PrivateKey>>,
    peer_id: AccountAddress,
    directory: PathBuf,
    config: NodeConfig,
}

impl LocalNode {
    pub fn new(
        version: LocalVersion,
        name: String,
        index: usize,
        directory: PathBuf,
        account_private_key: Option<ConfigKey<Ed25519PrivateKey>>,
    ) -> Result<Self> {
        let config_path = directory.join("node.yaml");
        let config = NodeConfig::load_from_path(&config_path).map_err(|error| {
            anyhow!(
                "Failed to load NodeConfig from file: {:?}. Error: {:?}",
                config_path,
                error
            )
        })?;
        let peer_id = config
            .get_peer_id()
            .ok_or_else(|| anyhow!("unable to retrieve PeerId from config"))?;

        Ok(Self {
            version,
            process: std::sync::Mutex::new(None),
            name,
            index,
            account_private_key,
            peer_id,
            directory,
            config,
        })
    }

    pub fn base_dir(&self) -> PathBuf {
        self.directory.clone()
    }

    pub fn config_path(&self) -> PathBuf {
        self.directory.join("node.yaml")
    }

    pub fn log_path(&self) -> PathBuf {
        self.directory.join("log")
    }

    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn account_private_key(&self) -> &Option<ConfigKey<Ed25519PrivateKey>> {
        &self.account_private_key
    }

    pub fn start(&self) -> Result<()> {
        let mut process_locker = self.process.lock().unwrap();
        ensure!(
            process_locker.is_none(),
            "node {} already running",
            self.name
        );

        // Ensure log file exists
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.log_path())?;

        // Start node process
        let mut node_command = Command::new(self.version.bin());
        node_command
            .current_dir(&self.directory)
            .arg("-f")
            .arg(self.config_path());
        if env::var("RUST_LOG").is_err() {
            // Only set our RUST_LOG if its not present in environment
            node_command.env("RUST_LOG", "debug");
        }
        node_command.stdout(log_file.try_clone()?).stderr(log_file);
        let process = node_command.spawn().with_context(|| {
            format!(
                "Error launching node process with binary: {:?}",
                self.version.bin()
            )
        })?;

        // We print out the commands and PIDs for debugging of local swarms
        info!(
            "Started node {} (PID: {}) with command: {:?}, log_path: {:?}",
            self.name,
            process.id(),
            node_command,
            self.log_path(),
        );

        // We print out the API endpoints of each node for local debugging
        info!(
            "Node {}: REST API is listening at: http://127.0.0.1:{}",
            self.name,
            self.config.api.address.port()
        );
        info!(
            "Node {}: Inspection service is listening at http://127.0.0.1:{}",
            self.name, self.config.inspection_service.port
        );
        info!(
            "Node {}: Admin service is listening at http://127.0.0.1:{}",
            self.name, self.config.admin_service.port
        );
        info!(
            "Node {}: Backup service is listening at http://127.0.0.1:{}",
            self.name,
            self.config.storage.backup_service_address.port()
        );

        *process_locker = Some(Process(process));

        Ok(())
    }

    pub fn stop(&self) {
        *(self.process.lock().unwrap()) = None;
    }

    pub fn port(&self) -> u16 {
        self.config.api.address.port()
    }

    pub fn inspection_service_port(&self) -> u16 {
        self.config.inspection_service.port
    }

    pub fn config(&self) -> &NodeConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut NodeConfig {
        &mut self.config
    }

    pub fn upgrade(&mut self, version: LocalVersion) -> Result<()> {
        self.stop();
        self.version = version;
        self.start()
    }

    pub fn get_log_contents(&self) -> Result<String> {
        fs::read_to_string(self.log_path()).map_err(Into::into)
    }

    pub async fn health_check(&self) -> Result<(), HealthCheckError> {
        debug!("Health check on node '{}'", self.name);

        {
            let mut process_locker = self.process.lock().unwrap();
            let process = process_locker.as_mut();
            if let Some(p) = process {
                match p.0.try_wait() {
                    // This would mean the child process has crashed
                    Ok(Some(status)) => {
                        let error = format!("Node '{}' crashed with: {}", self.name, status);
                        return Err(HealthCheckError::NotRunning(error));
                    },

                    // This is the case where the node is still running
                    Ok(None) => {},

                    // Some other unknown error
                    Err(e) => {
                        return Err(HealthCheckError::Unknown(e.into()));
                    },
                }
            } else {
                let error = format!("Node '{}' is stopped", self.name);
                return Err(HealthCheckError::NotRunning(error));
            }
        }

        self.inspection_client()
            .get_forge_metrics()
            .await
            .map(|_| ())
            .map_err(HealthCheckError::Failure)?;

        self.rest_client()
            .get_ledger_information()
            .await
            .map(|_| ())
            .map_err(|err| HealthCheckError::Failure(err.into()))
    }
}

#[async_trait::async_trait]
impl Node for LocalNode {
    fn peer_id(&self) -> PeerId {
        self.peer_id()
    }

    fn index(&self) -> usize {
        self.index
    }

    fn name(&self) -> &str {
        self.name()
    }

    fn version(&self) -> Version {
        self.version.version()
    }

    fn rest_api_endpoint(&self) -> Url {
        let ip = self.config().api.address.ip();
        let port = self.config().api.address.port();
        Url::from_str(&format!("http://{}:{}/v1", ip, port)).expect("Invalid URL.")
    }

    fn inspection_service_endpoint(&self) -> Url {
        Url::parse(&format!(
            "http://localhost:{}",
            self.inspection_service_port()
        ))
        .unwrap()
    }

    fn config(&self) -> &NodeConfig {
        self.config()
    }

    async fn start(&self) -> Result<()> {
        self.start()
    }

    async fn stop(&self) -> Result<()> {
        self.stop();
        Ok(())
    }

    async fn get_identity(&self) -> Result<String> {
        todo!()
    }

    async fn set_identity(&self, _k8s_secret_name: String) -> Result<()> {
        todo!()
    }

    async fn clear_storage(&self) -> Result<()> {
        // Remove all storage files (i.e., blockchain data, consensus data and state sync data)
        let node_config = self.config();
        let ledger_db_path = node_config.storage.dir().join(LEDGER_DB_NAME);
        let state_db_path = node_config.storage.dir().join(STATE_MERKLE_DB_NAME);
        let secure_storage_path = node_config.get_working_dir().join(SECURE_STORAGE_FILENAME);
        let state_sync_db_path = node_config.storage.dir().join(STATE_SYNC_DB_NAME);
        let secondary_db_path = node_config.storage.dir().join(SECONDARY_DB_DIR);

        debug!(
            "Deleting ledger, state, secure and state sync db paths ({:?}, {:?}, {:?}, {:?}, {:?}) for node {:?}",
            ledger_db_path.as_path(),
            state_db_path.as_path(),
            secure_storage_path.as_path(),
            state_sync_db_path.as_path(),
            secondary_db_path.as_path(),
            self.name
        );

        // Verify the files exist
        assert!(ledger_db_path.as_path().exists() && state_db_path.as_path().exists());
        assert!(state_sync_db_path.as_path().exists());
        if self.config.base.role.is_validator() {
            assert!(secure_storage_path.as_path().exists());
        }

        // Remove the primary DB files
        fs::remove_dir_all(ledger_db_path)
            .map_err(anyhow::Error::from)
            .context("Failed to delete ledger_db_path")?;
        fs::remove_dir_all(state_db_path)
            .map_err(anyhow::Error::from)
            .context("Failed to delete state_db_path")?;
        fs::remove_dir_all(state_sync_db_path)
            .map_err(anyhow::Error::from)
            .context("Failed to delete state_sync_db_path")?;

        // Remove the secondary DB files
        if secondary_db_path.as_path().exists() {
            fs::remove_dir_all(secondary_db_path)
                .map_err(anyhow::Error::from)
                .context("Failed to delete secondary_db_path")?;
        }

        // Remove the secure storage file
        if self.config.base.role.is_validator() {
            fs::remove_file(secure_storage_path)
                .map_err(anyhow::Error::from)
                .context("Failed to delete secure_storage_db_path")?;
        }

        // Stop the node to clear buffers
        self.stop();

        Ok(())
    }

    async fn health_check(&self) -> Result<(), HealthCheckError> {
        self.health_check().await
    }

    fn counter(&self, _counter: &str, _port: u64) -> Result<f64> {
        todo!()
    }

    // local node does not need to expose metric end point
    fn expose_metric(&self) -> Result<u64> {
        Ok(0)
    }

    fn service_name(&self) -> Option<String> {
        None
    }
}

impl Validator for LocalNode {}
impl FullNode for LocalNode {}

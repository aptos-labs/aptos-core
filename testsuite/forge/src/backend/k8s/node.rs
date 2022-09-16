// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::backend::k8s::stateful_set;
use crate::{
    get_free_port, scale_stateful_set_replicas, FullNode, HealthCheckError, Node, NodeExt, Result,
    Validator, Version, KUBECTL_BIN, LOCALHOST, NODE_METRIC_PORT, REST_API_HAPROXY_SERVICE_PORT,
    REST_API_SERVICE_PORT,
};
use anyhow::{anyhow, format_err};
use aptos_config::config::NodeConfig;
use aptos_logger::info;
use aptos_rest_client::Client as RestClient;
use aptos_sdk::types::PeerId;
use aptos_secure_storage::SECURE_STORAGE_DB_NAME;
use aptosdb::{LEDGER_DB_NAME, STATE_MERKLE_DB_NAME};
use reqwest::Url;
use serde_json::Value;
use state_sync_driver::metadata_storage::STATE_SYNC_DB_NAME;
use std::{
    fmt::{Debug, Formatter},
    process::{Command, Stdio},
    str::FromStr,
    thread,
    time::{Duration, Instant},
};

const APTOS_DATA_DIR: &str = "/opt/aptos/data";

pub struct K8sNode {
    pub(crate) name: String,
    pub(crate) stateful_set_name: String,
    pub(crate) peer_id: PeerId,
    pub(crate) index: usize,
    pub(crate) service_name: String,
    pub(crate) rest_api_port: u32,
    pub version: Version,
    pub namespace: String,
    // whether this node has HAProxy in front of it
    pub haproxy_enabled: bool,
    // whether we should try using port-forward on the Service to reach this node
    pub port_forward_enabled: bool,
}

impl K8sNode {
    fn rest_api_port(&self) -> u32 {
        self.rest_api_port
    }

    fn service_name(&self) -> String {
        self.service_name.clone()
    }

    pub(crate) fn rest_client(&self) -> RestClient {
        RestClient::new(self.rest_api_endpoint())
    }

    pub fn stateful_set_name(&self) -> &str {
        &self.stateful_set_name
    }

    fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Start a port-forward to the node's Service
    fn port_forward(&self, port: u32, remote_port: u32) -> Result<()> {
        let port_forward_args = [
            "port-forward",
            "-n",
            self.namespace(),
            &format!("svc/{}", self.service_name()),
            &format!("{}:{}", port, remote_port),
        ];
        // spawn a port-forward child process
        let cmd = Command::new(KUBECTL_BIN)
            .args(port_forward_args)
            .stdout(Stdio::null())
            // .stderr(Stdio::null())
            .spawn();
        match cmd {
            Ok(mut child) => {
                // sleep a bit and check if port-forward failed for some reason
                let timeout = Duration::from_secs(1);
                thread::sleep(timeout);
                match child.try_wait() {
                    Ok(Some(status)) => {
                        info!("Port-forward may have started already: exit {}", status);
                        Ok(())
                    }
                    Ok(None) => {
                        info!(
                            "Port-forward started for {:?} from {} --> {}",
                            self, port, remote_port
                        );
                        Ok(())
                    }
                    Err(err) => Err(anyhow!(
                        "Port-forward did not work: {:?} error {}",
                        port_forward_args,
                        err
                    )),
                }
            }
            Err(err) => Err(anyhow!(
                "Port-forward did not start: {:?} error {}",
                port_forward_args,
                err
            )),
        }
    }

    pub fn port_forward_rest_api(&self) -> Result<()> {
        let remote_rest_api_port = if self.haproxy_enabled {
            REST_API_HAPROXY_SERVICE_PORT
        } else {
            REST_API_SERVICE_PORT
        };
        self.port_forward(self.rest_api_port(), remote_rest_api_port)
    }
}

#[async_trait::async_trait]
impl Node for K8sNode {
    fn name(&self) -> &str {
        &self.name
    }

    fn index(&self) -> usize {
        self.index
    }

    fn peer_id(&self) -> PeerId {
        self.peer_id
    }

    async fn start(&mut self) -> Result<()> {
        scale_stateful_set_replicas(self.stateful_set_name(), self.namespace(), 1).await?;
        // need to port-forward again since the node is coming back
        // note that we will get a new port
        if self.port_forward_enabled {
            self.rest_api_port = get_free_port();
            self.port_forward_rest_api()?;
        }
        self.wait_until_healthy(Instant::now() + Duration::from_secs(60))
            .await
    }

    async fn stop(&mut self) -> Result<()> {
        info!("going to stop node {}", self.stateful_set_name());
        scale_stateful_set_replicas(self.stateful_set_name(), self.namespace(), 0).await
    }

    fn version(&self) -> Version {
        self.version.clone()
    }

    fn rest_api_endpoint(&self) -> Url {
        let host = if self.port_forward_enabled {
            LOCALHOST
        } else {
            &self.service_name
        };
        Url::from_str(&format!("http://{}:{}/v1", host, self.rest_api_port()))
            .expect("Invalid URL.")
    }

    async fn clear_storage(&mut self) -> Result<()> {
        // Remove all storage files
        let ledger_db_path = format!("{}/db/{}", APTOS_DATA_DIR, LEDGER_DB_NAME);
        let state_db_path = format!("{}/db/{}", APTOS_DATA_DIR, STATE_MERKLE_DB_NAME);
        let secure_storage_db_path = format!("{}/{}", APTOS_DATA_DIR, SECURE_STORAGE_DB_NAME);
        let state_sync_db_path = format!("{}/db/{}", APTOS_DATA_DIR, STATE_SYNC_DB_NAME);

        let delete_storage_paths = [
            "-n",
            self.namespace(),
            "exec",
            &format!("sts/{}", self.stateful_set_name()),
            "--",
            "rm",
            "-rf",
            &ledger_db_path,
            &state_db_path,
            &secure_storage_db_path,
            &state_sync_db_path,
        ];
        info!("{:?}", delete_storage_paths);
        let cleanup_output = Command::new(KUBECTL_BIN)
            .stdout(Stdio::inherit())
            .args(&delete_storage_paths)
            .output()
            .expect("failed to clear node storage");
        assert!(
            cleanup_output.status.success(),
            "{}",
            String::from_utf8(cleanup_output.stderr).unwrap()
        );

        // Stop the node to clear buffers
        // This step must be done after removing the storage files, since clearing storage involves exec into the (running) node
        self.stop().await?;

        Ok(())
    }

    fn config(&self) -> &NodeConfig {
        todo!()
    }

    // TODO: replace this with prometheus query?
    fn counter(&self, counter: &str, port: u64) -> Result<f64> {
        let response: Value =
            reqwest::blocking::get(format!("http://{}:{}/counters", LOCALHOST, port))?.json()?;
        if let Value::Number(ref response) = response[counter] {
            if let Some(response) = response.as_f64() {
                Ok(response)
            } else {
                Err(format_err!(
                    "Failed to parse counter({}) as f64: {:?}",
                    counter,
                    response
                ))
            }
        } else {
            Err(format_err!(
                "Counter({}) was not a Value::Number: {:?}",
                counter,
                response[counter]
            ))
        }
    }

    // TODO: verify this still works
    fn expose_metric(&self) -> Result<u64> {
        let port = get_free_port();
        self.port_forward(port, NODE_METRIC_PORT)?;

        Ok(port as u64)
    }

    async fn health_check(&mut self) -> Result<(), HealthCheckError> {
        self.rest_client()
            .get_ledger_information()
            .await
            .map(|_| ())
            .map_err(|e| {
                HealthCheckError::Failure(format_err!("K8s node health_check failed: {}", e))
            })
    }

    // TODO: verify this still works
    fn inspection_service_endpoint(&self) -> Url {
        Url::parse(&format!(
            "http://{}:{}",
            &self.service_name(),
            self.rest_api_port()
        ))
        .unwrap()
    }

    async fn get_identity(&mut self) -> Result<String> {
        stateful_set::get_identity(self.stateful_set_name(), self.namespace()).await
    }

    async fn set_identity(&mut self, k8s_secret_name: String) -> Result<()> {
        stateful_set::set_identity(
            self.stateful_set_name(),
            self.namespace(),
            k8s_secret_name.as_str(),
        )
        .await
    }
}

impl Validator for K8sNode {}

impl FullNode for K8sNode {}

impl Debug for K8sNode {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let host = if self.port_forward_enabled {
            LOCALHOST
        } else {
            &self.service_name
        };
        write!(f, "{} @ {}", self.name, host)
    }
}

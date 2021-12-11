// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    scale_sts_replica, FullNode, HealthCheckError, Node, NodeExt, Result, Validator, Version,
};
use anyhow::{format_err, Context};
use diem_config::config::NodeConfig;
use diem_rest_client::Client as RestClient;
use diem_sdk::types::PeerId;
use reqwest::Url;
use serde_json::Value;
use std::{
    fmt::{Debug, Formatter},
    process::{Command, Stdio},
    str::FromStr,
    thread,
    time::{Duration, Instant},
};

const NODE_METRIC_PORT: u64 = 9101;

pub struct K8sNode {
    pub(crate) name: String,
    pub(crate) sts_name: String,
    pub(crate) peer_id: PeerId,
    pub(crate) node_id: usize,
    pub(crate) dns: String,
    pub(crate) ip: String,
    pub(crate) port: u32,
    pub(crate) rest_api_port: u32,
    pub version: Version,
}

impl K8sNode {
    fn port(&self) -> u32 {
        self.port
    }

    fn rest_api_port(&self) -> u32 {
        self.rest_api_port
    }

    #[allow(dead_code)]
    fn dns(&self) -> String {
        self.dns.clone()
    }

    fn ip(&self) -> String {
        self.ip.clone()
    }

    #[allow(dead_code)]
    fn node_id(&self) -> usize {
        self.node_id
    }

    pub(crate) fn rest_client(&self) -> RestClient {
        RestClient::new(self.rest_api_endpoint())
    }

    fn sts_name(&self) -> &str {
        &self.sts_name
    }
}

impl Node for K8sNode {
    fn peer_id(&self) -> PeerId {
        self.peer_id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> Version {
        self.version.clone()
    }

    fn json_rpc_endpoint(&self) -> Url {
        Url::from_str(&format!("http://{}:{}/v1", self.ip(), self.port())).expect("Invalid URL.")
    }

    fn rest_api_endpoint(&self) -> Url {
        Url::from_str(&format!("http://{}:{}", self.ip(), self.rest_api_port()))
            .expect("Invalid URL.")
    }

    fn debug_endpoint(&self) -> Url {
        Url::parse(&format!("http://{}:{}", self.ip(), self.port())).unwrap()
    }

    fn config(&self) -> &NodeConfig {
        todo!()
    }

    fn start(&mut self) -> Result<()> {
        scale_sts_replica(self.sts_name(), 1)?;
        self.wait_until_healthy(Instant::now() + Duration::from_secs(60))?;

        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        println!("going to stop node {}", self.sts_name());
        scale_sts_replica(self.sts_name(), 0)
    }

    fn clear_storage(&mut self) -> Result<()> {
        let sts_name = self.sts_name.clone();
        let pvc_name = if sts_name.contains("fullnode") {
            format!("fn-{}-0", sts_name)
        } else {
            sts_name
        };
        let delete_pvc_args = ["delete", "pvc", &pvc_name];
        println!("{:?}", delete_pvc_args);
        let cleanup_output = Command::new("kubectl")
            .stdout(Stdio::inherit())
            .args(&delete_pvc_args)
            .output()
            .expect("failed to scale sts replicas");
        assert!(
            cleanup_output.status.success(),
            "{}",
            String::from_utf8(cleanup_output.stderr).unwrap()
        );

        Ok(())
    }

    fn health_check(&mut self) -> Result<(), HealthCheckError> {
        reqwest::blocking::get(self.rest_api_endpoint()).map_err(|e| {
            HealthCheckError::Failure(format_err!("K8s node health_check failed: {}", e))
        })?;
        Ok(())
    }

    fn counter(&self, counter: &str, port: u64) -> Result<f64> {
        let response: Value =
            reqwest::blocking::get(format!("http://localhost:{}/counters", port))?.json()?;
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

    fn expose_metric(&self) -> Result<u64> {
        let pod_name = format!("{}-0", self.sts_name);
        let mut port = NODE_METRIC_PORT + 2 * self.node_id as u64;
        if pod_name.contains("fullnode") {
            port += 1;
        }
        let port_forward_args = [
            "port-forward",
            &format!("pod/{}", pod_name),
            &format!("{}:{}", port, NODE_METRIC_PORT),
        ];
        println!("{:?}", port_forward_args);
        let _ = Command::new("kubectl")
            .stdout(Stdio::null())
            .args(&port_forward_args)
            .spawn()
            .with_context(|| format!("Error port forwarding for node {}", pod_name))?;
        thread::sleep(Duration::from_secs(5));

        Ok(port)
    }
}

impl Validator for K8sNode {}

impl FullNode for K8sNode {}

impl Debug for K8sNode {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

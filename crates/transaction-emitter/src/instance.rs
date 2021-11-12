// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::{format_err, Result};
use diem_client::Client as JsonRpcClient;
use reqwest::{Client, Url};
use std::{
    fmt,
    str::FromStr,
    time::{Duration, Instant},
};
use tokio::time;

#[derive(Clone)]
pub struct Instance {
    peer_name: String,
    ip: String,
    ac_port: u32,
    debug_interface_port: Option<u32>,
    http_client: Client,
}

impl Instance {
    pub fn new(
        peer_name: String,
        ip: String,
        ac_port: u32,
        debug_interface_port: Option<u32>,
        http_client: Client,
    ) -> Instance {
        Instance {
            peer_name,
            ip,
            ac_port,
            debug_interface_port,
            http_client,
        }
    }

    pub async fn try_json_rpc(&self) -> Result<()> {
        self.json_rpc_client().batch(Vec::new()).await?;
        Ok(())
    }

    pub async fn wait_json_rpc(&self, deadline: Instant) -> Result<()> {
        while self.try_json_rpc().await.is_err() {
            if Instant::now() > deadline {
                return Err(format_err!("wait_json_rpc for {} timed out", self));
            }
            time::sleep(Duration::from_secs(3)).await;
        }
        Ok(())
    }

    pub fn peer_name(&self) -> &String {
        &self.peer_name
    }

    pub fn ip(&self) -> &String {
        &self.ip
    }

    pub fn ac_port(&self) -> u32 {
        self.ac_port
    }

    pub fn json_rpc_url(&self) -> Url {
        Url::from_str(&format!("http://{}:{}/v1", self.ip(), self.ac_port())).expect("Invalid URL.")
    }

    pub fn debug_interface_port(&self) -> Option<u32> {
        self.debug_interface_port
    }

    pub fn json_rpc_client(&self) -> JsonRpcClient {
        JsonRpcClient::new(self.json_rpc_url().to_string())
    }
}

impl fmt::Display for Instance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}({})", self.peer_name, self.ip)
    }
}

impl fmt::Debug for Instance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

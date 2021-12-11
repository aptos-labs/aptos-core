// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::{format_err, Result};
use diem_rest_client::Client as RestClient;
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
    // todo: remove this, no where is using it
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

    pub async fn wait_server_ready(&self, deadline: Instant) -> Result<()> {
        while self.rest_client().get_ledger_information().await.is_err() {
            if Instant::now() > deadline {
                return Err(format_err!("wait_server_ready for {} timed out", self));
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

    pub fn api_url(&self) -> Url {
        Url::from_str(&format!("http://{}:{}", self.ip(), self.ac_port())).expect("Invalid URL.")
    }

    pub fn debug_interface_port(&self) -> Option<u32> {
        self.debug_interface_port
    }

    pub fn rest_client(&self) -> RestClient {
        RestClient::new(self.api_url())
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

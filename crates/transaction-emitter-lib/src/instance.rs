// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_rest_client::{Client as RestClient, USER_AGENT};
use reqwest::Url;
use std::{fmt, time::Duration};

#[derive(Clone)]
pub struct Instance {
    peer_name: String,
    url: Url,
    debug_interface_port: Option<u32>,
}

impl Instance {
    pub fn new(peer_name: String, url: Url, debug_interface_port: Option<u32>) -> Instance {
        Instance {
            peer_name,
            url,
            debug_interface_port,
        }
    }

    pub fn peer_name(&self) -> &String {
        &self.peer_name
    }

    pub fn api_url(&self) -> Url {
        self.url.clone()
    }

    pub fn debug_interface_port(&self) -> Option<u32> {
        self.debug_interface_port
    }

    pub fn rest_client(&self) -> RestClient {
        let inner = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent(USER_AGENT)
            .cookie_store(true)
            .build()
            .unwrap();
        RestClient::from((inner, self.api_url()))
    }
}

impl fmt::Display for Instance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}({})", self.peer_name, self.api_url())
    }
}

impl fmt::Debug for Instance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

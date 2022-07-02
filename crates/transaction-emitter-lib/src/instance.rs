// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_rest_client::Client as RestClient;
use reqwest::Url;
use std::fmt;

#[derive(Clone)]
pub struct Instance {
    peer_name: String,
    url: Url,
    inspection_service_port: Option<u32>,
}

impl Instance {
    pub fn new(peer_name: String, url: Url, inspection_service_port: Option<u32>) -> Instance {
        Instance {
            peer_name,
            url,
            inspection_service_port,
        }
    }

    pub fn peer_name(&self) -> &String {
        &self.peer_name
    }

    pub fn api_url(&self) -> Url {
        self.url.clone()
    }

    pub fn inspection_service_port(&self) -> Option<u32> {
        self.inspection_service_port
    }

    pub fn rest_client(&self) -> RestClient {
        RestClient::new(self.api_url())
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

// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_rest_client::{velor_api_types, VelorBaseUrl, Client as RestClient};
use reqwest::Url;
use std::fmt;

// Custom header value to identify the client
const X_VELOR_CLIENT_VALUE: &str = "velor-transaction-emitter";

#[derive(Clone)]
pub struct Instance {
    peer_name: String,
    url: Url,
    inspection_service_port: Option<u32>,
    api_key: Option<String>,
}

impl Instance {
    pub fn new(
        peer_name: String,
        url: Url,
        inspection_service_port: Option<u32>,
        api_key: Option<String>,
    ) -> Instance {
        Instance {
            peer_name,
            url,
            inspection_service_port,
            api_key,
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
        let client = RestClient::builder(VelorBaseUrl::Custom(self.api_url()))
            .header(velor_api_types::X_VELOR_CLIENT, X_VELOR_CLIENT_VALUE)
            .expect("Failed to initialize REST Client instance");

        // add the API key if it is provided
        let client = if let Some(api_key) = &self.api_key {
            client.api_key(api_key)
        } else {
            Ok(client)
        };

        client
            .expect("Failed to build REST Client instance")
            .build()
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

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_rest_client::Client;
use std::ops::Deref;

pub struct MovementRestClient(Client);

impl MovementRestClient {
    pub fn new(url: &str) -> Result<Self, anyhow::Error> {
        let client = Client::new(
            url.parse()
                .map_err(|e| anyhow::anyhow!("failed to parse Movement rest api url: {}", e))?,
        );
        Ok(Self(client))
    }
}

impl Deref for MovementRestClient {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct MovementAptosRestClient(Client);

impl MovementAptosRestClient {
    pub fn new(url: &str) -> Result<Self, anyhow::Error> {
        let client =
            Client::new(url.parse().map_err(|e| {
                anyhow::anyhow!("failed to parse Movement Aptos rest api url: {}", e)
            })?);
        Ok(Self(client))
    }
}

impl Deref for MovementAptosRestClient {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

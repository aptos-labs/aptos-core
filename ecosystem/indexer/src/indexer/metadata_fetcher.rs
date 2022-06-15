// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::models::metadata::TokenMetaFromURI;
use anyhow::Result;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};

pub enum UriType {
    ARWEAVE { uri: String },
    IPFS { uri: String },
    UNKNOWN { uri: String },
}

pub fn get_type(uri: String) -> UriType {
    if uri.contains("IPFS/") {
        UriType::IPFS { uri }
    } else if uri.contains("arweave.net/") {
        UriType::ARWEAVE { uri }
    } else {
        UriType::UNKNOWN { uri }
    }
}

pub struct MetaDataFetcher {
    restclient: ClientWithMiddleware,
}

impl MetaDataFetcher {
    pub fn new() -> Self {
        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
        MetaDataFetcher {
            restclient: ClientBuilder::new(reqwest::Client::new())
                .with(RetryTransientMiddleware::new_with_policy(retry_policy))
                .build(),
        }
    }

    async fn read_http_uri(&self, uri: String) -> Result<serde_json::Value> {
        let resp = self
            .restclient
            .get(uri)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        Ok(resp)
    }

    fn parse_json(&self, value: serde_json::Value) -> Result<TokenMetaFromURI> {
        Ok(serde_json::value::from_value::<TokenMetaFromURI>(value)?)
    }

    pub async fn get_metadata(&self, uri: String) -> Option<TokenMetaFromURI> {
        match get_type(uri) {
            UriType::ARWEAVE { uri } => match self.read_http_uri(uri).await {
                Ok(value) => self.parse_json(value).ok(),
                _ => None,
            },
            _ => None,
        }
    }
}

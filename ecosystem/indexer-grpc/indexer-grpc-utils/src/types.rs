// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    ops::Deref,
    str::FromStr,
};
use url::Url;

/// A URL that only allows the redis:// scheme.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RedisUrl(pub Url);

impl FromStr for RedisUrl {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url = Url::parse(s)?;
        if url.scheme() != "redis" {
            return Err(anyhow::anyhow!("Invalid scheme: {}", url.scheme()));
        }
        Ok(RedisUrl(url))
    }
}

impl<'de> Deserialize<'de> for RedisUrl {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let url = Url::deserialize(deserializer)?;
        if url.scheme() != "redis" {
            return Err(serde::de::Error::custom(format!(
                "Invalid scheme: {}",
                url.scheme()
            )));
        }
        Ok(Self(url))
    }
}

impl Deref for RedisUrl {
    type Target = Url;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<RedisUrl> for Url {
    fn from(redis_url: RedisUrl) -> Self {
        redis_url.0
    }
}

impl Display for RedisUrl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

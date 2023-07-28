// Copyright © Aptos Foundation

use regex::Regex;
use std::env;
use url::Url;

pub struct URIParser;

impl URIParser {
    /// Attempts to parse IPFS URI to use dedicated gateway.
    /// Returns the original URI if parsing fails.
    pub fn parse(uri: String) -> anyhow::Result<String> {
        let modified_uri = if uri.starts_with("ipfs://") {
            uri.replace("ipfs://", "https://ipfs.com/ipfs/")
        } else {
            uri
        };

        let re = Regex::new(r"^(ipfs/)(?P<cid>[a-zA-Z0-9]+)(?P<path>/.*)?$")?;

        let path = Url::parse(&modified_uri)?
            .path_segments()
            .map(|segments| segments.collect::<Vec<_>>().join("/"));

        if let Some(captures) = re.captures(&path.unwrap_or_default()) {
            let cid = captures["cid"].to_string();
            let path = captures.name("path").map(|m| m.as_str().to_string());

            Ok(format!(
                "{}/{}{}",
                env::var("IPFS_PREFIX")?,
                cid,
                path.unwrap_or_default()
            ))
        } else {
            Err(anyhow::anyhow!("Invalid IPFS URI"))
        }
    }
}

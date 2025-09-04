// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils::{
    constants::IPFS_AUTH_KEY,
    counters::{PARSE_URI_INVOCATION_COUNT, PARSE_URI_TYPE_COUNT},
};
use regex::{Captures, Regex};
use url::Url;

pub struct URIParser;

impl URIParser {
    /// Attempts to parse IPFS URI to use dedicated gateway.
    /// Returns the original URI if parsing fails.
    pub fn parse(
        ipfs_prefix: &str,
        uri: &str,
        ipfs_auth_key: Option<&str>,
    ) -> anyhow::Result<String> {
        PARSE_URI_INVOCATION_COUNT.inc();
        if uri.contains("arweave.net") {
            PARSE_URI_TYPE_COUNT.with_label_values(&["arweave"]).inc();
            return Ok(uri.to_string());
        }

        let modified_uri = if uri.starts_with("ipfs://") {
            uri.replace("ipfs://", "https://ipfs.com/ipfs/")
        } else {
            uri.to_string()
        };

        let ipfs_auth_param = if ipfs_auth_key.is_some() {
            Some(format!("?{}={}", IPFS_AUTH_KEY, ipfs_auth_key.unwrap()))
        } else {
            None
        };

        // Expects the following format for provided URIs `ipfs/{CID}/{path}`
        let re = Regex::new(r"^(ipfs/)(?P<cid>[a-zA-Z0-9]+)(?P<path>/.*)?$")?;

        // Expects the following format for provided URIs `https://{CID}.ipfs.com/{path}`
        let redir_re = Regex::new(r"https:\/\/(?P<cid>[^\.]+)\.ipfs\.[^\/]+(?P<path>\/.+)?")?;

        let path = Url::parse(&modified_uri)?
            .path_segments()
            .map(|segments| segments.collect::<Vec<_>>().join("/"));

        if let Some(captures) = re
            .captures(&path.unwrap_or_default())
            .or_else(|| redir_re.captures(&modified_uri))
        {
            return Self::format_capture(captures, ipfs_prefix, ipfs_auth_param);
        }
        Err(anyhow::anyhow!("Invalid IPFS URI"))
    }

    /// Formats a capture group into a URI.
    fn format_capture(
        captures: Captures<'_>,
        ipfs_prefix: &str,
        ipfs_auth_param: Option<String>,
    ) -> anyhow::Result<String> {
        let cid = captures["cid"].to_string();
        let path = captures.name("path").map(|m| m.as_str().to_string());

        PARSE_URI_TYPE_COUNT.with_label_values(&["ipfs"]).inc();
        Ok(format!(
            "{}{}{}{}",
            ipfs_prefix,
            cid,
            path.unwrap_or_default(),
            ipfs_auth_param.unwrap_or_default()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IPFS_PREFIX: &str = "https://testipfsprefix.com/ipfs/";
    const IPFS_AUTH: &str = "token";
    const CID: &str = "testcid";
    const PATH: &str = "testpath";

    #[test]
    fn test_parse_ipfs_uri() {
        let test_ipfs_uri = format!("ipfs://{}/{}", CID, PATH);
        let parsed_uri = URIParser::parse(IPFS_PREFIX, &test_ipfs_uri, Some(IPFS_AUTH)).unwrap();
        assert_eq!(
            parsed_uri,
            format!("{IPFS_PREFIX}{CID}/{PATH}?{IPFS_AUTH_KEY}={IPFS_AUTH}")
        );

        // Path is optional for IPFS URIs
        let test_ipfs_uri_no_path = format!("ipfs://{}/{}", CID, "");
        let parsed_uri = URIParser::parse(IPFS_PREFIX, &test_ipfs_uri_no_path, None).unwrap();
        assert_eq!(parsed_uri, format!("{}{}/{}", IPFS_PREFIX, CID, ""));

        // IPFS URIs must contain a CID, expect error here
        let test_ipfs_uri_no_cid = format!("ipfs://{}/{}", "", PATH);
        let parsed_uri = URIParser::parse(IPFS_PREFIX, &test_ipfs_uri_no_cid, None);
        assert!(parsed_uri.is_err());
    }

    #[test]
    fn test_parse_public_gateway_uri() {
        let test_public_gateway_uri = format!("https://ipfs.io/ipfs/{}/{}", CID, PATH);
        let parsed_uri =
            URIParser::parse(IPFS_PREFIX, &test_public_gateway_uri, Some(IPFS_AUTH)).unwrap();
        assert_eq!(
            parsed_uri,
            format!("{IPFS_PREFIX}{CID}/{PATH}?{IPFS_AUTH_KEY}={IPFS_AUTH}")
        );

        // Path is optional for public gateway URIs
        let test_public_gateway_uri_no_path = format!("https://ipfs.io/ipfs/{}/{}", CID, "");
        let parsed_uri =
            URIParser::parse(IPFS_PREFIX, &test_public_gateway_uri_no_path, None).unwrap();
        assert_eq!(parsed_uri, format!("{}{}/{}", IPFS_PREFIX, CID, ""));

        // Some submitted URIs are in the redirected format
        let test_ipfs_redirect = format!("https://{}.ipfs.re.dir.io/{}", CID, PATH);
        let parsed_uri = URIParser::parse(IPFS_PREFIX, &test_ipfs_redirect, None).unwrap();
        assert_eq!(parsed_uri, format!("{IPFS_PREFIX}{CID}/{PATH}"));

        // Public gateway URIs must contain a CID, expect error here
        let test_public_gateway_uri_no_cid = format!("https://ipfs.io/ipfs/{}/{}", "", PATH);
        let parsed_uri = URIParser::parse(IPFS_PREFIX, &test_public_gateway_uri_no_cid, None);
        assert!(parsed_uri.is_err());
    }

    #[test]
    fn test_parse_non_ipfs_uri_fail() {
        // Expects an error if parsing a non-IPFS URI
        let test_non_ipfs_uri = "https://tesetnotipfsuri.com/notipfspath.json".to_string();
        let parsed_uri = URIParser::parse(IPFS_PREFIX, &test_non_ipfs_uri, None);
        assert!(parsed_uri.is_err());
    }
}

// Copyright © Aptos Foundation

use crate::utils::counters::{PARSE_URI_INVOCATION_COUNT, PARSE_URI_TYPE_COUNT};
use regex::{Captures, Regex};
use url::Url;

pub struct URIParser;

impl URIParser {
    /// Attempts to parse IPFS URI to use dedicated gateway.
    /// Returns the original URI if parsing fails.
    pub fn parse(ipfs_prefix: String, uri: String) -> anyhow::Result<String> {
        PARSE_URI_INVOCATION_COUNT.inc();
        if uri.contains("arweave.net") {
            PARSE_URI_TYPE_COUNT.with_label_values(&["arweave"]).inc();
            return Ok(uri);
        }

        let modified_uri = if uri.starts_with("ipfs://") {
            uri.replace("ipfs://", "https://ipfs.com/ipfs/")
        } else {
            uri
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
            return Self::format_capture(captures, ipfs_prefix);
        }
        Err(anyhow::anyhow!("Invalid IPFS URI"))
    }

    /// Formats a capture group into a URI.
    fn format_capture(captures: Captures<'_>, ipfs_prefix: String) -> anyhow::Result<String> {
        let cid = captures["cid"].to_string();
        let path = captures.name("path").map(|m| m.as_str().to_string());

        PARSE_URI_TYPE_COUNT.with_label_values(&["ipfs"]).inc();
        Ok(format!(
            "{}{}{}",
            ipfs_prefix,
            cid,
            path.unwrap_or_default()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IPFS_PREFIX: &str = "https://testipfsprefix.com/ipfs/";
    const CID: &str = "testcid";
    const PATH: &str = "testpath";

    #[test]
    fn test_parse_ipfs_uri() {
        let test_ipfs_uri = format!("ipfs://{}/{}", CID, PATH);
        let parsed_uri = URIParser::parse(IPFS_PREFIX.to_string(), test_ipfs_uri).unwrap();
        assert_eq!(parsed_uri, format!("{IPFS_PREFIX}{CID}/{PATH}"));

        // Path is optional for IPFS URIs
        let test_ipfs_uri_no_path = format!("ipfs://{}/{}", CID, "");
        let parsed_uri = URIParser::parse(IPFS_PREFIX.to_string(), test_ipfs_uri_no_path).unwrap();
        assert_eq!(parsed_uri, format!("{}{}/{}", IPFS_PREFIX, CID, ""));

        // IPFS URIs must contain a CID, expect error here
        let test_ipfs_uri_no_cid = format!("ipfs://{}/{}", "", PATH);
        let parsed_uri = URIParser::parse(IPFS_PREFIX.to_string(), test_ipfs_uri_no_cid);
        assert!(parsed_uri.is_err());
    }

    #[test]
    fn test_parse_public_gateway_uri() {
        let test_public_gateway_uri = format!("https://ipfs.io/ipfs/{}/{}", CID, PATH);
        let parsed_uri =
            URIParser::parse(IPFS_PREFIX.to_string(), test_public_gateway_uri).unwrap();
        assert_eq!(parsed_uri, format!("{IPFS_PREFIX}{CID}/{PATH}",));

        // Path is optional for public gateway URIs
        let test_public_gateway_uri_no_path = format!("https://ipfs.io/ipfs/{}/{}", CID, "");
        let parsed_uri =
            URIParser::parse(IPFS_PREFIX.to_string(), test_public_gateway_uri_no_path).unwrap();
        assert_eq!(parsed_uri, format!("{}{}/{}", IPFS_PREFIX, CID, ""));

        // Some submitted URIs are in the redirected format
        let test_ipfs_redirect = format!("https://{}.ipfs.re.dir.io/{}", CID, PATH);
        let parsed_uri = URIParser::parse(IPFS_PREFIX.to_string(), test_ipfs_redirect).unwrap();
        assert_eq!(parsed_uri, format!("{IPFS_PREFIX}{CID}/{PATH}"));

        // Public gateway URIs must contain a CID, expect error here
        let test_public_gateway_uri_no_cid = format!("https://ipfs.io/ipfs/{}/{}", "", PATH);
        let parsed_uri = URIParser::parse(IPFS_PREFIX.to_string(), test_public_gateway_uri_no_cid);
        assert!(parsed_uri.is_err());
    }

    #[test]
    fn test_parse_non_ipfs_uri_fail() {
        // Expects an error if parsing a non-IPFS URI
        let test_non_ipfs_uri = "https://tesetnotipfsuri.com/notipfspath.json".to_string();
        let parsed_uri = URIParser::parse(IPFS_PREFIX.to_string(), test_non_ipfs_uri);
        assert!(parsed_uri.is_err());
    }
}

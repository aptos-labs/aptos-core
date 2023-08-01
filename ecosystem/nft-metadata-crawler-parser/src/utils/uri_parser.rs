// Copyright © Aptos Foundation

use regex::Regex;
use url::Url;

pub struct URIParser;

impl URIParser {
    /// Attempts to parse IPFS URI to use dedicated gateway.
    /// Returns the original URI if parsing fails.
    pub fn parse(ipfs_prefix: String, uri: String) -> anyhow::Result<String> {
        let modified_uri = if uri.starts_with("ipfs://") {
            uri.replace("ipfs://", "https://ipfs.com/ipfs/")
        } else {
            uri
        };

        // Expects the following format for provided URIs `ipfs/{CID}/{path}`
        let re = Regex::new(r"^(ipfs/)(?P<cid>[a-zA-Z0-9]+)(?P<path>/.*)?$")?;

        let path = Url::parse(&modified_uri)?
            .path_segments()
            .map(|segments| segments.collect::<Vec<_>>().join("/"));

        if let Some(captures) = re.captures(&path.unwrap_or_default()) {
            let cid = captures["cid"].to_string();
            let path = captures.name("path").map(|m| m.as_str().to_string());

            Ok(format!(
                "{}/{}{}",
                ipfs_prefix,
                cid,
                path.unwrap_or_default()
            ))
        } else {
            Err(anyhow::anyhow!("Invalid IPFS URI"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IPFS_PREFIX: &str = "https://testipfsprefix.com/ipfs";
    const CID: &str = "testcid";
    const PATH: &str = "testpath";

    #[test]
    fn test_parse_ipfs_uri() {
        let test_ipfs_uri = format!("ipfs://{}/{}", CID, PATH);
        let parsed_uri = URIParser::parse(IPFS_PREFIX.to_string(), test_ipfs_uri).unwrap();
        assert_eq!(parsed_uri, format!("{IPFS_PREFIX}/{CID}/{PATH}"));

        // Path is optional for IPFS URIs
        let test_ipfs_uri_no_path = format!("ipfs://{}/{}", CID, "");
        let parsed_uri = URIParser::parse(IPFS_PREFIX.to_string(), test_ipfs_uri_no_path).unwrap();
        assert_eq!(parsed_uri, format!("{}/{}/{}", IPFS_PREFIX, CID, ""));

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
        assert_eq!(parsed_uri, format!("{IPFS_PREFIX}/{CID}/{PATH}",));

        // Path is optional for public gateway URIs
        let test_public_gateway_uri_no_path = format!("https://ipfs.io/ipfs/{}/{}", CID, "");
        let parsed_uri =
            URIParser::parse(IPFS_PREFIX.to_string(), test_public_gateway_uri_no_path).unwrap();
        assert_eq!(parsed_uri, format!("{}/{}/{}", IPFS_PREFIX, CID, ""));

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

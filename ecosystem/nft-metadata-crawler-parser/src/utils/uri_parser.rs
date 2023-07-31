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

    #[test]
    fn test_parse_ipfs_uri() {
        let test_ipfs_prefix = "https://testipfsprefix.com/ipfs".to_string();

        let cid = "testcid".to_string();
        let path = "testpath".to_string();

        let test_ipfs_uri = format!("ipfs://{}/{}", cid, path);
        let parsed_uri = URIParser::parse(test_ipfs_prefix.clone(), test_ipfs_uri).unwrap();
        assert_eq!(parsed_uri, format!("{test_ipfs_prefix}/{cid}/{path}"));

        // Path is optional for IPFS URIs
        let test_ipfs_uri_no_path = format!("ipfs://{}/{}", cid, "");
        let parsed_uri = URIParser::parse(test_ipfs_prefix.clone(), test_ipfs_uri_no_path).unwrap();
        assert_eq!(parsed_uri, format!("{}/{}/{}", test_ipfs_prefix, cid, ""));

        // IPFS URIs must contain a CID, expect error here
        let test_ipfs_uri_no_cid = format!("ipfs://{}/{}", "", path);
        let parsed_uri = URIParser::parse(test_ipfs_prefix.clone(), test_ipfs_uri_no_cid);
        assert!(parsed_uri.is_err());
    }

    #[test]
    fn test_parse_public_gateway_uri() {
        let test_ipfs_prefix = "https://testipfsprefix.com/ipfs".to_string();

        let cid = "testcid".to_string();
        let path = "testpath".to_string();

        let test_public_gateway_uri = format!("https://ipfs.io/ipfs/{}/{}", cid, path);
        let parsed_uri =
            URIParser::parse(test_ipfs_prefix.clone(), test_public_gateway_uri).unwrap();
        assert_eq!(parsed_uri, format!("{test_ipfs_prefix}/{cid}/{path}",));

        // Path is optional for public gateway URIs
        let test_public_gateway_uri_no_path = format!("https://ipfs.io/ipfs/{}/{}", cid, "");
        let parsed_uri =
            URIParser::parse(test_ipfs_prefix.clone(), test_public_gateway_uri_no_path).unwrap();
        assert_eq!(parsed_uri, format!("{}/{}/{}", test_ipfs_prefix, cid, ""));

        // Public gateway URIs must contain a CID, expect error here
        let test_public_gateway_uri_no_cid = format!("https://ipfs.io/ipfs/{}/{}", "", path);
        let parsed_uri = URIParser::parse(test_ipfs_prefix.clone(), test_public_gateway_uri_no_cid);
        assert!(parsed_uri.is_err());
    }

    #[test]
    fn test_parse_non_ipfs_uri_fail() {
        let test_ipfs_prefix = "https://testipfsprefix.com/ipfs".to_string();
        let test_non_ipfs_uri = "https://tesetnotipfsuri.com/notipfspath.json".to_string();

        // Expects an error if parsing a non-IPFS URI
        let parsed_uri = URIParser::parse(test_ipfs_prefix.clone(), test_non_ipfs_uri);
        assert!(parsed_uri.is_err());
    }
}

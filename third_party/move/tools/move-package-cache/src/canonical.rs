// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_core_types::account_address::AccountAddress;
use url::Url;

/// Sanitizes a URL by replacing dots with underscores and converting to lowercase.
///
/// TODO: The current scheme is not bijective - if the original URL contains underscores, they will
/// be preserved while dots are converted to underscores, creating ambiguity.
fn sanitized_url(url: &Url) -> String {
    url.host_str()
        .unwrap_or("unknown")
        .replace('.', "_")
        .to_ascii_lowercase()
}

/// Creates a canonical name for a git repository from its URL.
/// This is used to identify and index the repo in the package cache.
///
/// # Example
/// `https://github.com/aptos-labs/aptos-core` -> `github_com+aptos-labs+aptos-core`
pub fn canonical_repo_name_from_url(url: &Url) -> String {
    let mut s = String::new();
    s.push_str(&sanitized_url(url));
    if let Some(segments) = url.path_segments() {
        for segment in segments {
            s.push('+');
            s.push_str(&segment.replace('.', "_").to_ascii_lowercase());
        }
    }
    s
}

/// Creates a canonical name for an on-chain package, used to identify and index the package.
///
/// # Example
/// `fullnode.mainnet.aptoslabs.com, version 12345, 0x1::std`
///   -> `fullnode_mainnet_aptoslabs_com+12345+0x1+std`
pub fn canonical_on_chain_package_name(
    fullnode_url: &Url,
    ledger_version: u64,
    address: AccountAddress,
    package_name: &str,
) -> String {
    format!(
        "{}+{}+{}+{}",
        sanitized_url(fullnode_url),
        ledger_version,
        address,
        package_name
    )
}

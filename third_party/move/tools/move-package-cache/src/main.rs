// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use move_core_types::account_address::AccountAddress;
use move_package_cache::{DebugPackageCacheListener, PackageCache};
use std::str::FromStr;
use url::Url;

// Note: this is just sample workflow demonstrating how the package cache can be used as a library.
// It will likely be removed later as the package cache is intended to be integrated into
// other tools rather than used as a standalone executable.

#[tokio::main]
async fn main() -> Result<()> {
    let cache = PackageCache::new_with_listener("./data", DebugPackageCacheListener).unwrap();

    let aptos_framework_url =
        Url::from_str("https://github.com/aptos-labs/aptos-framework").unwrap();

    let oid = cache
        .resolve_git_revision(&aptos_framework_url, "main")
        .await?;
    cache.checkout_git_repo(&aptos_framework_url, oid).await?;

    cache
        .fetch_on_chain_package(
            Url::from_str("https://fullnode.mainnet.aptoslabs.com").unwrap(),
            3022354983,
            AccountAddress::ONE,
            "MoveStdlib",
        )
        .await?;

    Ok(())
}

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Live, network-dependent end-to-end tests for the on-chain dependency
//! resolver and the root-level `[patch]` override.
//!
//! These tests are gated behind `#[ignore]` because they hit a public Aptos
//! fullnode (default: `https://fullnode.mainnet.aptoslabs.com/v1`). Running
//! them in default CI on every PR would make the test suite flaky against
//! upstream availability and could compound to rate-limit-relevant traffic
//! when many PRs run in parallel from the same CI egress IP.
//!
//! Run manually with:
//!
//! ```text
//! cargo test -p aptos-move-cli --lib --tests online_onchain -- --ignored --nocapture
//! ```
//!
//! The fullnode endpoint and addresses can be overridden via env vars:
//! - `APTOS_ONLINE_TEST_NODE_URL` — fullnode REST URL
//!   (default `https://fullnode.mainnet.aptoslabs.com/v1`)
//! - `APTOS_ONLINE_TEST_LIQUIDSWAP_ADDR` — Liquidswap publisher address
//!   (default `0x190d44266241744264b964a37b8f09863167a12d3e70cda39376cfb4e3561e12`)
//!
//! Override these to point at a private/throttled fullnode or a fork before
//! enabling these tests in scheduled CI.

use crate::tests::common;
use std::{env, fs, path::Path};

const DEFAULT_NODE_URL: &str = "https://fullnode.mainnet.aptoslabs.com/v1";
/// Mainnet `Liquidswap` package, published under
/// <https://github.com/pontem-network/liquidswap>.
const DEFAULT_LIQUIDSWAP_ADDR: &str =
    "0x190d44266241744264b964a37b8f09863167a12d3e70cda39376cfb4e3561e12";

fn node_url() -> String {
    env::var("APTOS_ONLINE_TEST_NODE_URL").unwrap_or_else(|_| DEFAULT_NODE_URL.to_string())
}

fn liquidswap_addr() -> String {
    env::var("APTOS_ONLINE_TEST_LIQUIDSWAP_ADDR")
        .unwrap_or_else(|_| DEFAULT_LIQUIDSWAP_ADDR.to_string())
}

/// Resolve a Move package whose Liquidswap dep is declared on-chain. The
/// real value of this test is that Liquidswap's *transitive* on-chain deps
/// (`LiquidswapLP`, `LiquidswapInit`, `U256`, `UQ64x64`, plus the framework /
/// stdlibs at `0x1`) must also be auto-resolved via the manifest rewrite in
/// `AptosPackageHooks::save_package_to_disk_with_node`.
///
/// We use `--fetch-deps-only` to avoid the full compile (which would pull in
/// Liquidswap source and likely slow the test down a lot), but the
/// `--fetch-deps-only` path still runs the whole resolver, including
/// `download_and_update_if_remote` and named-address unification. So a pass
/// proves the on-chain transitive expansion is wired up end-to-end.
#[test]
#[ignore = "hits public Aptos mainnet fullnode; run with --ignored"]
fn fetch_deps_only_liquidswap_onchain_transitive() {
    let dir = tempfile::tempdir().expect("tempdir");
    let pkg_dir = dir.path();
    write_manifest(
        pkg_dir,
        "OnChainDepsConsumer",
        &format!(
            r#"
[package]
name = "OnChainDepsConsumer"
version = "0.0.0"

[addresses]
consumer = "0xCAFE"

[dependencies]
Liquidswap = {{ aptos = "{node}", address = "{addr}" }}
"#,
            node = node_url(),
            addr = liquidswap_addr(),
        ),
    );
    fs::create_dir_all(pkg_dir.join("sources")).expect("create sources/");
    fs::write(
        pkg_dir.join("sources/empty.move"),
        "module consumer::empty {}\n",
    )
    .expect("write empty.move");

    let pkg_str = pkg_dir.to_str().unwrap();
    let output = common::run_cli(&[
        "compile",
        "--package-dir",
        pkg_str,
        "--skip-fetch-latest-git-deps",
        "--fetch-deps-only",
    ]);
    if let Err(e) = &output.result {
        panic!(
            "fetch-deps-only failed with on-chain Liquidswap dep:\n{}\n--- stderr ---\n{}",
            e, output.stderr,
        );
    }
}

/// Same scenario, but the user starts with a *local* dep declaration that
/// would normally fail to resolve (the path doesn't exist on disk) and uses
/// the new `[patch]` table to rewrite it to the on-chain Liquidswap. This
/// proves the local→on-chain replacement path that motivated this feature.
#[test]
#[ignore = "hits public Aptos mainnet fullnode; run with --ignored"]
fn patch_rewrites_local_dep_to_onchain_liquidswap() {
    let dir = tempfile::tempdir().expect("tempdir");
    let pkg_dir = dir.path();
    write_manifest(
        pkg_dir,
        "PatchedConsumer",
        &format!(
            r#"
[package]
name = "PatchedConsumer"
version = "0.0.0"

[addresses]
consumer = "0xCAFE"

[dependencies]
Liquidswap = {{ local = "./does-not-exist" }}

[patch]
Liquidswap = {{ aptos = "{node}", address = "{addr}" }}
"#,
            node = node_url(),
            addr = liquidswap_addr(),
        ),
    );
    fs::create_dir_all(pkg_dir.join("sources")).expect("create sources/");
    fs::write(
        pkg_dir.join("sources/empty.move"),
        "module consumer::empty {}\n",
    )
    .expect("write empty.move");

    let pkg_str = pkg_dir.to_str().unwrap();
    let output = common::run_cli(&[
        "compile",
        "--package-dir",
        pkg_str,
        "--skip-fetch-latest-git-deps",
        "--fetch-deps-only",
    ]);
    if let Err(e) = &output.result {
        panic!(
            "fetch-deps-only failed when [patch] should have rewritten the dep:\n{}\n--- stderr ---\n{}",
            e, output.stderr,
        );
    }
}

fn write_manifest(pkg_dir: &Path, _pkg_name: &str, manifest: &str) {
    fs::create_dir_all(pkg_dir).expect("create pkg dir");
    fs::write(pkg_dir.join("Move.toml"), manifest.trim_start()).expect("write Move.toml");
}

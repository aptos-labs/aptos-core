// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Runs the `benches-e2e` packages' Move unit tests on V1 and MonoMove.
//!
//! These packages are only ever exercised by the perf harness, which takes
//! hours. Running their unit tests in the normal test job catches a broken
//! package before anyone pays for that.

mod common;

use aptos_framework::extended_checks;
use aptos_types::account_address::AccountAddress;
use move_package::{source_package::std_lib::StdVersion, BuildConfig, CompilerConfig};
use std::{collections::BTreeMap, path::PathBuf};
use tempfile::{tempdir, TempDir};

/// The address `Move.toml` declares for every package in the suite. The value
/// is arbitrary here; on chain the publisher address is substituted in.
const BENCH_ADDRESS: &str = "0xb0";

fn package_path(package: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("benches-e2e")
        .join(package)
}

fn framework_path() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("repository root")
        .join("aptos-move/framework")
        .to_string_lossy()
        .to_string()
}

fn build_config(install_dir: &TempDir) -> BuildConfig {
    let named_addresses = BTreeMap::from([(
        "bench".to_string(),
        AccountAddress::from_hex_literal(BENCH_ADDRESS).unwrap(),
    )]);
    BuildConfig {
        test_mode: true,
        install_dir: Some(install_dir.path().to_path_buf()),
        override_std: Some(StdVersion::Local(framework_path())),
        additional_named_addresses: named_addresses,
        compiler_config: CompilerConfig {
            known_attributes: extended_checks::get_all_attribute_names().clone(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Runs one package on V1 and MonoMove from the same build, so both see
/// identical bytecode.
fn run_package(package: &str) {
    // Held for the whole run: dropping it early leaves the build output behind
    // in a directory nothing ever cleans up.
    let install_dir = tempdir().unwrap();
    common::run_package_on_both(&package_path(package), build_config(&install_dir));
}

#[test]
fn test_clob_avl() {
    run_package("clob_avl");
}

#[test]
fn test_lending_market() {
    run_package("lending_market");
}

#[test]
fn test_clmm_swap() {
    run_package("clmm_swap");
}

#[test]
fn test_stableswap() {
    run_package("stableswap");
}

#[test]
fn test_bridge_relay() {
    run_package("bridge_relay");
}

#[test]
fn test_airdrop_fanout() {
    run_package("airdrop_fanout");
}

#[test]
fn test_oracle_batch() {
    run_package("oracle_batch");
}

#[test]
fn test_cdp_liquidation() {
    run_package("cdp_liquidation");
}

#[test]
fn test_dex_aggregator() {
    run_package("dex_aggregator");
}

#[test]
fn test_nft_mint_market() {
    run_package("nft_mint_market");
}

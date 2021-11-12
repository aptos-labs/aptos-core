// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use move_core_types::account_address::AccountAddress;
use move_package::{
    resolution::resolution_graph as RG, source_package::manifest_parser as MP, BuildConfig,
};
use std::{collections::BTreeMap, path::Path};
use tempfile::tempdir;

#[test]
fn test_additonal_addresses() {
    let path = Path::new(
        "tests/test_sources/resolution/basic_no_deps_address_not_assigned_with_dev_assignment",
    );
    let pm = MP::parse_move_manifest_from_file(path).unwrap();

    let mut additional_named_addresses = BTreeMap::new();
    additional_named_addresses.insert(
        "A".to_string(),
        AccountAddress::from_hex_literal("0x1").unwrap(),
    );

    assert!(RG::ResolutionGraph::new(
        pm.clone(),
        path.parent().unwrap().to_path_buf(),
        BuildConfig {
            install_dir: Some(tempdir().unwrap().path().to_path_buf()),
            additional_named_addresses,
            ..Default::default()
        },
    )
    .unwrap()
    .resolve()
    .is_ok());

    assert!(RG::ResolutionGraph::new(
        pm,
        path.parent().unwrap().to_path_buf(),
        BuildConfig {
            install_dir: Some(tempdir().unwrap().path().to_path_buf()),
            ..Default::default()
        },
    )
    .unwrap()
    .resolve()
    .is_err());
}

#[test]
fn test_additonal_addresses_already_assigned_same_value() {
    let path = Path::new("tests/test_sources/resolution/basic_no_deps_address_assigned");
    let pm = MP::parse_move_manifest_from_file(path).unwrap();

    let mut additional_named_addresses = BTreeMap::new();
    additional_named_addresses.insert(
        "A".to_string(),
        AccountAddress::from_hex_literal("0x0").unwrap(),
    );

    assert!(RG::ResolutionGraph::new(
        pm,
        path.parent().unwrap().to_path_buf(),
        BuildConfig {
            install_dir: Some(tempdir().unwrap().path().to_path_buf()),
            additional_named_addresses,
            ..Default::default()
        },
    )
    .unwrap()
    .resolve()
    .is_ok());
}

#[test]
fn test_additonal_addresses_already_assigned_different_value() {
    let path = Path::new("tests/test_sources/resolution/basic_no_deps_address_assigned");
    let pm = MP::parse_move_manifest_from_file(path).unwrap();

    let mut additional_named_addresses = BTreeMap::new();
    additional_named_addresses.insert(
        "A".to_string(),
        AccountAddress::from_hex_literal("0x1").unwrap(),
    );

    assert!(RG::ResolutionGraph::new(
        pm,
        path.parent().unwrap().to_path_buf(),
        BuildConfig {
            install_dir: Some(tempdir().unwrap().path().to_path_buf()),
            additional_named_addresses,
            ..Default::default()
        },
    )
    .is_err());
}

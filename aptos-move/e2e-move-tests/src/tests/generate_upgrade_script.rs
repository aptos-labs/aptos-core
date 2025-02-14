// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{build_package, tests::common, MoveHarness};
use aptos_framework::{BuildOptions, ReleasePackage};
use aptos_package_builder::PackageBuilder;
use aptos_types::account_address::AccountAddress;
use move_package::compilation::package_layout::CompiledPackageLayout;

#[test]
fn generate_upgrade_script() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Construct two packages: one for which a proposal is created, the other for
    // holding the proposal script.

    let mut upgrade = PackageBuilder::new("Pack");
    upgrade.add_source(
        "test",
        &format!(
            "\
module 0x{}::test {{
    public entry fun hi(_s: &signer){{
    }}
}}",
            acc.address().to_hex()
        ),
    );
    let upgrade_dir = upgrade.write_to_temp().unwrap();

    let mut proposal = PackageBuilder::new("Proposal");
    proposal.add_local_dep(
        "SupraFramework",
        &common::framework_dir_path("supra-framework").to_string_lossy(),
    );
    let proposal_dir = proposal.write_to_temp().unwrap();

    let upgrade_release = ReleasePackage::new(
        build_package(upgrade_dir.path().to_path_buf(), BuildOptions::default()).unwrap(),
    )
    .unwrap();

    // Generate the proposal and compile it.
    upgrade_release
        .generate_script_proposal(
            AccountAddress::ONE,
            proposal_dir
                .path()
                .to_path_buf()
                .join(CompiledPackageLayout::Sources.path())
                .join("proposal.move"),
            "main".to_string(),
        )
        .unwrap();
    let _ = build_package(proposal_dir.path().to_path_buf(), BuildOptions::default()).unwrap();

    // TODO: maybe execute the proposal.
}

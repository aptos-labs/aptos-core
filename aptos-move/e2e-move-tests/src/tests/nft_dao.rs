// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{assert_success, build_package, tests::common, MoveHarness};
use aptos_types::account_address::create_resource_address;
use rstest::rstest;

#[rstest(
    stateless_account1,
    stateless_account2,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, true, false, false),
    case(true, true, true, false),
    case(true, true, true, true),
    case(true, false, false, false),
    case(true, false, true, false),
    case(true, false, true, true),
    case(false, true, false, false),
    case(false, true, true, false),
    case(false, true, true, true),
    case(false, false, false, false),
    case(false, false, true, false),
    case(false, false, true, true)
)]
// Test the txn argument works as expected
fn test_nft_dao_txn_arguments(
    stateless_account1: bool,
    stateless_account2: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);

    let acc = h.new_account_with_key_pair(if stateless_account1 { None } else { Some(0) });
    let mut build_options = aptos_framework::BuildOptions::default();
    build_options
        .named_addresses
        .insert("dao_platform".to_string(), *acc.address());

    // build the package from our example code
    let package = build_package(
        common::test_dir_path("../../../move-examples/dao/nft_dao"),
        build_options,
    )
    .expect("building package must succeed");

    let code = package.extract_code();
    let metadata = package
        .extract_metadata()
        .expect("extracting package metadata must succeed");

    let result = h.run_transaction_payload(
        &acc,
        aptos_cached_packages::aptos_stdlib::code_publish_package_txn(
            bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
            code,
        ),
    );
    assert_success!(result);
    // setup NFT creation and distribution for testing
    let collection_name = "col".to_owned().into_bytes();
    let token_names: Vec<Vec<u8>> = vec!["tok1", "tok2", "tok3", "tok4"]
        .into_iter()
        .map(|e| e.to_owned().into_bytes())
        .collect();
    let desc = "desc".to_owned().into_bytes();
    let voter = h.new_account_with_key_pair(if stateless_account2 { None } else { Some(0) });
    assert_success!(h.run_transaction_payload(
        &acc,
        aptos_cached_packages::aptos_token_sdk_builder::token_create_collection_script(
            collection_name.clone(),
            desc,
            "".to_owned().into_bytes(),
            5,
            vec![false, false, false],
        ),
    ));
    for tok in &token_names {
        assert_success!(h.run_transaction_payload(
            &acc,
            aptos_cached_packages::aptos_token_sdk_builder::token_create_token_script(
                collection_name.clone(),
                tok.clone(),
                "".to_owned().into_bytes(),
                1,
                1,
                "".to_owned().into_bytes(),
                *acc.address(),
                1,
                0,
                vec![false, false, false, false, false],
                vec![],
                vec![],
                vec![],
            ),
        ));
    }

    // create a DAO
    assert_success!(h.run_entry_function(
        &acc,
        str::parse(&format!("{}::nft_dao::create_dao", acc.address())).unwrap(),
        vec![],
        vec![
            bcs::to_bytes("dao").unwrap(),
            bcs::to_bytes(&1u64).unwrap(),
            bcs::to_bytes(&8000u64).unwrap(),
            bcs::to_bytes(acc.address()).unwrap(),
            bcs::to_bytes(&collection_name).unwrap(),
            bcs::to_bytes(&1u64).unwrap(),
        ],
    ));
    // get DAO address
    let mut salt = bcs::to_bytes("dao").unwrap();
    salt.append(&mut bcs::to_bytes(acc.address()).unwrap());
    salt.append(&mut bcs::to_bytes(&collection_name).unwrap());
    let dao = create_resource_address(*acc.address(), salt.as_slice());

    // transfer two NFTs to DAO and transfer 1 NFT to voter
    assert_success!(h.run_transaction_payload(
        &acc,
        aptos_cached_packages::aptos_token_sdk_builder::token_transfer_with_opt_in(
            *acc.address(),
            collection_name.clone(),
            token_names[0].clone(),
            0,
            dao,
            1,
        ),
    ));
    assert_success!(h.run_transaction_payload(
        &acc,
        aptos_cached_packages::aptos_token_sdk_builder::token_transfer_with_opt_in(
            *acc.address(),
            collection_name.clone(),
            token_names[1].clone(),
            0,
            dao,
            1,
        ),
    ));
    // voter opt-in direct transfer
    assert_success!(h.run_transaction_payload(
        &voter,
        aptos_cached_packages::aptos_token_sdk_builder::token_opt_in_direct_transfer(true),
    ));
    assert_success!(h.run_transaction_payload(
        &acc,
        aptos_cached_packages::aptos_token_sdk_builder::token_transfer_with_opt_in(
            *acc.address(),
            collection_name.clone(),
            token_names[2].clone(),
            0,
            *voter.address(),
            1,
        ),
    ));
    let fnames = vec!["offer_nft".as_bytes(), "offer_nft".as_bytes()];
    let arg_name = vec![
        "creator".as_bytes(),
        "collection".as_bytes(),
        "token_name".as_bytes(),
        "property_version".as_bytes(),
        "dst".as_bytes(),
    ];
    let arg_names = vec![arg_name.clone(), arg_name];
    let arg_type = vec![
        "address".as_bytes(),
        "0x1::string::String".as_bytes(),
        "0x1::string::String".as_bytes(),
        "u64".as_bytes(),
        "address".as_bytes(),
    ];
    let arg_types = vec![arg_type.clone(), arg_type];
    let arg_values = vec![
        vec![
            bcs::to_bytes(acc.address()).unwrap(),
            bcs::to_bytes(&collection_name).unwrap(),
            bcs::to_bytes(&token_names[0]).unwrap(),
            bcs::to_bytes(&0u64).unwrap(),
            bcs::to_bytes(voter.address()).unwrap(),
        ],
        vec![
            bcs::to_bytes(acc.address()).unwrap(),
            bcs::to_bytes(&collection_name).unwrap(),
            bcs::to_bytes(&token_names[1]).unwrap(),
            bcs::to_bytes(&0u64).unwrap(),
            bcs::to_bytes(voter.address()).unwrap(),
        ],
    ];

    // propose to transfer two NFTs to voter
    assert_success!(h.run_entry_function(
        &acc,
        str::parse(&format!("{}::nft_dao::create_proposal", acc.address())).unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&dao).unwrap(),
            bcs::to_bytes("proposal").unwrap(),
            bcs::to_bytes("desc").unwrap(),
            bcs::to_bytes(&fnames).unwrap(),
            bcs::to_bytes(&arg_names).unwrap(),
            bcs::to_bytes(&arg_values).unwrap(),
            bcs::to_bytes(&arg_types).unwrap(),
            bcs::to_bytes(&1u64).unwrap(),
            bcs::to_bytes(&vec![token_names[3].clone()]).unwrap(),
            bcs::to_bytes(&vec![0u64]).unwrap(),
        ],
    ));
    // vote
    h.new_epoch();
    assert_success!(h.run_entry_function(
        &voter,
        str::parse(&format!("{}::nft_dao::vote", acc.address())).unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&dao).unwrap(),
            bcs::to_bytes(&1u64).unwrap(),
            bcs::to_bytes(&true).unwrap(),
            bcs::to_bytes(&vec![token_names[2].clone()]).unwrap(),
            bcs::to_bytes(&vec![0u64]).unwrap(),
        ],
    ));
    // resolve
    h.new_epoch();
    let res = h.run_entry_function(
        &voter,
        str::parse(&format!("{}::nft_dao::resolve", acc.address())).unwrap(),
        vec![],
        vec![bcs::to_bytes(&1u64).unwrap(), bcs::to_bytes(&dao).unwrap()],
    );
    assert_success!(res);
}

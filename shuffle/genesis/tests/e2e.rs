// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use diem_config::config::NodeConfig;
use diem_crypto::PrivateKey;
use diem_sdk::{
    client::BlockingClient,
    transaction_builder::{
        stdlib::encode_create_parent_vasp_account_script_function, Currency, TransactionFactory,
    },
    types::LocalAccount,
};
use diem_types::{
    account_config, chain_id::ChainId, on_chain_config::VMPublishingOption,
    transaction::authenticator::AuthenticationKey,
};
use std::path::Path;

#[test]
fn node_e2e() -> Result<()> {
    let node_config_dir = diem_temppath::TempPath::new();
    let validator_config = shuffle_custom_node::generate_validator_config(
        node_config_dir.path(),
        Path::new("../move/storage/0x00000000000000000000000000000001/modules"),
        VMPublishingOption::open(),
        diem_framework_releases::current_module_blobs().to_vec(),
    )?;
    let node_config = NodeConfig::load(validator_config.config_path())?;
    let root_key = generate_key::load_key(node_config_dir.path().join("mint.key"));

    let json_rpc_url = format!("http://0.0.0.0:{}", node_config.json_rpc.address.port());
    // Start the node in a separate thread using our config
    let _diem_node_thread = std::thread::spawn(move || {
        diem_node::start(&node_config, None);
    });
    // TODO: figure out a better solution for this.
    // Wait for node to start.
    std::thread::sleep(std::time::Duration::from_secs(1));

    let new_account_key = generate_key::generate_key();

    // Connect to the node we have started in the other thread
    let client = BlockingClient::new(json_rpc_url);
    let tc_seq = client
        .get_account(account_config::treasury_compliance_account_address())?
        .into_inner()
        .unwrap()
        .sequence_number;
    let mut tc_account = LocalAccount::new(
        account_config::treasury_compliance_account_address(),
        root_key,
        tc_seq,
    );
    let new_account = LocalAccount::new(
        AuthenticationKey::ed25519(&new_account_key.public_key()).derived_address(),
        new_account_key,
        0,
    );

    // Send a txn to create a new account
    let create_new_account_txn =
        tc_account.sign_with_transaction_builder(TransactionFactory::new(ChainId::test()).payload(
            encode_create_parent_vasp_account_script_function(
                Currency::XUS.type_tag(),
                0,
                new_account.address(),
                new_account.authentication_key().prefix().to_vec(),
                vec![],
                false,
            ),
        ));

    send(&client, create_new_account_txn)
}

fn send(client: &BlockingClient, tx: diem_types::transaction::SignedTransaction) -> Result<()> {
    use diem_json_rpc_types::views::VMStatusView;

    client.submit(&tx)?;
    assert_eq!(
        client
            .wait_for_signed_transaction(&tx, Some(std::time::Duration::from_secs(60)), None)?
            .into_inner()
            .vm_status,
        VMStatusView::Executed,
    );
    Ok(())
}

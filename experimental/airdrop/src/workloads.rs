// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, bail, Result};
use aptos_logger::info;
use aptos_sdk::{
    move_types::{account_address::AccountAddress, ident_str, language_storage::ModuleId},
    rest_client::{aptos_api_types::TransactionOnChainData, Client},
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{
        chain_id::ChainId,
        contract_event::ContractEvent,
        serde_helper::bcs_utils::bcs_size_of_byte_array,
        transaction::{EntryFunction, SignedTransaction},
        LocalAccount,
    },
};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::{fs::read_to_string, path::Path, str::FromStr};

pub trait SignedTransactionBuilder<T> {
    fn build(
        &self,
        data: &T,
        account: &LocalAccount,
        txn_factory: &TransactionFactory,
    ) -> SignedTransaction;

    fn success_output(&self, data: &T, txn_out: &Option<TransactionOnChainData>) -> String;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DepositMoveStruct {
    account: AccountAddress,
    amount: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggregatorSnapshotu64MoveStruct {
    value: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MintMoveStruct {
    collection: AccountAddress,
    index: AggregatorSnapshotu64MoveStruct,
    token: AccountAddress,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BurnMoveStruct {
    collection: AccountAddress,
    index: u64,
    token: AccountAddress,
    previous_owner: AccountAddress,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateCollectionConfigMoveStruct {
    collection_config: AccountAddress,
    collection: AccountAddress,
    ready_to_mint: bool,
}

pub fn create_account_addresses_work(destinations_file: &str, only_success: bool) -> Result<Vec<AccountAddress>> {
    Ok(read_to_string(Path::new(destinations_file))?
        .lines()
        .filter(|s| !only_success || s.ends_with("\tsuccess"))
        .filter_map(|s| s.split('\t').next())
        .filter(|s| !s.is_empty())
        .map(|text| AccountAddress::from_str(text).map_err(|e| anyhow!("failed to parse {}, {:?}", text, e)))
        .collect::<Result<Vec<_>, _>>()?)
}

fn parse_line_vec(line: &str) -> Result<(AccountAddress, AccountAddress)> {
    let mut parts = line.split('\t');
    let first = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("No first part"))?;
    let second = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("No second part"))?;
    Ok((
        AccountAddress::from_str_strict(first)?,
        AccountAddress::from_str_strict(second)?,
    ))
}

pub async fn create_account_address_pairs_work(
    destinations_file: &str, only_success: bool
) -> Result<Vec<(AccountAddress, AccountAddress)>> {
    read_to_string(Path::new(destinations_file))?
        .lines()
        .filter(|s| !only_success || s.ends_with("\tsuccess"))
        .map(parse_line_vec)
        .collect::<Result<Vec<_>, _>>()
}

fn rand_string(len: usize) -> String {
    let res = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect();
    assert_eq!(
        bcs::serialized_size(&res).unwrap(),
        bcs_size_of_byte_array(len)
    );
    res
}

const AIRDROP_DEVNET_ADDRESS: &str =
    "0x1bdddd6b9e15bf2ecda5183177baf7044b852a9f07f9e42bc37a5dcb3cf1c30f";
const AIRDROP_TESTNET_ADDRESS: &str =
    "0x0a0266208da8f0ed87ba7383272f79f3ed98b39777dc70c87ad0438135af6639";
const AIRDROP_MAINNET_ADDRESS: &str =
    "0xe3185a0112cbac069cc58c64afff312a63f8bd40a32b67a410a2aab6784e8371";


fn only_on_aptos_contract_address(chain_id: ChainId) -> AccountAddress {
    if chain_id.is_mainnet() {
        AccountAddress::from_str_strict(AIRDROP_MAINNET_ADDRESS).unwrap()
    } else if chain_id.is_testnet() {
        AccountAddress::from_str_strict(AIRDROP_TESTNET_ADDRESS).unwrap()
    } else {
        AccountAddress::from_str_strict(AIRDROP_DEVNET_ADDRESS).unwrap()
    }
}

fn only_on_aptos_module_id(chain_id: ChainId) -> ModuleId {
    ModuleId::new(
        only_on_aptos_contract_address(chain_id),
        ident_str!("only_on_aptos").to_owned(),
    )
}

pub struct NftMintSignedTransactionBuilder {
    airdrop_contract: ModuleId,
    collection_owner_address: AccountAddress,
}

impl NftMintSignedTransactionBuilder {
    pub async fn new(
        admin_account: LocalAccount,
        client: &Client,
        txn_factory: TransactionFactory,
    ) -> Result<Self> {
        let airdrop_contract = only_on_aptos_module_id(txn_factory.get_chain_id());

        let collection_name = format!("Test Collection {}", rand_string(10));

        let create_collection_txn = admin_account.sign_with_transaction_builder(
            txn_factory.entry_function(EntryFunction::new(
                airdrop_contract.clone(),
                ident_str!("create_collection").to_owned(),
                vec![],
                vec![
                    bcs::to_bytes(&collection_name).unwrap(), // collection_name
                    bcs::to_bytes(&"collection description").unwrap(),              // collection_description
                    bcs::to_bytes(&"htpps://some.collection.uri.test").unwrap(),              // collection_uri
                    bcs::to_bytes(&"test token #").unwrap(),  // token_name_prefix
                    bcs::to_bytes(&"test token description").unwrap(),              // token_description
                    bcs::to_bytes(&vec!["htpps://some.uri1.test", "htpps://some.uri2.test"]).unwrap(), // token_uris: vector<String>,
                    bcs::to_bytes(&vec![10u64, 1u64]).unwrap(), // token_uris_weights: vector<u64>,
                    bcs::to_bytes(&true).unwrap(),           // mutable_collection_metadata
                    bcs::to_bytes(&true).unwrap(),           // mutable_token_metadata
                    bcs::to_bytes(&true).unwrap(),            // tokens_burnable_by_collection_owner
                    bcs::to_bytes(&false).unwrap(), // tokens_transferrable_by_collection_owner
                    bcs::to_bytes(&Some(1000000u64)).unwrap(), // max_supply
                    bcs::to_bytes(&Option::<u64>::None).unwrap(), // royalty_numerator
                    bcs::to_bytes(&Option::<u64>::None).unwrap(), // royalty_denominator
                ],
            )),
        );

        let output = client
            .submit_and_wait_bcs(&create_collection_txn)
            .await?
            .into_inner();
        assert!(output.info.status().is_success(), "{:?}", output);
        info!("create_collection txn: {:?}", output.info);
        let create_collection_event: CreateCollectionConfigMoveStruct = search_single_event_data(
            &output.events,
            &format!("{}::CreateCollectionConfig", airdrop_contract),
        )?;

        let collection_owner_address = create_collection_event.collection_config;

        let start_minting_txn = admin_account.sign_with_transaction_builder(
            txn_factory.entry_function(EntryFunction::new(
                airdrop_contract.clone(),
                ident_str!("set_minting_status").to_owned(),
                vec![],
                vec![
                    bcs::to_bytes(&collection_owner_address).unwrap(),
                    bcs::to_bytes(&true).unwrap(),
                ],
            )),
        );
        let output = client
            .submit_and_wait_bcs(&start_minting_txn)
            .await?
            .into_inner();
        assert!(output.info.status().is_success(), "{:?}", output);
        info!("set_minting_status txn: {:?}", output.info);

        info!("collection_owner_address: {:?}", collection_owner_address);
        Ok(Self {
            airdrop_contract,
            collection_owner_address,
        })
    }
}

impl SignedTransactionBuilder<AccountAddress> for NftMintSignedTransactionBuilder {
    fn build(
        &self,
        data: &AccountAddress,
        account: &LocalAccount,
        txn_factory: &TransactionFactory,
    ) -> SignedTransaction {
        account.sign_with_transaction_builder(
            txn_factory.entry_function(EntryFunction::new(
                self.airdrop_contract.clone(),
                ident_str!("mint_to_recipient").to_owned(),
                vec![],
                vec![
                    bcs::to_bytes(&self.collection_owner_address).unwrap(),
                    bcs::to_bytes(data).unwrap(),
                ],
            )),
        )
    }

    fn success_output(&self, data: &AccountAddress, txn_out: &Option<TransactionOnChainData>) -> String {
        let (status, token) = match txn_out {
            Some(txn_out) => match get_mint_token_addr(&txn_out.events) {
                Ok(dst) => ("success".to_string(), dst.to_standard_string()),
                Err(e) => (e.to_string(), "".to_string()),
            },
            None => ("missing".to_string(), "".to_string()),
        };
        format!(
            "{}\t{}\t{}\t{}",
            token,
            self.collection_owner_address.to_standard_string(),
            data,
            status
        )
    }
}

pub struct NftBurnSignedTransactionBuilder {
    admin_account: LocalAccount,
    airdrop_contract: ModuleId,
}

impl NftBurnSignedTransactionBuilder {
    pub fn new(admin_account: LocalAccount, chain_id: ChainId) -> Result<Self> {
        let airdrop_contract = only_on_aptos_module_id(chain_id);
        Ok(Self {
            admin_account,
            airdrop_contract,
        })
    }
}

impl SignedTransactionBuilder<(AccountAddress, AccountAddress)>
    for NftBurnSignedTransactionBuilder
{
    fn build(
        &self,
        data: &(AccountAddress, AccountAddress),
        account: &LocalAccount,
        txn_factory: &TransactionFactory,
    ) -> SignedTransaction {
        account.sign_multi_agent_with_transaction_builder(
            vec![&self.admin_account],
            txn_factory.entry_function(EntryFunction::new(
                self.airdrop_contract.clone(),
                ident_str!("burn_with_admin_worker").to_owned(),
                vec![],
                vec![
                    bcs::to_bytes(&data.1).unwrap(),
                    bcs::to_bytes(&data.0).unwrap(),
                ],
            )),
        )
    }

    fn success_output(
        &self,
        data: &(AccountAddress, AccountAddress),
        txn_out: &Option<TransactionOnChainData>,
    ) -> String {
        let (status, refund_addr) = match txn_out {
            Some(txn_out) => match get_burn_token_addr(&txn_out.events) {
                Ok(_dst) => (
                    "success".to_string(),
                    txn_out
                        .transaction
                        .try_as_signed_user_txn()
                        .unwrap()
                        .sender()
                        .to_standard_string(),
                ),
                Err(e) => (e.to_string(), "".to_string()),
            },
            None => ("missing".to_string(), "".to_string()),
        };
        format!(
            "{}\t{}\t{}\t{}",
            refund_addr,
            data.0.to_standard_string(),
            data.1.to_standard_string(),
            status
        )
    }
}

pub fn get_mint_token_addr(events: &[ContractEvent]) -> Result<AccountAddress> {
    let mint_event: MintMoveStruct = search_single_event_data(
        events,
        "0000000000000000000000000000000000000000000000000000000000000004::collection::Mint",
    )?;
    Ok(mint_event.token)
}

pub fn get_burn_token_addr(events: &[ContractEvent]) -> Result<AccountAddress> {
    let burn_event: BurnMoveStruct = search_single_event_data(
        events,
        "0000000000000000000000000000000000000000000000000000000000000004::collection::Burn",
    )?;
    Ok(burn_event.token)
}

pub fn search_event(events: &[ContractEvent], type_tag: &str) -> Vec<ContractEvent> {
    events
        .iter()
        .filter(|event| event.type_tag().to_canonical_string() == type_tag)
        .cloned()
        .collect::<Vec<_>>()
}

pub fn search_single_event_data<T>(events: &[ContractEvent], type_tag: &str) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let matching_events = search_event(events, type_tag);
    if matching_events.len() != 1 {
        bail!(
            "Expected 1 event, found: {}, events: {:?}",
            matching_events.len(),
            events
                .iter()
                .map(|event| event.type_tag().to_canonical_string())
                .collect::<Vec<_>>()
        );
    }
    let event = matching_events
        .first()
        .ok_or_else(|| anyhow::anyhow!("No deposit event found"))?;
    Ok(bcs::from_bytes::<T>(event.event_data())?)
}

pub fn get_deposit_dst(events: &[ContractEvent]) -> Result<AccountAddress> {
    let deposit_event: DepositMoveStruct = search_single_event_data(events, "0000000000000000000000000000000000000000000000000000000000000001::coin::Deposit<0000000000000000000000000000000000000000000000000000000000000001::aptos_coin::AptosCoin>")?;
    Ok(deposit_event.account)
}

pub struct TransferAptSignedTransactionBuilder;

impl SignedTransactionBuilder<AccountAddress> for TransferAptSignedTransactionBuilder {
    fn build(
        &self,
        data: &AccountAddress,
        account: &LocalAccount,
        txn_factory: &TransactionFactory,
    ) -> SignedTransaction {
        account.sign_with_transaction_builder(
            txn_factory.payload(aptos_stdlib::aptos_coin_transfer(*data, 1)),
        )
    }

    fn success_output(&self, data: &AccountAddress, txn_out: &Option<TransactionOnChainData>) -> String {
        let (status, dst) = match txn_out {
            Some(txn_out) => match get_deposit_dst(&txn_out.events) {
                Ok(dst) => {
                    assert_eq!(&dst, data);
                    ("success".to_string(), dst.to_standard_string())
                },
                Err(e) => (e.to_string(), data.to_standard_string()),
            },
            None => ("missing".to_string(), data.to_standard_string()),
        };
        format!("{}\t{}", dst, status)
    }
}

pub struct CreateAndTransferAptSignedTransactionBuilder;

impl SignedTransactionBuilder<AccountAddress> for CreateAndTransferAptSignedTransactionBuilder {
    fn build(
        &self,
        data: &AccountAddress,
        account: &LocalAccount,
        txn_factory: &TransactionFactory,
    ) -> SignedTransaction {
        account.sign_with_transaction_builder(
            txn_factory.payload(aptos_stdlib::aptos_account_transfer(*data, 1)),
        )
    }

    fn success_output(&self, data: &AccountAddress, txn_out: &Option<TransactionOnChainData>) -> String {
        let (status, dst) = match txn_out {
            Some(txn_out) => match get_deposit_dst(&txn_out.events) {
                Ok(dst) => {
                    assert_eq!(&dst, data);
                    ("success", dst.to_standard_string())
                },
                Err(_e) => ("error", data.to_standard_string()),
            },
            None => ("missing", data.to_standard_string()),
        };
        format!("{}\t{}", dst, status)
    }
}

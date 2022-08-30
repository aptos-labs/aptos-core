// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use anyhow::Result;
use aptos_node_checker_lib::EvaluationSummary;
use aptos_sdk::rest_client::Client as AptosClient;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::types::account_config::CORE_CODE_ADDRESS;
use aptos_sdk::types::network_address::NetworkAddress;
use aptos_sdk::types::on_chain_config::ValidatorSet;
use aptos_sdk::types::validator_info::ValidatorInfo;
use clap::Parser;
use log::{debug, info};
use reqwest::Client as ReqwestClient;
use reqwest::Url;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use tokio::sync::Semaphore;

use crate::helpers::extract_network_address;

// Unfortunately we don't have any way, on chain or not, to know what the API
// port is, so we just assume it is one of these. If their VFN is inaccessible at,
// any of these ports, e.g. because it runs on 7777 and they have an LB not registered
// on chain in front of it that listens at 80 just for the API, we're just out of luck.
// If we get this kind of information from elsewhere at some point, we could look at
// joining it here.
pub const API_PORTS: &[u16] = &[80, 8080, 443];

#[derive(Debug, Parser)]
pub struct CheckArgs {
    /// Address of any node (of any type) connected to the network you want
    /// to evaluate.
    #[clap(long)]
    pub node_address: Url,

    /// Address where NHC is running.
    #[clap(long)]
    pub nhc_address: Url,

    /// Baseline config to use when talking to NHC.
    #[clap(long)]
    pub nhc_baseline_config_name: String,

    /// How long to wait when talking to NHC. The check should be quick unless
    /// we're using the TPS evaluator.
    #[clap(long, default_value_t = 60)]
    pub nhc_timeout_secs: u64,

    /// Max number of requests to NHC we can have running concurrently.
    #[clap(long, default_value_t = 4)]
    pub max_concurrent_nhc_requests: u16,

    /// Only run against these account addresses.
    #[clap(long)]
    pub account_address_allowlist: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SingleCheck {
    pub result: SingleCheckResult,
    pub timestamp: Duration,
    pub vfn_address: Option<NetworkAddress>,
}

impl SingleCheck {
    pub fn new(result: SingleCheckResult, vfn_address: Option<NetworkAddress>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get current time");
        Self {
            result,
            timestamp: now,
            vfn_address,
        }
    }
}

/// We use this struct to capture the result of checking a node, or the lack thereof.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SingleCheckResult {
    /// The node was successfully checked. Note: The evaulation itself could
    /// indicate, a problem with the node, this just states that we were able
    /// to check the node sucessfully with NHC.
    Success(EvaluationSummary),

    /// Something went wrong with checking the node.
    Failure(SingleCheckFailure),

    /// The account does not have a VFN registered on chain.
    NoVfnRegistered(NoVfnRegistered),
}

#[derive(Debug, Serialize)]
pub struct SingleCheckFailure {
    pub message: String,
    pub code: SingleCheckFailureCode,
}

impl SingleCheckFailure {
    pub fn new(message: String, code: SingleCheckFailureCode) -> Self {
        Self { message, code }
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub enum SingleCheckFailureCode {
    // The network address in the validator set config cannot be used for
    // querying NHC.
    UnsupportedNetworkAddressType,

    // Something went wrong when sending / receiving the request.
    RequestFlowError,

    // The response from NHC was not a 200, implying a problem with NHC.
    ResponseNot200,

    // The response from NHC couldn't be deserialized.
    CouldNotDeserializeResponse,

    // NHC returned an evaluation that indicates that the API port is closed.
    ApiPortClosed,
}

// This is necessary because we can't just use a unit type for this enum variant.
#[derive(Debug, Serialize)]
pub struct NoVfnRegistered;

/// Get all the on chain validator info.
pub async fn get_validator_info(node_address: Url) -> Result<Vec<ValidatorInfo>> {
    let client = AptosClient::new(node_address.clone());
    let response = client
        .get_account_resource_bcs::<ValidatorSet>(CORE_CODE_ADDRESS, "0x1::stake::ValidatorSet")
        .await?;
    let active_validators = response.into_inner().active_validators;
    info!(
        "Pulled {} active validators. First: {}. Last: {}",
        active_validators.len(),
        active_validators.first().unwrap().account_address(),
        active_validators.last().unwrap().account_address()
    );
    Ok(active_validators)
}

/// Check all VFNs from the validator set.
pub async fn check_vfns(
    nhc_client: &ReqwestClient,
    check_args: &CheckArgs,
    validator_infos: Vec<ValidatorInfo>,
) -> Result<HashMap<AccountAddress, Vec<SingleCheck>>> {
    let mut nhc_address = check_args.nhc_address.clone();
    nhc_address.set_path("/check_node");

    let semaphore = Arc::new(Semaphore::new(
        check_args.max_concurrent_nhc_requests as usize,
    ));

    let mut nhc_responses = HashMap::new();
    let mut futures = vec![];
    for validator_info in validator_infos {
        let account_address = validator_info.account_address();
        if !check_args.account_address_allowlist.is_empty()
            && !check_args
                .account_address_allowlist
                .contains(&account_address.to_string())
        {
            continue;
        }
        let vfn_addresses = validator_info
            .config()
            .fullnode_network_addresses()
            .context("Failed to deserialize VFN network addresses")?;
        if vfn_addresses.is_empty() {
            nhc_responses
                .entry(*account_address)
                .or_insert_with(Vec::new)
                .push(SingleCheck::new(
                    SingleCheckResult::NoVfnRegistered(NoVfnRegistered),
                    None,
                ));
            continue;
        }
        for vfn_address in vfn_addresses.into_iter() {
            let single_check_future = check_single_vfn_wrapper(
                *account_address,
                nhc_client,
                &nhc_address,
                &check_args.nhc_baseline_config_name,
                vfn_address,
                semaphore.clone(),
            );
            futures.push(single_check_future);
        }
    }
    let checks = futures::future::join_all(futures).await;
    for (account_address, vfn_address, single_check_result) in checks {
        match single_check_result {
            SingleCheckResult::Success(_) => {
                info!("NHC returned a 200 for {}", vfn_address);
            }
            SingleCheckResult::Failure(_) => {
                info!("NHC returned a non 200 for {}", vfn_address);
            }
            SingleCheckResult::NoVfnRegistered(_) => panic!("Shouldn't be possible"),
        }
        nhc_responses
            .entry(account_address)
            .or_insert_with(Vec::new)
            .push(SingleCheck::new(single_check_result, Some(vfn_address)));
    }
    Ok(nhc_responses)
}

// This just exists to make joining futures easy.
async fn check_single_vfn_wrapper(
    account_address: AccountAddress,
    nhc_client: &ReqwestClient,
    nhc_address: &Url,
    nhc_baseline_config_name: &str,
    vfn_address: NetworkAddress,
    semaphore: Arc<Semaphore>,
) -> (AccountAddress, NetworkAddress, SingleCheckResult) {
    let _permit = semaphore.acquire().await.unwrap();
    info!("Checking VFNs for account {}", account_address,);
    (
        account_address,
        vfn_address.clone(),
        check_single_vfn(
            nhc_client,
            nhc_address,
            nhc_baseline_config_name,
            vfn_address,
        )
        .await,
    )
}

/// Make a query to NHC for a single VFN. This may result in multiple queries to
/// NHC as we try different API ports.
async fn check_single_vfn(
    nhc_client: &ReqwestClient,
    nhc_address: &Url,
    nhc_baseline_config_name: &str,
    vfn_address: NetworkAddress,
) -> SingleCheckResult {
    // Get a string representation of the vfn address if possible.
    let vfn_url_string = match extract_network_address(&vfn_address) {
        Ok(vfn_url_string) => vfn_url_string,
        Err(e) => {
            return SingleCheckResult::Failure(SingleCheckFailure::new(
                format!("Network address was an unsupported type: {}", e),
                SingleCheckFailureCode::UnsupportedNetworkAddressType,
            ));
        }
    };

    info!("Checking VFN {}", vfn_url_string);

    let mut index = 0;
    loop {
        let single_check_result = check_single_vfn_one_api_port(
            nhc_client,
            nhc_address,
            nhc_baseline_config_name,
            &vfn_url_string,
            API_PORTS[index],
        )
        .await;
        // If the response was a success, return.
        if let SingleCheckResult::Success(_) = &single_check_result {
            break single_check_result;
        }
        // If the response was a failure, return unless it was because it
        // seems like the API wasn't open, in which case we keep looping
        // to try another port.
        if let SingleCheckResult::Failure(failure) = &single_check_result {
            if failure.code != SingleCheckFailureCode::ApiPortClosed {
                break single_check_result;
            }
        }
        // We've tried every port, return the last result.
        if index == API_PORTS.len() - 1 {
            break single_check_result;
        }
        index += 1;
    }
}

/// Check NHC just once, with a single VFN for a single API port.
async fn check_single_vfn_one_api_port(
    nhc_client: &ReqwestClient,
    nhc_address: &Url,
    nhc_baseline_config_name: &str,
    vfn_url_string: &str,
    api_port: u16,
) -> SingleCheckResult {
    // Build up query params.
    let mut params = HashMap::new();
    let api_port_string = api_port.to_string();
    params.insert("node_url", vfn_url_string);
    params.insert("api_port", &api_port_string);
    params.insert("baseline_configuration_name", nhc_baseline_config_name);

    debug!(
        "Querying NHC at address: {}:{}",
        vfn_url_string, api_port_string
    );

    // Send the request and parse the response.
    let response = match nhc_client
        .get(nhc_address.clone())
        .query(&params)
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            return SingleCheckResult::Failure(SingleCheckFailure::new(
                format!("Error with request flow to NHC: {:#}", e),
                SingleCheckFailureCode::RequestFlowError,
            ));
        }
    };

    // Handle the case where NHC itself throws an error (as opposed to a success
    // response from NHC indicating evaluation where the node performed poorly).
    if let Err(e) = response.error_for_status_ref() {
        return SingleCheckResult::Failure(SingleCheckFailure::new(
            format!("{:#}: {:?}", e, response.text().await),
            SingleCheckFailureCode::ResponseNot200,
        ));
    };

    // Confirm the response is valid JSON.
    let evaluation_summary = match response.json::<EvaluationSummary>().await {
        Ok(evaluation_summary) => evaluation_summary,
        Err(e) => {
            return SingleCheckResult::Failure(SingleCheckFailure::new(
                format!("{:#}", e),
                SingleCheckFailureCode::CouldNotDeserializeResponse,
            ))
        }
    };

    // Check specifically if the API port is closed.
    if evaluation_summary
        .evaluation_results
        .iter()
        .any(|evaluation_result| {
            evaluation_result.evaluator_name == "index_response" && evaluation_result.score == 0
        })
    {
        return SingleCheckResult::Failure(SingleCheckFailure::new(
            format!(
                "Couldn't talk to API on any of the expected ports {:?}: {:?}",
                API_PORTS, evaluation_summary
            ),
            SingleCheckFailureCode::ApiPortClosed,
        ));
    }

    SingleCheckResult::Success(evaluation_summary)
}

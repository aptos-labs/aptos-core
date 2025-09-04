// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This file contains logic for checking a node, common to both VFNs and PFNs.
//! At this point, some earlier code has processed the input information, e.g.
//! VFN information on chain or PFN information from a file, and has converted
//! it into a common format that these functions can ingest.

use velor_logger::{debug, info};
use velor_node_checker_lib::CheckSummary;
use velor_sdk::{
    crypto::{x25519, ValidCryptoMaterialStringExt},
    types::account_address::AccountAddress,
};
use clap::Parser;
use futures::{stream::FuturesUnordered, StreamExt};
use reqwest::{Client as ReqwestClient, Url};
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::Semaphore;

// Unfortunately we don't have any way, on chain or not, to know what the API
// port is, so we just assume it is one of these. If their node is inaccessible at
// any of these ports, e.g. because it runs on 7777 and they have an LB not registered
// on chain in front of it that listens at 80 just for the API, we're just out of luck.
// If we get this kind of information from elsewhere at some point, we could look at
// joining it here.
pub const API_PORTS: &[u16] = &[80, 8080, 443];

#[derive(Debug, Parser)]
pub struct NodeHealthCheckerArgs {
    /// Address where we can expect to find an NHC instance.
    #[clap(long)]
    pub nhc_address: Url,

    /// Baseline config to use when talking to NHC.
    #[clap(long)]
    pub nhc_baseline_config_name: String,

    /// How long to wait when talking to NHC. The check should be quick unless
    /// we're using the TPS checker.
    #[clap(long, default_value_t = 60)]
    pub nhc_timeout_secs: u64,

    /// Max number of requests to NHC we can have running concurrently.
    #[clap(long, default_value_t = 32)]
    pub max_concurrent_nhc_requests: u16,
}

impl NodeHealthCheckerArgs {
    /// Per account address, check all nodes. This may result in multiple calls
    /// to NHC even for a single node in the case that we don't know the API
    /// port and we're just guessing.
    pub async fn check_nodes(
        &self,
        address_to_nodes: HashMap<AccountAddress, Vec<NodeInfo>>,
    ) -> HashMap<AccountAddress, Vec<SingleCheck>> {
        let nhc_client = ReqwestClient::builder()
            .timeout(Duration::from_secs(self.nhc_timeout_secs))
            .build()
            .expect("Somehow failed to build reqwest client");

        let mut nhc_address = self.nhc_address.clone();
        nhc_address.set_path("/check_node");

        let semaphore = Arc::new(Semaphore::new(self.max_concurrent_nhc_requests as usize));

        // Build up futures for checking each node.
        let mut nhc_responses = HashMap::new();
        let mut futures = FuturesUnordered::new();
        for (account_address, node_infos) in address_to_nodes {
            for node_info in node_infos {
                let single_check_future = self.check_single_fn_wrapper(
                    account_address,
                    &nhc_client,
                    &nhc_address,
                    node_info,
                    semaphore.clone(),
                );
                futures.push(single_check_future);
            }
        }

        // Go through all the futures, log some information about their results,
        // and insert them into the output.
        while let Some((account_address, node_url, single_check_result)) = futures.next().await {
            match single_check_result {
                SingleCheckResult::Success(_) => {
                    info!("NHC returned a 200 for {}", node_url);
                },
                SingleCheckResult::NodeCheckFailure(_) => {
                    info!("NHC did not return a 200 for {}", node_url);
                },
                wildcard => {
                    panic!(
                        "Shouldn't be possible for checK_single_fn_wrapper to return {:?}",
                        wildcard
                    )
                },
            }
            nhc_responses
                .entry(account_address)
                .or_insert_with(Vec::new)
                .push(SingleCheck::new(single_check_result, Some(node_url)));
        }
        nhc_responses
    }

    /// This just exists to make joining futures easy. This function returns the
    /// account address and node address alongside the check so we can put it in
    /// the output easily.
    async fn check_single_fn_wrapper(
        &self,
        account_address: AccountAddress,
        nhc_client: &ReqwestClient,
        nhc_address: &Url,
        node_info: NodeInfo,
        semaphore: Arc<Semaphore>,
    ) -> (AccountAddress, Url, SingleCheckResult) {
        let _permit = semaphore.acquire().await.unwrap();
        (
            account_address,
            node_info.node_url.clone(),
            self.check_single_fn(nhc_client, nhc_address, node_info)
                .await,
        )
    }

    /// Make a query to NHC for a single node. This may result in multiple
    /// queries to NHC as we try different API ports.
    async fn check_single_fn(
        &self,
        nhc_client: &ReqwestClient,
        nhc_address: &Url,
        node_info: NodeInfo,
    ) -> SingleCheckResult {
        match node_info.api_port {
            Some(api_port) => {
                self.check_single_fn_one_api_port(
                    nhc_client,
                    nhc_address,
                    &node_info.node_url,
                    api_port,
                    node_info.noise_port,
                    node_info.public_key,
                )
                .await
            },
            None => {
                let mut index = 0;
                loop {
                    let single_check_result = self
                        .check_single_fn_one_api_port(
                            nhc_client,
                            nhc_address,
                            &node_info.node_url,
                            API_PORTS[index],
                            node_info.noise_port,
                            node_info.public_key,
                        )
                        .await;
                    // If the response was a success, return.
                    if let SingleCheckResult::Success(_) = &single_check_result {
                        break single_check_result;
                    }
                    // If the response was a failure, return unless it was because it
                    // seems like the API wasn't open, in which case we keep looping
                    // to try another port.
                    if let SingleCheckResult::NodeCheckFailure(failure) = &single_check_result {
                        if failure.code != NodeCheckFailureCode::ApiPortClosed {
                            break single_check_result;
                        }
                    }
                    // We've tried every port, return the last result.
                    if index == API_PORTS.len() - 1 {
                        break single_check_result;
                    }
                    index += 1;
                }
            },
        }
    }

    /// Check NHC just once, with a single node for a single API port.
    async fn check_single_fn_one_api_port(
        &self,
        nhc_client: &ReqwestClient,
        nhc_address: &Url,
        node_url: &Url,
        api_port: u16,
        noise_port: u16,
        public_key: Option<x25519::PublicKey>,
    ) -> SingleCheckResult {
        // Build up query params.
        let mut params = HashMap::new();
        params.insert("node_url", node_url.to_string());
        params.insert("api_port", api_port.to_string());
        params.insert("noise_port", noise_port.to_string());
        params.insert(
            "baseline_configuration_name",
            self.nhc_baseline_config_name.clone(),
        );
        if let Some(public_key) = public_key {
            params.insert("public_key", public_key.to_encoded_string().unwrap());
        }

        // This is just for pretty logging / output purposes.
        let address_single_string = {
            let mut node_url = node_url.clone();
            let _ = node_url.set_port(Some(api_port));
            node_url.to_string()
        };
        debug!("Querying NHC at address: {}", address_single_string);

        // Send the request and parse the response.
        let response = match nhc_client
            .get(nhc_address.clone())
            .query(&params)
            .send()
            .await
        {
            Ok(response) => response,
            Err(e) => {
                return SingleCheckResult::NodeCheckFailure(NodeCheckFailure::new(
                    format!("Error with request flow to NHC: {:#}", e),
                    NodeCheckFailureCode::RequestResponseError,
                ));
            },
        };

        // Handle the case where NHC itself throws an error (as opposed to a success
        // response from NHC indicating evaluation where the node performed poorly).
        if let Err(e) = response.error_for_status_ref() {
            return SingleCheckResult::NodeCheckFailure(NodeCheckFailure::new(
                format!("{:#}: {:?}", e, response.text().await),
                NodeCheckFailureCode::ResponseNot200,
            ));
        };

        // Confirm the response is valid JSON.
        let check_summary = match response.json::<CheckSummary>().await {
            Ok(check_summary) => check_summary,
            Err(e) => {
                return SingleCheckResult::NodeCheckFailure(NodeCheckFailure::new(
                    format!("{:#}", e),
                    NodeCheckFailureCode::CouldNotDeserializeResponse,
                ))
            },
        };

        // Check specifically if the API port is closed.
        if check_summary.check_results.iter().any(|check_result| {
            check_result.checker_name == "NodeIdentityChecker" && check_result.score == 0
        }) {
            return SingleCheckResult::NodeCheckFailure(NodeCheckFailure::new(
                format!(
                    "Couldn't talk to API on any of the expected ports {:?}: {:?}",
                    API_PORTS, check_summary
                ),
                NodeCheckFailureCode::ApiPortClosed,
            ));
        }

        SingleCheckResult::Success(SingleCheckSuccess::new(
            check_summary,
            address_single_string,
        ))
    }
}

/// Helper struct for putting all the FN address information in one place.
#[derive(Debug)]
pub struct NodeInfo {
    /// This should include the scheme, e.g. http://
    pub node_url: Url,

    /// If given, we will use this. If not, we'll try each of API_PORTS.
    pub api_port: Option<u16>,

    /// This will be included in the request to NHC.
    pub noise_port: u16,

    /// If this is included, we'll include this in the NHC request.
    pub public_key: Option<x25519::PublicKey>,
}

#[derive(Debug, Serialize)]
pub struct SingleCheck {
    pub result: SingleCheckResult,
    pub timestamp: Duration,
    pub node_url: Option<Url>,
}

impl SingleCheck {
    pub fn new(result: SingleCheckResult, node_url: Option<Url>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get current time");
        Self {
            result,
            timestamp: now,
            node_url,
        }
    }
}

/// We use this struct to capture the result of checking a node, or the lack thereof.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SingleCheckResult {
    /// The node was successfully checked. Note: The evaulation itself could
    /// still indicate a problem with the node, this just states that we were
    /// able to check the node sucessfully with NHC.
    Success(SingleCheckSuccess),

    /// Something went wrong with checking the node with NHC.
    NodeCheckFailure(NodeCheckFailure),

    /// The account does not have a VFN registered on chain.
    NoVfnRegistered(NoVfnRegistered),

    /// The network address could not be deserialized.
    CouldNotDeserializeNetworkAddress(CouldNotDeserializeNetworkAddress),

    /// The network address was incomplete, e.g. missing an IP, port, public
    /// key, etc.
    IncompleteNetworkAddress(IncompleteNetworkAddress),
}

#[derive(Debug, Serialize)]
pub struct SingleCheckSuccess {
    /// The evaluation summary returned by NHC. This doesn't necessarily imply
    /// that the node passed the evaluation, just that an evaluation was returned
    /// successfully and it passed the API available check.
    pub check_summary: CheckSummary,

    /// This is the address that we used to get this successful evaluation.
    /// This is presented in a normal URL format, not the NetworkAddress
    /// representation. Example value for this field: http://65.109.17.29:8080.
    /// Note, sometimes the address we started with was a DNS name, and we resolved
    /// it to an IP address. As such, this IP address may become incorrect down
    /// the line. In that case, refer to fn_address in SingleCheck, or just
    /// run this tool again.
    pub fn_address_url: String,
}

impl SingleCheckSuccess {
    pub fn new(check_summary: CheckSummary, fn_address_url: String) -> Self {
        Self {
            check_summary,
            fn_address_url,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NodeCheckFailure {
    pub message: String,
    pub code: NodeCheckFailureCode,
}

impl NodeCheckFailure {
    pub fn new(message: String, code: NodeCheckFailureCode) -> Self {
        Self { message, code }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub enum NodeCheckFailureCode {
    // Something went wrong when sending / receiving the request.
    RequestResponseError,

    // The response from NHC was not a 200, implying a problem with NHC.
    ResponseNot200,

    // The response from NHC couldn't be deserialized.
    CouldNotDeserializeResponse,

    // NHC returned an evaluation that indicates that the API port is closed.
    ApiPortClosed,
}

// These are necessary because we can't just use a unit type for this enum variant
// because we serialize SingleCheckResult using internal tagging, in which case
// serde requires that all variants have values.

#[derive(Debug, Serialize)]
pub struct NoVfnRegistered;

#[derive(Debug, Serialize)]
pub struct CouldNotDeserializeNetworkAddress {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct IncompleteNetworkAddress {
    pub message: String,
}

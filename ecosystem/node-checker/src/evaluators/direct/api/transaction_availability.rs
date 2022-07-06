// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::{super::DirectEvaluatorInput, ApiEvaluatorError, API_CATEGORY};
use crate::{
    configuration::EvaluatorArgs,
    evaluator::{EvaluationResult, Evaluator},
    evaluators::EvaluatorType,
};
use anyhow::{anyhow, Result};
use aptos_rest_client::{aptos_api_types::TransactionInfo, Client as AptosRestClient, Transaction};
use aptos_sdk::crypto::HashValue;
use clap::Parser;
use poem_openapi::Object as PoemObject;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const TRANSACTIONS_ENDPOINT: &str = "/transactions";

#[derive(Clone, Debug, Deserialize, Parser, PoemObject, Serialize)]
pub struct TransactionAvailabilityEvaluatorArgs {
    #[clap(long, default_value_t = 5)]
    pub transaction_fetch_delay_secs: u64,
}

#[derive(Debug)]
pub struct TransactionAvailabilityEvaluator {
    args: TransactionAvailabilityEvaluatorArgs,
}

impl TransactionAvailabilityEvaluator {
    pub fn new(args: TransactionAvailabilityEvaluatorArgs) -> Self {
        Self { args }
    }

    /// Get the transaction info of the latest transaction.
    async fn get_transaction_info(
        client: &AptosRestClient,
    ) -> Result<TransactionInfo, ApiEvaluatorError> {
        Self::unwrap_transaction_info(
            client
                .get_transactions(None, Some(1))
                .await
                .map_err(|e| {
                    ApiEvaluatorError::EndpointError(
                        TRANSACTIONS_ENDPOINT.to_string(),
                        e.context("The node API failed to return a transaction".to_string()),
                    )
                })?
                .into_inner()
                .into_iter()
                .next()
                .ok_or_else(|| {
                    ApiEvaluatorError::EndpointError(
                        TRANSACTIONS_ENDPOINT.to_string(),
                        anyhow!("The node API returned success but with no transactions"),
                    )
                })?,
        )
    }

    /// Fetch a transaction by hash and return it.
    async fn get_transaction_from_hash(
        client: &AptosRestClient,
        hash: HashValue,
    ) -> Result<Transaction, ApiEvaluatorError> {
        Ok(client
            .get_transaction(hash)
            .await
            .map_err(|e| {
                ApiEvaluatorError::EndpointError(
                    TRANSACTIONS_ENDPOINT.to_string(),
                    e.context(format!(
                        "The node API failed to return the requested transaction: {}",
                        hash
                    )),
                )
            })?
            .into_inner())
    }

    /// Helper to get transaction info from a transaction.
    fn unwrap_transaction_info(
        transaction: Transaction,
    ) -> Result<TransactionInfo, ApiEvaluatorError> {
        transaction
            .transaction_info()
            .map_err(|e| {
                ApiEvaluatorError::EndpointError(
                    "/transactions".to_string(),
                    e.context("The node API returned a transaction with no info".to_string()),
                )
            })
            .map(|info| info.clone())
    }
}

#[async_trait::async_trait]
impl Evaluator for TransactionAvailabilityEvaluator {
    type Input = DirectEvaluatorInput;
    type Error = ApiEvaluatorError;

    /// Assert that the target node can produce the same transaction that the
    /// baseline produced after a delay. We confirm that the transactions are
    /// same by looking at the version.
    async fn evaluate(&self, input: &Self::Input) -> Result<Vec<EvaluationResult>, Self::Error> {
        let baseline_client =
            AptosRestClient::new(input.baseline_node_information.node_address.get_api_url());

        let latest_baseline_transaction_info = Self::get_transaction_info(&baseline_client).await?;

        tokio::time::sleep(Duration::from_secs(self.args.transaction_fetch_delay_secs)).await;

        let target_client = AptosRestClient::new(input.target_node_address.get_api_url());
        let evaluation = match Self::get_transaction_from_hash(
            &target_client,
            HashValue::from(latest_baseline_transaction_info.hash),
        )
        .await
        {
            Ok(latest_target_transaction) => {
                match Self::unwrap_transaction_info(latest_target_transaction) {
                    Ok(latest_target_transaction_info) => {
                        if latest_baseline_transaction_info.accumulator_root_hash
                            == latest_target_transaction_info.accumulator_root_hash
                        {
                            self.build_evaluation_result(
                                "Target node produced valid recent transaction".to_string(),
                                100,
                                format!(
                                    "We got the latest transaction from the baseline node ({}), waited {} \
                                        seconds, and then asked your node to give us that transaction, and \
                                        it did. Great! This implies that your node is keeping up with other \
                                        nodes in the network.",
                                    latest_baseline_transaction_info.hash, self.args.transaction_fetch_delay_secs,
                                ),
                            )
                        } else {
                            self.build_evaluation_result(
                                "Target node produced recent transaction, but it was invalid".to_string(),
                                0,
                                format!(
                                    "We got the latest transaction from the baseline node ({}), waited {} \
                                        seconds, and then asked your node to give us that transaction, and \
                                        it did. However, the transaction was invalid compared to the baseline \
                                        as the accumulator root hash of the transaction ({}) was different \
                                        compared to the baseline ({}).",
                                    latest_baseline_transaction_info.hash,
                                    self.args.transaction_fetch_delay_secs,
                                    latest_target_transaction_info.accumulator_root_hash,
                                    latest_baseline_transaction_info.accumulator_root_hash,
                                ),
                            )
                        }
                    }
                    Err(e) => self.build_evaluation_result(
                        "Target node produced recent transaction, but it was missing metadata"
                            .to_string(),
                        10,
                        format!(
                            "We got the latest transaction from the baseline node ({}), waited {} \
                                seconds, and then asked your node to give us that transaction, and \
                                it did. However, the transaction was missing metadata such as the
                                version, accumulator root hash, etc. Error: {}",
                            latest_baseline_transaction_info.hash,
                            self.args.transaction_fetch_delay_secs,
                            e,
                        ),
                    ),
                }
            }
            Err(e) => self.build_evaluation_result(
                "Target node failed to produce recent transaction".to_string(),
                50,
                format!(
                    "We got the latest transaction from the baseline node ({}), waited {} \
                        seconds, and then asked your node to give us that transaction, and \
                        it could not. This implies that your node is lagging behind the \
                        baseline by at least {} seconds, or some other issue with \
                        the API of your node, such as an issue with the transaction \
                        pruner. Error from retrieving the transaction: {}",
                    latest_baseline_transaction_info.hash,
                    self.args.transaction_fetch_delay_secs,
                    self.args.transaction_fetch_delay_secs,
                    e,
                ),
            ),
        };

        Ok(vec![evaluation])
    }

    fn get_category_name() -> String {
        API_CATEGORY.to_string()
    }

    fn get_evaluator_name() -> String {
        "transaction_availability".to_string()
    }

    fn from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<Self> {
        Ok(Self::new(
            evaluator_args.transaction_availability_args.clone(),
        ))
    }

    fn evaluator_type_from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<EvaluatorType> {
        Ok(EvaluatorType::Api(Box::new(Self::from_evaluator_args(
            evaluator_args,
        )?)))
    }
}

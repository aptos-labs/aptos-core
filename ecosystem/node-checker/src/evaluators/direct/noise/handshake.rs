// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    configuration::{EvaluatorArgs, NodeAddress},
    evaluator::{EvaluationResult, Evaluator},
    evaluators::EvaluatorType,
};
use anyhow::{format_err, Result};
use aptos_network_checker::{
    args::{CheckEndpointArgs, HandshakeArgs, NodeAddressArgs},
    check_endpoint::check_endpoint,
};
use clap::Parser;
use poem_openapi::Object as PoemObject;
use serde::{Deserialize, Serialize};

use super::super::DirectEvaluatorInput;
use super::{NoiseEvaluatorError, NOISE_CATEGORY};

#[derive(Clone, Debug, Default, Deserialize, Parser, PoemObject, Serialize)]
pub struct HandshakeEvaluatorArgs {
    #[clap(flatten)]
    #[oai(skip)]
    pub handshake_args: HandshakeArgs,
}

#[derive(Debug)]
pub struct HandshakeEvaluator {
    args: HandshakeEvaluatorArgs,
}

impl HandshakeEvaluator {
    pub fn new(args: HandshakeEvaluatorArgs) -> Self {
        Self { args }
    }
}

#[async_trait::async_trait]
impl Evaluator for HandshakeEvaluator {
    type Input = DirectEvaluatorInput;
    type Error = NoiseEvaluatorError;

    /// Assert that we can establish a noise connection with the target node
    /// with the given public key. If we cannot, it implies that either the
    /// node is not listening on that port, or the node is not running with
    /// the private key matching the public key provided as part of the request
    /// to NHC.
    async fn evaluate(&self, input: &Self::Input) -> Result<Vec<EvaluationResult>, Self::Error> {
        // Confirm that we can build the target node into a NetworkAddress.
        // TODO: Given that at this point we've already confirmed that the target
        // node is there, how else could this possibly fail? I figure it can't,
        // because all that could fail is DNS lookup, which should've already worked.
        let target_network_address = match input.target_node_address.as_noise_network_address() {
            Ok(network_address) => network_address,
            Err(e) => {
                return Ok(vec![self.build_evaluation_result(
                    "Invalid node address".to_string(),
                    0,
                    format!(
                        "Failed to resolve given address as noise NetworkAddress, \
                        ensure your address is valid: {:#}",
                        e
                    ),
                )]);
            }
        };
        Ok(vec![match check_endpoint(
            &CheckEndpointArgs {
                node_address_args: NodeAddressArgs {
                    address: target_network_address,
                    chain_id: input.get_target_chain_id(),
                },
                handshake_args: self.args.handshake_args.clone(),
            },
            None,
        )
        .await
        {
            Ok(message) => self.build_evaluation_result(
                "Noise connection established successfully".to_string(),
                100,
                format!(
                    "{}. This indicates your noise port ({}) is open and the node is \
                    running with the private key matching the provided public key.",
                    message,
                    input.target_node_address.get_noise_port()
                ),
            ),
            Err(e) => self.build_evaluation_result(
                "Failed to establish noise connection".to_string(),
                0,
                format!(
                    "{:#}. Either the noise port ({}) is closed or the node is not \
                    running with the private key matching the provided public key.",
                    e,
                    input.target_node_address.get_noise_port()
                ),
            ),
        }])
    }

    fn get_category_name() -> String {
        NOISE_CATEGORY.to_string()
    }

    fn get_evaluator_name() -> String {
        "handshake".to_string()
    }

    fn from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<Self> {
        Ok(Self::new(evaluator_args.handshake_args.clone()))
    }

    fn evaluator_type_from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<EvaluatorType> {
        Ok(EvaluatorType::Noise(Box::new(Self::from_evaluator_args(
            evaluator_args,
        )?)))
    }

    fn validate_check_node_call(&self, target_node_address: &NodeAddress) -> anyhow::Result<()> {
        if target_node_address.get_public_key().is_none() {
            return Err(format_err!(
                "A public key must be provided to use the handshake evaluator"
            ));
        }
        Ok(())
    }
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{components::get_signer_arg, utils::generate_governance_proposal};
use aptos_crypto::HashValue;
use aptos_types::on_chain_config::OnChainRandomnessConfig;
use move_model::{code_writer::CodeWriter, emitln, model::Loc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum ReleaseFriendlyRandomnessConfig {
    Off,
    V1 {
        secrecy_threshold_in_percentage: u64,
        reconstruct_threshold_in_percentage: u64,
    },
    V2 {
        secrecy_threshold_in_percentage: u64,
        reconstruct_threshold_in_percentage: u64,
        fast_path_secrecy_threshold_in_percentage: u64,
    },
}

impl From<ReleaseFriendlyRandomnessConfig> for OnChainRandomnessConfig {
    fn from(value: ReleaseFriendlyRandomnessConfig) -> Self {
        match value {
            ReleaseFriendlyRandomnessConfig::Off => OnChainRandomnessConfig::Off,
            ReleaseFriendlyRandomnessConfig::V1 {
                secrecy_threshold_in_percentage,
                reconstruct_threshold_in_percentage,
            } => OnChainRandomnessConfig::new_v1(
                secrecy_threshold_in_percentage,
                reconstruct_threshold_in_percentage,
            ),
            ReleaseFriendlyRandomnessConfig::V2 {
                secrecy_threshold_in_percentage,
                reconstruct_threshold_in_percentage,
                fast_path_secrecy_threshold_in_percentage,
            } => OnChainRandomnessConfig::new_v2(
                secrecy_threshold_in_percentage,
                reconstruct_threshold_in_percentage,
                fast_path_secrecy_threshold_in_percentage,
            ),
        }
    }
}

pub fn generate_randomness_config_update_proposal(
    config: &ReleaseFriendlyRandomnessConfig,
    is_testnet: bool,
    next_execution_hash: Option<HashValue>,
    is_multi_step: bool,
) -> anyhow::Result<Vec<(String, String)>> {
    let signer_arg = get_signer_arg(is_testnet, &next_execution_hash);
    let mut result = vec![];

    let writer = CodeWriter::new(Loc::default());

    let proposal = generate_governance_proposal(
        &writer,
        is_testnet,
        next_execution_hash,
        is_multi_step,
        &[
            "aptos_framework::randomness_config",
            "aptos_std::fixed_point64",
        ],
        |writer| {
            match config {
                ReleaseFriendlyRandomnessConfig::Off => {
                    emitln!(
                        writer,
                        "randomness_config::set_for_next_epoch({}, randomness_config::new_off());",
                        signer_arg
                    );
                },
                ReleaseFriendlyRandomnessConfig::V1 {
                    secrecy_threshold_in_percentage,
                    reconstruct_threshold_in_percentage,
                } => {
                    emitln!(writer, "let v1 = randomness_config::new_v1(");
                    emitln!(
                        writer,
                        "    fixed_point64::create_from_rational({}, 100),",
                        secrecy_threshold_in_percentage
                    );
                    emitln!(
                        writer,
                        "    fixed_point64::create_from_rational({}, 100),",
                        reconstruct_threshold_in_percentage
                    );
                    emitln!(writer, ");");
                    emitln!(
                        writer,
                        "randomness_config::set_for_next_epoch({}, v1);",
                        signer_arg
                    );
                },
                ReleaseFriendlyRandomnessConfig::V2 {
                    secrecy_threshold_in_percentage,
                    reconstruct_threshold_in_percentage,
                    fast_path_secrecy_threshold_in_percentage,
                } => {
                    emitln!(writer, "let v2 = randomness_config::new_v2(");
                    emitln!(
                        writer,
                        "    fixed_point64::create_from_rational({}, 100),",
                        secrecy_threshold_in_percentage
                    );
                    emitln!(
                        writer,
                        "    fixed_point64::create_from_rational({}, 100),",
                        reconstruct_threshold_in_percentage
                    );
                    emitln!(
                        writer,
                        "    fixed_point64::create_from_rational({}, 100),",
                        fast_path_secrecy_threshold_in_percentage
                    );
                    emitln!(writer, ");");
                    emitln!(
                        writer,
                        "randomness_config::set_for_next_epoch({}, v2);",
                        signer_arg
                    );
                },
            }
            emitln!(writer, "aptos_governance::reconfigure({});", signer_arg);
        },
    );

    result.push(("randomness-config".to_string(), proposal));
    Ok(result)
}

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{components::get_signer_arg, utils::generate_governance_proposal};
use aptos_crypto::HashValue;
use aptos_types::on_chain_config::OnChainJWKConsensusConfig;
use move_model::{code_writer::CodeWriter, emitln, model::Loc};

pub fn generate_jwk_consensus_config_update_proposal(
    config: &OnChainJWKConsensusConfig,
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
        &["aptos_framework::jwk_consensus_config", "std::string::utf8"],
        |writer| {
            match config {
                OnChainJWKConsensusConfig::Off => {
                    emitln!(writer, "jwk_consensus_config::set_for_next_epoch({}, jwk_consensus_config::new_off());", signer_arg);
                },
                OnChainJWKConsensusConfig::V1(v1) => {
                    emitln!(writer, "let config = jwk_consensus_config::new_v1(vector[");
                    for p in v1.oidc_providers.iter() {
                        emitln!(writer, "jwk_consensus_config::new_oidc_provider(utf8(b\"{}\"), utf8(b\"{}\")),", p.name, p.config_url);
                    }
                    emitln!(writer, "]);");
                    emitln!(
                        writer,
                        "jwk_consensus_config::set_for_next_epoch({}, config);",
                        signer_arg
                    );
                },
            }
            emitln!(writer, "aptos_governance::reconfigure({});", signer_arg);
        },
    );

    result.push(("jwk-consensus-config".to_string(), proposal));
    Ok(result)
}

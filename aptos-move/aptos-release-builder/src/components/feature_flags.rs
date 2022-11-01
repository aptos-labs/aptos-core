// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::utils::*;
use anyhow::Result;
use aptos_types::on_chain_config::FeatureFlag;
use move_model::{code_writer::CodeWriter, emit, emitln, model::Loc};

pub struct Features {
    pub enabled: Vec<FeatureFlag>,
    pub disabled: Vec<FeatureFlag>,
}

pub fn generate_feature_upgrade_proposal(
    features: &Features,
    is_testnet: bool,
) -> Result<Vec<(String, String)>> {
    let mut result = vec![];

    let enabled = bcs::to_bytes(
        &features
            .enabled
            .iter()
            .map(|f| *f as u64)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let disabled = bcs::to_bytes(
        &features
            .disabled
            .iter()
            .map(|f| *f as u64)
            .collect::<Vec<_>>(),
    )
    .unwrap();

    assert!(enabled.len() < 65536);
    assert!(disabled.len() < 65536);

    let writer = CodeWriter::new(Loc::default());

    if is_testnet {
        generate_testnet_header(&writer, "features");
    } else {
        generate_governance_proposal_header(&writer, "features");
    }

    emit!(writer, "let enabled_blob: vector<u8> = ");
    generate_blob(&writer, &enabled);
    emitln!(writer, ";\n");

    emit!(writer, "let disabled_blob: vector<u8> = ");
    generate_blob(&writer, &disabled);
    emitln!(writer, ";\n");

    emitln!(
        writer,
        "gas_schedule::set_gas_schedule(framework_signer, gas_schedule_blob);"
    );

    result.push(("features".to_string(), finish_with_footer(&writer)));
    Ok(result)
}

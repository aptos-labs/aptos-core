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

fn generate_features_blob(writer: &CodeWriter, data: &[u64]) {
    emitln!(writer, "vector[");
    writer.indent();
    for (i, b) in data.iter().enumerate() {
        if i % 20 == 0 {
            if i > 0 {
                emitln!(writer);
            }
        } else {
            emit!(writer, " ");
        }
        emit!(writer, "{},", b);
    }
    emitln!(writer);
    writer.unindent();
    emit!(writer, "]")
}

pub fn generate_feature_upgrade_proposal(
    features: &Features,
    is_testnet: bool,
) -> Result<Vec<(String, String)>> {
    let mut result = vec![];

    let enabled = features
        .enabled
        .iter()
        .map(|f| *f as u64)
        .collect::<Vec<_>>();
    let disabled = features
        .disabled
        .iter()
        .map(|f| *f as u64)
        .collect::<Vec<_>>();

    assert!(enabled.len() < u16::MAX as usize);
    assert!(disabled.len() < u16::MAX as usize);

    let writer = CodeWriter::new(Loc::default());

    if is_testnet {
        generate_testnet_header(&writer, "std::features");
    } else {
        generate_governance_proposal_header(&writer, "std::features");
    }

    emit!(writer, "let enabled_blob: vector<u64> = ");
    generate_features_blob(&writer, &enabled);
    emitln!(writer, ";\n");

    emit!(writer, "let disabled_blob: vector<u64> = ");
    generate_features_blob(&writer, &disabled);
    emitln!(writer, ";\n");

    emitln!(
        writer,
        "features::change_feature_flags(framework_signer, enabled_blob, disabled_blob);"
    );

    result.push(("features".to_string(), finish_with_footer(&writer)));
    Ok(result)
}

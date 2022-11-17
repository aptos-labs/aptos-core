// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::utils::*;
use anyhow::Result;
use aptos_types::on_chain_config::FeatureFlag as AFeatureFlag;
use move_model::{code_writer::CodeWriter, emit, emitln, model::Loc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct Features {
    pub enabled: Vec<FeatureFlag>,
    pub disabled: Vec<FeatureFlag>,
}

#[derive(Clone, Deserialize, PartialEq, Eq, Serialize)]
#[allow(non_camel_case_types)]
#[serde(rename_all = "snake_case")]
pub enum FeatureFlag {
    CodeDependencyCheck,
    TreatFriendAsPrivate,
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
        .map(|f| AFeatureFlag::from(f.clone()) as u64)
        .collect::<Vec<_>>();
    let disabled = features
        .disabled
        .iter()
        .map(|f| AFeatureFlag::from(f.clone()) as u64)
        .collect::<Vec<_>>();

    assert!(enabled.len() < u16::MAX as usize);
    assert!(disabled.len() < u16::MAX as usize);

    let writer = CodeWriter::new(Loc::default());

    let proposal = generate_governance_proposal(&writer, is_testnet, "std::features", |writer| {
        emit!(writer, "let enabled_blob: vector<u64> = ");
        generate_features_blob(writer, &enabled);
        emitln!(writer, ";\n");

        emit!(writer, "let disabled_blob: vector<u64> = ");
        generate_features_blob(writer, &disabled);
        emitln!(writer, ";\n");

        emitln!(
            writer,
            "features::change_feature_flags(framework_signer, enabled_blob, disabled_blob);"
        );
    });

    result.push(("features".to_string(), proposal));
    Ok(result)
}

impl From<FeatureFlag> for AFeatureFlag {
    fn from(f: FeatureFlag) -> Self {
        match f {
            FeatureFlag::CodeDependencyCheck => AFeatureFlag::CODE_DEPENDENCY_CHECK,
            FeatureFlag::TreatFriendAsPrivate => AFeatureFlag::TREAT_FRIEND_AS_PRIVATE,
        }
    }
}

// We don't need this implementation. Just to make sure we have an exhaustive 1-1 mapping between the two structs.
impl From<AFeatureFlag> for FeatureFlag {
    fn from(f: AFeatureFlag) -> Self {
        match f {
            AFeatureFlag::CODE_DEPENDENCY_CHECK => FeatureFlag::CodeDependencyCheck,
            AFeatureFlag::TREAT_FRIEND_AS_PRIVATE => FeatureFlag::TreatFriendAsPrivate,
        }
    }
}

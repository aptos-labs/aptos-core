// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use kube::CustomResource;
use serde::{Deserialize, Serialize};

pub enum Chaos {
    Network(NetworkChaos),
    Stress(StressChaos),
}

#[derive(CustomResource, Deserialize, Default, Serialize, Clone, Debug)]
#[kube(
    group = "chaos-mesh.org",
    version = "v1alpha1",
    kind = "NetworkChaos",
    status = "ChaosStatus",
    plural = "networkchaos",
    namespaced,
    schema = "disabled"
)]
pub struct NetworkChaosSpec {}

#[derive(CustomResource, Default, Serialize, Deserialize, Clone, Debug)]
#[kube(
    group = "chaos-mesh.org",
    version = "v1alpha1",
    kind = "StressChaos",
    status = "ChaosStatus",
    plural = "stresschaos",
    namespaced,
    schema = "disabled"
)]
pub struct StressChaosSpec {}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ChaosStatus {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<ChaosCondition>>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ChaosCondition {
    #[serde(rename = "type")]
    pub r#type: ChaosConditionType,

    pub status: ConditionStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ConditionStatus {
    False,
    True,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ChaosConditionType {
    Selected,
    AllInjected,
    AllRecovered,
    Paused,
}

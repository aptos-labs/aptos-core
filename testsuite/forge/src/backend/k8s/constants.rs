// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

/// A collection of constants and default values for configuring various Forge components.

// These are test keys for forge ephemeral networks. Do not use these elsewhere!
pub const DEFAULT_ROOT_KEY: &str =
    "48136DF3174A3DE92AFDB375FFE116908B69FF6FAB9B1410E548A33FEA1D159D";
pub const DEFAULT_ROOT_PRIV_KEY: &str =
    "E25708D90C72A53B400B27FC7602C4D546C7B7469FA6E12544F0EBFB2F16AE19";

// Seed to generate keys for forge tests.
pub const FORGE_KEY_SEED: &str = "80000";

// binaries expected to be present on test runner
pub const HELM_BIN: &str = "helm";
pub const KUBECTL_BIN: &str = "kubectl";

// helm release names and helm chart paths
pub const APTOS_NODE_HELM_RELEASE_NAME: &str = "aptos-node";
pub const GENESIS_HELM_RELEASE_NAME: &str = "genesis";
pub const APTOS_NODE_HELM_CHART_PATH: &str = "terraform/helm/aptos-node";
pub const GENESIS_HELM_CHART_PATH: &str = "terraform/helm/genesis";

// cleanup namespaces after 30 minutes unless "keep = true"
pub const NAMESPACE_CLEANUP_THRESHOLD_SECS: u64 = 1800;
// Leave a buffer of around 20 minutes for test provisioning and cleanup to be done before cleaning
// up underlying resources.
pub const NAMESPACE_CLEANUP_DURATION_BUFFER_SECS: u64 = 1200;
pub const POD_CLEANUP_THRESHOLD_SECS: u64 = 86400;
pub const MANAGEMENT_CONFIGMAP_PREFIX: &str = "forge-management";

// this is the port on the validator service itself, as opposed to 80 on the validator haproxy service
pub const NODE_METRIC_PORT: u32 = 9101;
pub const REST_API_SERVICE_PORT: u32 = 8080;
pub const REST_API_HAPROXY_SERVICE_PORT: u32 = 80;
// when we interact with the node over port-forward
pub const LOCALHOST: &str = "127.0.0.1";

// kubernetes service names
pub const VALIDATOR_SERVICE_SUFFIX: &str = "validator";
pub const FULLNODE_SERVICE_SUFFIX: &str = "fullnode";
pub const VALIDATOR_HAPROXY_SERVICE_SUFFIX: &str = "validator-lb";
pub const FULLNODE_HAPROXY_SERVICE_SUFFIX: &str = "fullnode-lb";
pub const HAPROXY_SERVICE_SUFFIX: &str = "lb";

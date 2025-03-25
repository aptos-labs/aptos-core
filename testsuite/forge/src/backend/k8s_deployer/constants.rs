// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub const FORGE_INDEXER_DEPLOYER_DOCKER_IMAGE_REPO: &str =
    "us-docker.pkg.dev/aptos-registry/docker/forge-indexer-deployer";
pub const FORGE_TESTNET_DEPLOYER_DOCKER_IMAGE_REPO: &str =
    "us-docker.pkg.dev/aptos-registry/docker/forge-testnet-deployer";
pub const VALIDATOR_DOCKER_IMAGE_REPO: &str = "us-docker.pkg.dev/aptos-registry/docker/validator";
pub const INDEXER_GRPC_DOCKER_IMAGE_REPO: &str =
    "us-docker.pkg.dev/aptos-registry/docker/indexer-grpc";

/// The version of the forge deployer image to use.
pub const DEFAULT_FORGE_DEPLOYER_IMAGE_TAG: &str = "90865ea9b15feb0e1fc234f6e08bc3f8db98c4b7"; // default to the latest stable build from the main branch

/// This is the service account name that the deployer will use to deploy the forge components. It may require extra permissions and additonal setup
pub const FORGE_DEPLOYER_SERVICE_ACCOUNT_NAME: &str = "forge";

/// This is the environment variable that is required to be set in the pod to provide the deployer
pub const FORGE_DEPLOYER_VALUES_ENV_VAR_NAME: &str = "FORGE_DEPLOY_VALUES_JSON";

pub const DEFAULT_FORGE_DEPLOYER_PROFILE: &str = "forge";

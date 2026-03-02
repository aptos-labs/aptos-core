// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub const FORGE_INDEXER_DEPLOYER_DOCKER_IMAGE_REPO: &str =
    "us-docker.pkg.dev/aptos-registry/docker/forge-indexer-deployer";
pub const FORGE_TESTNET_DEPLOYER_DOCKER_IMAGE_REPO: &str =
    "us-docker.pkg.dev/aptos-registry/docker/forge-testnet-deployer";
pub const FORGE_PFN_DEPLOYER_DOCKER_IMAGE_REPO: &str =
    "us-docker.pkg.dev/aptos-registry/docker/forge-pfn-deployer";
pub const VALIDATOR_DOCKER_IMAGE_REPO: &str = "us-docker.pkg.dev/aptos-registry/docker/validator";
pub const INDEXER_GRPC_DOCKER_IMAGE_REPO: &str =
    "us-docker.pkg.dev/aptos-registry/docker/indexer-grpc";

/// The version of the forge deployer image to use.
/// TODO: update to the final tag after https://github.com/aptos-labs/internal-ops/pull/7222 lands
pub const DEFAULT_FORGE_DEPLOYER_IMAGE_TAG: &str = "5c9e62b0577ca92a942eaeed79b64827b08055af";

/// This is the service account name that the deployer will use to deploy the forge components. It may require extra permissions and additonal setup
pub const FORGE_DEPLOYER_SERVICE_ACCOUNT_NAME: &str = "forge";

/// This is the environment variable that is required to be set in the pod to provide the deployer
pub const FORGE_DEPLOYER_VALUES_ENV_VAR_NAME: &str = "FORGE_DEPLOY_VALUES_JSON";

pub const DEFAULT_FORGE_DEPLOYER_PROFILE: &str = "forge";

pub const FORGE_GENESIS_SHARED_BUCKET: &str = "gs://aptos-forge-shared-genesis-bucket/genesis";

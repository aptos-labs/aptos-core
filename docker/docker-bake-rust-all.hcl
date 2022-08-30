# This is a docker bake file in HCL syntax.
# It provides a high-level mechenanism to build multiple dockerfiles in one shot.
# Check https://crazymax.dev/docker-allhands2-buildx-bake and https://docs.docker.com/engine/reference/commandline/buildx_bake/#file-definition for an intro.

variable "CI" {
  # whether this build runs in aptos-labs' CI environment which makes certain assumptions about certain registries being available to push to cache layers.
  # for local builds we simply default to relying on dockers local caching.
  default = "false"
}
variable "TARGET_CACHE_ID" {}
variable "TARGET_CACHE_TYPE" {
  // must be "normalized_branch_or_pr" | "git_sha"
  default = "normalized_branch_or_pr"
}
variable "BUILD_DATE" {}
// this is the full GIT_SHA - let's use that as primary identifier going forward
variable "GIT_SHA" {}

variable "GIT_BRANCH" {}

variable "GIT_TAG" {}

variable "BUILT_VIA_BUILDKIT" {}

variable "LAST_GREEN_COMMIT" {}

variable "GCP_DOCKER_ARTIFACT_REPO" {}

variable "AWS_ECR_ACCOUNT_NUM" {}

variable "TARGET_REGISTRY" {
  // must be "aws" | "gcp" | "local", informs which docker tags are being generated
  default = CI == "true" ? "gcp" : "local"
}

variable "ecr_base" {
  default = "${AWS_ECR_ACCOUNT_NUM}.dkr.ecr.us-west-2.amazonaws.com/aptos"
}

variable "normalized_branch_or_pr" {
  default = regex_replace("${TARGET_CACHE_ID}", "[^a-zA-Z0-9]", "-")
}
variable "BUILD_TEST_IMAGES" {
  // Whether to build test images
  default = "false"
}
variable "PROFILE" {
  // Cargo compilation profile
  default = "release"
}
variable "FEATURES" {
  // Cargo features to enable, as a comma separated string
  default = ""
}
variable "normalized_features_list" {
  default = regex_replace("${FEATURES}", "[^a-zA-Z0-9]", "-")
}
variable "image_tag_prefix" {
  // prefix to add to the image tag in the case of a non-default (cargo profile or feature) build
  default = PROFILE == "release" ? (
    FEATURES == "" ? "" : "${normalized_features_list}_"
    ) : (
    FEATURES == "" ? "${PROFILE}_" : "${PROFILE}_${normalized_features_list}_"
  )
}

target "builder" {
  target     = "builder"
  dockerfile = "docker/rust-all.Dockerfile"
  context    = "."
  cache-from = generate_cache_from("builder")
  cache-to   = generate_cache_to("builder")
  tags       = generate_tags("builder")
  args = {
    PROFILE            = "${PROFILE}"
    FEATURES           = "${FEATURES}"
    GIT_SHA            = "${GIT_SHA}"
    GIT_BRANCH         = "${GIT_BRANCH}"
    GIT_TAG            = "${GIT_TAG}"
    BUILT_VIA_BUILDKIT = "true"
  }
}

group "all" {
  targets = flatten([
    "validator",
    "indexer",
    "node-checker",
    "tools",
    "faucet",
    "forge",
    "telemetry-service",
    BUILD_TEST_IMAGES == "true" ? [
      "validator-testing"
    ] : []
  ])
}

target "_common" {
  dockerfile = "docker/rust-all.Dockerfile"
  context    = "."
  cache-from = flatten([
    // need to repeat all images here until https://github.com/docker/buildx/issues/934 is resolved
    generate_cache_from("builder"),
    generate_cache_from("validator"),
    generate_cache_from("indexer"),
    generate_cache_from("node-checker"),
    generate_cache_from("tools"),
    generate_cache_from("faucet"),
    generate_cache_from("forge"),
    generate_cache_from("telemetry-service"),

    // testing targets
    generate_cache_from("validator-testing"),
  ])
  labels = {
    "org.label-schema.schema-version" = "1.0",
    "org.label-schema.build-date"     = "${BUILD_DATE}"
    "org.label-schema.git-sha"        = "${GIT_SHA}"
  }
  args = {
    GIT_SHA            = "${GIT_SHA}"
    GIT_BRANCH         = "${GIT_BRANCH}"
    GIT_TAG            = "${GIT_TAG}"
    BUILD_DATE         = "${BUILD_DATE}"
    BUILT_VIA_BUILDKIT = "true"
  }
}

target "validator" {
  inherits = ["_common"]
  target   = "validator"
  cache-to = generate_cache_to("validator")
  tags     = generate_tags("validator")
}

target "validator-testing" {
  inherits = ["_common"]
  target   = "validator-testing"
  cache-to = generate_cache_to("validator-testing")
  tags     = generate_tags("validator-testing")
}

target "indexer" {
  inherits = ["_common"]
  target   = "indexer"
  cache-to = generate_cache_to("indexer")
  tags     = generate_tags("indexer")
}

target "node-checker" {
  inherits = ["_common"]
  target   = "node-checker"
  cache-to = generate_cache_to("node-checker")
  tags     = generate_tags("node-checker")
}

target "tools" {
  inherits = ["_common"]
  target   = "tools"
  cache-to = generate_cache_to("tools")
  tags     = generate_tags("tools")
}

target "faucet" {
  inherits = ["_common"]
  target   = "faucet"
  cache-to = generate_cache_to("faucet")
  tags     = generate_tags("faucet")
}

target "forge" {
  inherits = ["_common"]
  target   = "forge"
  cache-to = generate_cache_to("forge")
  tags     = generate_tags("forge")
}

target "telemetry-service" {
  inherits = ["_common"]
  target   = "telemetry-service"
  cache-to = generate_cache_to("telemetry-service")
  tags     = generate_tags("telemetry-service")
}

function "generate_cache_from" {
  params = [target]
  result = CI == "true" ? [
    "type=registry,ref=${GCP_DOCKER_ARTIFACT_REPO}/${target}:${image_tag_prefix}main",
    "type=registry,ref=${GCP_DOCKER_ARTIFACT_REPO}/${target}:${image_tag_prefix}${normalized_branch_or_pr}",
    "type=registry,ref=${GCP_DOCKER_ARTIFACT_REPO}/${target}:${image_tag_prefix}${GIT_SHA}",
  ] : []
}

## we only cache to GCP because AWS ECR doesn't support cache manifests
function "generate_cache_to" {
  params = [target]
  result = TARGET_REGISTRY == "gcp" ? ["type=inline"] : []
}

function "generate_tags" {
  params = [target]
  result = TARGET_REGISTRY == "gcp" ? [
    "${GCP_DOCKER_ARTIFACT_REPO}/${target}:${image_tag_prefix}${GIT_SHA}",
    "${GCP_DOCKER_ARTIFACT_REPO}/${target}:${image_tag_prefix}${normalized_branch_or_pr}",
    ] : TARGET_REGISTRY == "aws" ? [
    "${ecr_base}/${target}:${image_tag_prefix}${GIT_SHA}",
    ] : [
    "aptos-core/${target}:${image_tag_prefix}${GIT_SHA}-from-local",
    "aptos-core/${target}:${image_tag_prefix}from-local",
  ]
}

# This is a docker bake file in HCL syntax.
# It provides a high-level mechenanism to build multiple dockerfiles in one shot.
# Check https://crazymax.dev/docker-allhands2-buildx-bake and https://docs.docker.com/engine/reference/commandline/buildx_bake/#file-definition for an intro.

variable "CI" {
  # whether this build runs in aptos-labs' CI environment which makes certain assumptions about certain registries being available to push to cache layers.
  # for local builds we simply default to relying on dockers local caching.
  default = "false"
}
variable "TARGET_CACHE_ID" {}
variable "BUILD_DATE" {}
// this is the full GIT_SHA - let's use that as primary identifier going forward
variable "GIT_SHA" {}

variable "GIT_BRANCH" {}

variable "GIT_TAG" {}

variable "GIT_CREDENTIALS" {}

variable "BUILT_VIA_BUILDKIT" {}

variable "GCP_DOCKER_ARTIFACT_REPO" {}

variable "GCP_DOCKER_ARTIFACT_REPO_US" {}

variable "AWS_ECR_ACCOUNT_NUM" {}

variable "TARGET_REGISTRY" {
  // must be "gcp" | "local" | "remote-all" | "remote" (deprecated, but kept for backwards compatibility. Same as "gcp"), informs which docker tags are being generated
  default = CI == "true" ? "remote" : "local"
}

variable "ecr_base" {
  default = "${AWS_ECR_ACCOUNT_NUM}.dkr.ecr.us-west-2.amazonaws.com/aptos"
}

variable "NORMALIZED_GIT_BRANCH_OR_PR" {}
variable "IMAGE_TAG_PREFIX" {}
variable "PROFILE" {
  // Cargo compilation profile
  default = "release"
}
variable "FEATURES" {
  // Cargo features to enable, as a comma separated string
}
variable "CARGO_TARGET_DIR" {
  // Cargo target directory
}

group "all" {
  targets = flatten([
    "validator",
    "node-checker",
    "tools",
    "faucet",
    "forge",
    "telemetry-service",
    "indexer-grpc",
    "validator-testing",
  ])
}

group "forge-images" {
  targets = ["validator-testing", "tools", "forge"]
}

target "debian-base" {
  dockerfile = "docker/builder/debian-base.Dockerfile"
  contexts = {
    debian = "docker-image://debian:bullseye@sha256:2c407480ad7c98bdc551dbb38b92acb674dc130c8298f2e0fa2ad34da9078637"
  }
}

target "builder-base" {
  dockerfile = "docker/builder/builder.Dockerfile"
  target     = "builder-base"
  context    = "."
  contexts = {
    rust = "docker-image://rust:1.71.1-bullseye@sha256:6b5a53fef2818e28548be943a622bfc52d73920fe0f8784f4296227bca30cdf1"
  }
  args = {
    PROFILE            = "${PROFILE}"
    FEATURES           = "${FEATURES}"
    CARGO_TARGET_DIR   = "${CARGO_TARGET_DIR}"
    BUILT_VIA_BUILDKIT = "true"
  }
  secret = [
    "id=GIT_CREDENTIALS"
  ]
}

target "aptos-node-builder" {
  dockerfile = "docker/builder/builder.Dockerfile"
  target     = "aptos-node-builder"
  contexts = {
    builder-base = "target:builder-base"
  }
  secret = [
    "id=GIT_CREDENTIALS"
  ]
}

target "tools-builder" {
  dockerfile = "docker/builder/builder.Dockerfile"
  target     = "tools-builder"
  contexts = {
    builder-base = "target:builder-base"
  }
  secret = [
    "id=GIT_CREDENTIALS"
  ]
}

target "_common" {
  contexts = {
    debian-base   = "target:debian-base"
    node-builder  = "target:aptos-node-builder"
    tools-builder = "target:tools-builder"
  }
  labels = {
    "org.label-schema.schema-version" = "1.0",
    "org.label-schema.build-date"     = "${BUILD_DATE}"
    "org.label-schema.git-sha"        = "${GIT_SHA}"
  }
  args = {
    PROFILE    = "${PROFILE}"
    FEATURES   = "${FEATURES}"
    GIT_SHA    = "${GIT_SHA}"
    GIT_BRANCH = "${GIT_BRANCH}"
    GIT_TAG    = "${GIT_TAG}"
    BUILD_DATE = "${BUILD_DATE}"
  }
}

target "validator-testing" {
  inherits   = ["_common"]
  dockerfile = "docker/builder/validator-testing.Dockerfile"
  target     = "validator-testing"
  cache-from = generate_cache_from("validator-testing")
  cache-to   = generate_cache_to("validator-testing")
  tags       = generate_tags("validator-testing")
}

target "tools" {
  inherits   = ["_common"]
  dockerfile = "docker/builder/tools.Dockerfile"
  target     = "tools"
  cache-from = generate_cache_from("tools")
  cache-to   = generate_cache_to("tools")
  tags       = generate_tags("tools")
}

target "forge" {
  inherits   = ["_common"]
  dockerfile = "docker/builder/forge.Dockerfile"
  target     = "forge"
  cache-from = generate_cache_from("forge")
  cache-to   = generate_cache_to("forge")
  tags       = generate_tags("forge")
}

target "validator" {
  inherits   = ["_common"]
  dockerfile = "docker/builder/validator.Dockerfile"
  target     = "validator"
  cache-from = generate_cache_from("validator")
  cache-to   = generate_cache_to("validator")
  tags       = generate_tags("validator")
}

target "tools" {
  inherits   = ["_common"]
  dockerfile = "docker/builder/tools.Dockerfile"
  target     = "tools"
  cache-from = generate_cache_from("tools")
  cache-to   = generate_cache_to("tools")
  tags       = generate_tags("tools")
}

target "node-checker" {
  inherits   = ["_common"]
  dockerfile = "docker/builder/node-checker.Dockerfile"
  target     = "node-checker"
  cache-from = generate_cache_from("node-checker")
  cache-to   = generate_cache_to("node-checker")
  tags       = generate_tags("node-checker")
}

target "faucet" {
  inherits   = ["_common"]
  dockerfile = "docker/builder/faucet.Dockerfile"
  target     = "faucet"
  cache-from = generate_cache_from("faucet")
  cache-to   = generate_cache_to("faucet")
  tags       = generate_tags("faucet")
}

target "telemetry-service" {
  inherits   = ["_common"]
  dockerfile = "docker/builder/telemetry-service.Dockerfile"
  target     = "telemetry-service"
  cache-from = generate_cache_from("telemetry-service")
  cache-to   = generate_cache_to("telemetry-service")
  tags       = generate_tags("telemetry-service")
}

target "indexer-grpc" {
  inherits   = ["_common"]
  dockerfile = "docker/builder/indexer-grpc.Dockerfile"
  target     = "indexer-grpc"
  cache-to   = generate_cache_to("indexer-grpc")
  tags       = generate_tags("indexer-grpc")
}

function "generate_cache_from" {
  params = [target]
  result = CI == "true" ? [
    "type=registry,ref=${GCP_DOCKER_ARTIFACT_REPO}/${target}:cache-${IMAGE_TAG_PREFIX}main",
    "type=registry,ref=${GCP_DOCKER_ARTIFACT_REPO}/${target}:cache-${IMAGE_TAG_PREFIX}${NORMALIZED_GIT_BRANCH_OR_PR}",
    "type=registry,ref=${GCP_DOCKER_ARTIFACT_REPO}/${target}:cache-${IMAGE_TAG_PREFIX}${GIT_SHA}",
  ] : []
}

function "generate_cache_to" {
  params = [target]
  result = TARGET_REGISTRY != "local" ? [
    "type=registry,ref=${GCP_DOCKER_ARTIFACT_REPO}/${target}:cache-${IMAGE_TAG_PREFIX}${NORMALIZED_GIT_BRANCH_OR_PR}",
    "type=registry,ref=${GCP_DOCKER_ARTIFACT_REPO}/${target}:cache-${IMAGE_TAG_PREFIX}${GIT_SHA}"
  ] : []
}

function "generate_tags" {
  params = [target]
  result = TARGET_REGISTRY == "remote-all" ? [
    "${GCP_DOCKER_ARTIFACT_REPO}/${target}:${IMAGE_TAG_PREFIX}${GIT_SHA}",
    "${GCP_DOCKER_ARTIFACT_REPO}/${target}:${IMAGE_TAG_PREFIX}${NORMALIZED_GIT_BRANCH_OR_PR}",
    "${GCP_DOCKER_ARTIFACT_REPO_US}/${target}:${IMAGE_TAG_PREFIX}${GIT_SHA}",
    "${GCP_DOCKER_ARTIFACT_REPO_US}/${target}:${IMAGE_TAG_PREFIX}${NORMALIZED_GIT_BRANCH_OR_PR}",
    "${ecr_base}/${target}:${IMAGE_TAG_PREFIX}${GIT_SHA}",
    "${ecr_base}/${target}:${IMAGE_TAG_PREFIX}${NORMALIZED_GIT_BRANCH_OR_PR}",
    ] : (
    TARGET_REGISTRY == "gcp" || TARGET_REGISTRY == "remote" ? [
      "${GCP_DOCKER_ARTIFACT_REPO}/${target}:${IMAGE_TAG_PREFIX}${GIT_SHA}",
      "${GCP_DOCKER_ARTIFACT_REPO}/${target}:${IMAGE_TAG_PREFIX}${NORMALIZED_GIT_BRANCH_OR_PR}",
      "${GCP_DOCKER_ARTIFACT_REPO_US}/${target}:${IMAGE_TAG_PREFIX}${GIT_SHA}",
      "${GCP_DOCKER_ARTIFACT_REPO_US}/${target}:${IMAGE_TAG_PREFIX}${NORMALIZED_GIT_BRANCH_OR_PR}",
      ] : [ // "local" or any other value
      "aptos-core/${target}:${IMAGE_TAG_PREFIX}${GIT_SHA}-from-local",
      "aptos-core/${target}:${IMAGE_TAG_PREFIX}from-local",
    ]
  )
}

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
    "keyless-pepper-service",
    "indexer-grpc",
    "validator-testing",
    "nft-metadata-crawler",
  ])
}

group "forge-images" {
  targets = ["validator-testing", "tools", "forge"]
}

target "debian-base" {
  dockerfile = "docker/builder/debian-base.Dockerfile"
  contexts = {
    # Run `docker buildx imagetools inspect debian:bullseye` to find the latest multi-platform hash
    debian = "docker-image://debian:bullseye@sha256:2a7f95bcf104c8410bf4d3b13c52f6e0e4334bb2edf8d80c7f9881e49447effe"
  }
}

target "builder-base" {
  dockerfile = "docker/builder/builder.Dockerfile"
  target     = "builder-base"
  context    = "."
  contexts = {
    # Run `docker buildx imagetools inspect rust:1.78.0-bullseye` to find the latest multi-platform hash
    rust = "docker-image://rust:1.78.0-bullseye@sha256:c8f85185bd2e482d88e1b8a90705435309ca9d54ccc3bcccf24a32378b8ff1a8"
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

target "indexer-builder" {
  dockerfile = "docker/builder/builder.Dockerfile"
  target     = "indexer-builder"
  contexts = {
    builder-base = "target:builder-base"
  }
  secret = [
    "id=GIT_CREDENTIALS"
  ]
}

target "_common" {
  contexts = {
    debian-base     = "target:debian-base"
    node-builder    = "target:aptos-node-builder"
    tools-builder   = "target:tools-builder"
    indexer-builder = "target:indexer-builder"
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
  output     = ["type=image,compression=zstd,force-compression=true"]
}

target "validator-testing" {
  inherits   = ["_common"]
  dockerfile = "docker/builder/validator-testing.Dockerfile"
  target     = "validator-testing"
  tags       = generate_tags("validator-testing")
}

target "tools" {
  inherits   = ["_common"]
  dockerfile = "docker/builder/tools.Dockerfile"
  target     = "tools"
  tags       = generate_tags("tools")
}

target "forge" {
  inherits   = ["_common"]
  dockerfile = "docker/builder/forge.Dockerfile"
  target     = "forge"
  tags       = generate_tags("forge")
}

target "validator" {
  inherits   = ["_common"]
  dockerfile = "docker/builder/validator.Dockerfile"
  target     = "validator"
  tags       = generate_tags("validator")
}

target "tools" {
  inherits   = ["_common"]
  dockerfile = "docker/builder/tools.Dockerfile"
  target     = "tools"
  tags       = generate_tags("tools")
}

target "node-checker" {
  inherits   = ["_common"]
  dockerfile = "docker/builder/node-checker.Dockerfile"
  target     = "node-checker"
  tags       = generate_tags("node-checker")
}

target "faucet" {
  inherits   = ["_common"]
  dockerfile = "docker/builder/faucet.Dockerfile"
  target     = "faucet"
  tags       = generate_tags("faucet")
}

target "telemetry-service" {
  inherits   = ["_common"]
  dockerfile = "docker/builder/telemetry-service.Dockerfile"
  target     = "telemetry-service"
  tags       = generate_tags("telemetry-service")
}

target "keyless-pepper-service" {
  inherits   = ["_common"]
  dockerfile = "docker/builder/keyless-pepper-service.Dockerfile"
  target     = "keyless-pepper-service"
  tags       = generate_tags("keyless-pepper-service")
}

target "indexer-grpc" {
  inherits   = ["_common"]
  dockerfile = "docker/builder/indexer-grpc.Dockerfile"
  target     = "indexer-grpc"
  tags       = generate_tags("indexer-grpc")
}

target "nft-metadata-crawler" {
  inherits   = ["_common"]
  target     = "nft-metadata-crawler"
  dockerfile = "docker/builder/nft-metadata-crawler.Dockerfile"
  tags       = generate_tags("nft-metadata-crawler")
}

function "generate_tags" {
  params = [target]
  result = TARGET_REGISTRY == "remote-all" ? [
    "${GCP_DOCKER_ARTIFACT_REPO}/${target}:${IMAGE_TAG_PREFIX}${GIT_SHA}",
    "${GCP_DOCKER_ARTIFACT_REPO}/${target}:${IMAGE_TAG_PREFIX}${NORMALIZED_GIT_BRANCH_OR_PR}",
    "${ecr_base}/${target}:${IMAGE_TAG_PREFIX}${GIT_SHA}",
    "${ecr_base}/${target}:${IMAGE_TAG_PREFIX}${NORMALIZED_GIT_BRANCH_OR_PR}",
    ] : (
    TARGET_REGISTRY == "gcp" || TARGET_REGISTRY == "remote" ? [
      "${GCP_DOCKER_ARTIFACT_REPO}/${target}:${IMAGE_TAG_PREFIX}${GIT_SHA}",
      "${GCP_DOCKER_ARTIFACT_REPO}/${target}:${IMAGE_TAG_PREFIX}${NORMALIZED_GIT_BRANCH_OR_PR}",
      ] : [ // "local" or any other value
      "aptos-core/${target}:${IMAGE_TAG_PREFIX}${GIT_SHA}-from-local",
      "aptos-core/${target}:${IMAGE_TAG_PREFIX}from-local",
    ]
  )
}

# This is a docker bake file in HCL syntax.
# It provides a high-level mechenanism to build multiple dockerfiles in one shot.
# Check https://crazymax.dev/docker-allhands2-buildx-bake and https://docs.docker.com/engine/reference/commandline/buildx_bake/#file-definition for an intro.

variable "BUILD_DATE" {}

variable "GITHUB_SHA" {}
// this is the full GIT_SHA - let's use that as primary identifier going forward
variable "GIT_SHA" {
  default = "${GITHUB_SHA}"
}
// this is the short GIT_SHA (8 chars). Tagging our docker images with that one is kinda deprecated and we might remove this in future.
variable "GIT_REV" {
  default = substr("${GIT_SHA}", 0, 8)
}

variable "GIT_BRANCH" {}

variable "GCP_DOCKER_ARTIFACT_REPO" {}

variable "AWS_ECR_ACCOUNT_NUM" {}

variable "ecr_base" {
  default = "${AWS_ECR_ACCOUNT_NUM}.dkr.ecr.us-west-2.amazonaws.com/aptos"
}

variable "gh_image_cache" {
  default = "ghcr.io/aptos-labs/aptos-core"
}

variable "normalized_git_branch" {
  default = regex_replace("${GIT_BRANCH}", "[^a-zA-Z0-9]", "-")
}

# images with IMAGE_TARGET=release for rust build
group "release" {
  targets = [
    "validator",
    "indexer",
    "node-checker",
    "safety-rules",
    "tools",
    "init",
    "txn-emitter",
  ]
}

# images with IMAGE_TARGET=test for rust build
group "test" {
  targets = [
    "faucet",
    "forge",
  ]
}

target "_common" {
  dockerfile = "docker/rust-all.Dockerfile"
  context    = "."
  cache-from = flatten([
    // need to repeat all images here until https://github.com/docker/buildx/issues/934 is resolved
    generate_cache_from("validator"),
    generate_cache_from("indexer"),
    generate_cache_from("node-checker"),
    generate_cache_from("validator_tcb"),
    generate_cache_from("tools"),
    generate_cache_from("init"),
    generate_cache_from("txn-emitter"),
    generate_cache_from("faucet"),
    generate_cache_from("forge"),
  ])
  labels = {
    "org.label-schema.schema-version" = "1.0",
    "org.label-schema.build-date"     = "${BUILD_DATE}"
    "org.label-schema.vcs-ref"        = "${GIT_REV}"
  }
  args = {
    IMAGE_TARGET = "release"
  }
}

target "validator" {
  inherits = ["_common"]
  target   = "validator"
  cache-to = generate_cache_to("validator")
  tags     = generate_tags("validator")
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

target "safety-rules" {
  inherits = ["_common"]
  target   = "safety-rules"
  cache-to = generate_cache_to("validator_tcb")
  tags     = generate_tags("validator_tcb")
}

target "tools" {
  inherits = ["_common"]
  target   = "tools"
  cache-to = generate_cache_to("tools")
  tags     = generate_tags("tools")
}

target "init" {
  inherits = ["_common"]
  target   = "init"
  cache-to = generate_cache_to("init")
  tags     = generate_tags("init")
}

target "txn-emitter" {
  inherits = ["_common"]
  target   = "txn-emitter"
  cache-to = generate_cache_to("txn-emitter")
  tags     = generate_tags("txn-emitter")
}

target "faucet" {
  inherits = ["_common"]
  target   = "faucet"
  cache-to = generate_cache_to("faucet")
  tags     = generate_tags("faucet")
  args = {
    IMAGE_TARGET = "test"
  }
}

target "forge" {
  inherits = ["_common"]
  target   = "forge"
  cache-to = generate_cache_to("forge")
  tags     = generate_tags("forge")
  args = {
    IMAGE_TARGET = "test"
  }
}

function "generate_cache_from" {
  params = [target]
  result = [
    "type=registry,ref=${GCP_DOCKER_ARTIFACT_REPO}/${target}:cache-main",
    "type=registry,ref=${GCP_DOCKER_ARTIFACT_REPO}/${target}:cache-auto",
    "type=registry,ref=${GCP_DOCKER_ARTIFACT_REPO}/${target}:cache-${normalized_git_branch}"
  ]
}

function "generate_cache_to" {
  params = [target]
  result = ["type=registry,ref=${GCP_DOCKER_ARTIFACT_REPO}/${target}:cache-${normalized_git_branch},mode=max"]
}

function "generate_tags" {
  params = [target]
  result = [
    "${GCP_DOCKER_ARTIFACT_REPO}/${target}:${GIT_SHA}",
    "${ecr_base}/${target}:${GIT_SHA}", // only tag with full GIT_SHA unless it turns out we really need any of the other variations
  ]
}

# This is a docker bake file in HCL syntax.
# It provides a high-level mechenanism to build multiple dockerfiles in one shot.
# Check https://crazymax.dev/docker-allhands2-buildx-bake and https://docs.docker.com/engine/reference/commandline/buildx_bake/#file-definition for an intro.

variable "TARGET_CACHE_ID" {}

variable "BUILD_DATE" {}
variable "CI" {
  # whether this build runs in aptos-labs' CI environment which makes certain assumptions about certain registries being available to push to cache layers.
  # for local builds we simply default to relying on dockers local caching.
  default = "false"
}

// this is the full GIT_SHA - let's use that as primary identifier going forward
variable "GIT_SHA" {}

variable "LAST_GREEN_COMMIT" {}

variable "GCP_DOCKER_ARTIFACT_REPO" {}

variable "AWS_ECR_ACCOUNT_NUM" {}

variable "ecr_base" {
  default = "${AWS_ECR_ACCOUNT_NUM}.dkr.ecr.us-west-2.amazonaws.com/aptos"
}

variable "normalized_git_branch" {
  default = regex_replace("${TARGET_CACHE_ID}", "[^a-zA-Z0-9]", "-")
}

group "default" {
  targets = [
    "validator",
    "indexer",
    "node-checker",
    "tools",
    "faucet",
    "forge"
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
    generate_cache_from("tools"),
    generate_cache_from("faucet"),
    generate_cache_from("forge"),
  ])
  labels = {
    "org.label-schema.schema-version" = "1.0",
    "org.label-schema.build-date"     = "${BUILD_DATE}"
    "org.label-schema.git-sha"        = "${GIT_SHA}"
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

function "generate_cache_from" {
  params = [target]
  result = CI == "true" ? [
    "type=registry,ref=${GCP_DOCKER_ARTIFACT_REPO}/${target}:cache-main",
    "type=registry,ref=${GCP_DOCKER_ARTIFACT_REPO}/${target}:cache-auto",
    "type=registry,ref=${GCP_DOCKER_ARTIFACT_REPO}/${target}:cache-${normalized_git_branch}",
  ] : []
}

function "generate_cache_to" {
  params = [target]
  result = CI == "true" ? [
    "type=registry,ref=${GCP_DOCKER_ARTIFACT_REPO}/${target}:cache-${normalized_git_branch},mode=max"
  ] : []
}

function "generate_tags" {
  params = [target]
  result = CI == "true" ? [
    "${GCP_DOCKER_ARTIFACT_REPO}/${target}:${GIT_SHA}",
    "${ecr_base}/${target}:${GIT_SHA}", // only tag with full GIT_SHA unless it turns out we really need any of the other variations
  ] : ["aptoslabs/aptos-core/${target}:${GIT_SHA}-from-local"]
}

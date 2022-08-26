# This is a docker bake file in HCL syntax.
# It provides a high-level mechenanism to build multiple dockerfiles in one shot.
# Check https://crazymax.dev/docker-allhands2-buildx-bake and https://docs.docker.com/engine/reference/commandline/buildx_bake/#file-definition for an intro.


variable "TARGET_CACHE_ID" {}
variable "GIT_SHA" {}
variable "AWS_ECR_ACCOUNT_NUM" {}
variable "GCP_DOCKER_ARTIFACT_REPO" {}
variable "ecr_base" {
  default = "${AWS_ECR_ACCOUNT_NUM}.dkr.ecr.us-west-2.amazonaws.com/aptos"
}

variable "normalized_target_cache_id" {
  default = regex_replace("${TARGET_CACHE_ID}", "[^a-zA-Z0-9]", "-")
}

group "default" {
  targets = [
    "community-platform",
  ]
}

target "community-platform" {
  dockerfile = "Dockerfile"
  context    = "."
  cache-from = [
    "type=registry,ref=${GCP_DOCKER_ARTIFACT_REPO}/community-platform:cache-main",
    "type=registry,ref=${GCP_DOCKER_ARTIFACT_REPO}/community-platform:cache-auto",
    "type=registry,ref=${GCP_DOCKER_ARTIFACT_REPO}/community-platform:cache-${normalized_target_cache_id}",
  ]
  cache-to = ["type=registry,ref=${GCP_DOCKER_ARTIFACT_REPO}/community-platform:cache-${normalized_target_cache_id},mode=max"]
  tags = [
    "${ecr_base}/community-platform:${GIT_SHA}",
    "${GCP_DOCKER_ARTIFACT_REPO}/community-platform:${GIT_SHA}",
  ]
}

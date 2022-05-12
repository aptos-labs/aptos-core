# This is a docker bake file in HCL syntax.
# It provides a high-level mechenanism to build multiple dockerfiles in one shot.
# Check https://crazymax.dev/docker-allhands2-buildx-bake and https://docs.docker.com/engine/reference/commandline/buildx_bake/#file-definition for an intro.


variable "GIT_SHA1" {}
variable "AWS_ECR_ACCOUNT_URL" {}
variable "GCP_DOCKER_ARTIFACT_REPO" {}

variable "gh_image_cache" {
  default = "ghcr.io/aptos-labs/aptos-core/community-platform"
}

group "default" {
  targets = [
    "community-platform",
  ]
}

target "community-platform" {
  dockerfile = "Dockerfile"
  context    = "."
  cache-from = ["type=registry,ref=${gh_image_cache}"]
  cache-to   = ["type=registry,ref=${gh_image_cache},mode=max"]
  tags = [
    "${AWS_ECR_ACCOUNT_URL}/aptos/community-platform:${GIT_SHA1}",
    "${GCP_DOCKER_ARTIFACT_REPO}/community-platform:${GIT_SHA1}",
  ]
}

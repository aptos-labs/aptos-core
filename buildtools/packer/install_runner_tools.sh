#!/bin/bash
# Installs the system tools CI jobs expect on RunsOn runners. Runs as root during the AMI build
# and must work on both the full and the minimal RunsOn Ubuntu images.
set -euxo pipefail

export DEBIAN_FRONTEND=noninteractive

# Minimal images only grant the ubuntu user sudo; dev_setup.sh and CI jobs run as runner.
if ! sudo -l -U runner | grep -q NOPASSWD; then
  echo "runner ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/91-runner-nopasswd
  chmod 0440 /etc/sudoers.d/91-runner-nopasswd
  visudo -cf /etc/sudoers.d/91-runner-nopasswd
fi

apt-get update
apt-get install -y --no-install-recommends ca-certificates curl gpg jq zip unzip

install -d -m 0755 /etc/apt/keyrings

curl -fsSL https://packages.cloud.google.com/apt/doc/apt-key.gpg \
  | gpg --dearmor --yes -o /usr/share/keyrings/cloud.google.gpg
echo "deb [signed-by=/usr/share/keyrings/cloud.google.gpg] https://packages.cloud.google.com/apt cloud-sdk main" \
  > /etc/apt/sources.list.d/google-cloud-sdk.list

# The full image already ships gh with its own apt source; a second source for it fails apt.
if ! command -v gh >/dev/null; then
  curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg \
    -o /etc/apt/keyrings/githubcli-archive-keyring.gpg
  echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" \
    > /etc/apt/sources.list.d/github-cli.list
fi

apt-get update
apt-get install -y --no-install-recommends kubectl google-cloud-cli google-cloud-cli-gke-gcloud-auth-plugin
command -v gh >/dev/null || apt-get install -y --no-install-recommends gh

gcloud --version
kubectl version --client
gke-gcloud-auth-plugin --version
gh --version
jq --version
command -v zip

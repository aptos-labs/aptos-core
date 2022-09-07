#!/bin/bash

# This script formats, validates, and lints the terraform modules in the repo.
# It is intended to be run from the root of the repo.

set -e

echo "##### Installing tflint tool #####"
if ! command -v tflint &>/dev/null; then
    echo "tflint could not be found"
    echo "installing it..."
    if [[ "$(uname)" == "Darwin" ]]; then
        brew install tflint
    else # Assume Linux
        wget https://github.com/terraform-linters/tflint/releases/download/v0.39.3/tflint_linux_amd64.zip
        sha=$(shasum -a 256 tflint_linux_amd64.zip | awk '{ print $1 }')
        [ "$sha" != "53ab21354c3dedc8ae4296b236330b8b0e76a777d2013a6549107822c60631ef" ] && echo "shasum mismatch" && exit 1
        unzip tflint_linux_amd64.zip
        chmod +x tflint
    fi
else
    echo "tflint already installed"
fi

echo "##### Terraform version #####"
terraform version

# Find all the terraform module directories in the repo
# Assume that they all contain a main.tf file, which is best practices
echo "##### Terraform modules in aptos-core #####"
tf_dirs=($(find . -xdev -name main.tf -exec dirname {} \;))
for dir in ${tf_dirs[@]}; do
    echo $dir
done

echo "##### Terraform fmt #####"
terraform fmt -recursive -check .

# Validate all the terraform modules
echo "##### Terraform validate #####"
base_dir=$(pwd)
for dir in ${tf_dirs[@]}; do
    echo "Validating $dir"
    cd $dir
    terraform init -backend=false && terraform validate
    cd $base_dir
done

# Run tflint
echo "##### tflint #####"
base_dir=$(pwd)
./tflint --init --config="${base_dir}/terraform/.tflint.hcl"
for dir in ${tf_dirs[@]}; do
    echo "Linting $dir"
    ./tflint --config="${base_dir}/terraform/.tflint.hcl" --var-file="${base_dir}/terraform/tflint.tfvars" $dir
done

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
        wget https://raw.githubusercontent.com/terraform-linters/tflint/21a0c1c86c5aa0e3e95916e6e25ded69efcf13f3/install_linux.sh
        sha=$(shasum -a 256 install_linux.sh | awk '{ print $1 }')
        [ "$sha" != "54e1b264b0f4b3e183d873273d5ee2053c80222a4422eea0b35d2e88114fbff9" ] && echo "shasum mismatch" && exit 1
        chmod +x install_linux.sh
        ./install_linux.sh
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
tflint --init
base_dir=$(pwd)
for dir in ${tf_dirs[@]}; do
    echo "Lintint $dir"
    cd $dir
    tflint --config "${base_dir}/.tflint.hcl"
    cd $base_dir
done

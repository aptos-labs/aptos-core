#!/bin/bash

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

# A light wrapper for the new forge python script

# show the contents of the forge.env file for debug purposes
echo "Forge environment variables from forge.env:"
echo "------------------------------------------"
cat testsuite/forge.env
echo "------------------------------------------"

FAIL_AFTER_FORGE_RUNS=false
if grep -vE '^\s*#|^\s*$' testsuite/forge.env | grep -q .; then
    echo "WARNING!!!"
    echo "WARNING!!! Envs are set in forge.env. Use forge.env for test only"
    echo "WARNING!!! Forcing Forge to fail after it runs"
    echo "WARNING!!!"
    FAIL_AFTER_FORGE_RUNS=true
fi

# source the forge.env file to set the environment variables which are used as feature flags for the forge script
set -a # export all variables when we source the file
source testsuite/forge.env
set +a # stop exporting variables

echo "Executing python testsuite/forge.py test $@"
exec python3 testsuite/forge.py test "$@"

if $FAIL_AFTER_FORGE_RUNS; then
    echo "WARNING!!! Forge failed since FAIL_AFTER_FORGE_RUNS is set to true"
    echo "WARNING!!! forge.env has likely been set, and this protects against committing it"
    exit 1
fi

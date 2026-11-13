#!/bin/bash

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

# A light wrapper for the new forge python script

# show the contents of the forge_env.sh file for debug purposes
echo "Forge environment variables from forge_env.sh:"
echo "------------------------------------------"
cat testsuite/forge_env.sh
echo "------------------------------------------"

FAIL_AFTER_FORGE_RUNS=false
if grep -vE '^\s*#|^\s*$' testsuite/forge_env.sh | grep -q .; then
    echo "WARNING!!!"
    echo "WARNING!!! Envs are set in forge_env.sh. Use forge_env.sh for test only"
    echo "WARNING!!! Forcing Forge to fail after it runs"
    echo "WARNING!!!"
    FAIL_AFTER_FORGE_RUNS=true
fi

# source the forge_env.sh file to set the environment variables which are used as feature flags for the forge script
set -a # export all variables when we source the file
source testsuite/forge_env.sh
set +a # stop exporting variables

echo "Executing python testsuite/forge.py test $@"
exec python3 testsuite/forge.py test "$@"

if $FAIL_AFTER_FORGE_RUNS; then
    echo "WARNING!!! Forge failed since FAIL_AFTER_FORGE_RUNS is set to true"
    echo "WARNING!!! forge_env.sh has likely been set, and this protects against committing it"
    exit 1
fi

#!/bin/bash

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

# A light wrapper for the new forge python script

# show the contents of the forge.env file for debug purposes
echo "Forge environment variables from forge.env:"
echo "------------------------------------------"
cat testsuite/forge.env
echo "------------------------------------------"
if grep -v '^#' testsuite/forge.env | grep -q .; then
    echo "WARNING!!!"
    echo "WARNING!!! Envs are set in forge.env. Use forge.env for test only"
    echo "WARNING!!!"
fi
# source the forge.env file to set the environment variables which are used as feature flags for the forge script
source testsuite/forge.env

echo "Executing python testsuite/forge.py test $@"
exec python3 testsuite/forge.py test "$@"

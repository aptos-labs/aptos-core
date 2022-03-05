#!/bin/sh
# Copyright (c) The Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
set -e

DIR="$( cd "$( dirname "$0" )" && pwd )"

$DIR/../aptos-build.sh $DIR/../validator/Dockerfile diem/validator "$@"

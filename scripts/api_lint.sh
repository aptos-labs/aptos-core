#!/bin/bash

# Copyright (c) The Diem Core Contributors
# SPDX-License-Identifier: Apache-2.0

set -e

npx @redocly/openapi-cli lint api/doc/openapi.yaml --skip-rule no-empty-servers

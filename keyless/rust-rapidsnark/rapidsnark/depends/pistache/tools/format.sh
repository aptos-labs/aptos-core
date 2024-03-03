#!/bin/sh

# SPDX-FileCopyrightText: 2021 Mathieu Stefani
#
# SPDX-License-Identifier: Apache-2.0

set -eu

find_files() {
    git ls-files --cached --exclude-standard --others | grep -E '\.(cc|cpp|h)$'
}

find_files | xargs clang-format -i

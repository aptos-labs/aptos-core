# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# Exception to use when a test fails, for the CLI did something unexpected,
# an expected output was missing, etc.
#
# For errors with the framework itself, use RuntimeError.
class TestError(Exception):
    pass

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import os
from dataclasses import dataclass
from enum import Enum

NODE_PORT = 8080
FAUCET_PORT = 8081


class Network(Enum):
    DEVNET = "devnet"
    TESTNET = "testnet"
    MAINNET = "mainnet"

    def __str__(self):
        return self.value


# Information for some accounts we use for testing.
@dataclass
class AccountInfo:
    private_key: str
    public_key: str
    account_address: str


OTHER_ACCOUNT_ONE = AccountInfo(
    private_key="0x37368b46ce665362562c6d1d4ec01a08c8644c488690df5a17e13ba163e20221",
    public_key="0x25caf00522e4d4664ec0a27166a69e8a32b5078959d0fc398da70d40d2893e8f",
    account_address="0x585fc9f0f0c54183b039ffc770ca282ebd87307916c215a3e692f2f8e4305e82",
)


def build_image_name(image_repo_with_project: str, tag: str):
    return f"{image_repo_with_project}/tools:{tag}"


def recursive_chmod(path, perms):
    for dirpath, _, filenames in os.walk(path):
        os.chmod(dirpath, perms)
        for filename in filenames:
            os.chmod(os.path.join(dirpath, filename), perms)


# Exception to use when a test fails, for the CLI did something unexpected, an
# expected output was missing, etc. This is just a convenience, the framework
# will still work if a different error is raised.
#
# For errors within the framework itself, use RuntimeError.
class TestError(Exception):
    pass

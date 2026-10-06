# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import json
import os
import re
from dataclasses import dataclass
from enum import Enum
from aptos_sdk.account_address import AccountAddress
NODE_PORT = 8080
METRICS_PORT = 9101
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
    account_address: AccountAddress
    network: Network


# This is an account that use for testing, for example to create it with the init
# account, send funds to it, etc. This is not the account created by the `aptos init`
# test. To get details about that account use get_account_info on the RunHelper.
OTHER_ACCOUNT_ONE = AccountInfo(
    private_key="0x37368b46ce665362562c6d1d4ec01a08c8644c488690df5a17e13ba163e20221",
    public_key="0x25caf00522e4d4664ec0a27166a69e8a32b5078959d0fc398da70d40d2893e8f",
    account_address=AccountAddress.from_str("0x585fc9f0f0c54183b039ffc770ca282ebd87307916c215a3e692f2f8e4305e82"),
    network=Network.DEVNET,
)


def build_image_name(image_repo_with_project: str, tag: str):
    # If no repo is specified, leave it that way. Otherwise make sure we have a slash
    # between the image repo and the image name.
    image_repo_with_project = image_repo_with_project.rstrip("/")
    if image_repo_with_project != "":
        image_repo_with_project = f"{image_repo_with_project}/"
    repository = f"{image_repo_with_project}tools"
    protected_tag = os.getenv("PROTECTED_IMAGE_TAG")
    raw_digests = os.getenv("PROTECTED_IMAGE_DIGESTS")
    if not protected_tag and not raw_digests:
        return f"{repository}:{tag}"
    if not protected_tag or not raw_digests:
        raise ValueError("PROTECTED_IMAGE_TAG and PROTECTED_IMAGE_DIGESTS must be set together")
    if str(tag) != protected_tag:
        return f"{repository}:{tag}"
    try:
        digests = json.loads(raw_digests)
    except json.JSONDecodeError as exc:
        raise ValueError("PROTECTED_IMAGE_DIGESTS is not valid JSON") from exc
    if not isinstance(digests, dict):
        raise ValueError("PROTECTED_IMAGE_DIGESTS must be an object")
    digest = digests.get(repository)
    if not isinstance(digest, str) or not re.fullmatch(r"sha256:[0-9a-f]{64}", digest):
        raise ValueError(f"Missing or invalid protected image digest for {repository}")
    return f"{repository}@{digest}"


# Exception to use when a test fails, for the CLI did something unexpected, an
# expected output was missing, etc. This is just a convenience, the framework
# will still work if a different error is raised.
#
# For errors within the framework itself, use RuntimeError.
class TestError(Exception):
    pass

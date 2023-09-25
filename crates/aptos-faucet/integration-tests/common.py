# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

from enum import Enum

NODE_PORT = 8080


class Network(Enum):
    DEVNET = "devnet"
    TESTNET = "testnet"

    def __str__(self):
        return self.value


class CustomNetwork(Network):
    def __init__(self, tag: str):
        self._tag = tag

    def __str__(self) -> str:
        return self._tag

    def tag(self) -> str:
        return self._tag


VALID_NETWORK_OPTIONS = [DEVNET, TESTNET, CUSTOM]


def network_from_str(str: str, tag: Optional[str]) -> Network:
    if str == DEVNET:
        return DevnetNetwork()
    elif str == TESTNET:
        return TestnetNetwork()
    else:
        if not tag:
            raise ValueError("--tag must be provided for custom network")
        return CustomNetwork(tag)


def build_image_name(image_repo_with_project: str, tag: str):
    # If no repo is specified, leave it that way. Otherwise make sure we have a slash
    # between the image repo and the image name.
    image_repo_with_project = image_repo_with_project.rstrip("/")
    if image_repo_with_project != "":
        image_repo_with_project = f"{image_repo_with_project}/"
    return f"{image_repo_with_project}tools:{tag}"

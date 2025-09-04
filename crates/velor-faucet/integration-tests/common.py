# Copyright © Velor Foundation
# SPDX-License-Identifier: Apache-2.0

from dataclasses import dataclass
from typing import Optional

NODE_PORT = 8080


DEVNET = "devnet"
TESTNET = "testnet"
CUSTOM = "custom"


@dataclass
class Network:
    def __str__(self) -> str:
        raise NotImplementedError()


class DevnetNetwork(Network):
    def __str__(self) -> str:
        return DEVNET

    def tag(self) -> str:
        return str(self)


class TestnetNetwork(Network):
    def __str__(self):
        return TESTNET

    def tag(self) -> str:
        return str(self)


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

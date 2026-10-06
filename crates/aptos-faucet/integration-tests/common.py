# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import json
import os
import re
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

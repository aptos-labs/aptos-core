# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

from enum import Enum

NODE_PORT = 8080


class Network(Enum):
    DEVNET = "devnet"
    TESTNET = "testnet"

    def __str__(self):
        return self.value


def build_image_name(image_repo_with_project: str, tag: str):
    return f"{image_repo_with_project}/tools:{tag}"

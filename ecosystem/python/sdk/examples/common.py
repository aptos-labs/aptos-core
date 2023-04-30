# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import os

# :!:>section_1
NODE_URL = os.getenv("APTOS_NODE_URL", "https://fullnode.devnet.aptoslabs.com/v1")
FAUCET_URL = os.getenv(
    "APTOS_FAUCET_URL",
    "https://faucet.devnet.aptoslabs.com",
)  # <:!:section_1

NODE_URL = os.getenv("APTOS_NODE_URL", "http://127.0.0.1:8080/v1")
FAUCET_URL = os.getenv(
    "APTOS_FAUCET_URL",
    "http://127.0.0.1:8081",
)  # <:!:section_1

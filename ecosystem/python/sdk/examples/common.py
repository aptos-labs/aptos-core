# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import os

# :!:>section_1
NODE_URL = os.getenv("APTOS_NODE_URL", "https://fullnode.testnet.aptoslabs.com/v1")
FAUCET_URL = os.getenv(
    "APTOS_FAUCET_URL",
    "https://faucet.testnet.aptoslabs.com",
)  # <:!:section_1

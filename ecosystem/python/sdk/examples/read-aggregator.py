# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

from aptos_sdk.client import RestClient

from .common import NODE_URL

if __name__ == "__main__":
    rest_client = RestClient(NODE_URL)
    total_apt = rest_client.aggregator_value(
        "0x1", "0x1::coin::CoinInfo<0x1::aptos_coin::AptosCoin>", ["supply"]
    )
    print(f"Total circulating APT: {total_apt}")
    rest_client.close()

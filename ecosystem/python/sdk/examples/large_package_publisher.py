# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
"""
This example depends on the MoonCoin.move module having already been published to the destination blockchain.
One method to do so is to use the CLI:
    * Acquire the Aptos CLI, see https://aptos.dev/cli-tools/aptos-cli-tool/install-aptos-cli
    * `python -m examples.your-coin ~/aptos-core/aptos-move/move-examples/moon_coin`.
    * Open another terminal and `aptos move compile --package-dir ~/aptos-core/aptos-move/move-examples/moon_coin --save-metadata --named-addresses MoonCoin=<Alice address from above step>`.
    * Return to the first terminal and press enter.
"""
import asyncio
import os
import sys

import aptos_sdk.cli as aptos_sdk_cli
from aptos_sdk.account import Account
from aptos_sdk.account_address import AccountAddress
from aptos_sdk.aptos_cli_wrapper import AptosCLIWrapper
from aptos_sdk.async_client import ClientConfig, FaucetClient, RestClient
from aptos_sdk.package_publisher import MODULE_ADDRESS, PackagePublisher

from .common import FAUCET_URL, NODE_URL


async def publish_large_packages(large_packages_dir) -> AccountAddress:
    rest_client = RestClient(NODE_URL)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)

    alice = Account.generate()
    await faucet_client.fund_account(alice.address(), 1_000_000_000)
    await aptos_sdk_cli.publish_package(
        large_packages_dir, {"large_packages": alice.address()}, alice, NODE_URL
    )
    return alice.address()


async def main(
    large_package_example_dir,
    large_packages_account: AccountAddress = MODULE_ADDRESS,
):
    client_config = ClientConfig()
    client_config.transaction_wait_in_seconds = 120
    client_config.max_gas_amount = 1_000_000
    rest_client = RestClient(NODE_URL, client_config)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)

    alice = Account.generate()
    req0 = faucet_client.fund_account(alice.address(), 1_000_000_000)
    req1 = faucet_client.fund_account(alice.address(), 1_000_000_000)
    req2 = faucet_client.fund_account(alice.address(), 1_000_000_000)
    await asyncio.gather(*[req0, req1, req2])
    alice_balance = await rest_client.account_balance(alice.address())
    print(f"Alice: {alice.address()} {alice_balance}")

    if AptosCLIWrapper.does_cli_exist():
        AptosCLIWrapper.compile_package(
            large_package_example_dir, {"large_package_example": alice.address()}
        )
    else:
        input("\nUpdate the module with Alice's address, compile, and press Enter.")

    publisher = PackagePublisher(rest_client)
    await publisher.publish_package_in_path(
        alice, large_package_example_dir, large_packages_account
    )


if __name__ == "__main__":
    if len(sys.argv) == 2:
        large_package_example_dir = sys.argv[1]
    else:
        large_package_example_dir = os.path.join(
            "..",
            "..",
            "..",
            "aptos-move",
            "move-examples",
            "large_packages",
            "large_package_example",
        )
    asyncio.run(main(large_package_example_dir))

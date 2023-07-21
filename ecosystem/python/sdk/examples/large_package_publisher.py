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

from aptos_sdk.account import Account
from aptos_sdk.async_client import ClientConfig, FaucetClient, RestClient
from aptos_sdk.package_publisher import PackagePublisher

from .common import FAUCET_URL, NODE_URL


async def main():
    client_config = ClientConfig()
    client_config.transaction_wait_in_seconds = 120
    client_config.max_gas_amount = 1_000_000
    rest_client = RestClient(NODE_URL, client_config)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)

    alice = Account.generate()
    await faucet_client.fund_account(alice.address(), 1_000_000_000)
    alice_balance = await rest_client.account_balance(alice.address())
    print(f"Alice: {alice.address()} {alice_balance}")
    input("\nPress Enter to continue...")

    package_build_path = "../../../aptos-move/move-examples/large_packages/large_package_example/build/LargePackageExample"
    module_directory = os.path.join(package_build_path, "bytecode_modules")
    module_paths = os.listdir(module_directory)
    modules = []
    for module_path in module_paths:
        module_path = os.path.join(module_directory, module_path)
        if not os.path.isfile(module_path) and not module_path.endswith(".mv"):
            continue
        with open(module_path, "rb") as f:
            module = f.read()
            modules.append(module)

    metadata_path = os.path.join(package_build_path, "package-metadata.bcs")
    with open(metadata_path, "rb") as f:
        metadata = f.read()

    publisher = PackagePublisher(rest_client)
    await publisher.publish_package_experimental(alice, metadata, modules)


if __name__ == "__main__":
    asyncio.run(main())

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""
This example depends on the hello_blockchain.move module having already been published to the destination blockchain.

One method to do so is to use the CLI:
    * Acquire the Aptos CLI
    * `cd ~`
    * `aptos init`
    * `cd ~/aptos-core/aptos-move/move-examples/hello_blockchain`
    * `aptos move publish --named-addresses hello_blockchain=${your_address_from_aptos_init}`
    * `python -m examples.hello-blockchain ${your_address_from_aptos_init}`
"""

import asyncio
import os
import sys
from typing import Any, Dict, Optional

from aptos_sdk.account import Account
from aptos_sdk.account_address import AccountAddress
from aptos_sdk.aptos_cli_wrapper import AptosCLIWrapper
from aptos_sdk.async_client import FaucetClient, ResourceNotFound, RestClient
from aptos_sdk.bcs import Serializer
from aptos_sdk.package_publisher import PackagePublisher
from aptos_sdk.transactions import (
    EntryFunction,
    TransactionArgument,
    TransactionPayload,
)

from .common import FAUCET_URL, NODE_URL


class HelloBlockchainClient(RestClient):
    async def get_message(
        self, contract_address: AccountAddress, account_address: AccountAddress
    ) -> Optional[Dict[str, Any]]:
        """Retrieve the resource message::MessageHolder::message"""
        try:
            return await self.account_resource(
                account_address, f"{contract_address}::message::MessageHolder"
            )
        except ResourceNotFound:
            return None

    async def set_message(
        self, contract_address: AccountAddress, sender: Account, message: str
    ) -> str:
        """Potentially initialize and set the resource message::MessageHolder::message"""

        payload = EntryFunction.natural(
            f"{contract_address}::message",
            "set_message",
            [],
            [TransactionArgument(message, Serializer.str)],
        )
        signed_transaction = await self.create_bcs_signed_transaction(
            sender, TransactionPayload(payload)
        )
        return await self.submit_bcs_transaction(signed_transaction)


async def publish_contract(package_dir: str) -> AccountAddress:
    contract_publisher = Account.generate()
    rest_client = HelloBlockchainClient(NODE_URL)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)
    await faucet_client.fund_account(contract_publisher.address(), 10_000_000)

    AptosCLIWrapper.compile_package(
        package_dir, {"hello_blockchain": contract_publisher.address()}
    )

    module_path = os.path.join(
        package_dir, "build", "Examples", "bytecode_modules", "message.mv"
    )
    with open(module_path, "rb") as f:
        module = f.read()

    metadata_path = os.path.join(
        package_dir, "build", "Examples", "package-metadata.bcs"
    )
    with open(metadata_path, "rb") as f:
        metadata = f.read()

    package_publisher = PackagePublisher(rest_client)
    txn_hash = await package_publisher.publish_package(
        contract_publisher, metadata, [module]
    )
    await rest_client.wait_for_transaction(txn_hash)

    await rest_client.close()

    return contract_publisher.address()


async def main(contract_address: AccountAddress):
    alice = Account.generate()
    bob = Account.generate()

    print("\n=== Addresses ===")
    print(f"Alice: {alice.address()}")
    print(f"Bob: {bob.address()}")

    rest_client = HelloBlockchainClient(NODE_URL)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)

    alice_fund = faucet_client.fund_account(alice.address(), 10_000_000)
    bob_fund = faucet_client.fund_account(bob.address(), 10_000_000)
    await asyncio.gather(*[alice_fund, bob_fund])

    alice_balance = rest_client.account_balance(alice.address())
    bob_balance = rest_client.account_balance(bob.address())
    [alice_balance, bob_balance] = await asyncio.gather(*[alice_balance, bob_balance])

    print("\n=== Initial Balances ===")
    print(f"Alice: {alice_balance}")
    print(f"Bob: {bob_balance}")

    print("\n=== Testing Alice ===")
    message = await rest_client.get_message(contract_address, alice.address())
    print(f"Initial value: {message}")
    print('Setting the message to "Hello, Blockchain"')
    txn_hash = await rest_client.set_message(
        contract_address, alice, "Hello, Blockchain"
    )
    await rest_client.wait_for_transaction(txn_hash)

    message = await rest_client.get_message(contract_address, alice.address())
    print(f"New value: {message}")

    print("\n=== Testing Bob ===")
    message = await rest_client.get_message(contract_address, bob.address())
    print(f"Initial value: {message}")
    print('Setting the message to "Hello, Blockchain"')
    txn_hash = await rest_client.set_message(contract_address, bob, "Hello, Blockchain")
    await rest_client.wait_for_transaction(txn_hash)

    message = await rest_client.get_message(contract_address, bob.address())
    print(f"New value: {message}")

    await rest_client.close()


if __name__ == "__main__":
    assert len(sys.argv) == 2, "Expecting the contract address"
    contract_address = sys.argv[1]

    asyncio.run(main(AccountAddress.from_str(contract_address)))

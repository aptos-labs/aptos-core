# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

"""
This example depends on the MoonCoin.move module having already been published to the destination blockchain.

One method to do so is to use the CLI:
    * Acquire the Aptos CLI, see https://aptos.dev/cli-tools/aptos-cli-tool/install-aptos-cli
    * `python -m examples.your-coin ~/aptos-core/aptos-move/move-examples/moon_coin`.
    * Open another terminal and `aptos move compile --package-dir ~/aptos-core/aptos-move/move-examples/moon_coin --save-metadata --named-addresses MoonCoin=<Alice address from above step>`.
    * Return to the first terminal and press enter.
"""

import os
import sys

from aptos_sdk.account import Account
from aptos_sdk.account_address import AccountAddress
from aptos_sdk.bcs import Serializer
from aptos_sdk.client import FaucetClient, RestClient
from aptos_sdk.transactions import (
    EntryFunction,
    TransactionArgument,
    TransactionPayload,
)
from aptos_sdk.type_tag import StructTag, TypeTag

from .common import FAUCET_URL, NODE_URL


class CoinClient(RestClient):
    def register_coin(self, coin_address: AccountAddress, sender: Account) -> str:
        """Register the receiver account to receive transfers for the new coin."""

        payload = EntryFunction.natural(
            "0x1::managed_coin",
            "register",
            [TypeTag(StructTag.from_str(
                f"{coin_address}::moon_coin::MoonCoin"))],
            [],
        )
        signed_transaction = self.create_single_signer_bcs_transaction(
            sender, TransactionPayload(payload)
        )
        return self.submit_bcs_transaction(signed_transaction)

    def mint_coin(
        self, minter: Account, receiver_address: AccountAddress, amount: int
    ) -> str:
        """Mints the newly created coin to a specified receiver address."""

        payload = EntryFunction.natural(
            "0x1::managed_coin",
            "mint",
            [TypeTag(StructTag.from_str(
                f"{minter.address()}::moon_coin::MoonCoin"))],
            [
                TransactionArgument(receiver_address, Serializer.struct),
                TransactionArgument(amount, Serializer.u64),
            ],
        )
        signed_transaction = self.create_single_signer_bcs_transaction(
            minter, TransactionPayload(payload)
        )
        return self.submit_bcs_transaction(signed_transaction)

    def get_balance(
        self,
        coin_address: AccountAddress,
        account_address: AccountAddress,
    ) -> str:
        """Returns the coin balance of the given account"""

        balance = self.account_resource(
            account_address,
            f"0x1::coin::CoinStore<{coin_address}::moon_coin::MoonCoin>",
        )
        return balance["data"]["coin"]["value"]


if __name__ == "__main__":
    assert (
        len(sys.argv) == 2
    ), "Expecting an argument that points to the moon_coin directory."

    alice = Account.generate()
    bob = Account.generate()

    print("\n=== Addresses ===")
    print(f"Alice: {alice.address()}")
    print(f"Bob: {bob.address()}")

    rest_client = CoinClient(NODE_URL)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)

    faucet_client.fund_account(alice.address(), 20_000)
    faucet_client.fund_account(bob.address(), 20_000)

    input("\nUpdate the module with Alice's address, compile, and press enter.")

    #:!:>publish
    moon_coin_path = sys.argv[1]
    module_path = os.path.join(
        moon_coin_path, "build", "Examples", "bytecode_modules", "moon_coin.mv"
    )
    with open(module_path, "rb") as f:
        module = f.read()

    metadata_path = os.path.join(
        moon_coin_path, "build", "Examples", "package-metadata.bcs"
    )
    with open(metadata_path, "rb") as f:
        metadata = f.read()

    print("\nPublishing MoonCoin package.")
    txn_hash = rest_client.publish_package(alice, metadata, [module])
    rest_client.wait_for_transaction(txn_hash)
    # <:!:publish

    print("\nBob registers the newly created coin so he can receive it from Alice.")
    txn_hash = rest_client.register_coin(alice.address(), bob)
    rest_client.wait_for_transaction(txn_hash)
    print(
        f"Bob's initial MoonCoin balance: {rest_client.get_balance(alice.address(), bob.address())}."
    )

    print("Alice mints Bob some of the new coin.")
    txn_hash = rest_client.mint_coin(alice, bob.address(), 100)
    rest_client.wait_for_transaction(txn_hash)
    print(
        f"Bob's updated MoonCoin balance: {rest_client.get_balance(alice.address(), bob.address())}."
    )

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

"""
This example depends on the HelloBlockchain.move module having already been published to the destination blockchain.

One method to do so is to use the CLI:
    * Acquire the Aptos CLI
    * `cd ~`
    * `aptos init`
    * `cd ~/aptos-core/aptos-move/move-examples/moon_coin
    * `aptos move publish --named-address MoonCoinType=${your_address_from_aptos_init}`
    * Copy the private key from `cat ~/.aptos/config.yaml`
    * `python -m examples.your-coin ${private_key_from_aptos_init}`
"""

import sys
from typing import Optional

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
    def initialize_coin(self, sender: Account) -> Optional[str]:
        """Initialize a new coin with the given coin type."""

        payload = EntryFunction.natural(
            "0x1::managed_coin",
            "initialize",
            [TypeTag(StructTag.from_str(f"{sender.address()}::moon_coin::MoonCoin"))],
            [
                TransactionArgument("Moon Coin", Serializer.str),
                TransactionArgument("MOON", Serializer.str),
                TransactionArgument(6, Serializer.u64),
                TransactionArgument(False, Serializer.bool),
            ],
        )
        signed_transaction = self.create_single_signer_bcs_transaction(
            sender, TransactionPayload(payload)
        )
        return self.submit_bcs_transaction(signed_transaction)

    def register_coin(self, coin_address: AccountAddress, sender: Account) -> str:
        """Register the receiver account to receive transfers for the new coin."""

        print(f"{coin_address}::moon_coin::MoonCoin")
        payload = EntryFunction.natural(
            "0x1::managed_coin",
            "register",
            [TypeTag(StructTag.from_str(f"{coin_address}::moon_coin::MoonCoin"))],
            [],
        )
        signed_transaction = self.create_single_signer_bcs_transaction(
            sender, TransactionPayload(payload)
        )
        return self.submit_bcs_transaction(signed_transaction)

    def mint_coin(
        self, minter: Account, receiver_address: AccountAddress, amount: int
    ) -> str:
        """Register the receiver account to receive transfers for the new coin."""

        payload = EntryFunction.natural(
            "0x1::managed_coin",
            "mint",
            [TypeTag(StructTag.from_str(f"{minter.address()}::moon_coin::MoonCoin"))],
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
        print(balance)
        return balance["data"]["coin"]["value"]


if __name__ == "__main__":
    assert (
        len(sys.argv) == 2
    ), "Expecting the private key for the account that published the contract"
    key = sys.argv[1]

    alice = Account.load_key(key)
    bob = Account.generate()

    print("\n=== Addresses ===")
    print(f"Alice: {alice.address()}")
    print(f"Bob: {bob.address()}")

    rest_client = CoinClient(NODE_URL)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)

    faucet_client.fund_account(alice.address(), 20_000)
    faucet_client.fund_account(bob.address(), 20_000)

    print("Alice will initialize the new coin")
    txn_hash = rest_client.initialize_coin(alice)
    rest_client.wait_for_transaction(txn_hash)

    print("Bob registers the newly created coin so he can receive it from Alice")
    txn_hash = rest_client.register_coin(alice.address(), bob)
    rest_client.wait_for_transaction(txn_hash)
    print(
        f"Bob's updated MoonCoinType balance: {rest_client.get_balance(alice.address(), bob.address())}"
    )

    print("Alice mints Bob some of the new coin")
    txn_hash = rest_client.mint_coin(alice, bob.address(), 100)
    rest_client.wait_for_transaction(txn_hash)
    print(
        f"Bob's updated MoonCoinType balance: {rest_client.get_balance(alice.address(), bob.address())}"
    )

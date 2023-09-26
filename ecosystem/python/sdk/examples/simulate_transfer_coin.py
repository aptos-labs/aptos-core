# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import asyncio
import json

from aptos_sdk.account import Account
from aptos_sdk.async_client import FaucetClient, RestClient
from aptos_sdk.bcs import Serializer
from aptos_sdk.transactions import (
    EntryFunction,
    TransactionArgument,
    TransactionPayload,
)
from aptos_sdk.type_tag import StructTag, TypeTag

from .common import FAUCET_URL, NODE_URL


async def main():
    rest_client = RestClient(NODE_URL)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)  # <:!:section_1

    alice = Account.generate()
    bob = Account.generate()

    print("\n=== Addresses ===")
    print(f"Alice: {alice.address()}")
    print(f"Bob: {bob.address()}")

    await faucet_client.fund_account(alice.address(), 100_000_000)

    payload = EntryFunction.natural(
        "0x1::coin",
        "transfer",
        [TypeTag(StructTag.from_str("0x1::aptos_coin::AptosCoin"))],
        [
            TransactionArgument(bob.address(), Serializer.struct),
            TransactionArgument(100_000, Serializer.u64),
        ],
    )
    transaction = await rest_client.create_bcs_transaction(
        alice, TransactionPayload(payload)
    )

    print("\n=== Simulate before creating Bob's Account ===")
    output = await rest_client.simulate_transaction(transaction, alice)
    assert output[0]["vm_status"] != "Executed successfully", "This shouldn't succeed"
    print(json.dumps(output, indent=4, sort_keys=True))

    print("\n=== Simulate after creating Bob's Account ===")
    await faucet_client.fund_account(bob.address(), 0)
    output = await rest_client.simulate_transaction(transaction, alice)
    assert output[0]["vm_status"] == "Executed successfully", "This should succeed"
    print(json.dumps(output, indent=4, sort_keys=True))

    await rest_client.close()


if __name__ == "__main__":
    asyncio.run(main())

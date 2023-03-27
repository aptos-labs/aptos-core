# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import asyncio

from aptos_sdk.account import Account
from aptos_sdk.async_client import FaucetClient, RestClient

from .common import FAUCET_URL, NODE_URL


async def main():
    # :!:>section_1
    rest_client = RestClient(NODE_URL)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)  # <:!:section_1

    # :!:>section_2
    alice = Account.generate()
    bob = Account.generate()  # <:!:section_2

    print("\n=== Addresses ===")
    print(f"Alice: {alice.address()}")
    print(f"Bob: {bob.address()}")

    # :!:>section_3
    alice_fund = faucet_client.fund_account(alice.address(), 100_000_000)
    bob_fund = faucet_client.fund_account(bob.address(), 0)  # <:!:section_3
    await asyncio.gather(*[alice_fund, bob_fund])

    print("\n=== Initial Balances ===")
    # :!:>section_4
    alice_balance = rest_client.account_balance(alice.address())
    bob_balance = rest_client.account_balance(bob.address())
    [alice_balance, bob_balance] = await asyncio.gather(*[alice_balance, bob_balance])
    print(f"Alice: {alice_balance}")
    print(f"Bob: {bob_balance}")  # <:!:section_4

    # Have Alice give Bob 1_000 coins
    # :!:>section_5
    txn_hash = await rest_client.transfer(alice, bob.address(), 1_000)  # <:!:section_5
    # :!:>section_6
    await rest_client.wait_for_transaction(txn_hash)  # <:!:section_6

    print("\n=== Intermediate Balances ===")
    alice_balance = rest_client.account_balance(alice.address())
    bob_balance = rest_client.account_balance(bob.address())
    [alice_balance, bob_balance] = await asyncio.gather(*[alice_balance, bob_balance])
    print(f"Alice: {alice_balance}")
    print(f"Bob: {bob_balance}")  # <:!:section_4

    # Have Alice give Bob another 1_000 coins using BCS
    txn_hash = await rest_client.bcs_transfer(alice, bob.address(), 1_000)
    await rest_client.wait_for_transaction(txn_hash)

    print("\n=== Final Balances ===")
    alice_balance = rest_client.account_balance(alice.address())
    bob_balance = rest_client.account_balance(bob.address())
    [alice_balance, bob_balance] = await asyncio.gather(*[alice_balance, bob_balance])
    print(f"Alice: {alice_balance}")
    print(f"Bob: {bob_balance}")

    await rest_client.close()


if __name__ == "__main__":
    asyncio.run(main())

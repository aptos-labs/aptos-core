# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import asyncio
import os

from aptos_sdk.account import Account
from aptos_sdk.async_client import FaucetClient, RestClient
from aptos_sdk.transactions import Script, ScriptArgument, TransactionPayload

from .common import FAUCET_URL, NODE_URL


async def main():
    rest_client = RestClient(NODE_URL)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)

    alice = Account.generate()
    bob = Account.generate()
    carol = Account.generate()
    david = Account.generate()

    print("\n=== Addresses ===")
    print(f"Alice: {alice.address()}")
    print(f"Bob: {bob.address()}")
    print(f"Carol: {carol.address()}")
    print(f"David: {david.address()}")

    alice_fund = faucet_client.fund_account(alice.address(), 100_000_000)
    bob_fund = faucet_client.fund_account(bob.address(), 100_000_000)
    carol_fund = faucet_client.fund_account(carol.address(), 0)
    david_fund = faucet_client.fund_account(david.address(), 0)
    await asyncio.gather(*[alice_fund, bob_fund, carol_fund, david_fund])

    alice_balance = rest_client.account_balance(alice.address())
    bob_balance = rest_client.account_balance(bob.address())
    carol_balance = rest_client.account_balance(carol.address())
    david_balance = rest_client.account_balance(david.address())
    [alice_balance, bob_balance, carol_balance, david_balance] = await asyncio.gather(
        *[alice_balance, bob_balance, carol_balance, david_balance]
    )

    print("\n=== Initial Balances ===")
    print(f"Alice: {alice_balance}")
    print(f"Bob: {bob_balance}")
    print(f"Carol: {carol_balance}")
    print(f"David: {david_balance}")

    path = os.path.dirname(__file__)
    filepath = os.path.join(path, "two_by_two_transfer.mv")
    with open(filepath, mode="rb") as file:
        code = file.read()

    script_arguments = [
        ScriptArgument(ScriptArgument.U64, 100),
        ScriptArgument(ScriptArgument.U64, 200),
        ScriptArgument(ScriptArgument.ADDRESS, carol.address()),
        ScriptArgument(ScriptArgument.ADDRESS, david.address()),
        ScriptArgument(ScriptArgument.U64, 50),
    ]

    payload = TransactionPayload(Script(code, [], script_arguments))
    txn = await rest_client.create_multi_agent_bcs_transaction(alice, [bob], payload)
    txn_hash = await rest_client.submit_bcs_transaction(txn)
    await rest_client.wait_for_transaction(txn_hash)

    alice_balance = rest_client.account_balance(alice.address())
    bob_balance = rest_client.account_balance(bob.address())
    carol_balance = rest_client.account_balance(carol.address())
    david_balance = rest_client.account_balance(david.address())
    [alice_balance, bob_balance, carol_balance, david_balance] = await asyncio.gather(
        *[alice_balance, bob_balance, carol_balance, david_balance]
    )

    print("\n=== Final Balances ===")
    print(f"Alice: {alice_balance}")
    print(f"Bob: {bob_balance}")
    print(f"Carol: {carol_balance}")
    print(f"David: {david_balance}")

    await rest_client.close()


if __name__ == "__main__":
    asyncio.run(main())

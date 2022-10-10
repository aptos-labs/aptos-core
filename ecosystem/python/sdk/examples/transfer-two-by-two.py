# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

import os

from aptos_sdk.account import Account
from aptos_sdk.client import FaucetClient, RestClient
from aptos_sdk.transactions import Script, ScriptArgument, TransactionPayload

from .common import FAUCET_URL, NODE_URL

if __name__ == "__main__":
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

    faucet_client.fund_account(alice.address(), 100_000_000)
    faucet_client.fund_account(bob.address(), 100_000_000)
    faucet_client.fund_account(carol.address(), 0)
    faucet_client.fund_account(david.address(), 0)

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
    txn = rest_client.create_multi_agent_bcs_transaction(alice, [bob], payload)
    txn_hash = rest_client.submit_bcs_transaction(txn)
    rest_client.wait_for_transaction(txn_hash)

    print("\n=== Initial Balances ===")
    print(f"Alice: {rest_client.account_balance(alice.address())}")
    print(f"Bob: {rest_client.account_balance(bob.address())}")
    print(f"Carol: {rest_client.account_balance(carol.address())}")
    print(f"David: {rest_client.account_balance(david.address())}")

    print("\n=== Final Balances ===")
    print(f"Alice: {rest_client.account_balance(alice.address())}")
    print(f"Bob: {rest_client.account_balance(bob.address())}")
    print(f"Carol: {rest_client.account_balance(carol.address())}")
    print(f"David: {rest_client.account_balance(david.address())}")

    rest_client.close()

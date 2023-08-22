# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import argparse
import asyncio

from aptos_sdk.account import Account
from aptos_sdk.async_client import FaucetClient, ResourceNotFound, RestClient

from .common import FAUCET_URL, NODE_URL


def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument("auth_token")
    return parser.parse_args()


async def main():
    args = parse_args()

    alice = Account.generate()

    print("=== Addresses ===")
    print(f"Alice: {alice.address()}")

    rest_client = RestClient(NODE_URL)
    faucet_client = FaucetClient(FAUCET_URL, rest_client, auth_token=args.auth_token)

    try:
        balance = await rest_client.account_balance(alice.address())
    except ResourceNotFound:
        balance = 0

    print("\n=== Balance before ===")
    print(f"Alice: {balance}")

    await faucet_client.fund_account(alice.address(), 20_000_000)

    balance = await rest_client.account_balance(alice.address())

    print("\n=== Balance after ===")
    print(f"Alice: {balance}")


if __name__ == "__main__":
    asyncio.run(main())

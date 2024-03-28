# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
"""
This demonstrates how to use the simple inscriptions package.
"""
import asyncio
from urllib.request import urlretrieve

import aptos_sdk.cli as aptos_sdk_cli
import aptos_sdk.inscriptions_as_state_client as inscriptions
from aptos_sdk.account import Account
from aptos_sdk.account_address import AccountAddress
from aptos_sdk.aptos_token_client import AptosTokenClient
from aptos_sdk.async_client import FaucetClient, RestClient
from aptos_sdk.inscriptions_as_state_client import InscriptionsClient

from .common import FAUCET_URL, NODE_URL


async def publish_inscriptions(inscriptions_dir: str) -> AccountAddress:
    rest_client = RestClient(NODE_URL)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)

    alice = Account.generate()
    await faucet_client.fund_account(alice.address(), 1_000_000_000)
    await aptos_sdk_cli.publish_package(
        inscriptions_dir, {"inscriptions": alice.address()}, alice, NODE_URL
    )
    return alice.address()


async def main(inscriptions_account: AccountAddress = inscriptions.MODULE_ADDRESS):
    inscriptions.set_module_address(inscriptions_account)

    rest_client = RestClient(NODE_URL)
    aptos_token_client = AptosTokenClient(rest_client)
    inscriptions_client = InscriptionsClient(aptos_token_client)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)

    alice = Account.generate()
    await faucet_client.fund_account(alice.address(), 1_000_000_000)
    await rest_client.account_balance(alice.address())

    collection_name = "Immutable Inscriptions Demo"

    txn_hash = await inscriptions_client.create_collection(
        alice,
        "Behold the power of Inscriptions on Aptos",
        100,
        collection_name,
        0,
        1,
        alice.address(),
        "",
    )
    await rest_client.wait_for_transaction(txn_hash)

    path, headers = urlretrieve("https://aptos.dev/img/nyan.jpeg")
    with open(path, "rb") as file:
        data = file.read()

    txn_hash = await inscriptions_client.mint_token(
        alice,
        collection_name,
        data,
        "Nyan, a cat for the next generation",
        "Nyan",
        "https://aptos.dev/img/nyan.jpeg",
    )
    await rest_client.wait_for_transaction(txn_hash)

    tokens = await aptos_token_client.tokens_minted_from_transaction(txn_hash)
    assert len(tokens) == 1
    value = await inscriptions_client.read_object(tokens[0])
    inscription_events = await inscriptions_client.inscriptions_from_transaction(
        txn_hash
    )
    assert len(inscription_events) == 1
    assert value.resources[inscriptions.InscriptionData].data == data
    print(f"Inscription created at {tokens[0]}")

    rest_client.close()


if __name__ == "__main__":
    asyncio.run(main())

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
import asyncio

from aptos_sdk.account import Account
from aptos_sdk.account_address import AccountAddress
from aptos_sdk.aptos_token_client import AptosTokenClient, Property, PropertyMap
from aptos_sdk.async_client import FaucetClient, RestClient

# from .common import FAUCET_URL, NODE_URL


async def main():
    # :!:>section 1
    rest_client = RestClient("http://0.0.0.0:8080/v1")
    # RestClient(NODE_URL)
    faucet_client = FaucetClient("http://0.0.0.0:8081", rest_client)
    # FaucetClient(FAUCET_URL, rest_client)
    token_client = AptosTokenClient(rest_client)  # <:!:section_1

    # :!:>section 2
    alice = Account.generate()
    bob = Account.generate()  # <:!:section_2

    collection_name = "Alice's"
    token_name = "Alice's first token"

    print("\n=== Addresses ===")
    print(f"Alice: {alice.address()}")
    print(f"Bob: {bob.address()}")

    # :!:>section 3
    alice_fund = faucet_client.fund_account(alice.address(), 100_000_000)
    bob_fund = faucet_client.fund_account(bob.address(), 100_000_000)
    await asyncio.gather(*[alice_fund, bob_fund])  # <:!:section_3

    print("\n=== Initial Coin Balances ===")
    alice_balance = rest_client.account_balance(alice.address())
    bob_balance = rest_client.account_balance(bob.address())
    [alice_balance, bob_balance] = await asyncio.gather(*[alice_balance, bob_balance])
    print(f"Alice: {alice_balance}")
    print(f"Bob: {bob_balance}")

    print("\n=== Creating Collection and Minting Token ===")

    # :!:>section_4
    txn_hash = await token_client.create_collection(
        # creator account
        alice,
        # collection description
        "Alice's simple collection",
        # max supply
        1,
        # collection name
        collection_name,
        "https://aptos.dev",
        # if the collection description is mutable
        True,
        # if the collection royalty is mutable
        True,
        #  if the collection uri is mutable
        True,
        # if the token description is mutable
        True,
        # if the token name is mutable
        True,
        # if the token properties are mutable
        True,
        # if the  token uri is mutable
        True,
        # if the token is burnable by the creator
        True,
        # if the token is freezable by the creator
        True,
        # royalty numerator
        0,
        # royalty denominator
        1,
    )  # <:!:section_4
    await rest_client.wait_for_transaction(txn_hash)

    # This is a hack, once we add support for reading events or indexer, this will be easier
    resp = await rest_client.account_resource(alice.address(), "0x1::account::Account")
    creation_num = int(resp["data"]["guid_creation_num"])

    # :!:>section_5
    txn_hash = await token_client.mint_token(
        alice,
        collection_name,
        "Alice's simple token",
        token_name,
        "https://aptos.dev/img/nyan.jpeg",
        PropertyMap([Property.string("string", "string value")]),
    )  # <:!:section_5
    await rest_client.wait_for_transaction(txn_hash)

    # :!:>section_6
    collection_addr = AccountAddress.for_named_collection(
        alice.address(), collection_name
    )
    collection_data = await token_client.read_object(collection_addr)
    print(f"Alice's collection: {collection_data}")

    token_addr = AccountAddress.for_guid_object(alice.address(), creation_num)
    token_data = await token_client.read_object(token_addr)
    print(f"Alice's token data: {token_data}")  # <:!:section_6

    # :!:>section_7
    print(f"\n=== Transferring the token to Bob ===")

    txn_hash = await token_client.transfer_token(
        alice,
        token_addr,
        bob.address(),
    )
    # <:!:section_7
    await rest_client.wait_for_transaction(txn_hash)
    token_data = await token_client.read_object(token_addr)
    print(f"Bob's token: {token_data}")

    await rest_client.close()


if __name__ == "__main__":
    asyncio.run(main())

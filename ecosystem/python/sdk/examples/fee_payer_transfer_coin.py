# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import asyncio

from aptos_sdk.account import Account
from aptos_sdk.async_client import ApiError, FaucetClient, ResourceNotFound, RestClient
from aptos_sdk.authenticator import Authenticator, FeePayerAuthenticator
from aptos_sdk.bcs import Serializer
from aptos_sdk.transactions import (
    EntryFunction,
    FeePayerRawTransaction,
    SignedTransaction,
    TransactionArgument,
    TransactionPayload,
)

from .common import FAUCET_URL, NODE_URL


async def main():
    # :!:>section_1
    rest_client = RestClient(NODE_URL)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)  # <:!:section_1

    # :!:>section_2
    alice = Account.generate()
    bob = Account.generate()
    sponsor = Account.generate()  # <:!:section_2

    print("\n=== Addresses ===")
    print(f"Alice: {alice.address()}")
    print(f"Bob: {bob.address()}")
    print(f"Sponsor: {sponsor.address()}")

    # :!:>section_3
    await faucet_client.fund_account(sponsor.address(), 100_000_000)  # <:!:section_3

    print("\n=== Initial Data ===")
    # :!:>section_4
    try:
        alice_sequence_number = await rest_client.account_sequence_number(
            alice.address()
        )
        raise Exception("This should never be accessed")
    except ApiError:
        alice_sequence_number = 0

    try:
        bob_balance = await rest_client.account_balance(bob.address())
        raise Exception("This should never be accessed")
    except ResourceNotFound:
        bob_balance = None

    sponsor_balance = await rest_client.account_balance(sponsor.address())
    print(f"Alice sequence number: {alice_sequence_number}")
    print(f"Bob balance: {bob_balance}")
    print(f"Sponsor balance: {sponsor_balance}")  # <:!:section_4

    # Have Alice give Bob 1_000 coins via a sponsored transaction
    # :!:>section_5
    transaction_arguments = [
        TransactionArgument(bob.address(), Serializer.struct),
    ]

    payload = EntryFunction.natural(
        "0x1::aptos_account",
        "create_account",
        [],
        transaction_arguments,
    )
    raw_transaction = await rest_client.create_bcs_transaction(
        alice, TransactionPayload(payload), alice_sequence_number
    )
    fee_payer_transaction = FeePayerRawTransaction(raw_transaction, [], None)
    sender_authenticator = alice.sign_transaction(fee_payer_transaction)
    fee_payer_transaction = FeePayerRawTransaction(
        raw_transaction, [], sponsor.address()
    )
    sponsor_authenticator = sponsor.sign_transaction(fee_payer_transaction)
    fee_payer_authenticator = FeePayerAuthenticator(
        sender_authenticator, [], (sponsor.address(), sponsor_authenticator)
    )
    signed_transaction = SignedTransaction(
        raw_transaction, Authenticator(fee_payer_authenticator)
    )
    txn_hash = await rest_client.submit_bcs_transaction(
        signed_transaction
    )  # <:!:section_5
    # :!:>section_6
    await rest_client.wait_for_transaction(txn_hash)  # <:!:section_6

    print("\n=== Final Data ===")
    alice_sequence_number = rest_client.account_sequence_number(alice.address())
    bob_balance = rest_client.account_balance(bob.address())
    sponsor_balance = rest_client.account_balance(sponsor.address())
    [alice_sequence_number, bob_balance, sponsor_balance] = await asyncio.gather(
        *[alice_sequence_number, bob_balance, sponsor_balance]
    )
    print(f"Alice sequence number: {alice_sequence_number}")
    print(f"Bob balance: {bob_balance}")
    print(f"Sponsor balance: {sponsor_balance}")  # <:!:section_4

    await rest_client.close()


if __name__ == "__main__":
    asyncio.run(main())

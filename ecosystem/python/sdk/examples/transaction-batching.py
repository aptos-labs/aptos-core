# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import asyncio
import logging
import time

from aptos_sdk.account import Account
from aptos_sdk.account_address import AccountAddress
from aptos_sdk.async_client import ClientConfig, FaucetClient, RestClient
from aptos_sdk.authenticator import Authenticator, Ed25519Authenticator
from aptos_sdk.bcs import Serializer
from aptos_sdk.transactions import (
    EntryFunction,
    RawTransaction,
    SignedTransaction,
    TransactionArgument,
    TransactionPayload,
)
from aptos_sdk.type_tag import StructTag, TypeTag

from .common import FAUCET_URL, NODE_URL


async def main():
    client_config = ClientConfig()
    # Toggle to benchmark
    client_config.http2 = False
    client_config.http2 = True
    rest_client = RestClient(NODE_URL, client_config)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)

    num_accounts = 5
    read_amplification = 1000
    first_pass = 100
    start = time.time()

    print("Starting...")

    accounts = []
    recipient_accounts = []
    for _ in range(num_accounts):
        accounts.append(Account.generate())
        recipient_accounts.append(Account.generate())

    last = time.time()
    print(f"Accounts generated at {last - start}")

    funds = []
    for account in accounts:
        funds.append(faucet_client.fund_account(account.address(), 100_000_000))
    for account in recipient_accounts:
        funds.append(faucet_client.fund_account(account.address(), 0))
    await asyncio.gather(*funds)

    print(f"Funded accounts at {time.time() - start} {time.time() - last}")
    last = time.time()

    balances = []
    for _ in range(read_amplification):
        for account in accounts:
            balances.append(rest_client.account_balance(account.address()))
    await asyncio.gather(*balances)

    print(f"Accounts checked at {time.time() - start} {time.time() - last}")
    last = time.time()

    account_sequence_numbers = []
    await_account_sequence_numbers = []
    for account in accounts:
        account_sequence_number = AccountSequenceNumber(rest_client, account.address())
        await_account_sequence_numbers.append(account_sequence_number.initialize())
        account_sequence_numbers.append(account_sequence_number)
    await asyncio.gather(*await_account_sequence_numbers)

    print(f"Accounts initialized at {time.time() - start} {time.time() - last}")
    last = time.time()

    txn_hashes = []
    for _ in range(first_pass):
        for idx in range(num_accounts):
            sender = accounts[idx]
            recipient = recipient_accounts[idx].address()
            sequence_number = await account_sequence_numbers[idx].next_sequence_number()
            txn_hash = transfer(rest_client, sender, recipient, sequence_number, 1)
            txn_hashes.append(txn_hash)
    txn_hashes = await asyncio.gather(*txn_hashes)

    print(f"Transactions submitted at {time.time() - start} {time.time() - last}")
    last = time.time()

    wait_for = []
    for txn_hash in txn_hashes:
        wait_for.append(account_sequence_number.synchronize())
    await asyncio.gather(*wait_for)

    print(f"Transactions committed at {time.time() - start} {time.time() - last}")
    last = time.time()

    await rest_client.close()


class AccountSequenceNumber:
    """
    A managed wrapper around sequence numbers that implements the trivial flow control used by the
    Aptos faucet:
    * Submit up to 50 transactions per account in parallel with a timeout of 20 seconds
    * If local assumes 50 are in flight, determine the actual committed state from the network
    * If there are less than 50 due to some being committed, adjust the window
    * If 50 are in flight Wait .1 seconds before re-evaluating
    * If ever waiting more than 30 seconds restart the sequence number to the current on-chain state

    Assumptions:
    * Accounts are expected to be managed by a single AccountSequenceNumber and not used otherwise.
    * They are initialized to the current on-chain state, so if there are already transactions in flight, they make take some time to reset.
    * Accounts are automatically initialized if not explicitly
    *
    """

    client: RestClient
    account: AccountAddress
    last_committed_number: int
    current_number: int
    maximum_in_flight: int = 50
    lock = asyncio.Lock
    sleep_time = 0.01
    maximum_wait_time = 30

    def __init__(self, client: RestClient, account: AccountAddress):
        self.client = client
        self.account = account
        self.last_uncommitted_number = None
        self.current_number = None
        self.lock = asyncio.Lock()

    async def next_sequence_number(self) -> int:
        await self.lock.acquire()
        try:
            if self.last_uncommitted_number is None or self.current_number is None:
                await self.initialize()

            if (
                self.current_number - self.last_uncommitted_number
                >= self.maximum_in_flight
            ):
                await self.__update()

                start_time = time.time()
                while (
                    self.last_uncommitted_number - self.current_number
                    >= self.maximum_in_flight
                ):
                    asyncio.sleep(self.sleep_time)
                    if time.time() - start_time > self.maximum_wait_time:
                        logging.warn(
                            f"Waited over 30 seconds for a transaction to commit, resyncing {self.account.address()}"
                        )
                        await self.__initialize()
                    else:
                        await self.__update()

            next_number = self.current_number
            self.current_number += 1
        finally:
            self.lock.release()

        return next_number

    async def initialize(self):
        self.current_number = await self.__current_sequence_number()
        self.last_uncommitted_number = self.current_number

    async def synchronize(self):
        if self.last_uncommitted_number == self.current_number:
            return

        await self.__update()
        start_time = time.time()
        while self.last_uncommitted_number != self.current_number:
            if time.time() - start_time > self.maximum_wait_time:
                logging.warn(
                    f"Waited over 30 seconds for a transaction to commit, resyncing {self.account.address()}"
                )
                await self.__initialize()
            else:
                await asyncio.sleep(self.sleep_time)
                await self.__update()

    async def __update(self):
        self.last_uncommitted_number = await self.__current_sequence_number()
        return self.last_uncommitted_number

    async def __current_sequence_number(self) -> int:
        return await self.client.account_sequence_number(self.account)


async def transfer(
    client: RestClient,
    sender: Account,
    recipient: AccountAddress,
    sequence_number: int,
    amount: int,
):
    transaction_arguments = [
        TransactionArgument(recipient, Serializer.struct),
        TransactionArgument(amount, Serializer.u64),
    ]
    payload = EntryFunction.natural(
        "0x1::coin",
        "transfer",
        [TypeTag(StructTag.from_str("0x1::aptos_coin::AptosCoin"))],
        transaction_arguments,
    )

    raw_transaction = RawTransaction(
        sender.address(),
        sequence_number,
        TransactionPayload(payload),
        client.client_config.max_gas_amount,
        client.client_config.gas_unit_price,
        int(time.time()) + client.client_config.expiration_ttl,
        await client.chain_id(),
    )

    signature = sender.sign(raw_transaction.keyed())
    authenticator = Authenticator(Ed25519Authenticator(sender.public_key(), signature))
    signed_transaction = SignedTransaction(raw_transaction, authenticator)
    return await client.submit_bcs_transaction(signed_transaction)


if __name__ == "__main__":
    asyncio.run(main())

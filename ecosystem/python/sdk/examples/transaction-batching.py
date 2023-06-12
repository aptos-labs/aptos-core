# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

import asyncio
import logging
import time
from multiprocessing import Pipe, Process
from multiprocessing.connection import Connection
from typing import Any, List

from aptos_sdk.account import Account
from aptos_sdk.account_address import AccountAddress
from aptos_sdk.account_sequence_number import AccountSequenceNumber
from aptos_sdk.aptos_token_client import AptosTokenClient, Property, PropertyMap
from aptos_sdk.async_client import ClientConfig, FaucetClient, RestClient
from aptos_sdk.bcs import Serializer
from aptos_sdk.transaction_worker import TransactionWorker
from aptos_sdk.transactions import (
    EntryFunction,
    SignedTransaction,
    TransactionArgument,
    TransactionPayload,
)

from .common import FAUCET_URL, NODE_URL


class TransactionGenerator:
    """
    Demonstrate how one might make a harness for submitting transactions. This class just keeps
    submitting the same transaction payload. In practice, this could be a queue, where new payloads
    accumulate and are consumed by the call to next_transaction.

    Todo: add tracking of transaction status to this and come up with some general logic to retry
    or exit upon failure.
    """

    _client: RestClient
    _recipient: AccountAddress
    _offset: int
    _remaining_transactions: int
    _waiting_for_more = asyncio.Event
    _complete = asyncio.Event
    _lock = asyncio.Lock

    def __init__(self, client: RestClient, recipient: AccountAddress):
        self._client = client
        self._recipient = recipient
        self._waiting_for_more = asyncio.Event()
        self._waiting_for_more.clear()
        self._complete = asyncio.Event()
        self._complete.set()
        self._lock = asyncio.Lock()
        self._remaining_transactions = 0

    async def next_transaction(
        self, sender: Account, sequence_number: int
    ) -> SignedTransaction:
        while self._remaining_transactions == 0:
            await self._waiting_for_more.wait()

        async with self._lock:
            self._remaining_transactions -= 1
            if self._remaining_transactions == 0:
                self._waiting_for_more.clear()
                self._complete.set()

        return await transfer_transaction(
            self._client, sender, sequence_number, self._recipient, 0
        )

    async def increase_transaction_count(self, number: int):
        if number <= 0:
            return

        async with self._lock:
            self._remaining_transactions += number
            self._waiting_for_more.set()
            self._complete.clear()

    async def wait(self):
        await self._complete.wait()


class WorkerContainer:
    _conn: Connection
    _process: Process

    def __init__(self, node_url: str, account: Account, recipient: AccountAddress):
        (self._conn, conn) = Pipe()
        self._process = Process(
            target=Worker.run, args=(conn, node_url, account, recipient)
        )

    def get(self) -> Any:
        self._conn.recv()

    def join(self):
        self._process.join()

    def put(self, value: Any):
        self._conn.send(value)

    def start(self):
        self._process.start()


class Worker:
    _conn: Connection
    _rest_client: RestClient
    _account: Account
    _recipient: AccountAddress
    _txn_generator: TransactionGenerator
    _txn_worker: TransactionWorker

    def __init__(
        self,
        conn: Connection,
        node_url: str,
        account: Account,
        recipient: AccountAddress,
    ):
        self._conn = conn
        self._rest_client = RestClient(node_url)
        self._account = account
        self._recipient = recipient
        self._txn_generator = TransactionGenerator(self._rest_client, self._recipient)
        self._txn_worker = TransactionWorker(
            self._account, self._rest_client, self._txn_generator.next_transaction
        )

    def run(queue: Pipe, node_url: str, account: Account, recipient: AccountAddress):
        worker = Worker(queue, node_url, account, recipient)
        asyncio.run(worker.async_run())

    async def async_run(self):
        try:
            self._txn_worker.start()

            self._conn.send(True)
            num_txns = self._conn.recv()

            await self._txn_generator.increase_transaction_count(num_txns)

            logging.info(f"Increase txns from {self._account.address()}")
            self._conn.send(True)
            self._conn.recv()

            txn_hashes = []
            while num_txns != 0:
                if num_txns % 100 == 0:
                    logging.info(
                        f"{self._txn_worker.address()} remaining transactions {num_txns}"
                    )
                num_txns -= 1
                (
                    sequence_number,
                    txn_hash,
                    exception,
                ) = await self._txn_worker.next_processed_transaction()
                if exception:
                    logging.error(
                        f"Account {self._txn_worker.address()}, transaction {sequence_number} submission failed.",
                        exc_info=exception,
                    )
                else:
                    txn_hashes.append(txn_hash)

            logging.info(f"Submitted txns from {self._account.address()}")
            self._conn.send(True)
            self._conn.recv()

            for txn_hash in txn_hashes:
                await self._rest_client.wait_for_transaction(txn_hash)

            await self._rest_client.close()
            logging.info(f"Verified txns from {self._account.address()}")
            self._conn.send(True)
        except Exception as e:
            logging.error(
                "Failed during run.",
                exc_info=e,
            )


# This performs a simple p2p transaction
async def transfer_transaction(
    client: RestClient,
    sender: Account,
    sequence_number: int,
    recipient: AccountAddress,
    amount: int,
) -> str:
    transaction_arguments = [
        TransactionArgument(recipient, Serializer.struct),
        TransactionArgument(amount, Serializer.u64),
    ]
    payload = EntryFunction.natural(
        "0x1::aptos_account",
        "transfer",
        [],
        transaction_arguments,
    )

    return await client.create_bcs_signed_transaction(
        sender, TransactionPayload(payload), sequence_number
    )


# This will create a collection in the first transaction and then create NFTs thereafter.
# Note: Please adjust the sequence number and the name of the collection if run on the same set of
# accounts, otherwise you may end up not creating a collection and failing all transactions.
async def token_transaction(
    client: RestClient,
    sender: Account,
    sequence_number: int,
    recipient: AccountAddress,
    amount: int,
) -> str:
    collection_name = "Funky Alice's"
    if sequence_number == 8351:
        payload = AptosTokenClient.create_collection_payload(
            "Alice's simple collection",
            20000000000,
            collection_name,
            "https://aptos.dev",
            True,
            True,
            True,
            True,
            True,
            True,
            True,
            True,
            True,
            0,
            1,
        )
    else:
        payload = AptosTokenClient.mint_token_payload(
            collection_name,
            "Alice's simple token",
            f"token {sequence_number}",
            "https://aptos.dev/img/nyan.jpeg",
            PropertyMap([Property.string("string", "string value")]),
        )
    return await client.create_bcs_signed_transaction(sender, payload, sequence_number)


class Accounts:
    source: Account
    senders: List[Account]
    receivers: List[Account]

    def __init__(self, source, senders, receivers):
        self.source = source
        self.senders = senders
        self.receivers = receivers

    def generate(path: str, num_accounts: int) -> Accounts:
        source = Account.generate()
        source.store(f"{path}/source.txt")
        senders = []
        receivers = []
        for idx in range(num_accounts):
            senders.append(Account.generate())
            receivers.append(Account.generate())
            senders[-1].store(f"{path}/sender_{idx}.txt")
            receivers[-1].store(f"{path}/receiver_{idx}.txt")
        return Accounts(source, senders, receivers)

    def load(path: str, num_accounts: int) -> Accounts:
        source = Account.load(f"{path}/source.txt")
        senders = []
        receivers = []
        for idx in range(num_accounts):
            senders.append(Account.load(f"{path}/sender_{idx}.txt"))
            receivers.append(Account.load(f"{path}/receiver_{idx}.txt"))
        return Accounts(source, senders, receivers)


async def fund_from_faucet(rest_client: RestClient, source: Account):
    faucet_client = FaucetClient(FAUCET_URL, rest_client)

    fund_txns = []
    for _ in range(40):
        fund_txns.append(faucet_client.fund_account(source.address(), 100_000_000_000))
    await asyncio.gather(*fund_txns)


async def distribute_portionally(
    rest_client: RestClient,
    source: Account,
    senders: List[Account],
    receivers: List[Account],
):
    balance = int(await rest_client.account_balance(source.address()))
    per_node_balance = balance // (len(senders) + 1)
    await distribute(rest_client, source, senders, receivers, per_node_balance)


async def distribute(
    rest_client: RestClient,
    source: Account,
    senders: List[Account],
    receivers: List[Account],
    per_node_amount: int,
):
    all_accounts = list(map(lambda account: (account.address(), True), senders))
    all_accounts.extend(map(lambda account: (account.address(), False), receivers))

    account_sequence_number = AccountSequenceNumber(rest_client, source.address())

    txns = []
    txn_hashes = []

    for (account, fund) in all_accounts:
        sequence_number = await account_sequence_number.next_sequence_number(
            block=False
        )
        if sequence_number is None:
            txn_hashes.extend(await asyncio.gather(*txns))
            txns = []
            sequence_number = await account_sequence_number.next_sequence_number()
        amount = per_node_amount if fund else 0
        txn = await transfer_transaction(
            rest_client, source, sequence_number, account, amount
        )
        txns.append(rest_client.submit_bcs_transaction(txn))

    txn_hashes.extend(await asyncio.gather(*txns))
    for txn_hash in txn_hashes:
        await rest_client.wait_for_transaction(txn_hash)
    await account_sequence_number.synchronize()


async def main():
    client_config = ClientConfig()
    client_config.http2 = True
    rest_client = RestClient(NODE_URL, client_config)

    num_accounts = 64
    transactions = 100000
    start = time.time()

    logging.getLogger().setLevel(20)

    print("Starting...")

    # Generate will create new accounts, load will load existing accounts
    all_accounts = Accounts.generate("nodes", num_accounts)
    # all_accounts = Accounts.load("nodes", num_accounts)
    accounts = all_accounts.senders
    receivers = all_accounts.receivers
    source = all_accounts.source

    print(f"source: {source.address()}")

    last = time.time()
    print(f"Accounts generated / loaded at {last - start}")

    await fund_from_faucet(rest_client, source)

    print(f"Initial account funded at {time.time() - start} {time.time() - last}")
    last = time.time()

    balance = await rest_client.account_balance(source.address())
    amount = int(balance * 0.9 / num_accounts)
    await distribute(rest_client, source, accounts, receivers, amount)

    print(f"Funded all accounts at {time.time() - start} {time.time() - last}")
    last = time.time()

    balances = []
    for account in accounts:
        balances.append(rest_client.account_balance(account.address()))
    await asyncio.gather(*balances)

    print(f"Accounts checked at {time.time() - start} {time.time() - last}")
    last = time.time()

    workers = []
    for (account, recipient) in zip(accounts, receivers):
        workers.append(WorkerContainer(NODE_URL, account, recipient.address()))
        workers[-1].start()

    for worker in workers:
        worker.get()

    print(f"Workers started at {time.time() - start} {time.time() - last}")
    last = time.time()

    to_take = (transactions // num_accounts) + (
        1 if transactions % num_accounts != 0 else 0
    )
    remaining_transactions = transactions
    for worker in workers:
        taking = min(to_take, remaining_transactions)
        remaining_transactions -= taking
        worker.put(taking)

    for worker in workers:
        worker.get()

    print(f"Transactions submitted at {time.time() - start} {time.time() - last}")
    last = time.time()

    for worker in workers:
        worker.put(True)

    for worker in workers:
        worker.get()

    print(f"Transactions processed at {time.time() - start} {time.time() - last}")
    last = time.time()

    for worker in workers:
        worker.put(True)

    for worker in workers:
        worker.get()

    print(f"Transactions verified at {time.time() - start} {time.time() - last}")
    last = time.time()

    await rest_client.close()


if __name__ == "__main__":
    asyncio.run(main())

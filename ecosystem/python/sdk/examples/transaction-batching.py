# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import asyncio
import sys
import time
import typing
from multiprocessing import Pipe, Process
from multiprocessing.connection import Connection

from aptos_sdk.account import Account
from aptos_sdk.account_address import AccountAddress
from aptos_sdk.account_sequence_number import AccountSequenceNumber
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

    def get(self) -> typing.Any:
        self._conn.recv()

    def join(self):
        self._process.join()

    def put(self, value: typing.Any):
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
        asyncio.run(worker.arun())

    async def arun(self):
        print(f"hello from {self._account.address()}", flush=True)
        try:
            self._txn_worker.start()

            self._conn.send(True)
            num_txns = self._conn.recv()

            await self._txn_generator.increase_transaction_count(num_txns)

            print(f"Increase txns from {self._account.address()}", flush=True)
            self._conn.send(True)
            self._conn.recv()

            txn_hashes = []
            while num_txns != 0:
                num_txns -= 1
                (
                    sequence_number,
                    txn_hash,
                    exception,
                ) = await self._txn_worker.next_processed_transaction()
                if exception:
                    print(
                        f"Account {self._txn_worker.account()}, transaction {sequence_number} submission failed: {exception}"
                    )
                else:
                    txn_hashes.append(txn_hash)

            print(f"Submit txns from {self._account.address()}", flush=True)
            self._conn.send(True)
            self._conn.recv()

            for txn_hash in txn_hashes:
                await self._rest_client.wait_for_transaction(txn_hash)

            await self._rest_client.close()
            print(f"Verified txns from {self._account.address()}", flush=True)
            self._conn.send(True)
        except Exception as e:
            print(e)
            sys.stdout.flush()


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


async def main():
    client_config = ClientConfig()
    # Toggle to benchmark
    client_config.http2 = False
    client_config.http2 = True
    rest_client = RestClient(NODE_URL, client_config)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)

    num_accounts = 8
    transactions = 1000
    start = time.time()

    print("Starting...")

    accounts = []
    recipients = []

    for account in range(num_accounts):
        recipients.append(Account.generate())
        accounts.append(Account.generate())

    last = time.time()
    print(f"Accounts generated at {last - start}")

    source = Account.generate()
    await faucet_client.fund_account(source.address(), 100_000_000 * num_accounts)
    balance = int(await rest_client.account_balance(source.address()))

    per_node_balance = balance // (num_accounts + 1)
    account_sequence_number = AccountSequenceNumber(rest_client, source.address())

    print(f"Initial account funded at {time.time() - start} {time.time() - last}")
    last = time.time()

    all_accounts = list(map(lambda account: (account.address(), True), accounts))
    all_accounts.extend(map(lambda account: (account.address(), False), recipients))

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
        amount = per_node_balance if fund else 0
        txn = await transfer_transaction(
            rest_client, source, sequence_number, account, amount
        )
        txns.append(rest_client.submit_bcs_transaction(txn))

    txn_hashes.extend(await asyncio.gather(*txns))
    for txn_hash in txn_hashes:
        await rest_client.wait_for_transaction(txn_hash)
    await account_sequence_number.synchronize()

    print(f"Funded all accounts at {time.time() - start} {time.time() - last}")
    last = time.time()

    balances = []
    for account in accounts:
        balances.append(rest_client.account_balance(account.address()))
    await asyncio.gather(*balances)

    print(f"Accounts checked at {time.time() - start} {time.time() - last}")
    last = time.time()

    workers = []
    for (account, recipient) in zip(accounts, recipients):
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

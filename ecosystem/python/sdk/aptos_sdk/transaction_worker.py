# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import asyncio
import logging
import typing

from aptos_sdk.account import Account
from aptos_sdk.account_address import AccountAddress
from aptos_sdk.account_sequence_number import AccountSequenceNumber
from aptos_sdk.async_client import RestClient
from aptos_sdk.transactions import SignedTransaction, TransactionPayload


class TransactionWorker:
    """
    The TransactionWorker provides a simple framework for receiving payloads to be processed. It
    acquires new sequence numbers and calls into the callback to produce a signed transaction, and
    then submits the transaction. In another task, it waits for resolution of the submission
    process or get pre-execution validation error.

    Note: This is not a particularly robust solution, as it lacks any framework to handle failed
    transactions with functionality like retries or checking whether the framework is online.
    This is the responsibility of a higher-level framework.
    """

    _account: Account
    _account_sequence_number: AccountSequenceNumber
    _rest_client: RestClient
    _transaction_generator: typing.Callable[
        [Account, int], typing.Awaitable[SignedTransaction]
    ]
    _started: bool
    _stopped: bool
    _outstanding_transactions: asyncio.Queue
    _outstanding_transactions_task: typing.Optional[asyncio.Task]
    _processed_transactions: asyncio.Queue
    _process_transactions_task: typing.Optional[asyncio.Task]

    def __init__(
        self,
        account: Account,
        rest_client: RestClient,
        transaction_generator: typing.Callable[
            [Account, int], typing.Awaitable[SignedTransaction]
        ],
    ):
        self._account = account
        self._account_sequence_number = AccountSequenceNumber(
            rest_client, account.address()
        )
        self._rest_client = rest_client
        self._transaction_generator = transaction_generator

        self._started = False
        self._stopped = False
        self._outstanding_transactions = asyncio.Queue()
        self._processed_transactions = asyncio.Queue()

    def address(self) -> AccountAddress:
        return self._account.address()

    async def _submit_transactions_task(self):
        try:
            while True:
                sequence_number = (
                    await self._account_sequence_number.next_sequence_number()
                )
                transaction = await self._transaction_generator(
                    self._account, sequence_number
                )
                txn_hash_awaitable = self._rest_client.submit_bcs_transaction(
                    transaction
                )
                await self._outstanding_transactions.put(
                    (txn_hash_awaitable, sequence_number)
                )
        except asyncio.CancelledError:
            return
        except Exception as e:
            # This is insufficient, if we hit this we either need to bail or resolve the potential errors
            logging.error(e, exc_info=True)

    async def _process_transactions_task(self):
        try:
            while True:
                # Always start waiting for one, that way we can acquire a batch in the loop below.
                (
                    txn_hash_awaitable,
                    sequence_number,
                ) = await self._outstanding_transactions.get()
                awaitables = [txn_hash_awaitable]
                sequence_numbers = [sequence_number]

                # Now acquire our batch.
                while not self._outstanding_transactions.empty():
                    (
                        txn_hash_awaitable,
                        sequence_number,
                    ) = await self._outstanding_transactions.get()
                    awaitables.append(txn_hash_awaitable)
                    sequence_numbers.append(sequence_number)

                outputs = await asyncio.gather(*awaitables, return_exceptions=True)

                for (output, sequence_number) in zip(outputs, sequence_numbers):
                    if isinstance(output, BaseException):
                        await self._processed_transactions.put(
                            (sequence_number, None, output)
                        )
                    else:
                        await self._processed_transactions.put(
                            (sequence_number, output, None)
                        )
        except asyncio.CancelledError:
            return
        except Exception as e:
            # This is insufficient, if we hit this we either need to bail or resolve the potential errors
            logging.error(e, exc_info=True)

    async def next_processed_transaction(
        self,
    ) -> (int, typing.Optional[str], typing.Optional[Exception]):
        return await self._processed_transactions.get()

    def stop(self):
        """Stop the tasks for managing transactions"""
        if not self._started:
            raise Exception("Start not yet called")
        if self._stopped:
            raise Exception("Already stopped")
        self._stopped = True

        self._submit_transactions_task.cancel()
        self._process_transactions_task.cancel()

    def start(self):
        """Begin the tasks for managing transactions"""
        if self._started:
            raise Exception("Already started")
        self._started = True

        self._submit_transactions_task = asyncio.create_task(
            self._submit_transactions_task()
        )
        self._process_transactions_task = asyncio.create_task(
            self._process_transactions_task()
        )


class TransactionQueue:
    """Provides a queue model for pushing transactions into the TransactionWorker."""

    _client: RestClient
    _outstanding_transactions: asyncio.Queue

    def __init__(self, client: RestClient):
        self._client = client
        self._outstanding_transactions = asyncio.Queue()

    async def push(self, payload: TransactionPayload):
        await self._outstanding_transactions.put(payload)

    async def next(self, sender: Account, sequence_number: int) -> SignedTransaction:
        payload = await self._outstanding_transactions.get()
        return await self._client.create_bcs_signed_transaction(
            sender, payload, sequence_number=sequence_number
        )


import unittest
import unittest.mock

from aptos_sdk.bcs import Serializer
from aptos_sdk.transactions import EntryFunction, TransactionArgument


class Test(unittest.IsolatedAsyncioTestCase):
    async def test_common_path(self):
        transaction_arguments = [
            TransactionArgument(AccountAddress.from_hex("b0b"), Serializer.struct),
            TransactionArgument(100, Serializer.u64),
        ]
        payload = EntryFunction.natural(
            "0x1::aptos_accounts",
            "transfer",
            [],
            transaction_arguments,
        )

        seq_num_patcher = unittest.mock.patch(
            "aptos_sdk.async_client.RestClient.account_sequence_number", return_value=0
        )
        seq_num_patcher.start()
        submit_txn_patcher = unittest.mock.patch(
            "aptos_sdk.async_client.RestClient.submit_bcs_transaction",
            return_value="0xff",
        )
        submit_txn_patcher.start()

        rest_client = RestClient("https://fullnode.devnet.aptoslabs.com/v1")
        txn_queue = TransactionQueue(rest_client)
        txn_worker = TransactionWorker(Account.generate(), rest_client, txn_queue.next)
        txn_worker.start()

        await txn_queue.push(payload)
        processed_txn = await txn_worker.next_processed_transaction()
        self.assertEqual(processed_txn[0], 0)
        self.assertEqual(processed_txn[1], "0xff")
        self.assertEqual(processed_txn[2], None)

        submit_txn_patcher.stop()
        exception = Exception("Power overwhelming")
        submit_txn_patcher = unittest.mock.patch(
            "aptos_sdk.async_client.RestClient.submit_bcs_transaction",
            side_effect=exception,
        )
        submit_txn_patcher.start()

        await txn_queue.push(payload)
        processed_txn = await txn_worker.next_processed_transaction()
        self.assertEqual(processed_txn[0], 1)
        self.assertEqual(processed_txn[1], None)
        self.assertEqual(processed_txn[2], exception)

        txn_worker.stop()

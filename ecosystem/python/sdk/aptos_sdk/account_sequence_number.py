# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
import asyncio
import logging
import time
from typing import Optional

from aptos_sdk.account_address import AccountAddress
from aptos_sdk.async_client import RestClient


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
    * They are initialized to the current on-chain state, so if there are already transactions in
      flight, they make take some time to reset.
    * Accounts are automatically initialized if not explicitly

    Notes:
    * This is co-routine safe, that is many async tasks can be reading from this concurrently.
    * The synchronize method will create a barrier that prevents additional next_sequence_number
      calls until it is complete.
    * This only manages the distribution of sequence numbers it does not help handle transaction
      failures.
    """

    client: RestClient
    account: AccountAddress
    lock = asyncio.Lock

    maximum_in_flight: int = 100
    maximum_wait_time = 30
    sleep_time = 0.01

    last_committed_number: Optional[int]
    current_number: Optional[int]

    def __init__(self, client: RestClient, account: AccountAddress):
        self.client = client
        self.account = account
        self.lock = asyncio.Lock()

        self.last_uncommitted_number = None
        self.current_number = None

    async def next_sequence_number(self, block: bool = True) -> Optional[int]:
        """
        Returns the next sequence number available on this account. This leverages a lock to
        guarantee first-in, first-out ordering of requests.
        """
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
                    self.current_number - self.last_uncommitted_number
                    >= self.maximum_in_flight
                ):
                    if not block:
                        return None
                    await asyncio.sleep(self.sleep_time)
                    if time.time() - start_time > self.maximum_wait_time:
                        logging.warn(
                            f"Waited over 30 seconds for a transaction to commit, resyncing {self.account.address().hex()}"
                        )
                        await self.initialize()
                    else:
                        await self.__update()
            next_number = self.current_number
            self.current_number += 1
        finally:
            self.lock.release()
        return next_number

    async def initialize(self):
        """Optional initializer. called by next_sequence_number if not called prior."""
        self.current_number = await self.__current_sequence_number()
        self.last_uncommitted_number = self.current_number

    async def synchronize(self):
        """
        Poll the network until all submitted transactions have either been committed or until
        the maximum wait time has elapsed. This will prevent any calls to next_sequence_number
        until this called has returned.
        """
        if self.last_uncommitted_number == self.current_number:
            return

        await self.lock.acquire()
        try:
            await self.__update()
            start_time = time.time()
            while self.last_uncommitted_number != self.current_number:
                print(f"{self.last_uncommitted_number} {self.current_number}")
                if time.time() - start_time > self.maximum_wait_time:
                    logging.warn(
                        f"Waited over 30 seconds for a transaction to commit, resyncing {self.account.address}"
                    )
                    await self.initialize()
                else:
                    await asyncio.sleep(self.sleep_time)
                    await self.__update()
        finally:
            self.lock.release()

    async def __update(self):
        self.last_uncommitted_number = await self.__current_sequence_number()
        return self.last_uncommitted_number

    async def __current_sequence_number(self) -> int:
        return await self.client.account_sequence_number(self.account)


import unittest
import unittest.mock


class Test(unittest.IsolatedAsyncioTestCase):
    async def test_common_path(self):
        """
        Verifies that:
        * AccountSequenceNumber returns sequential numbers starting from 0
        * When the account has been updated on-chain include that in computations 100 -> 105
        * Ensure that none is returned if the call for next_sequence_number would block
        * Ensure that synchronize completes if the value matches on-chain
        """
        patcher = unittest.mock.patch(
            "aptos_sdk.async_client.RestClient.account_sequence_number", return_value=0
        )
        patcher.start()

        rest_client = RestClient("https://fullnode.devnet.aptoslabs.com/v1")
        account_sequence_number = AccountSequenceNumber(
            rest_client, AccountAddress.from_hex("b0b")
        )
        last_seq_num = 0
        for seq_num in range(5):
            last_seq_num = await account_sequence_number.next_sequence_number()
            self.assertEqual(last_seq_num, seq_num)

        patcher.stop()
        patcher = unittest.mock.patch(
            "aptos_sdk.async_client.RestClient.account_sequence_number", return_value=5
        )
        patcher.start()

        for seq_num in range(AccountSequenceNumber.maximum_in_flight):
            last_seq_num = await account_sequence_number.next_sequence_number()
            self.assertEqual(last_seq_num, seq_num + 5)

        self.assertEqual(
            await account_sequence_number.next_sequence_number(block=False), None
        )
        next_sequence_number = last_seq_num + 1
        patcher.stop()
        patcher = unittest.mock.patch(
            "aptos_sdk.async_client.RestClient.account_sequence_number",
            return_value=next_sequence_number,
        )
        patcher.start()

        self.assertNotEqual(account_sequence_number.current_number, last_seq_num)
        await account_sequence_number.synchronize()
        self.assertEqual(account_sequence_number.current_number, next_sequence_number)

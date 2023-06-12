# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

import asyncio
import logging
from typing import Callable, Optional

from aptos_sdk.account_address import AccountAddress
from aptos_sdk.async_client import ApiError, RestClient


class AccountSequenceNumberConfig:
    """Common configuration for account number generation"""

    maximum_in_flight: int = 100
    maximum_wait_time: int = 30
    sleep_time: float = 0.01


class AccountSequenceNumber:
    """
    A managed wrapper around sequence numbers that implements the trivial flow control used by the
    Aptos faucet:
    * Submit up to 100 transactions per account in parallel with a timeout of 20 seconds
    * If local assumes 100 are in flight, determine the actual committed state from the network
    * If there are less than 100 due to some being committed, adjust the window
    * If 100 are in flight Wait .1 seconds before re-evaluating
    * If ever waiting more than 30 seconds restart the sequence number to the current on-chain state
    Assumptions:
    * Accounts are expected to be managed by a single AccountSequenceNumber and not used otherwise.
    * They are initialized to the current on-chain state, so if there are already transactions in
      flight, they may take some time to reset.
    * Accounts are automatically initialized if not explicitly

    Notes:
    * This is co-routine safe, that is many async tasks can be reading from this concurrently.
    * The state of an account cannot be used across multiple AccountSequenceNumber services.
    * The synchronize method will create a barrier that prevents additional next_sequence_number
      calls until it is complete.
    * This only manages the distribution of sequence numbers it does not help handle transaction
      failures.
    * If a transaction fails, you should call synchronize and wait for timeouts.
    * Mempool limits the number of transactions per account to 100, hence why we chose 100.
    """

    _client: RestClient
    _account: AccountAddress
    _lock: asyncio.Lock

    _maximum_in_flight: int = 100
    _maximum_wait_time: int = 30
    _sleep_time: float = 0.01

    _last_committed_number: Optional[int]
    _current_number: Optional[int]

    def __init__(
        self,
        client: RestClient,
        account: AccountAddress,
        config: AccountSequenceNumberConfig = AccountSequenceNumberConfig(),
    ):
        self._client = client
        self._account = account
        self._lock = asyncio.Lock()

        self._last_uncommitted_number = None
        self._current_number = None

        self._maximum_in_flight = config.maximum_in_flight
        self._maximum_wait_time = config.maximum_wait_time
        self._sleep_time = config.sleep_time

    async def next_sequence_number(self, block: bool = True) -> Optional[int]:
        """
        Returns the next sequence number available on this account. This leverages a lock to
        guarantee first-in, first-out ordering of requests.
        """
        async with self._lock:
            if self._last_uncommitted_number is None or self._current_number is None:
                await self._initialize()
            # If there are more than self._maximum_in_flight in flight, wait for a slot.
            # Or at least check to see if there is a slot and exit if in non-blocking mode.
            if (
                self._current_number - self._last_uncommitted_number
                >= self._maximum_in_flight
            ):
                await self._update()
                if (
                    self._current_number - self._last_uncommitted_number
                    >= self._maximum_in_flight
                ):
                    if not block:
                        return None
                    await self._resync(
                        lambda acn: acn._current_number - acn._last_uncommitted_number
                        >= acn._maximum_in_flight
                    )

            next_number = self._current_number
            self._current_number += 1
        return next_number

    async def _initialize(self):
        """Optional initializer. called by next_sequence_number if not called prior."""
        self._current_number = await self._current_sequence_number()
        self._last_uncommitted_number = self._current_number

    async def synchronize(self):
        """
        Poll the network until all submitted transactions have either been committed or until
        the maximum wait time has elapsed. This will prevent any calls to next_sequence_number
        until this called has returned.
        """
        async with self._lock:
            await self._update()
            await self._resync(
                lambda acn: acn._last_uncommitted_number != acn._current_number
            )

    async def _resync(self, check: Callable[[AccountSequenceNumber], bool]):
        """Forces a resync with the upstream, this should be called within the lock"""
        start_time = await self._client.current_timestamp()
        failed = False
        while check(self):
            ledger_time = await self._client.current_timestamp()
            if ledger_time - start_time > self._maximum_wait_time:
                logging.warn(
                    f"Waited over {self._maximum_wait_time} seconds for a transaction to commit, resyncing {self._account}"
                )
                failed = True
                break
            else:
                await asyncio.sleep(self._sleep_time)
                await self._update()
        if not failed:
            return
        for seq_num in range(self._last_uncommitted_number + 1, self._current_number):
            while True:
                try:
                    result = (
                        await self._client.account_transaction_sequence_number_status(
                            self._account, seq_num
                        )
                    )
                    if result:
                        break
                except ApiError as error:
                    if error.status_code == 404:
                        break
                    raise
        await self._initialize()

    async def _update(self):
        self._last_uncommitted_number = await self._current_sequence_number()
        return self._last_uncommitted_number

    async def _current_sequence_number(self) -> int:
        return await self._client.account_sequence_number(self._account)


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

        for seq_num in range(AccountSequenceNumber._maximum_in_flight):
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

        self.assertNotEqual(account_sequence_number._current_number, last_seq_num)
        await account_sequence_number.synchronize()
        self.assertEqual(account_sequence_number._current_number, next_sequence_number)

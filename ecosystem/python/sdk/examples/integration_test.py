# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""
Provides a test harness for treating examples as integration tests.
"""

import os
import unittest
from typing import Optional

from aptos_sdk.aptos_cli_wrapper import AptosCLIWrapper, AptosInstance


class Test(unittest.IsolatedAsyncioTestCase):
    _node: Optional[AptosInstance] = None

    async def asyncSetUp(self):
        self._node = AptosCLIWrapper.start_node()
        operational = await self._node.wait_until_operational()
        if not operational:
            raise Exception("".join(self._node.errors()))

        os.environ["APTOS_NODE_URL"] = "http://127.0.0.1:8080/v1"
        os.environ["APTOS_FAUCET_URL"] = "http://127.0.0.1:8081"

    async def test_read_aggreagtor(self):
        from . import read_aggregator

        await read_aggregator.main()

    async def test_rotate_key(self):
        from . import rotate_key

        await rotate_key.main()

    async def test_simple_nft(self):
        from . import simple_nft

        await simple_nft.main()

    async def test_simulate_transfer_coin(self):
        from . import simulate_transfer_coin

        await simulate_transfer_coin.main()

    async def test_transfer_coin(self):
        from . import transfer_coin

        await transfer_coin.main()

    async def test_transfer_two_by_two(self):
        from . import transfer_two_by_two

        await transfer_two_by_two.main()

    def tearDown(self):
        self._node.stop()


if __name__ == "__main__":
    unittest.main(buffer=True)

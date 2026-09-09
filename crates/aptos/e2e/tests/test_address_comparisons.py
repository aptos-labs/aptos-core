# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import json
import unittest
from types import SimpleNamespace
from unittest.mock import Mock, patch

from aptos_sdk.account_address import AccountAddress
from cases.account import test_account_resource_account
from cases.move import test_move_publish
from common import TestError


def response(result):
    return SimpleNamespace(stdout=json.dumps({"Result": result}))


class AddressComparisonTests(unittest.TestCase):
    def publish(self, address, name="cli_e2e_tests"):
        helper = Mock(base_network="mainnet")
        helper.get_account_info.return_value.account_address = (
            AccountAddress.from_str_relaxed("0x0519")
        )
        helper.run_command.side_effect = [
            None,
            response([{"abi": {"address": address, "name": name}}]),
        ]
        test_move_publish.__wrapped__(helper, test_name="publish")

    def resource_account(self, listed, derived="0" * 60 + "04e2"):
        helper = Mock()
        helper.run_command.side_effect = [
            response({"sender": "0x1", "resource_account": "0" * 60 + "04e2"}),
            response(derived),
            response(
                [
                    {
                        "0x1::resource_account::Container": {
                            "store": {"data": [{"key": listed}]}
                        }
                    }
                ]
            ),
        ]
        with patch("cases.account.time.sleep"):
            test_account_resource_account.__wrapped__(
                helper, test_name="resource-account"
            )

    def test_published_module_accepts_equivalent_address_formats(self):
        for address in ("0x519", "0x0519", "0x" + "0" * 60 + "0519"):
            with self.subTest(address=address):
                self.publish(address)

    def test_published_module_rejects_different_address_or_name(self):
        with self.assertRaises(TestError):
            self.publish("0x520")
        with self.assertRaises(TestError):
            self.publish("0x519", "other_module")

    def test_resource_account_accepts_equivalent_address_formats(self):
        for address in ("0x4e2", "0x04e2", "0x" + "0" * 60 + "04e2"):
            with self.subTest(address=address):
                self.resource_account(address)

    def test_resource_account_rejects_different_listed_address(self):
        with self.assertRaises(TestError):
            self.resource_account("0x4e3")

    def test_resource_account_accepts_equivalent_derived_address(self):
        self.resource_account("0x4e2", "0x04e2")

    def test_resource_account_rejects_different_derived_address(self):
        with self.assertRaises(TestError):
            self.resource_account("0x4e2", "0x4e3")


if __name__ == "__main__":
    unittest.main()

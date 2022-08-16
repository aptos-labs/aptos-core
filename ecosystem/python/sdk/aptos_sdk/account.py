# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

import json
import tempfile
import unittest

from . import ed25519
from .account_address import AccountAddress


class Account:
    """Represents an account as well as the private, public key-pair for the Aptos blockchain."""

    account_address: AccountAddress
    private_key: ed25519.PrivateKey

    def __init__(
        self, account_address: AccountAddress, private_key: ed25519.PrivateKey
    ):
        self.account_address = account_address
        self.private_key = private_key

    def __eq__(self, other: Account) -> bool:
        return (
            self.account_address == other.account_address
            and self.private_key == other.private_key
        )

    def generate() -> Account:
        private_key = ed25519.PrivateKey.random()
        account_address = AccountAddress.from_key(private_key.public_key())
        return Account(account_address, private_key)

    def load_key(key: str) -> Account:
        private_key = ed25519.PrivateKey.from_hex(key)
        account_address = AccountAddress.from_key(private_key.public_key())
        return Account(account_address, private_key)

    def load(path: str) -> Account:
        with open(path) as file:
            data = json.load(file)
        return Account(
            AccountAddress.from_hex(data["account_address"]),
            ed25519.PrivateKey.from_hex(data["private_key"]),
        )

    def store(self, path: str):
        data = {
            "account_address": self.account_address.hex(),
            "private_key": self.private_key.hex(),
        }
        with open(path, "w") as file:
            json.dump(data, file)

    def address(self) -> AccountAddress:
        """Returns the address associated with the given account"""

        return self.account_address

    def auth_key(self) -> str:
        """Returns the auth_key for the associated account"""

        return AccountAddress.from_key(self.private_key.public_key()).hex()

    def sign(self, data: bytes) -> ed25519.Signature:
        return self.private_key.sign(data)

    def public_key(self) -> ed25519.PublicKey:
        """Returns the public key for the associated account"""

        return self.private_key.public_key()


class Test(unittest.TestCase):
    def test_load_and_store(self):
        (file, path) = tempfile.mkstemp()
        start = Account.generate()
        start.store(path)
        load = Account.load(path)

        self.assertEqual(start, load)
        # Auth key and Account address should be the same at start
        self.assertEqual(start.address().hex(), start.auth_key())

    def test_key(self):
        message = b"test message"
        account = Account.generate()
        signature = account.sign(message)
        self.assertTrue(account.public_key().verify(message, signature))

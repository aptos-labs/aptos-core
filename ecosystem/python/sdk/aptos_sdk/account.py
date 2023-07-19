# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

import json
import tempfile
import unittest

from . import ed25519
from .account_address import AccountAddress
from .bcs import Serializer


class Account:
    """Represents an account as well as the private, public key-pair for the Aptos blockchain."""

    account_address: AccountAddress
    private_key: ed25519.PrivateKey

    def __init__(
        self, account_address: AccountAddress, private_key: ed25519.PrivateKey
    ):
        self.account_address = account_address
        self.private_key = private_key

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, Account):
            return NotImplemented
        return (
            self.account_address == other.account_address
            and self.private_key == other.private_key
        )

    @staticmethod
    def generate() -> Account:
        private_key = ed25519.PrivateKey.random()
        account_address = AccountAddress.from_key(private_key.public_key())
        return Account(account_address, private_key)

    @staticmethod
    def load_key(key: str) -> Account:
        private_key = ed25519.PrivateKey.from_str(key)
        account_address = AccountAddress.from_key(private_key.public_key())
        return Account(account_address, private_key)

    @staticmethod
    def load(path: str) -> Account:
        with open(path) as file:
            data = json.load(file)
        return Account(
            AccountAddress.from_str(data["account_address"]),
            ed25519.PrivateKey.from_str(data["private_key"]),
        )

    def store(self, path: str):
        data = {
            "account_address": str(self.account_address),
            "private_key": str(self.private_key),
        }
        with open(path, "w") as file:
            json.dump(data, file)

    def address(self) -> AccountAddress:
        """Returns the address associated with the given account"""

        return self.account_address

    def auth_key(self) -> str:
        """Returns the auth_key for the associated account"""
        return str(AccountAddress.from_key(self.private_key.public_key()))

    def sign(self, data: bytes) -> ed25519.Signature:
        return self.private_key.sign(data)

    def public_key(self) -> ed25519.PublicKey:
        """Returns the public key for the associated account"""

        return self.private_key.public_key()


class RotationProofChallenge:
    type_info_account_address: AccountAddress = AccountAddress.from_str("0x1")
    type_info_module_name: str = "account"
    type_info_struct_name: str = "RotationProofChallenge"
    sequence_number: int
    originator: AccountAddress
    current_auth_key: AccountAddress
    new_public_key: bytes

    def __init__(
        self,
        sequence_number: int,
        originator: AccountAddress,
        current_auth_key: AccountAddress,
        new_public_key: bytes,
    ):
        self.sequence_number = sequence_number
        self.originator = originator
        self.current_auth_key = current_auth_key
        self.new_public_key = new_public_key

    def serialize(self, serializer: Serializer):
        self.type_info_account_address.serialize(serializer)
        serializer.str(self.type_info_module_name)
        serializer.str(self.type_info_struct_name)
        serializer.u64(self.sequence_number)
        self.originator.serialize(serializer)
        self.current_auth_key.serialize(serializer)
        serializer.to_bytes(self.new_public_key)


class Test(unittest.TestCase):
    def test_load_and_store(self):
        (file, path) = tempfile.mkstemp()
        start = Account.generate()
        start.store(path)
        load = Account.load(path)

        self.assertEqual(start, load)
        # Auth key and Account address should be the same at start
        self.assertEqual(str(start.address()), start.auth_key())

    def test_key(self):
        message = b"test message"
        account = Account.generate()
        signature = account.sign(message)
        self.assertTrue(account.public_key().verify(message, signature))

    def test_rotation_proof_challenge(self):
        # Create originating account from private key.
        originating_account = Account.load_key(
            "005120c5882b0d492b3d2dc60a8a4510ec2051825413878453137305ba2d644b"
        )
        # Create target account from private key.
        target_account = Account.load_key(
            "19d409c191b1787d5b832d780316b83f6ee219677fafbd4c0f69fee12fdcdcee"
        )
        # Construct rotation proof challenge.
        rotation_proof_challenge = RotationProofChallenge(
            sequence_number=1234,
            originator=originating_account.address(),
            current_auth_key=originating_account.address(),
            new_public_key=target_account.public_key().key.encode(),
        )
        # Serialize transaction.
        serializer = Serializer()
        rotation_proof_challenge.serialize(serializer)
        rotation_proof_challenge_bcs = serializer.output().hex()
        # Compare against expected bytes.
        expected_bytes = (
            "0000000000000000000000000000000000000000000000000000000000000001"
            "076163636f756e7416526f746174696f6e50726f6f664368616c6c656e6765d2"
            "0400000000000015b67a673979c7c5dfc8d9c9f94d02da35062a19dd9d218087"
            "bd9076589219c615b67a673979c7c5dfc8d9c9f94d02da35062a19dd9d218087"
            "bd9076589219c620a1f942a3c46e2a4cd9552c0f95d529f8e3b60bcd44408637"
            "9ace35e4458b9f22"
        )
        self.assertEqual(rotation_proof_challenge_bcs, expected_bytes)

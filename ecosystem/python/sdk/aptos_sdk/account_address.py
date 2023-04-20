# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

import hashlib
import unittest

from . import ed25519
from .bcs import Deserializer, Serializer


class AuthKeyScheme:
    Ed25519: bytes = b"\x00"
    MultiEd25519: bytes = b"\x01"
    DeriveObjectAddressFromGuid: bytes = b"\xFD"
    DeriveObjectAddressFromSeed: bytes = b"\xFE"
    DeriveResourceAccountAddress: bytes = b"\xFF"


class AccountAddress:
    address: bytes
    LENGTH: int = 32

    def __init__(self, address: bytes):
        self.address = address

        if len(address) != AccountAddress.LENGTH:
            raise Exception("Expected address of length 32")

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, AccountAddress):
            return NotImplemented
        return self.address == other.address

    def __str__(self):
        return self.hex()

    def hex(self) -> str:
        return f"0x{self.address.hex()}"

    @staticmethod
    def from_hex(address: str) -> AccountAddress:
        addr = address

        if address[0:2] == "0x":
            addr = address[2:]

        if len(addr) < AccountAddress.LENGTH * 2:
            pad = "0" * (AccountAddress.LENGTH * 2 - len(addr))
            addr = pad + addr

        return AccountAddress(bytes.fromhex(addr))

    @staticmethod
    def from_key(key: ed25519.PublicKey) -> AccountAddress:
        hasher = hashlib.sha3_256()
        hasher.update(key.key.encode())
        hasher.update(AuthKeyScheme.Ed25519)
        return AccountAddress(hasher.digest())

    @staticmethod
    def from_multi_ed25519(keys: ed25519.MultiPublicKey) -> AccountAddress:
        hasher = hashlib.sha3_256()
        hasher.update(keys.to_bytes())
        hasher.update(AuthKeyScheme.MultiEd25519)
        return AccountAddress(hasher.digest())

    @staticmethod
    def for_resource_account(creator: AccountAddress, seed: bytes) -> AccountAddress:
        hasher = hashlib.sha3_256()
        hasher.update(creator.address)
        hasher.update(seed)
        hasher.update(AuthKeyScheme.DeriveResourceAccountAddress)
        return AccountAddress(hasher.digest())

    @staticmethod
    def for_guid_object(creator: AccountAddress, creation_num: int) -> AccountAddress:
        hasher = hashlib.sha3_256()
        serializer = Serializer()
        serializer.u64(creation_num)
        hasher.update(serializer.output())
        hasher.update(creator.address)
        hasher.update(AuthKeyScheme.DeriveObjectAddressFromGuid)
        return AccountAddress(hasher.digest())

    @staticmethod
    def for_named_object(creator: AccountAddress, seed: bytes) -> AccountAddress:
        hasher = hashlib.sha3_256()
        hasher.update(creator.address)
        hasher.update(seed)
        hasher.update(AuthKeyScheme.DeriveObjectAddressFromSeed)
        return AccountAddress(hasher.digest())

    @staticmethod
    def for_named_token(
        creator: AccountAddress, collection_name: str, token_name: str
    ) -> AccountAddress:
        collection_bytes = collection_name.encode()
        token_bytes = token_name.encode()
        return AccountAddress.for_named_object(
            creator, collection_bytes + b"::" + token_bytes
        )

    @staticmethod
    def for_named_collection(
        creator: AccountAddress, collection_name: str
    ) -> AccountAddress:
        return AccountAddress.for_named_object(creator, collection_name.encode())

    @staticmethod
    def deserialize(deserializer: Deserializer) -> AccountAddress:
        return AccountAddress(deserializer.fixed_bytes(AccountAddress.LENGTH))

    def serialize(self, serializer: Serializer):
        serializer.fixed_bytes(self.address)


class Test(unittest.TestCase):
    def test_multi_ed25519(self):
        private_key_1 = ed25519.PrivateKey.from_hex(
            "4e5e3be60f4bbd5e98d086d932f3ce779ff4b58da99bf9e5241ae1212a29e5fe"
        )
        private_key_2 = ed25519.PrivateKey.from_hex(
            "1e70e49b78f976644e2c51754a2f049d3ff041869c669523ba95b172c7329901"
        )
        multisig_public_key = ed25519.MultiPublicKey(
            [private_key_1.public_key(), private_key_2.public_key()], 1
        )

        expected = AccountAddress.from_hex(
            "835bb8c5ee481062946b18bbb3b42a40b998d6bf5316ca63834c959dc739acf0"
        )
        actual = AccountAddress.from_multi_ed25519(multisig_public_key)
        self.assertEqual(actual, expected)

    def test_resource_account(self):
        base_address = AccountAddress.from_hex("b0b")
        expected = AccountAddress.from_hex(
            "ee89f8c763c27f9d942d496c1a0dcf32d5eacfe78416f9486b8db66155b163b0"
        )
        actual = AccountAddress.for_resource_account(base_address, b"\x0b\x00\x0b")
        self.assertEqual(actual, expected)

    def test_named_object(self):
        base_address = AccountAddress.from_hex("b0b")
        expected = AccountAddress.from_hex(
            "f417184602a828a3819edf5e36285ebef5e4db1ba36270be580d6fd2d7bcc321"
        )
        actual = AccountAddress.for_named_object(base_address, b"bob's collection")
        self.assertEqual(actual, expected)

    def test_collection(self):
        base_address = AccountAddress.from_hex("b0b")
        expected = AccountAddress.from_hex(
            "f417184602a828a3819edf5e36285ebef5e4db1ba36270be580d6fd2d7bcc321"
        )
        actual = AccountAddress.for_named_collection(base_address, "bob's collection")
        self.assertEqual(actual, expected)

    def test_token(self):
        base_address = AccountAddress.from_hex("b0b")
        expected = AccountAddress.from_hex(
            "e20d1f22a5400ba7be0f515b7cbd00edc42dbcc31acc01e31128b2b5ddb3c56e"
        )
        actual = AccountAddress.for_named_token(
            base_address, "bob's collection", "bob's token"
        )
        self.assertEqual(actual, expected)

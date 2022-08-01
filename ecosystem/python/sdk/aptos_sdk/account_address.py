# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

import hashlib

from . import ed25519
from .bcs import Deserializer, Serializer


class AccountAddress:
    address: bytes
    LENGTH: int = 32

    def __init__(self, address: bytes):
        self.address = address

        if len(address) != AccountAddress.LENGTH:
            raise Exception("Expected address of length 32")

    def __eq__(self, other: AccountAddress) -> bool:
        return self.address == other.address

    def __str__(self):
        return self.hex()

    def hex(self) -> str:
        return f"0x{self.address.hex()}"

    def from_hex(address: str) -> AccountAddress:
        addr = address

        if address[0:2] == "0x":
            addr = address[2:]

        if len(addr) < AccountAddress.LENGTH * 2:
            pad = "0" * (AccountAddress.LENGTH * 2 - len(addr))
            addr = pad + addr

        return AccountAddress(bytes.fromhex(addr))

    def from_key(key: ed25519.PublicKey) -> AccountAddress:
        hasher = hashlib.sha3_256()
        hasher.update(key.key.encode() + b"\x00")
        return AccountAddress(hasher.digest())

    def deserialize(deserializer: Deserializer) -> AccountAddress:
        return AccountAddress(deserializer.fixed_bytes(AccountAddress.LENGTH))

    def serialize(self, serializer: Serializer):
        serializer.fixed_bytes(self.address)

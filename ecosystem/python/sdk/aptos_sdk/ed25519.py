# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

import hashlib
import unittest
from typing import List, Tuple

from nacl.signing import SigningKey, VerifyKey

from .bcs import Deserializer, Serializer


class PrivateKey:
    LENGTH: int = 32

    key: SigningKey

    def __init__(self, key: SigningKey):
        self.key = key

    def __eq__(self, other: object):
        if not isinstance(other, PrivateKey):
            return NotImplemented
        return self.key == other.key

    def __str__(self):
        return self.hex()

    @staticmethod
    def from_hex(value: str) -> PrivateKey:
        if value[0:2] == "0x":
            value = value[2:]
        return PrivateKey(SigningKey(bytes.fromhex(value)))

    def hex(self) -> str:
        return f"0x{self.key.encode().hex()}"

    def public_key(self) -> PublicKey:
        return PublicKey(self.key.verify_key)

    @staticmethod
    def random() -> PrivateKey:
        return PrivateKey(SigningKey.generate())

    def sign(self, data: bytes) -> Signature:
        return Signature(self.key.sign(data).signature)

    @staticmethod
    def deserialize(deserializer: Deserializer) -> PrivateKey:
        key = deserializer.to_bytes()
        if len(key) != PrivateKey.LENGTH:
            raise Exception("Length mismatch")

        return PrivateKey(SigningKey(key))

    def serialize(self, serializer: Serializer):
        serializer.to_bytes(self.key.encode())


class PublicKey:
    LENGTH: int = 32

    key: VerifyKey

    def __init__(self, key: VerifyKey):
        self.key = key

    def __eq__(self, other: object):
        if not isinstance(other, PublicKey):
            return NotImplemented
        return self.key == other.key

    def __str__(self) -> str:
        return f"0x{self.key.encode().hex()}"

    def verify(self, data: bytes, signature: Signature) -> bool:
        try:
            self.key.verify(data, signature.data())
        except Exception:
            return False
        return True

    @staticmethod
    def deserialize(deserializer: Deserializer) -> PublicKey:
        key = deserializer.to_bytes()
        if len(key) != PublicKey.LENGTH:
            raise Exception("Length mismatch")

        return PublicKey(VerifyKey(key))

    def serialize(self, serializer: Serializer):
        serializer.to_bytes(self.key.encode())


class MultiEd25519PublicKey:
    keys: List[PublicKey]
    threshold: int

    def __init__(self, keys: List[PublicKey], threshold: int):
        self.keys = keys
        self.threshold = threshold

    def __str__(self) -> str:
        return f"{self.threshold}-of-{len(self.keys)} Multi-Ed25519 public key"

    def auth_key(self) -> bytes:
        hasher = hashlib.sha3_256()
        hasher.update(self.to_bytes() + b"\x01")
        return hasher.digest()

    def to_bytes(self) -> bytes:
        concatenated_keys = bytes()
        for key in self.keys:
            concatenated_keys += key.key.encode()
        return concatenated_keys + bytes([self.threshold])

    @staticmethod
    def from_bytes(key: bytes) -> MultiEd25519PublicKey:
        n_signers = int(len(key) / PublicKey.LENGTH)
        keys = []
        for i in range(n_signers):
            start_byte = i * PublicKey.LENGTH
            end_byte = (i + 1) * PublicKey.LENGTH
            keys.append(PublicKey(VerifyKey(key[start_byte:end_byte])))
        return MultiEd25519PublicKey(keys, int(key[-1]))

    def serialize(self, serializer: Serializer):
        serializer.to_bytes(self.to_bytes())


class Signature:
    LENGTH: int = 64

    signature: bytes

    def __init__(self, signature: bytes):
        self.signature = signature

    def __eq__(self, other: object):
        if not isinstance(other, Signature):
            return NotImplemented
        return self.signature == other.signature

    def __str__(self) -> str:
        return f"0x{self.signature.hex()}"

    def data(self) -> bytes:
        return self.signature

    @staticmethod
    def deserialize(deserializer: Deserializer) -> Signature:
        signature = deserializer.to_bytes()
        if len(signature) != Signature.LENGTH:
            raise Exception("Length mismatch")

        return Signature(signature)

    def serialize(self, serializer: Serializer):
        serializer.to_bytes(self.signature)


class MultiEd25519Signature:
    signatures: List[Signature]
    bitmap: bytes

    def __init__(
        self,
        public_key: MultiEd25519PublicKey,
        signatures_map: List[Tuple[PublicKey, Signature]],
    ):
        self.signatures = list()
        bitmap = 0
        for entry in signatures_map:
            self.signatures.append(entry[1])
            index = public_key.keys.index(entry[0])
            shift = 31 - index  # 32 bit positions, left to right.
            bitmap = bitmap | (1 << shift)
        # 4-byte big endian bitmap.
        self.bitmap = bitmap.to_bytes(4, "big")

    def to_bytes(self) -> bytes:
        concatenated_signatures = bytes()
        for signature in self.signatures:
            concatenated_signatures += signature.data()
        return concatenated_signatures + self.bitmap

    def serialize(self, serializer: Serializer):
        serializer.to_bytes(self.to_bytes())


class Test(unittest.TestCase):
    def test_sign_and_verify(self):
        in_value = b"test_message"

        private_key = PrivateKey.random()
        public_key = private_key.public_key()

        signature = private_key.sign(in_value)
        self.assertTrue(public_key.verify(in_value, signature))

    def test_private_key_serialization(self):
        private_key = PrivateKey.random()
        ser = Serializer()

        private_key.serialize(ser)
        ser_private_key = PrivateKey.deserialize(Deserializer(ser.output()))
        self.assertEqual(private_key, ser_private_key)

    def test_public_key_serialization(self):
        private_key = PrivateKey.random()
        public_key = private_key.public_key()

        ser = Serializer()
        public_key.serialize(ser)
        ser_public_key = PublicKey.deserialize(Deserializer(ser.output()))
        self.assertEqual(public_key, ser_public_key)

    def test_signature_key_serialization(self):
        private_key = PrivateKey.random()
        in_value = b"another_message"
        signature = private_key.sign(in_value)

        ser = Serializer()
        signature.serialize(ser)
        ser_signature = Signature.deserialize(Deserializer(ser.output()))
        self.assertEqual(signature, ser_signature)

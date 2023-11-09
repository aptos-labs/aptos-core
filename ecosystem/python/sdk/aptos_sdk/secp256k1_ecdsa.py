# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

import hashlib
import unittest
from typing import cast

from ecdsa import SECP256k1, SigningKey, VerifyingKey, util

from . import asymmetric_crypto
from .bcs import Deserializer, Serializer


class PrivateKey(asymmetric_crypto.PrivateKey):
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
    def from_str(value: str) -> PrivateKey:
        if value[0:2] == "0x":
            value = value[2:]
        if len(value) != PrivateKey.LENGTH * 2:
            raise Exception("Length mismatch")
        return PrivateKey(
            SigningKey.from_string(bytes.fromhex(value), SECP256k1, hashlib.sha3_256)
        )

    def hex(self) -> str:
        return f"0x{self.key.to_string().hex()}"

    def public_key(self) -> PublicKey:
        return PublicKey(self.key.verifying_key)

    @staticmethod
    def random() -> PrivateKey:
        return PrivateKey(
            SigningKey.generate(curve=SECP256k1, hashfunc=hashlib.sha3_256)
        )

    def sign(self, data: bytes) -> Signature:
        sig = self.key.sign_deterministic(data, hashfunc=hashlib.sha3_256)
        n = SECP256k1.generator.order()
        r, s = util.sigdecode_string(sig, n)
        # The signature is valid for both s and -s, normalization ensures that only s < n // 2 is valid
        if s > (n // 2):
            mod_s = (s * -1) % n
            sig = util.sigencode_string(r, mod_s, n)
        return Signature(sig)

    @staticmethod
    def deserialize(deserializer: Deserializer) -> PrivateKey:
        key = deserializer.to_bytes()
        if len(key) != PrivateKey.LENGTH:
            raise Exception("Length mismatch")

        return PrivateKey(SigningKey.from_string(key, SECP256k1, hashlib.sha3_256))

    def serialize(self, serializer: Serializer):
        serializer.to_bytes(self.key.to_string())


class PublicKey(asymmetric_crypto.PublicKey):
    LENGTH: int = 64
    LENGTH_WITH_PREFIX_LENGTH: int = 65

    key: VerifyingKey

    def __init__(self, key: VerifyingKey):
        self.key = key

    def __eq__(self, other: object):
        if not isinstance(other, PublicKey):
            return NotImplemented
        return self.key == other.key

    def __str__(self) -> str:
        return self.hex()

    @staticmethod
    def from_str(value: str) -> PublicKey:
        if value[0:2] == "0x":
            value = value[2:]
        # We are measuring hex values which are twice the length of their binary counterpart.
        if (
            len(value) != PublicKey.LENGTH * 2
            and len(value) != PublicKey.LENGTH_WITH_PREFIX_LENGTH * 2
        ):
            raise Exception("Length mismatch")
        return PublicKey(
            VerifyingKey.from_string(bytes.fromhex(value), SECP256k1, hashlib.sha3_256)
        )

    def hex(self) -> str:
        return f"0x04{self.key.to_string().hex()}"

    def verify(self, data: bytes, signature: asymmetric_crypto.Signature) -> bool:
        try:
            signature = cast(Signature, signature)
            self.key.verify(signature.data(), data)
        except Exception:
            return False
        return True

    def to_crypto_bytes(self) -> bytes:
        return b"\x04" + self.key.to_string()

    @staticmethod
    def deserialize(deserializer: Deserializer) -> PublicKey:
        key = deserializer.to_bytes()
        if len(key) != PublicKey.LENGTH:
            # Some standards apply an extra byte to represent that this is a 64-byte key
            if len(key) == PublicKey.LENGTH_WITH_PREFIX_LENGTH:
                key = key[1:]
            else:
                raise Exception("Length mismatch")

        return PublicKey(VerifyingKey.from_string(key, SECP256k1, hashlib.sha3_256))

    def serialize(self, serializer: Serializer):
        serializer.to_bytes(self.to_crypto_bytes())


class Signature(asymmetric_crypto.Signature):
    LENGTH: int = 64

    signature: bytes

    def __init__(self, signature: bytes):
        self.signature = signature

    def __eq__(self, other: object):
        if not isinstance(other, Signature):
            return NotImplemented
        return self.signature == other.signature

    def __str__(self) -> str:
        return self.hex()

    def hex(self) -> str:
        return f"0x{self.signature.hex()}"

    @staticmethod
    def from_str(value: str) -> Signature:
        if value[0:2] == "0x":
            value = value[2:]
        if len(value) != Signature.LENGTH * 2:
            raise Exception("Length mismatch")
        return Signature(bytes.fromhex(value))

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


class Test(unittest.TestCase):
    def test_vectors(self):
        private_key_hex = (
            "0x306fa009600e27c09d2659145ce1785249360dd5fb992da01a578fe67ed607f4"
        )
        public_key_hex = "0x04210c9129e35337ff5d6488f90f18d842cf985f06e0baeff8df4bfb2ac4221863e2631b971a237b5db0aa71188e33250732dd461d56ee623cbe0426a5c2db79ef"
        signature_hex = "0xa539b0973e76fa99b2a864eebd5da950b4dfb399c7afe57ddb34130e454fc9db04dceb2c3d4260b8cc3d3952ab21b5d36c7dc76277fe3747764e6762d12bd9a9"
        data = b"Hello world"

        private_key = PrivateKey.from_str(private_key_hex)
        local_public_key = private_key.public_key()
        local_signature = private_key.sign(data)
        self.assertTrue(local_public_key.verify(data, local_signature))

        original_public_key = PublicKey.from_str(public_key_hex)
        self.assertTrue(original_public_key.verify(data, local_signature))
        self.assertEqual(public_key_hex[2:], local_public_key.to_crypto_bytes().hex())

        original_signature = Signature.from_str(signature_hex)
        self.assertTrue(original_public_key.verify(data, original_signature))

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

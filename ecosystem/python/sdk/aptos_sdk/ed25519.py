# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

import unittest
from typing import List, Tuple, cast

from nacl.signing import SigningKey, VerifyKey

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


class PublicKey(asymmetric_crypto.PublicKey):
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

    def verify(self, data: bytes, signature: asymmetric_crypto.Signature) -> bool:
        try:
            signature = cast(Signature, signature)
            self.key.verify(data, signature.data())
        except Exception:
            return False
        return True

    def to_crypto_bytes(self) -> bytes:
        return self.key.encode()

    @staticmethod
    def deserialize(deserializer: Deserializer) -> PublicKey:
        key = deserializer.to_bytes()
        if len(key) != PublicKey.LENGTH:
            raise Exception("Length mismatch")

        return PublicKey(VerifyKey(key))

    def serialize(self, serializer: Serializer):
        serializer.to_bytes(self.key.encode())


class MultiPublicKey(asymmetric_crypto.PublicKey):
    keys: List[PublicKey]
    threshold: int

    MIN_KEYS = 2
    MAX_KEYS = 32
    MIN_THRESHOLD = 1

    def __init__(self, keys: List[PublicKey], threshold: int):
        assert (
            self.MIN_KEYS <= len(keys) <= self.MAX_KEYS
        ), f"Must have between {self.MIN_KEYS} and {self.MAX_KEYS} keys."
        assert (
            self.MIN_THRESHOLD <= threshold <= len(keys)
        ), f"Threshold must be between {self.MIN_THRESHOLD} and {len(keys)}."

        self.keys = keys
        self.threshold = threshold

    def __str__(self) -> str:
        return f"{self.threshold}-of-{len(self.keys)} Multi-Ed25519 public key"

    def verify(self, data: bytes, signature: asymmetric_crypto.Signature) -> bool:
        try:
            signatures = cast(MultiSignature, signature)
            assert self.threshold <= len(
                signatures.signatures
            ), f"Insufficient signatures, {self.threshold} > {len(signatures.signatures)}"

            for idx, signature in signatures.signatures:
                assert (
                    len(self.keys) > idx
                ), f"Signature index exceeds available keys {len(self.keys)} < {idx}"
                assert self.keys[idx].verify(
                    data, signature
                ), "Unable to verify signature"
        except Exception:
            return False
        return True

    @staticmethod
    def from_crypto_bytes(indata: bytes) -> MultiPublicKey:
        total_keys = int(len(indata) / PublicKey.LENGTH)
        keys: List[PublicKey] = []
        for idx in range(total_keys):
            start = idx * PublicKey.LENGTH
            end = (idx + 1) * PublicKey.LENGTH
            keys.append(PublicKey(VerifyKey(indata[start:end])))
        threshold = indata[-1]
        return MultiPublicKey(keys, threshold)

    def to_crypto_bytes(self) -> bytes:
        key_bytes = bytearray()
        for key in self.keys:
            key_bytes.extend(key.to_crypto_bytes())
        key_bytes.append(self.threshold)
        return key_bytes

    @staticmethod
    def deserialize(deserializer: Deserializer) -> MultiPublicKey:
        indata = deserializer.to_bytes()
        return MultiPublicKey.from_crypto_bytes(indata)

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


class MultiSignature(asymmetric_crypto.Signature):
    signatures: List[Tuple[int, Signature]]
    BITMAP_NUM_OF_BYTES: int = 4

    def __init__(self, signatures: List[Tuple[int, Signature]]):
        for signature in signatures:
            assert (
                signature[0] < self.BITMAP_NUM_OF_BYTES * 8
            ), "bitmap value exceeds maximum value"
        self.signatures = signatures

    def __eq__(self, other: object):
        if not isinstance(other, MultiSignature):
            return NotImplemented
        return self.signatures == other.signatures

    def __str__(self) -> str:
        return f"{self.signatures}"

    @staticmethod
    def from_key_map(
        public_key: MultiPublicKey,
        signatures_map: List[Tuple[PublicKey, Signature]],
    ) -> MultiSignature:
        signatures = []

        for entry in signatures_map:
            signatures.append((public_key.keys.index(entry[0]), entry[1]))
        return MultiSignature(signatures)

    @staticmethod
    def deserialize(deserializer: Deserializer) -> MultiSignature:
        signature_bytes = deserializer.to_bytes()
        count = len(signature_bytes) // Signature.LENGTH
        assert count * Signature.LENGTH + MultiSignature.BITMAP_NUM_OF_BYTES == len(
            signature_bytes
        ), "MultiSignature length is invalid"

        bitmap = int.from_bytes(signature_bytes[-4:], "big")

        current = 0
        position = 0
        signatures = []
        while current < count:
            to_check = 1 << (31 - position)
            if to_check & bitmap:
                left = current * Signature.LENGTH
                signature = Signature(signature_bytes[left : left + Signature.LENGTH])
                signatures.append((position, signature))
                current += 1
            position += 1

        return MultiSignature(signatures)

    def serialize(self, serializer: Serializer):
        signature_bytes = bytearray()
        bitmap = 0

        for signature in self.signatures:
            shift = 31 - signature[0]
            bitmap = bitmap | (1 << shift)
            signature_bytes.extend(signature[1].data())

        signature_bytes.extend(
            bitmap.to_bytes(MultiSignature.BITMAP_NUM_OF_BYTES, "big")
        )
        serializer.to_bytes(signature_bytes)


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

    def test_multisig(self):
        # Generate signatory private keys.
        private_key_1 = PrivateKey.from_str(
            "4e5e3be60f4bbd5e98d086d932f3ce779ff4b58da99bf9e5241ae1212a29e5fe"
        )
        private_key_2 = PrivateKey.from_str(
            "1e70e49b78f976644e2c51754a2f049d3ff041869c669523ba95b172c7329901"
        )
        # Generate multisig public key with threshold of 1.
        multisig_public_key = MultiPublicKey(
            [private_key_1.public_key(), private_key_2.public_key()], 1
        )
        # Get public key BCS representation.
        serializer = Serializer()
        multisig_public_key.serialize(serializer)
        public_key_bcs = serializer.output().hex()
        # Check against expected BCS representation.
        expected_public_key_bcs = (
            "41754bb6a4720a658bdd5f532995955db0971ad3519acbde2f1149c3857348006c"
            "1634cd4607073f2be4a6f2aadc2b866ddb117398a675f2096ed906b20e0bf2c901"
        )
        self.assertEqual(public_key_bcs, expected_public_key_bcs)
        # Get public key bytes representation.
        public_key_bytes = multisig_public_key.to_bytes()
        # Convert back to multisig class instance from bytes.
        multisig_public_key = MultiPublicKey.from_bytes(public_key_bytes)
        # Get public key BCS representation.
        serializer = Serializer()
        multisig_public_key.serialize(serializer)
        public_key_bcs = serializer.output().hex()
        # Assert BCS representation is the same.
        self.assertEqual(public_key_bcs, expected_public_key_bcs)
        # Have one signer sign arbitrary message.
        signature = private_key_2.sign(b"multisig")
        # Compose multisig signature.
        multisig_signature = MultiSignature.from_key_map(
            multisig_public_key, [(private_key_2.public_key(), signature)]
        )
        # Get signature BCS representation.
        serializer = Serializer()
        multisig_signature.serialize(serializer)
        multisig_signature_bcs = serializer.output().hex()
        # Check against expected BCS representation.
        expected_multisig_signature_bcs = (
            "4402e90d8f300d79963cb7159ffa6f620f5bba4af5d32a7176bfb5480b43897cf"
            "4886bbb4042182f4647c9b04f02dbf989966f0facceec52d22bdcc7ce631bfc0c"
            "40000000"
        )
        self.assertEqual(multisig_signature_bcs, expected_multisig_signature_bcs)
        deserializer = Deserializer(bytes.fromhex(expected_multisig_signature_bcs))
        multisig_signature_deserialized = deserializer.struct(MultiSignature)
        self.assertEqual(multisig_signature_deserialized, multisig_signature)

        self.assertTrue(multisig_public_key.verify(b"multisig", multisig_signature))

    def test_multisig_range_checks(self):
        # Generate public keys.
        keys = [
            PrivateKey.random().public_key() for x in range(MultiPublicKey.MAX_KEYS + 1)
        ]
        # Verify failure for initializing multisig instance with too few keys.
        with self.assertRaisesRegex(AssertionError, "Must have between 2 and 32 keys."):
            MultiPublicKey([keys[0]], 1)
        # Verify failure for initializing multisig instance with too many keys.
        with self.assertRaisesRegex(AssertionError, "Must have between 2 and 32 keys."):
            MultiPublicKey(keys, 1)
        # Verify failure for initializing multisig instance with small threshold.
        with self.assertRaisesRegex(
            AssertionError, "Threshold must be between 1 and 4."
        ):
            MultiPublicKey(keys[0:4], 0)
        # Verify failure for initializing multisig instance with large threshold.
        with self.assertRaisesRegex(
            AssertionError, "Threshold must be between 1 and 4."
        ):
            MultiPublicKey(keys[0:4], 5)
        # Verify failure for initializing from bytes with too few keys.
        with self.assertRaisesRegex(AssertionError, "Must have between 2 and 32 keys."):
            MultiPublicKey.from_bytes(MultiPublicKey([keys[0]], 1).to_bytes())
        # Verify failure for initializing from bytes with too many keys.
        with self.assertRaisesRegex(AssertionError, "Must have between 2 and 32 keys."):
            MultiPublicKey.from_bytes(MultiPublicKey(keys, 1).to_bytes())
        # Verify failure for initializing from bytes with small threshold.
        with self.assertRaisesRegex(
            AssertionError, "Threshold must be between 1 and 4."
        ):
            MultiPublicKey.from_bytes(MultiPublicKey(keys[0:4], 0).to_bytes())
        # Verify failure for initializing from bytes with large threshold.
        with self.assertRaisesRegex(
            AssertionError, "Threshold must be between 1 and 4."
        ):
            MultiPublicKey.from_bytes(MultiPublicKey(keys[0:4], 5).to_bytes())

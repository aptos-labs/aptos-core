# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

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


class MultiPublicKey:
    keys: List[PublicKey]
    threshold: int

    MIN_KEYS = 2
    MAX_KEYS = 32
    MIN_THRESHOLD = 1

    def __init__(self, keys: List[PublicKey], threshold: int, checked=True):
        if checked:
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

    def to_bytes(self) -> bytes:
        concatenated_keys = bytes()
        for key in self.keys:
            concatenated_keys += key.key.encode()
        return concatenated_keys + bytes([self.threshold])

    @staticmethod
    def from_bytes(key: bytes) -> MultiPublicKey:
        # Get key count and threshold limits.
        min_keys = MultiPublicKey.MIN_KEYS
        max_keys = MultiPublicKey.MAX_KEYS
        min_threshold = MultiPublicKey.MIN_THRESHOLD
        # Get number of signers.
        n_signers = int(len(key) / PublicKey.LENGTH)
        assert (
            min_keys <= n_signers <= max_keys
        ), f"Must have between {min_keys} and {max_keys} keys."
        # Get threshold.
        threshold = int(key[-1])
        assert (
            min_threshold <= threshold <= n_signers
        ), f"Threshold must be between {min_threshold} and {n_signers}."
        keys = []  # Initialize empty keys list.
        for i in range(n_signers):  # Loop over all signers.
            # Extract public key for signle signer.
            start_byte = i * PublicKey.LENGTH
            end_byte = (i + 1) * PublicKey.LENGTH
            keys.append(PublicKey(VerifyKey(key[start_byte:end_byte])))
        return MultiPublicKey(keys, threshold)

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


class MultiSignature:
    signatures: List[Signature]
    bitmap: bytes

    def __init__(
        self,
        public_key: MultiPublicKey,
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
        multisig_signature = MultiSignature(
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
            MultiPublicKey.from_bytes(
                MultiPublicKey([keys[0]], 1, checked=False).to_bytes()
            )
        # Verify failure for initializing from bytes with too many keys.
        with self.assertRaisesRegex(AssertionError, "Must have between 2 and 32 keys."):
            MultiPublicKey.from_bytes(MultiPublicKey(keys, 1, checked=False).to_bytes())
        # Verify failure for initializing from bytes with small threshold.
        with self.assertRaisesRegex(
            AssertionError, "Threshold must be between 1 and 4."
        ):
            MultiPublicKey.from_bytes(
                MultiPublicKey(keys[0:4], 0, checked=False).to_bytes()
            )
        # Verify failure for initializing from bytes with large threshold.
        with self.assertRaisesRegex(
            AssertionError, "Threshold must be between 1 and 4."
        ):
            MultiPublicKey.from_bytes(
                MultiPublicKey(keys[0:4], 5, checked=False).to_bytes()
            )

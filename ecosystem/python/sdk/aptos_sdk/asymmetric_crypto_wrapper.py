# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

from . import asymmetric_crypto, ed25519, secp256k1_ecdsa
from .bcs import Deserializer, Serializer


class PublicKey(asymmetric_crypto.PublicKey):
    ED25519: int = 0
    SECP256K1_ECDSA: int = 1

    variant: int
    public_key: asymmetric_crypto.PublicKey

    def __init__(self, public_key: asymmetric_crypto.PublicKey):
        if isinstance(public_key, ed25519.PublicKey):
            self.variant = PublicKey.ED25519
        elif isinstance(public_key, secp256k1_ecdsa.PublicKey):
            self.variant = PublicKey.SECP256K1_ECDSA
        else:
            raise NotImplementedError()
        self.public_key = public_key

    def to_crypto_bytes(self) -> bytes:
        ser = Serializer()
        self.serialize(ser)
        return ser.output()

    def verify(self, data: bytes, signature: asymmetric_crypto.Signature) -> bool:
        return self.public_key.verify(data, signature)

    @staticmethod
    def deserialize(deserializer: Deserializer) -> PublicKey:
        variant = deserializer.uleb128()

        if variant == PublicKey.ED25519:
            public_key: asymmetric_crypto.PublicKey = ed25519.PublicKey.deserialize(
                deserializer
            )
        elif variant == Signature.SECP256K1_ECDSA:
            public_key = secp256k1_ecdsa.PublicKey.deserialize(deserializer)
        else:
            raise Exception(f"Invalid type: {variant}")

        return PublicKey(public_key)

    def serialize(self, serializer: Serializer):
        serializer.uleb128(self.variant)
        serializer.struct(self.public_key)


class Signature(asymmetric_crypto.Signature):
    ED25519: int = 0
    SECP256K1_ECDSA: int = 1

    variant: int
    signature: asymmetric_crypto.Signature

    def __init__(self, signature: asymmetric_crypto.Signature):
        if isinstance(signature, ed25519.Signature):
            self.variant = Signature.ED25519
        elif isinstance(signature, secp256k1_ecdsa.Signature):
            self.variant = Signature.SECP256K1_ECDSA
        else:
            raise NotImplementedError()
        self.signature = signature

    @staticmethod
    def deserialize(deserializer: Deserializer) -> Signature:
        variant = deserializer.uleb128()

        if variant == Signature.ED25519:
            signature: asymmetric_crypto.Signature = ed25519.Signature.deserialize(
                deserializer
            )
        elif variant == Signature.SECP256K1_ECDSA:
            signature = secp256k1_ecdsa.Signature.deserialize(deserializer)
        else:
            raise Exception(f"Invalid type: {variant}")

        return Signature(signature)

    def serialize(self, serializer: Serializer):
        serializer.uleb128(self.variant)
        serializer.struct(self.signature)

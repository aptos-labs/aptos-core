# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations
import io
import typing
import unittest

from account_address import AccountAddress
from bcs import Deserializer, Serializer
import ed25519


class Authenticator:
    """
    Each transaction submitted to the Aptos blockchain contains a `TransactionAuthenticator`.
    During transaction execution, the executor will check if every `AccountAuthenticator`'s
    signature on the transaction hash is well-formed and whether the sha3 hash of the
    `AccountAuthenticator`'s `AuthenticationKeyPreimage` matches the `AuthenticationKey` stored
    under the participating signer's account address.
    """

    ED25519: int = 0
    MULTI_ED25519: int = 1
    MULTI_AGENT: int = 2

    variant: int
    authenticator: typing.Any

    def __init__(self, authenticator: typing.Any):
        if isinstance(authenticator, Ed25519Authenticator):
            self.variant = Authenticator.ED25519
        elif isinstance(authenticator, MultiEd25519Authenticator):
            self.variant = Authenticator.MULTI_ED25519
        elif isinstance(authenticator, MultiAgentAuthenticator):
            self.variant = Authenticator.MULTI_AGENT
        else:
            raise Exception("Invalid type")
        self.authenticator = authenticator

    def __eq__(self, other: Authenticator) -> bool:
        return (
            self.variant == other.variant and self.authenticator == other.authenticator
        )

    def __str__(self) -> str:
        return self.authenticator.__str__()

    def verify(self, data: bytes) -> bool:
        return self.authenticator.verify(data)

    def deserialize(deserializer: Deserializer) -> Authenticator:
        variant = deserializer.uleb128()

        if variant == Authenticator.ED25519:
            authenticator = Ed25519Authenticator.deserialize(deserializer)
        elif variant == Authenticator.MULTI_ED25519:
            authenticator = MultiEd25519Authenticator.deserialize(deserializer)
        elif variant == Authenticator.MULTI_AGENT:
            authenticator = MultiAgentAuthenticator.deserialize(deserializer)
        else:
            raise Exception("Invalid type")

        return Authenticator(authenticator)

    def serialize(self, serializer: Serializer):
        serializer.uleb128(self.variant)
        self.authenticator.serialize(serializer)


class Ed25519Authenticator:
    public_key: ed25519.PublicKey
    signature: ed25519.Signature

    def __init__(self, public_key: ed25519.PublicKey, signature: ed25519.Signature):
        self.public_key = public_key
        self.signature = signature

    def __eq__(self, other: Ed25519Authenticator) -> bool:
        return self.public_key == other.public_key and self.signature == other.signature

    def __str__(self) -> str:
        return f"PublicKey: {self.public_key}, Signature: {self.signature}"

    def verify(self, data: bytes) -> bool:
        return self.public_key.verify(data, self.signature)

    def deserialize(deserializer: Deserializer) -> Ed25519Authenticator:
        key = deserializer.struct(ed25519.PublicKey)
        signature = deserializer.struct(ed25519.Signature)
        return Ed25519Authenticator(key, signature)

    def serialize(self, serializer: Serializer):
        self.public_key.serialize(serializer)
        self.signature.serialize(serializer)


class MultiAgentAuthenticator:
    def __init__(self):
        raise NotImplementedError

    def verify(self, data: bytes) -> bool:
        raise NotImplementedError

    def deserialize(deserializer: Deserializer) -> MultiEd25519Authenticator:
        raise NotImplementedError

    def serialize(self, serializer: Serializer):
        raise NotImplementedError


class MultiEd25519Authenticator:
    def __init__(self):
        raise NotImplementedError

    def verify(self, data: bytes) -> bool:
        raise NotImplementedError

    def deserialize(deserializer: Deserializer) -> MultiEd25519Authenticator:
        raise NotImplementedError

    def serialize(self, serializer: Serializer):
        raise NotImplementedError

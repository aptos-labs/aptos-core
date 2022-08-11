# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

import typing

from . import ed25519
from .account_address import AccountAddress
from .bcs import Deserializer, Serializer


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
        serializer.struct(self.authenticator)


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
        serializer.struct(self.public_key)
        serializer.struct(self.signature)


class MultiAgentAuthenticator:
    sender: Authenticator
    secondary_signers: List[(AccountAddress, Authenticator)]

    def __init__(
        self,
        sender: Authenticator,
        secondary_signers: List[(AccountAddress, Authenticator)],
    ):
        self.sender = sender
        self.secondary_signers = secondary_signers

    def __eq__(self, other: MultiAgentAuthenticator) -> bool:
        return (
            self.sender == other.sender
            and self.secondary_signers == other.secondary_signers
        )

    def secondary_addresses(self) -> List[AccountAddress]:
        return [x[0] for x in self.secondary_signers]

    def verify(self, data: bytes) -> bool:
        if not self.sender.verify(data):
            return False
        return all([x[1].verify(data) for x in self.secondary_signers])

    def deserialize(deserializer: Deserializer) -> MultiAgentAuthenticator:
        sender = deserializer.struct(Authenticator)
        secondary_addresses = deserializer.sequence(AccountAddress.deserialize)
        secondary_authenticators = deserializer.sequence(Authenticator.deserialize)
        return MultiAgentAuthenticator(
            sender, list(zip(secondary_addresses, secondary_authenticators))
        )

    def serialize(self, serializer: Serializer):
        serializer.struct(self.sender)
        serializer.sequence([x[0] for x in self.secondary_signers], Serializer.struct)
        serializer.sequence([x[1] for x in self.secondary_signers], Serializer.struct)


class MultiEd25519Authenticator:
    def __init__(self):
        raise NotImplementedError

    def verify(self, data: bytes) -> bool:
        raise NotImplementedError

    def deserialize(deserializer: Deserializer) -> MultiEd25519Authenticator:
        raise NotImplementedError

    def serialize(self, serializer: Serializer):
        raise NotImplementedError

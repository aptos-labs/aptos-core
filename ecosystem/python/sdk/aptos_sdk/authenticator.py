# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

import typing
from typing import List

from . import asymmetric_crypto, asymmetric_crypto_wrapper, ed25519
from .account_address import AccountAddress
from .bcs import Deserializer, Serializer


class Authenticator:
    """
    Each transaction submitted to the Aptos blockchain contains a `TransactionAuthenticator`.
    During transaction execution, the executor will check if every `AccountAuthenticator`'s
    signature on the transaction hash is well-formed and whether `AccountAuthenticator`'s  matches
    the `AuthenticationKey` stored under the participating signer's account address.
    """

    ED25519: int = 0
    MULTI_ED25519: int = 1
    MULTI_AGENT: int = 2
    FEE_PAYER: int = 3
    SINGLE_SENDER: int = 4

    variant: int
    authenticator: typing.Any

    def __init__(self, authenticator: typing.Any):
        if isinstance(authenticator, Ed25519Authenticator):
            self.variant = Authenticator.ED25519
        elif isinstance(authenticator, MultiEd25519Authenticator):
            self.variant = Authenticator.MULTI_ED25519
        elif isinstance(authenticator, MultiAgentAuthenticator):
            self.variant = Authenticator.MULTI_AGENT
        elif isinstance(authenticator, FeePayerAuthenticator):
            self.variant = Authenticator.FEE_PAYER
        elif isinstance(authenticator, SingleSenderAuthenticator):
            self.variant = Authenticator.SINGLE_SENDER
        else:
            raise Exception("Invalid type")
        self.authenticator = authenticator

    def from_key(key: asymmetric_crypto.PublicKey) -> int:
        if isinstance(key, ed25519.PublicKey):
            return Authenticator.ED25519
        elif isinstance(key, ed25519.MultiPublicKey):
            return Authenticator.MULTI_ED25519
        else:
            raise NotImplementedError()

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, Authenticator):
            return NotImplemented
        return (
            self.variant == other.variant and self.authenticator == other.authenticator
        )

    def __str__(self) -> str:
        return self.authenticator.__str__()

    def verify(self, data: bytes) -> bool:
        return self.authenticator.verify(data)

    @staticmethod
    def deserialize(deserializer: Deserializer) -> Authenticator:
        variant = deserializer.uleb128()

        if variant == Authenticator.ED25519:
            authenticator: typing.Any = Ed25519Authenticator.deserialize(deserializer)
        elif variant == Authenticator.MULTI_ED25519:
            authenticator = MultiEd25519Authenticator.deserialize(deserializer)
        elif variant == Authenticator.MULTI_AGENT:
            authenticator = MultiAgentAuthenticator.deserialize(deserializer)
        elif variant == Authenticator.FEE_PAYER:
            authenticator = FeePayerAuthenticator.deserialize(deserializer)
        elif variant == Authenticator.SINGLE_SENDER:
            authenticator = SingleSenderAuthenticator.deserialize(deserializer)
        else:
            raise Exception(f"Invalid type: {variant}")

        return Authenticator(authenticator)

    def serialize(self, serializer: Serializer):
        serializer.uleb128(self.variant)
        serializer.struct(self.authenticator)


class AccountAuthenticator:
    ED25519: int = 0
    MULTI_ED25519: int = 1
    SINGLE_KEY: int = 2
    MULTI_KEY: int = 3

    variant: int
    authenticator: typing.Any

    def __init__(self, authenticator: typing.Any):
        if isinstance(authenticator, Ed25519Authenticator):
            self.variant = AccountAuthenticator.ED25519
        elif isinstance(authenticator, MultiEd25519Authenticator):
            self.variant = AccountAuthenticator.MULTI_ED25519
        elif isinstance(authenticator, SingleKeyAuthenticator):
            self.variant = AccountAuthenticator.SINGLE_KEY
        else:
            raise Exception("Invalid type")
        self.authenticator = authenticator

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, AccountAuthenticator):
            return NotImplemented
        return (
            self.variant == other.variant and self.authenticator == other.authenticator
        )

    def __repr__(self) -> str:
        return self.__str__()

    def __str__(self) -> str:
        return self.authenticator.__str__()

    def verify(self, data: bytes) -> bool:
        return self.authenticator.verify(data)

    @staticmethod
    def deserialize(deserializer: Deserializer) -> AccountAuthenticator:
        variant = deserializer.uleb128()

        if variant == AccountAuthenticator.ED25519:
            authenticator: typing.Any = Ed25519Authenticator.deserialize(deserializer)
        elif variant == AccountAuthenticator.MULTI_ED25519:
            authenticator = MultiEd25519Authenticator.deserialize(deserializer)
        elif variant == AccountAuthenticator.SINGLE_KEY:
            authenticator = SingleKeyAuthenticator.deserialize(deserializer)
        else:
            raise Exception(f"Invalid type: {variant}")

        return AccountAuthenticator(authenticator)

    def serialize(self, serializer: Serializer):
        serializer.uleb128(self.variant)
        serializer.struct(self.authenticator)


class Ed25519Authenticator:
    public_key: ed25519.PublicKey
    signature: ed25519.Signature

    def __init__(self, public_key: ed25519.PublicKey, signature: ed25519.Signature):
        self.public_key = public_key
        self.signature = signature

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, Ed25519Authenticator):
            return NotImplemented

        return self.public_key == other.public_key and self.signature == other.signature

    def __str__(self) -> str:
        return f"PublicKey: {self.public_key}, Signature: {self.signature}"

    def verify(self, data: bytes) -> bool:
        return self.public_key.verify(data, self.signature)

    @staticmethod
    def deserialize(deserializer: Deserializer) -> Ed25519Authenticator:
        key = deserializer.struct(ed25519.PublicKey)
        signature = deserializer.struct(ed25519.Signature)
        return Ed25519Authenticator(key, signature)

    def serialize(self, serializer: Serializer):
        serializer.struct(self.public_key)
        serializer.struct(self.signature)


class FeePayerAuthenticator:
    sender: AccountAuthenticator
    secondary_signers: List[typing.Tuple[AccountAddress, AccountAuthenticator]]
    fee_payer: typing.Tuple[AccountAddress, AccountAuthenticator]

    def __init__(
        self,
        sender: AccountAuthenticator,
        secondary_signers: List[typing.Tuple[AccountAddress, AccountAuthenticator]],
        fee_payer: typing.Tuple[AccountAddress, AccountAuthenticator],
    ):
        self.sender = sender
        self.secondary_signers = secondary_signers
        self.fee_payer = fee_payer

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, FeePayerAuthenticator):
            return NotImplemented
        return (
            self.sender == other.sender
            and self.secondary_signers == other.secondary_signers
            and self.fee_payer == other.fee_payer
        )

    def __str__(self) -> str:
        return f"FeePayer: \n\tSender: {self.sender}\n\tSecondary Signers: {self.secondary_signers}\n\t{self.fee_payer}"

    def fee_payer_address(self) -> AccountAddress:
        return self.fee_payer[0]

    def secondary_addresses(self) -> List[AccountAddress]:
        return [x[0] for x in self.secondary_signers]

    def verify(self, data: bytes) -> bool:
        if not self.sender.verify(data):
            return False
        if not self.fee_payer[1].verify(data):
            return False
        return all([x[1].verify(data) for x in self.secondary_signers])

    @staticmethod
    def deserialize(deserializer: Deserializer) -> FeePayerAuthenticator:
        sender = deserializer.struct(AccountAuthenticator)
        secondary_addresses = deserializer.sequence(AccountAddress.deserialize)
        secondary_authenticators = deserializer.sequence(
            AccountAuthenticator.deserialize
        )
        fee_payer_address = deserializer.struct(AccountAddress)
        fee_payer_authenticator = deserializer.struct(AccountAuthenticator)
        return FeePayerAuthenticator(
            sender,
            list(zip(secondary_addresses, secondary_authenticators)),
            (fee_payer_address, fee_payer_authenticator),
        )

    def serialize(self, serializer: Serializer):
        serializer.struct(self.sender)
        serializer.sequence([x[0] for x in self.secondary_signers], Serializer.struct)
        serializer.sequence([x[1] for x in self.secondary_signers], Serializer.struct)
        serializer.struct(self.fee_payer[0])
        serializer.struct(self.fee_payer[1])


class MultiAgentAuthenticator:
    sender: AccountAuthenticator
    secondary_signers: List[typing.Tuple[AccountAddress, AccountAuthenticator]]

    def __init__(
        self,
        sender: AccountAuthenticator,
        secondary_signers: List[typing.Tuple[AccountAddress, AccountAuthenticator]],
    ):
        self.sender = sender
        self.secondary_signers = secondary_signers

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, MultiAgentAuthenticator):
            return NotImplemented
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

    @staticmethod
    def deserialize(deserializer: Deserializer) -> MultiAgentAuthenticator:
        sender = deserializer.struct(AccountAuthenticator)
        secondary_addresses = deserializer.sequence(AccountAddress.deserialize)
        secondary_authenticators = deserializer.sequence(
            AccountAuthenticator.deserialize
        )
        return MultiAgentAuthenticator(
            sender, list(zip(secondary_addresses, secondary_authenticators))
        )

    def serialize(self, serializer: Serializer):
        serializer.struct(self.sender)
        serializer.sequence([x[0] for x in self.secondary_signers], Serializer.struct)
        serializer.sequence([x[1] for x in self.secondary_signers], Serializer.struct)


class MultiEd25519Authenticator:
    public_key: ed25519.MultiPublicKey
    signature: ed25519.MultiSignature

    def __init__(self, public_key, signature):
        self.public_key = public_key
        self.signature = signature

    def verify(self, data: bytes) -> bool:
        raise NotImplementedError

    @staticmethod
    def deserialize(deserializer: Deserializer) -> MultiEd25519Authenticator:
        raise NotImplementedError

    def serialize(self, serializer: Serializer):
        serializer.struct(self.public_key)
        serializer.struct(self.signature)


class SingleSenderAuthenticator:
    sender: AccountAuthenticator

    def __init__(
        self,
        sender: AccountAuthenticator,
    ):
        self.sender = sender

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, SingleSenderAuthenticator):
            return NotImplemented
        return self.sender == other.sender

    def verify(self, data: bytes) -> bool:
        return self.sender.verify(data)

    @staticmethod
    def deserialize(deserializer: Deserializer) -> SingleSenderAuthenticator:
        sender = deserializer.struct(AccountAuthenticator)
        return SingleSenderAuthenticator(sender)

    def serialize(self, serializer: Serializer):
        serializer.struct(self.sender)


class SingleKeyAuthenticator:
    public_key: asymmetric_crypto_wrapper.PublicKey
    signature: asymmetric_crypto_wrapper.Signature

    def __init__(
        self,
        public_key: asymmetric_crypto.PublicKey,
        signature: asymmetric_crypto.Signature,
    ):
        if isinstance(public_key, asymmetric_crypto_wrapper.PublicKey):
            self.public_key = public_key
        else:
            self.public_key = asymmetric_crypto_wrapper.PublicKey(public_key)

        if isinstance(signature, asymmetric_crypto_wrapper.Signature):
            self.signature = signature
        else:
            self.signature = asymmetric_crypto_wrapper.Signature(signature)

    def verify(self, data: bytes) -> bool:
        return self.public_key.verify(data, self.signature.signature)

    @staticmethod
    def deserialize(deserializer: Deserializer) -> SingleKeyAuthenticator:
        public_key = deserializer.struct(asymmetric_crypto_wrapper.PublicKey)
        signature = deserializer.struct(asymmetric_crypto_wrapper.Signature)
        return SingleKeyAuthenticator(public_key, signature)

    def serialize(self, serializer: Serializer):
        serializer.struct(self.public_key)
        serializer.struct(self.signature)

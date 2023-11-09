# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

from typing_extensions import Protocol

from .bcs import Deserializable, Serializable


class PrivateKey(Deserializable, Serializable, Protocol):
    def hex(self) -> str:
        ...

    def public_key(self) -> PublicKey:
        ...

    def sign(self, data: bytes) -> Signature:
        ...


class PublicKey(Deserializable, Serializable, Protocol):
    def to_crypto_bytes(self) -> bytes:
        """
        A long time ago, someone decided that we should have both bcs and a special representation
        for MultiEd25519, so we use this to let keys self-define a special encoding.
        """
        ...

    def verify(self, data: bytes, signature: Signature) -> bool:
        ...


class Signature(Deserializable, Serializable, Protocol):
    ...

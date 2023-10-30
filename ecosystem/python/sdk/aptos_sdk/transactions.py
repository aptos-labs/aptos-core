# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""
This translates Aptos transactions to and from BCS for signing and submitting to the REST API.
"""

from __future__ import annotations

import hashlib
import unittest
from typing import Any, Callable, List, Optional, Union, cast

from typing_extensions import Protocol

from . import asymmetric_crypto, ed25519, secp256k1_ecdsa
from .account_address import AccountAddress
from .authenticator import (
    AccountAuthenticator,
    Authenticator,
    Ed25519Authenticator,
    FeePayerAuthenticator,
    MultiAgentAuthenticator,
    SingleKeyAuthenticator,
    SingleSenderAuthenticator,
)
from .bcs import Deserializer, Serializer
from .type_tag import StructTag, TypeTag


class RawTransactionInternal(Protocol):
    def keyed(self) -> bytes:
        ser = Serializer()
        self.serialize(ser)
        prehash = bytearray(self.prehash())
        prehash.extend(ser.output())
        return bytes(prehash)

    def prehash(self) -> bytes:
        ...

    def serialize(self, ser: Serializer):
        ...

    def sign(self, key: asymmetric_crypto.PrivateKey) -> AccountAuthenticator:
        signature = key.sign(self.keyed())
        if isinstance(signature, ed25519.Signature):
            return AccountAuthenticator(
                Ed25519Authenticator(
                    cast(ed25519.PublicKey, key.public_key()), signature
                )
            )
        return AccountAuthenticator(SingleKeyAuthenticator(key.public_key(), signature))

    def sign_simulated(self, key: asymmetric_crypto.PublicKey) -> AccountAuthenticator:
        if isinstance(key, ed25519.PublicKey):
            return AccountAuthenticator(
                Ed25519Authenticator(key, ed25519.Signature(b"\x00" * 64))
            )
        elif isinstance(key, secp256k1_ecdsa.PublicKey):
            return AccountAuthenticator(
                SingleKeyAuthenticator(key, secp256k1_ecdsa.Signature(b"\x00" * 64))
            )
        else:
            raise NotImplementedError()

    def verify(self, key: ed25519.PublicKey, signature: ed25519.Signature) -> bool:
        return key.verify(self.keyed(), signature)


class RawTransactionWithData(RawTransactionInternal, Protocol):
    raw_transaction: RawTransaction

    def inner(self) -> RawTransaction:
        return self.raw_transaction

    def prehash(self) -> bytes:
        hasher = hashlib.sha3_256()
        hasher.update(b"APTOS::RawTransactionWithData")
        return hasher.digest()


class RawTransaction(RawTransactionInternal):
    # Sender's address
    sender: AccountAddress
    # Sequence number of this transaction. This must match the sequence number in the sender's
    # account at the time of execution.
    sequence_number: int
    # The transaction payload, e.g., a script to execute.
    payload: TransactionPayload
    # Maximum total gas to spend for this transaction
    max_gas_amount: int
    # Price to be paid per gas unit.
    gas_unit_price: int
    # Expiration timestamp for this transaction, represented as seconds from the Unix epoch.
    expiration_timestamps_secs: int
    # Chain ID of the Aptos network this transaction is intended for.
    chain_id: int

    def __init__(
        self,
        sender: AccountAddress,
        sequence_number: int,
        payload: TransactionPayload,
        max_gas_amount: int,
        gas_unit_price: int,
        expiration_timestamps_secs: int,
        chain_id: int,
    ):
        self.sender = sender
        self.sequence_number = sequence_number
        self.payload = payload
        self.max_gas_amount = max_gas_amount
        self.gas_unit_price = gas_unit_price
        self.expiration_timestamps_secs = expiration_timestamps_secs
        self.chain_id = chain_id

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, RawTransaction):
            return NotImplemented
        return (
            self.sender == other.sender
            and self.sequence_number == other.sequence_number
            and self.payload == other.payload
            and self.max_gas_amount == other.max_gas_amount
            and self.gas_unit_price == other.gas_unit_price
            and self.expiration_timestamps_secs == other.expiration_timestamps_secs
            and self.chain_id == other.chain_id
        )

    def __str__(self):
        return f"""RawTransaction:
    sender: {self.sender}
    sequence_number: {self.sequence_number}
    payload: {self.payload}
    max_gas_amount: {self.max_gas_amount}
    gas_unit_price: {self.gas_unit_price}
    expiration_timestamps_secs: {self.expiration_timestamps_secs}
    chain_id: {self.chain_id}
"""

    def prehash(self) -> bytes:
        hasher = hashlib.sha3_256()
        hasher.update(b"APTOS::RawTransaction")
        return hasher.digest()

    @staticmethod
    def deserialize(deserializer: Deserializer) -> RawTransaction:
        return RawTransaction(
            AccountAddress.deserialize(deserializer),
            deserializer.u64(),
            TransactionPayload.deserialize(deserializer),
            deserializer.u64(),
            deserializer.u64(),
            deserializer.u64(),
            deserializer.u8(),
        )

    def serialize(self, serializer: Serializer):
        self.sender.serialize(serializer)
        serializer.u64(self.sequence_number)
        self.payload.serialize(serializer)
        serializer.u64(self.max_gas_amount)
        serializer.u64(self.gas_unit_price)
        serializer.u64(self.expiration_timestamps_secs)
        serializer.u8(self.chain_id)


class MultiAgentRawTransaction(RawTransactionWithData):
    secondary_signers: List[AccountAddress]

    def __init__(
        self, raw_transaction: RawTransaction, secondary_signers: List[AccountAddress]
    ):
        self.raw_transaction = raw_transaction
        self.secondary_signers = secondary_signers

    def serialize(self, serializer: Serializer):
        # This is a type indicator for an enum
        serializer.u8(0)
        serializer.struct(self.raw_transaction)
        serializer.sequence(self.secondary_signers, Serializer.struct)


class FeePayerRawTransaction(RawTransactionWithData):
    secondary_signers: List[AccountAddress]
    fee_payer: Optional[AccountAddress]

    def __init__(
        self,
        raw_transaction: RawTransaction,
        secondary_signers: List[AccountAddress],
        fee_payer: Optional[AccountAddress],
    ):
        self.raw_transaction = raw_transaction
        self.secondary_signers = secondary_signers
        self.fee_payer = fee_payer

    def serialize(self, serializer: Serializer):
        serializer.u8(1)
        serializer.struct(self.raw_transaction)
        serializer.sequence(self.secondary_signers, Serializer.struct)
        fee_payer = (
            AccountAddress.from_str("0x0") if self.fee_payer is None else self.fee_payer
        )
        serializer.struct(fee_payer)


class TransactionPayload:
    SCRIPT: int = 0
    MODULE_BUNDLE: int = 1
    SCRIPT_FUNCTION: int = 2

    variant: int
    value: Any

    def __init__(self, payload: Any):
        if isinstance(payload, Script):
            self.variant = TransactionPayload.SCRIPT
        elif isinstance(payload, ModuleBundle):
            self.variant = TransactionPayload.MODULE_BUNDLE
        elif isinstance(payload, EntryFunction):
            self.variant = TransactionPayload.SCRIPT_FUNCTION
        else:
            raise Exception("Invalid type")
        self.value = payload

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, TransactionPayload):
            return NotImplemented
        return self.variant == other.variant and self.value == other.value

    def __str__(self) -> str:
        return self.value.__str__()

    @staticmethod
    def deserialize(deserializer: Deserializer) -> TransactionPayload:
        variant = deserializer.uleb128()

        if variant == TransactionPayload.SCRIPT:
            payload: Any = Script.deserialize(deserializer)
        elif variant == TransactionPayload.MODULE_BUNDLE:
            payload = ModuleBundle.deserialize(deserializer)
        elif variant == TransactionPayload.SCRIPT_FUNCTION:
            payload = EntryFunction.deserialize(deserializer)
        else:
            raise Exception("Invalid type")

        return TransactionPayload(payload)

    def serialize(self, serializer: Serializer):
        serializer.uleb128(self.variant)
        self.value.serialize(serializer)


class ModuleBundle:
    def __init__(self):
        raise NotImplementedError

    @staticmethod
    def deserialize(deserializer: Deserializer) -> ModuleBundle:
        raise NotImplementedError

    def serialize(self, serializer: Serializer):
        raise NotImplementedError


class Script:
    code: bytes
    ty_args: List[TypeTag]
    args: List[ScriptArgument]

    def __init__(self, code: bytes, ty_args: List[TypeTag], args: List[ScriptArgument]):
        self.code = code
        self.ty_args = ty_args
        self.args = args

    @staticmethod
    def deserialize(deserializer: Deserializer) -> Script:
        code = deserializer.to_bytes()
        ty_args = deserializer.sequence(TypeTag.deserialize)
        args = deserializer.sequence(ScriptArgument.deserialize)
        return Script(code, ty_args, args)

    def serialize(self, serializer: Serializer):
        serializer.to_bytes(self.code)
        serializer.sequence(self.ty_args, Serializer.struct)
        serializer.sequence(self.args, Serializer.struct)

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, Script):
            return NotImplemented
        return (
            self.code == other.code
            and self.ty_args == other.ty_args
            and self.args == other.args
        )

    def __str__(self):
        return f"<{self.ty_args}>({self.args})"


class ScriptArgument:
    U8: int = 0
    U64: int = 1
    U128: int = 2
    ADDRESS: int = 3
    U8_VECTOR: int = 4
    BOOL: int = 5
    U16: int = 6
    U32: int = 7
    U256: int = 8

    variant: int
    value: Any

    def __init__(self, variant: int, value: Any):
        if variant < 0 or variant > 5:
            raise Exception("Invalid variant")

        self.variant = variant
        self.value = value

    @staticmethod
    def deserialize(deserializer: Deserializer) -> ScriptArgument:
        variant = deserializer.u8()
        if variant == ScriptArgument.U8:
            value: Any = deserializer.u8()
        elif variant == ScriptArgument.U16:
            value = deserializer.u16()
        elif variant == ScriptArgument.U32:
            value = deserializer.u32()
        elif variant == ScriptArgument.U64:
            value = deserializer.u64()
        elif variant == ScriptArgument.U128:
            value = deserializer.u128()
        elif variant == ScriptArgument.U256:
            value = deserializer.u256()
        elif variant == ScriptArgument.ADDRESS:
            value = AccountAddress.deserialize(deserializer)
        elif variant == ScriptArgument.U8_VECTOR:
            value = deserializer.to_bytes()
        elif variant == ScriptArgument.BOOL:
            value = deserializer.bool()
        else:
            raise Exception("Invalid variant")
        return ScriptArgument(variant, value)

    def serialize(self, serializer: Serializer):
        serializer.u8(self.variant)
        if self.variant == ScriptArgument.U8:
            serializer.u8(self.value)
        elif self.variant == ScriptArgument.U16:
            serializer.u16(self.value)
        elif self.variant == ScriptArgument.U32:
            serializer.u32(self.value)
        elif self.variant == ScriptArgument.U64:
            serializer.u64(self.value)
        elif self.variant == ScriptArgument.U128:
            serializer.u128(self.value)
        elif self.variant == ScriptArgument.U256:
            serializer.u256(self.value)
        elif self.variant == ScriptArgument.ADDRESS:
            serializer.struct(self.value)
        elif self.variant == ScriptArgument.U8_VECTOR:
            serializer.to_bytes(self.value)
        elif self.variant == ScriptArgument.BOOL:
            serializer.bool(self.value)
        else:
            raise Exception(f"Invalid ScriptArgument variant {self.variant}")

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, ScriptArgument):
            return NotImplemented
        return self.variant == other.variant and self.value == other.value

    def __str__(self):
        return f"[{self.variant}] {self.value}"


class EntryFunction:
    module: ModuleId
    function: str
    ty_args: List[TypeTag]
    args: List[bytes]

    def __init__(
        self, module: ModuleId, function: str, ty_args: List[TypeTag], args: List[bytes]
    ):
        self.module = module
        self.function = function
        self.ty_args = ty_args
        self.args = args

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, EntryFunction):
            return NotImplemented

        return (
            self.module == other.module
            and self.function == other.function
            and self.ty_args == other.ty_args
            and self.args == other.args
        )

    def __str__(self):
        return f"{self.module}::{self.function}::<{self.ty_args}>({self.args})"

    @staticmethod
    def natural(
        module: str,
        function: str,
        ty_args: List[TypeTag],
        args: List[TransactionArgument],
    ) -> EntryFunction:
        module_id = ModuleId.from_str(module)

        byte_args = []
        for arg in args:
            byte_args.append(arg.encode())
        return EntryFunction(module_id, function, ty_args, byte_args)

    @staticmethod
    def deserialize(deserializer: Deserializer) -> EntryFunction:
        module = ModuleId.deserialize(deserializer)
        function = deserializer.str()
        ty_args = deserializer.sequence(TypeTag.deserialize)
        args = deserializer.sequence(Deserializer.to_bytes)
        return EntryFunction(module, function, ty_args, args)

    def serialize(self, serializer: Serializer):
        self.module.serialize(serializer)
        serializer.str(self.function)
        serializer.sequence(self.ty_args, Serializer.struct)
        serializer.sequence(self.args, Serializer.to_bytes)


class ModuleId:
    address: AccountAddress
    name: str

    def __init__(self, address: AccountAddress, name: str):
        self.address = address
        self.name = name

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, ModuleId):
            return NotImplemented
        return self.address == other.address and self.name == other.name

    def __str__(self) -> str:
        return f"{self.address}::{self.name}"

    @staticmethod
    def from_str(module_id: str) -> ModuleId:
        split = module_id.split("::")
        return ModuleId(AccountAddress.from_str(split[0]), split[1])

    @staticmethod
    def deserialize(deserializer: Deserializer) -> ModuleId:
        addr = AccountAddress.deserialize(deserializer)
        name = deserializer.str()
        return ModuleId(addr, name)

    def serialize(self, serializer: Serializer):
        self.address.serialize(serializer)
        serializer.str(self.name)


class TransactionArgument:
    value: Any
    encoder: Callable[[Serializer, Any], bytes]

    def __init__(
        self,
        value: Any,
        encoder: Callable[[Serializer, Any], bytes],
    ):
        self.value = value
        self.encoder = encoder

    def encode(self) -> bytes:
        ser = Serializer()
        self.encoder(ser, self.value)
        return ser.output()


class SignedTransaction:
    transaction: RawTransaction
    authenticator: Authenticator

    def __init__(
        self,
        transaction: RawTransaction,
        authenticator: Union[AccountAuthenticator, Authenticator],
    ):
        self.transaction = transaction
        if isinstance(authenticator, AccountAuthenticator):
            if (
                authenticator.variant == AccountAuthenticator.ED25519
                or authenticator == AccountAuthenticator.MULTI_ED25519
            ):
                authenticator = Authenticator(authenticator.authenticator)
            else:
                authenticator = Authenticator(SingleSenderAuthenticator(authenticator))

        self.authenticator = authenticator

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, SignedTransaction):
            return NotImplemented
        return (
            self.transaction == other.transaction
            and self.authenticator == other.authenticator
        )

    def __str__(self) -> str:
        return f"Transaction: {self.transaction}Authenticator: {self.authenticator}"

    def bytes(self) -> bytes:
        ser = Serializer()
        ser.struct(self)
        return ser.output()

    def verify(self) -> bool:
        auth = self.authenticator.authenticator
        if isinstance(auth, MultiAgentAuthenticator):
            transaction: RawTransactionInternal = MultiAgentRawTransaction(
                self.transaction, auth.secondary_addresses()
            )
        elif isinstance(auth, FeePayerAuthenticator):
            transaction = cast(
                RawTransactionInternal,
                FeePayerRawTransaction(
                    self.transaction,
                    auth.secondary_addresses(),
                    auth.fee_payer_address(),
                ),
            )
        else:
            transaction = self.transaction
        return self.authenticator.verify(transaction.keyed())

    @staticmethod
    def deserialize(deserializer: Deserializer) -> SignedTransaction:
        transaction = RawTransaction.deserialize(deserializer)
        authenticator = Authenticator.deserialize(deserializer)
        return SignedTransaction(transaction, authenticator)

    def serialize(self, serializer: Serializer):
        self.transaction.serialize(serializer)
        self.authenticator.serialize(serializer)


class Test(unittest.TestCase):
    def test_entry_function(self):
        private_key = ed25519.PrivateKey.random()
        public_key = private_key.public_key()
        account_address = AccountAddress.from_key(public_key)

        another_private_key = ed25519.PrivateKey.random()
        another_public_key = another_private_key.public_key()
        recipient_address = AccountAddress.from_key(another_public_key)

        transaction_arguments = [
            TransactionArgument(recipient_address, Serializer.struct),
            TransactionArgument(5000, Serializer.u64),
        ]

        payload = EntryFunction.natural(
            "0x1::coin",
            "transfer",
            [TypeTag(StructTag.from_str("0x1::aptos_coin::AptosCoin"))],
            transaction_arguments,
        )

        raw_transaction = RawTransaction(
            account_address,
            0,
            TransactionPayload(payload),
            2000,
            0,
            18446744073709551615,
            4,
        )

        authenticator = raw_transaction.sign(private_key)
        signed_transaction = SignedTransaction(raw_transaction, authenticator)
        self.assertTrue(signed_transaction.verify())

    def test_entry_function_with_corpus(self):
        # Define common inputs
        sender_key_input = (
            "9bf49a6a0755f953811fce125f2683d50429c3bb49e074147e0089a52eae155f"
        )
        receiver_key_input = (
            "0564f879d27ae3c02ce82834acfa8c793a629f2ca0de6919610be82f411326be"
        )

        sequence_number_input = 11
        gas_unit_price_input = 1
        max_gas_amount_input = 2000
        expiration_timestamps_secs_input = 1234567890
        chain_id_input = 4
        amount_input = 5000

        # Accounts and crypto
        sender_private_key = ed25519.PrivateKey.from_str(sender_key_input)
        sender_public_key = sender_private_key.public_key()
        sender_account_address = AccountAddress.from_key(sender_public_key)

        receiver_private_key = ed25519.PrivateKey.from_str(receiver_key_input)
        receiver_public_key = receiver_private_key.public_key()
        receiver_account_address = AccountAddress.from_key(receiver_public_key)

        # Generate the transaction locally
        transaction_arguments = [
            TransactionArgument(receiver_account_address, Serializer.struct),
            TransactionArgument(amount_input, Serializer.u64),
        ]

        payload = EntryFunction.natural(
            "0x1::coin",
            "transfer",
            [TypeTag(StructTag.from_str("0x1::aptos_coin::AptosCoin"))],
            transaction_arguments,
        )

        raw_transaction_generated = RawTransaction(
            sender_account_address,
            sequence_number_input,
            TransactionPayload(payload),
            max_gas_amount_input,
            gas_unit_price_input,
            expiration_timestamps_secs_input,
            chain_id_input,
        )

        authenticator = raw_transaction_generated.sign(sender_private_key)
        signed_transaction_generated = SignedTransaction(
            raw_transaction_generated, authenticator
        )
        self.assertTrue(signed_transaction_generated.verify())

        # Validated corpus

        raw_transaction_input = "7deeccb1080854f499ec8b4c1b213b82c5e34b925cf6875fec02d4b77adbd2d60b0000000000000002000000000000000000000000000000000000000000000000000000000000000104636f696e087472616e73666572010700000000000000000000000000000000000000000000000000000000000000010a6170746f735f636f696e094170746f73436f696e0002202d133ddd281bb6205558357cc6ac75661817e9aaeac3afebc32842759cbf7fa9088813000000000000d0070000000000000100000000000000d20296490000000004"
        signed_transaction_input = "7deeccb1080854f499ec8b4c1b213b82c5e34b925cf6875fec02d4b77adbd2d60b0000000000000002000000000000000000000000000000000000000000000000000000000000000104636f696e087472616e73666572010700000000000000000000000000000000000000000000000000000000000000010a6170746f735f636f696e094170746f73436f696e0002202d133ddd281bb6205558357cc6ac75661817e9aaeac3afebc32842759cbf7fa9088813000000000000d0070000000000000100000000000000d202964900000000040020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a4920040f25b74ec60a38a1ed780fd2bef6ddb6eb4356e3ab39276c9176cdf0fcae2ab37d79b626abb43d926e91595b66503a4a3c90acbae36a28d405e308f3537af720b"

        self.verify_transactions(
            raw_transaction_input,
            raw_transaction_generated,
            signed_transaction_input,
            signed_transaction_generated,
        )

    def test_entry_function_multi_agent_with_corpus(self):
        # Define common inputs
        sender_key_input = (
            "9bf49a6a0755f953811fce125f2683d50429c3bb49e074147e0089a52eae155f"
        )
        receiver_key_input = (
            "0564f879d27ae3c02ce82834acfa8c793a629f2ca0de6919610be82f411326be"
        )

        sequence_number_input = 11
        gas_unit_price_input = 1
        max_gas_amount_input = 2000
        expiration_timestamps_secs_input = 1234567890
        chain_id_input = 4

        # Accounts and crypto
        sender_private_key = ed25519.PrivateKey.from_str(sender_key_input)
        sender_public_key = sender_private_key.public_key()
        sender_account_address = AccountAddress.from_key(sender_public_key)

        receiver_private_key = ed25519.PrivateKey.from_str(receiver_key_input)
        receiver_public_key = receiver_private_key.public_key()
        receiver_account_address = AccountAddress.from_key(receiver_public_key)

        # Generate the transaction locally
        transaction_arguments = [
            TransactionArgument(receiver_account_address, Serializer.struct),
            TransactionArgument("collection_name", Serializer.str),
            TransactionArgument("token_name", Serializer.str),
            TransactionArgument(1, Serializer.u64),
        ]

        payload = EntryFunction.natural(
            "0x3::token",
            "direct_transfer_script",
            [],
            transaction_arguments,
        )

        raw_transaction_generated = MultiAgentRawTransaction(
            RawTransaction(
                sender_account_address,
                sequence_number_input,
                TransactionPayload(payload),
                max_gas_amount_input,
                gas_unit_price_input,
                expiration_timestamps_secs_input,
                chain_id_input,
            ),
            [receiver_account_address],
        )

        sender_authenticator = raw_transaction_generated.sign(sender_private_key)
        receiver_authenticator = raw_transaction_generated.sign(receiver_private_key)

        authenticator = Authenticator(
            MultiAgentAuthenticator(
                sender_authenticator,
                [(receiver_account_address, receiver_authenticator)],
            )
        )

        signed_transaction_generated = SignedTransaction(
            raw_transaction_generated.inner(), authenticator
        )
        self.assertTrue(signed_transaction_generated.verify())

        # Validated corpus

        raw_transaction_input = "7deeccb1080854f499ec8b4c1b213b82c5e34b925cf6875fec02d4b77adbd2d60b0000000000000002000000000000000000000000000000000000000000000000000000000000000305746f6b656e166469726563745f7472616e736665725f7363726970740004202d133ddd281bb6205558357cc6ac75661817e9aaeac3afebc32842759cbf7fa9100f636f6c6c656374696f6e5f6e616d650b0a746f6b656e5f6e616d65080100000000000000d0070000000000000100000000000000d20296490000000004"
        signed_transaction_input = "7deeccb1080854f499ec8b4c1b213b82c5e34b925cf6875fec02d4b77adbd2d60b0000000000000002000000000000000000000000000000000000000000000000000000000000000305746f6b656e166469726563745f7472616e736665725f7363726970740004202d133ddd281bb6205558357cc6ac75661817e9aaeac3afebc32842759cbf7fa9100f636f6c6c656374696f6e5f6e616d650b0a746f6b656e5f6e616d65080100000000000000d0070000000000000100000000000000d20296490000000004020020b9c6ee1630ef3e711144a648db06bbb2284f7274cfbee53ffcee503cc1a4920040343e7b10aa323c480391a5d7cd2d0cf708d51529b96b5a2be08cbb365e4f11dcc2cf0655766cf70d40853b9c395b62dad7a9f58ed998803d8bf1901ba7a7a401012d133ddd281bb6205558357cc6ac75661817e9aaeac3afebc32842759cbf7fa9010020aef3f4a4b8eca1dfc343361bf8e436bd42de9259c04b8314eb8e2054dd6e82ab408a7f06e404ae8d9535b0cbbeafb7c9e34e95fe1425e4529758150a4f7ce7a683354148ad5c313ec36549e3fb29e669d90010f97467c9074ff0aec3ed87f76608"

        self.verify_transactions(
            raw_transaction_input,
            raw_transaction_generated.inner(),
            signed_transaction_input,
            signed_transaction_generated,
        )

    def verify_transactions(
        self,
        raw_transaction_input: str,
        raw_transaction_generated: RawTransaction,
        signed_transaction_input: str,
        signed_transaction_generated: SignedTransaction,
    ):
        # Produce serialized generated transactions
        ser = Serializer()
        ser.struct(raw_transaction_generated)
        raw_transaction_generated_bytes = ser.output().hex()

        ser = Serializer()
        ser.struct(signed_transaction_generated)
        signed_transaction_generated_bytes = ser.output().hex()

        # Verify the RawTransaction
        self.assertEqual(raw_transaction_input, raw_transaction_generated_bytes)
        raw_transaction = RawTransaction.deserialize(
            Deserializer(bytes.fromhex(raw_transaction_input))
        )
        self.assertEqual(raw_transaction_generated, raw_transaction)

        # Verify the SignedTransaction
        self.assertEqual(signed_transaction_input, signed_transaction_generated_bytes)
        signed_transaction = SignedTransaction.deserialize(
            Deserializer(bytes.fromhex(signed_transaction_input))
        )

        self.assertEqual(signed_transaction.transaction, raw_transaction)
        self.assertTrue(signed_transaction.verify())

    def verify_fee_payer(self):
        signed_transaction_input = "4629fa78b6a7810c6c3a45565707896944c4936a5583f9d3981c0692beb9e3fe010000000000000002915efe6647e0440f927d46e39bcb5eb040a7e567e1756e002073bc6e26f2cd230c63616e7661735f746f6b656e04647261770004205d45bb2a6f391440ba10444c7734559bd5ef9053930e3ef53d05be332518522bc90164850086008700880089008a008b008c008d008e008f0090009100920093009400950096009700980099009a009b009c009d009e009f00a000a100a200a300a400a500a600a700a800a900aa00ab00ac00ad00ae00af00b000b100b200b300b400b500b600b700b800b900ba00bb00bc00bd00be00bf00c000c100c200c300c4009f00a000a100a200a300a400a500a600a700a800a900aa00ab00ac00ad00ae00af00b000b100b200b300b400b500b600b700b800b900ba00bb00bc00bd00be00bf00c000c100c200c90164b701b701b701b701b701b701b701b701b701b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601b601130213021302130213021302130213021302130213021302130213021302130213021302130213021302130213021302130213021302130213021302130213021302130213021302656400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000400d030000000000640000000000000043663065000000000103002076585d13da61c3d65f786b082e75ef790be66639fa066e0fc3b6f427d6ceb89340e137736ee1a0b60e8bdac8d0c75f29f1e6c6e7378689928125ea7a13164f96244d98ed3584df98643f5db00624f0271931498ff19492558737fbd4dcd0e99c040000af621023eaa26d6f1139da3e146a43aa4757fd77552f73ceba34b00295c340ce0020c245d6e4f0ce0867b80f9b901c00be5d790ed73272f4e5126ce02a5a7d55a15c4002fbb70e7d79b536d692953e4bdc3f762b5a288839ab974f03c8597ebb1c51d1d7e0920991bd79ca8c0acd02a7fb7c38b9c1f4d7e53f19f88b130555b20ef60d"
        der = Deserializer()
        signed_txn = der.struct(SignedTransaction)

        ser = Serializer()
        ser.seriaize(signed_txn)
        self.assertEqual(ser.output().hex(), signed_transaction_input)

        self.assertTrue(
            isinstance(signed_txn.authenticator.authenticator, FeePayerAuthenticator)
        )
        self.assertTrue(signed_txn.verify())

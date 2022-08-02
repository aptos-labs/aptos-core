# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

import typing

from .account_address import AccountAddress
from .bcs import Deserializer, Serializer


class TypeTag:
    """TypeTag represents a primitive in Move."""

    BOOL: int = 0
    U8: int = 1
    U64: int = 2
    U128: int = 3
    ACCOUNT_ADDRESS: int = 4
    SIGNER: int = 5
    VECTOR: int = 6
    STRUCT: int = 7

    value: typing.Any

    def __init__(self, value: typing.Any):
        self.value = value

    def __eq__(self, other: TypeTag) -> bool:
        return (
            self.value.variant() == other.value.variant() and self.value == other.value
        )

    def __str__(self):
        return self.value.__str__()

    def __repr__(self):
        return self.__str__()

    def deserialize(deserializer: Deserializer) -> TypeTag:
        variant = deserializer.uleb128()
        if variant == TypeTag.BOOL:
            return TypeTag(BoolTag.deserialize(deserializer))
        elif variant == TypeTag.U8:
            return TypeTag(U8Tag.deserialize(deserializer))
        elif variant == TypeTag.U64:
            return TypeTag(U64Tag.deserialize(deserializer))
        elif variant == TypeTag.U128:
            return TypeTag(U128Tag.deserialize(deserializer))
        elif variant == TypeTag.ACCOUNT_ADDRESS:
            return TypeTag(AccountAddressTag.deserialize(deserializer))
        elif variant == TypeTag.SIGNER:
            raise NotImplementedError
        elif variant == TypeTag.VECTOR:
            raise NotImplementedError
        elif variant == TypeTag.STRUCT:
            return TypeTag(StructTag.deserialize(deserializer))
        raise NotImplementedError

    def serialize(self, serializer: Serializer):
        serializer.uleb128(self.value.variant())
        serializer.struct(self.value)


class BoolTag:
    value: bool

    def __init__(self, value: bool):
        self.value = value

    def __eq__(self, other: BoolTag) -> bool:
        return self.value == other.value

    def __str__(self):
        return self.value.__str__()

    def variant(self):
        return TypeTag.BOOL

    def deserialize(deserializer: Deserializer) -> Tag:
        return Tag(deserializer.bool())

    def serialize(self, serializer: Serializer):
        serializer.bool(self.value)


class U8Tag:
    value: int

    def __init__(self, value: int):
        self.value = value

    def __eq__(self, other: U8Tag) -> bool:
        return self.value == other.value

    def __str__(self):
        return self.value.__str__()

    def variant(self):
        return TypeTag.U8

    def deserialize(deserializer: Deserializer) -> Tag:
        return Tag(deserializer.u8())

    def serialize(self, serializer: Serializer):
        serializer.u8(self.value)


class U64Tag:
    value: int

    def __init__(self, value: int):
        self.value = value

    def __eq__(self, other: U64Tag) -> bool:
        return self.value == other.value

    def __str__(self):
        return self.value.__str__()

    def variant(self):
        return TypeTag.U64

    def deserialize(deserializer: Deserializer) -> Tag:
        return Tag(deserializer.u64())

    def serialize(self, serializer: Serializer):
        serializer.u64(self.value)


class U128Tag:
    value: int

    def __init__(self, value: int):
        self.value = value

    def __eq__(self, other: U128Tag) -> bool:
        return self.value == other.value

    def __str__(self):
        return self.value.__str__()

    def variant(self):
        return TypeTag.U128

    def deserialize(deserializer: Deserializer) -> Tag:
        return Tag(deserializer.u128())

    def serialize(self, serializer: Serializer):
        serializer.u128(self.value)


class AccountAddressTag:
    value: AccountAddress

    def __init__(self, value: AccountAddress):
        self.value = value

    def __eq__(self, other: AccountAddressTag) -> bool:
        return self.value == other.value

    def __str__(self):
        return self.value.__str__()

    def variant(self):
        return TypeTag.ACCOUNT_ADDRESS

    def deserialize(deserializer: Deserializer) -> Tag:
        return AccountAddressTag(deserializer.struct(AccountAddress))

    def serialize(self, serializer: Serializer):
        serializer.struct(self.value)


class StructTag:
    address: AccountAddress
    module: str
    name: str
    type_args: List[TypeTag]

    def __init__(self, address, module, name, type_args):
        self.address = address
        self.module = module
        self.name = name
        self.type_args = type_args

    def __eq__(self, other: StructTag) -> bool:
        return (
            self.address == other.address
            and self.module == other.module
            and self.name == other.name
            and self.type_args == other.type_args
        )

    def __str__(self) -> str:
        value = f"{self.address}::{self.module}::{self.name}"
        if len(self.type_args) > 0:
            value += f"<{self.type_args[0]}"
            for type_arg in type_args[1:]:
                value += f", {type_arg}"
            value += ">"
        return value

    def from_str(type_tag: str) -> StructTag:
        name = ""
        index = 0
        while index < len(type_tag):
            letter = type_tag[index]
            index += 1

            if letter == "<":
                raise NotImplementedError
            else:
                name += letter

        split = name.split("::")
        return StructTag(AccountAddress.from_hex(split[0]), split[1], split[2], [])

    def variant(self):
        return TypeTag.STRUCT

    def deserialize(deserializer: Deserializer) -> StructTag:
        address = deserializer.struct(AccountAddress)
        module = deserializer.str()
        name = deserializer.str()
        type_args = deserializer.sequence(TypeTag.deserialize)
        return StructTag(address, module, name, type_args)

    def serialize(self, serializer: Serializer):
        self.address.serialize(serializer)
        serializer.str(self.module)
        serializer.str(self.name)
        serializer.sequence(self.type_args, Serializer.struct)

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

import base64
from typing import Any, List

from .account import Account
from .account_address import AccountAddress
from .aptos_token_client import AptosTokenClient
from .async_client import RestClient
from .bcs import Serializer
from .transactions import EntryFunction, TransactionArgument, TransactionPayload

MODULE_ADDRESS: AccountAddress = AccountAddress.from_str(
    "0xfa3911d7715238b2e3bd5b26b6a35e11ffa16cff318bc11471e84eccee8bd291"
)


def set_module_address(module_address: AccountAddress):
    global MODULE_ADDRESS
    MODULE_ADDRESS = module_address


def get_module_address() -> AccountAddress:
    global MODULE_ADDRESS
    return MODULE_ADDRESS


class InscriptionData:
    inscription_id: int
    data: bytes

    @staticmethod
    def struct_tag() -> str:
        return f"{get_module_address()}::inscriptions::InscriptionData"

    def __init__(self, inscription_id: int, data: bytes):
        self.inscription_id = inscription_id
        self.data = data

    @staticmethod
    def parse(resource: dict[str, Any]) -> InscriptionData:
        print(resource)
        return InscriptionData(
            resource["inscription_id"],
            bytes.fromhex(resource["data"].lstrip("0x")),
        )

    def __str__(self) -> str:
        data = base64.b64encode(self.data)
        return f"InscriptionData[inscription_id: {self.inscription_id}, data: {data!r}]"


class InscriptionMetadata:
    inscription_id: int

    @staticmethod
    def struct_tag() -> str:
        return f"{get_module_address()}::inscriptions::InscriptionMetadata"

    def __init__(self, inscription_id):
        self.inscription_id = inscription_id

    @staticmethod
    def parse(resource: dict[str, Any]) -> InscriptionMetadata:
        return InscriptionMetadata(
            resource["inscription_id"],
        )

    def __str__(self) -> str:
        return f"Object[inscription_id: {self.inscription_id}]"


class InscriptionState:
    next_inscription_id: int

    @staticmethod
    def struct_tag() -> str:
        return f"{get_module_address()}::inscriptions::InscriptionState"

    def __init__(self, next_inscription_id):
        self.next_inscription_id = next_inscription_id

    @staticmethod
    def parse(resource: dict[str, Any]) -> InscriptionState:
        return InscriptionState(resource["next_inscription_id"])

    def __str__(self) -> str:
        return f"InscriptionState[next_inscription_id: {self.next_inscription_id}]"


class InscriptionsClient:
    rest_client: RestClient
    token_client: AptosTokenClient

    def __init__(self, client: AptosTokenClient):
        self.token_client = client
        self.rest_client = client.client

    @staticmethod
    def create_collection_payload(
        description: str,
        max_supply: int,
        name: str,
        royalty_numerator: int,
        royalty_denominator: int,
        royalty_payee_address: AccountAddress,
        uri: str,
    ) -> TransactionPayload:
        transaction_arguments = [
            TransactionArgument(description, Serializer.str),
            TransactionArgument(max_supply, Serializer.u64),
            TransactionArgument(name, Serializer.str),
            TransactionArgument(royalty_numerator, Serializer.u64),
            TransactionArgument(royalty_denominator, Serializer.u64),
            TransactionArgument(royalty_payee_address, Serializer.struct),
            TransactionArgument(uri, Serializer.str),
        ]

        payload = EntryFunction.natural(
            f"{get_module_address()}::immutable_collection",
            "create_collection",
            [],
            transaction_arguments,
        )

        return TransactionPayload(payload)

    async def create_collection(
        self,
        creator: Account,
        description: str,
        max_supply: int,
        name: str,
        royalty_numerator: int,
        royalty_denominator: int,
        royalty_payee_address: AccountAddress,
        uri: str,
    ) -> str:
        payload = InscriptionsClient.create_collection_payload(
            description,
            max_supply,
            name,
            royalty_numerator,
            royalty_denominator,
            royalty_payee_address,
            uri,
        )
        signed_transaction = await self.rest_client.create_bcs_signed_transaction(
            creator, payload
        )
        return await self.rest_client.submit_bcs_transaction(signed_transaction)

    @staticmethod
    def mint_token_payload(
        collection: str,
        data: bytes,
        description: str,
        name: str,
        uri: str,
    ) -> TransactionPayload:
        transaction_arguments = [
            TransactionArgument(collection, Serializer.str),
            TransactionArgument(data, Serializer.to_bytes),
            TransactionArgument(description, Serializer.str),
            TransactionArgument(name, Serializer.str),
            TransactionArgument(uri, Serializer.str),
        ]

        payload = EntryFunction.natural(
            f"{get_module_address()}::immutable_collection",
            "mint_token",
            [],
            transaction_arguments,
        )

        return TransactionPayload(payload)

    async def mint_token(
        self,
        creator: Account,
        collection: str,
        data: bytes,
        description: str,
        name: str,
        uri: str,
    ) -> str:
        payload = InscriptionsClient.mint_token_payload(
            collection,
            data,
            description,
            name,
            uri,
        )
        signed_transaction = await self.rest_client.create_bcs_signed_transaction(
            creator, payload
        )
        return await self.rest_client.submit_bcs_transaction(signed_transaction)

    async def inscriptions_from_transaction(
        self, txn_hash: str
    ) -> List[InscriptionData]:
        output = await self.rest_client.transaction_by_hash(txn_hash)
        mints = []
        for event in output["events"]:
            if event["type"] != InscriptionData.struct_tag():
                continue
            mints.append(InscriptionData.parse(event["data"]))
        return mints

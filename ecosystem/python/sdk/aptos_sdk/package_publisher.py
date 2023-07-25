# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

from typing import List

from .account import Account
from .account_address import AccountAddress
from .async_client import RestClient
from .bcs import Serializer
from .transactions import EntryFunction, TransactionArgument, TransactionPayload

# Maximum amount of publishing data, this gives us buffer for BCS overheads
MAX_TRANSACTION_SIZE: int = 62000

# The location of the large package publisher
MODULE_ADDRESS: AccountAddress = AccountAddress.from_hex(
    "0xd20f305e3090a24c00524604dc2a42925a75c67aa6020d33033d516cf0878c4a"
)


class PackagePublisher:
    """A wrapper around publishing packages."""

    client: RestClient

    def __init__(self, client: RestClient):
        self.client = client

    async def publish_package(
        self, sender: Account, package_metadata: bytes, modules: List[bytes]
    ) -> str:
        transaction_arguments = [
            TransactionArgument(package_metadata, Serializer.to_bytes),
            TransactionArgument(
                modules, Serializer.sequence_serializer(Serializer.to_bytes)
            ),
        ]

        payload = EntryFunction.natural(
            "0x1::code",
            "publish_package_txn",
            [],
            transaction_arguments,
        )

        signed_transaction = await self.client.create_bcs_signed_transaction(
            sender, TransactionPayload(payload)
        )
        return await self.client.submit_bcs_transaction(signed_transaction)

    async def publish_package_experimental(
        self, sender: Account, package_metadata: bytes, modules: List[bytes]
    ) -> List[str]:
        total_size = len(package_metadata)
        for module in modules:
            total_size += len(module)
        if total_size < MAX_TRANSACTION_SIZE:
            txn_hash = await self.publish_package(sender, package_metadata, modules)
            return [txn_hash]

        payloads = []
        read_metadata = 0
        taken_size = 0
        while read_metadata < len(package_metadata):
            start_read_data = read_metadata
            read_metadata = min(
                read_metadata + MAX_TRANSACTION_SIZE, len(package_metadata)
            )
            chunked_package_metadata = package_metadata[start_read_data:read_metadata]
            taken_size = len(chunked_package_metadata)
            if taken_size == MAX_TRANSACTION_SIZE:
                payloads.append(
                    PackagePublisher.create_large_package_publishing_payload(
                        MODULE_ADDRESS, chunked_package_metadata, [], False
                    )
                )
                chunked_package_metadata = b""

        chunked_modules: List[bytes] = []
        for module in modules:
            assert len(module) <= MAX_TRANSACTION_SIZE, "Module exceeds maximum size"
            if len(module) + taken_size > MAX_TRANSACTION_SIZE:
                taken_size = 0
                payloads.append(
                    PackagePublisher.create_large_package_publishing_payload(
                        MODULE_ADDRESS, chunked_package_metadata, chunked_modules, False
                    )
                )
                chunked_package_metadata = b""
                chunked_modules = []
            taken_size += len(module)
            chunked_modules.append(module)

        payloads.append(
            PackagePublisher.create_large_package_publishing_payload(
                MODULE_ADDRESS, chunked_package_metadata, chunked_modules, True
            )
        )

        txn_hashes = []
        for payload in payloads:
            print("Submitting transaction...")
            signed_txn = await self.client.create_bcs_signed_transaction(
                sender, payload
            )
            txn_hash = await self.client.submit_bcs_transaction(signed_txn)
            await self.client.wait_for_transaction(txn_hash)
            txn_hashes.append(txn_hash)
        return txn_hashes

    @staticmethod
    def create_large_package_publishing_payload(
        module_address: AccountAddress,
        chunked_package_metadata: bytes,
        chunked_modules: List[bytes],
        publish: bool,
    ) -> TransactionPayload:
        transaction_arguments = [
            TransactionArgument(chunked_package_metadata, Serializer.to_bytes),
            TransactionArgument(
                chunked_modules, Serializer.sequence_serializer(Serializer.to_bytes)
            ),
            TransactionArgument(publish, Serializer.bool),
        ]

        payload = EntryFunction.natural(
            f"{module_address}::large_packages",
            "stage_code",
            [],
            transaction_arguments,
        )

        return TransactionPayload(payload)

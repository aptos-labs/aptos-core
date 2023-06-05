# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import time
from typing import Any, Dict, List

import httpx

from . import ed25519
from .account import Account
from .account_address import AccountAddress
from .authenticator import Authenticator, Ed25519Authenticator, MultiAgentAuthenticator
from .bcs import Serializer
from .metadata import Metadata
from .transactions import (
    EntryFunction,
    MultiAgentRawTransaction,
    RawTransaction,
    SignedTransaction,
    TransactionArgument,
    TransactionPayload,
)
from .type_tag import StructTag, TypeTag

U64_MAX = 18446744073709551615


class ClientConfig:
    """Common configuration for clients, particularly for submitting transactions"""

    expiration_ttl: int = 600
    gas_unit_price: int = 100
    max_gas_amount: int = 100_000
    transaction_wait_in_seconds: int = 20


class RestClient:
    """A wrapper around the Aptos-core Rest API"""

    chain_id: int
    client: httpx.Client
    client_config: ClientConfig
    base_url: str

    def __init__(self, base_url: str, client_config: ClientConfig = ClientConfig()):
        self.base_url = base_url
        self.client = httpx.Client()
        self.client.headers[Metadata.APTOS_HEADER] = Metadata.get_aptos_header_val()
        self.client_config = client_config
        self.chain_id = int(self.info()["chain_id"])

    def close(self):
        self.client.close()

    #
    # Account accessors
    #

    def account(
        self, account_address: AccountAddress, ledger_version: int = None
    ) -> Dict[str, str]:
        """Returns the sequence number and authentication key for an account"""

        if not ledger_version:
            request = f"{self.base_url}/accounts/{account_address}"
        else:
            request = f"{self.base_url}/accounts/{account_address}?ledger_version={ledger_version}"

        response = self.client.get(request)
        if response.status_code >= 400:
            raise ApiError(f"{response.text} - {account_address}", response.status_code)
        return response.json()

    def account_balance(
        self, account_address: AccountAddress, ledger_version: int = None
    ) -> int:
        """Returns the test coin balance associated with the account"""
        resource = self.account_resource(
            account_address,
            "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>",
            ledger_version,
        )
        return resource["data"]["coin"]["value"]

    def account_sequence_number(
        self, account_address: AccountAddress, ledger_version: int = None
    ) -> int:
        account_res = self.account(account_address, ledger_version)
        return int(account_res["sequence_number"])

    def account_resource(
        self,
        account_address: AccountAddress,
        resource_type: str,
        ledger_version: int = None,
    ) -> Dict[str, Any]:
        if not ledger_version:
            request = (
                f"{self.base_url}/accounts/{account_address}/resource/{resource_type}"
            )
        else:
            request = f"{self.base_url}/accounts/{account_address}/resource/{resource_type}?ledger_version={ledger_version}"

        response = self.client.get(request)
        if response.status_code == 404:
            raise ResourceNotFound(resource_type, resource_type)
        if response.status_code >= 400:
            raise ApiError(f"{response.text} - {account_address}", response.status_code)
        return response.json()

    def get_table_item(
        self,
        handle: str,
        key_type: str,
        value_type: str,
        key: Any,
        ledger_version: int = None,
    ) -> Any:
        if not ledger_version:
            request = f"{self.base_url}/tables/{handle}/item"
        else:
            request = (
                f"{self.base_url}/tables/{handle}/item?ledger_version={ledger_version}"
            )
        response = self.client.post(
            request,
            json={
                "key_type": key_type,
                "value_type": value_type,
                "key": key,
            },
        )
        if response.status_code >= 400:
            raise ApiError(response.text, response.status_code)
        return response.json()

    def aggregator_value(
        self,
        account_address: AccountAddress,
        resource_type: str,
        aggregator_path: List[str],
    ) -> int:
        source_data = self.account_resource(account_address, resource_type)["data"]
        data = source_data

        while len(aggregator_path) > 0:
            key = aggregator_path.pop()
            if key not in data:
                raise ApiError(
                    f"aggregator path not found in data: {source_data}", source_data
                )
            data = data[key]

        if "vec" not in data:
            raise ApiError(f"aggregator not found in data: {source_data}", source_data)
        data = data["vec"]
        if len(data) != 1:
            raise ApiError(f"aggregator not found in data: {source_data}", source_data)
        data = data[0]
        if "aggregator" not in data:
            raise ApiError(f"aggregator not found in data: {source_data}", source_data)
        data = data["aggregator"]
        if "vec" not in data:
            raise ApiError(f"aggregator not found in data: {source_data}", source_data)
        data = data["vec"]
        if len(data) != 1:
            raise ApiError(f"aggregator not found in data: {source_data}", source_data)
        data = data[0]
        if "handle" not in data:
            raise ApiError(f"aggregator not found in data: {source_data}", source_data)
        if "key" not in data:
            raise ApiError(f"aggregator not found in data: {source_data}", source_data)
        handle = data["handle"]
        key = data["key"]
        return int(self.get_table_item(handle, "address", "u128", key))

    #
    # Ledger accessors
    #

    def info(self) -> Dict[str, str]:
        response = self.client.get(self.base_url)
        if response.status_code >= 400:
            raise ApiError(response.text, response.status_code)
        return response.json()

    #
    # Transactions
    #

    def simulate_transaction(
        self,
        transaction: RawTransaction,
        sender: Account,
    ) -> Dict[str, Any]:
        # Note that simulated transactions are not signed and have all 0 signatures!
        authenticator = Authenticator(
            Ed25519Authenticator(
                sender.public_key(),
                ed25519.Signature(b"\x00" * 64),
            )
        )
        signed_transaction = SignedTransaction(transaction, authenticator)

        headers = {"Content-Type": "application/x.aptos.signed_transaction+bcs"}
        response = self.client.post(
            f"{self.base_url}/transactions/simulate",
            headers=headers,
            content=signed_transaction.bytes(),
        )
        if response.status_code >= 400:
            raise ApiError(response.text, response.status_code)

        return response.json()

    def submit_bcs_transaction(self, signed_transaction: SignedTransaction) -> str:
        headers = {"Content-Type": "application/x.aptos.signed_transaction+bcs"}
        response = self.client.post(
            f"{self.base_url}/transactions",
            headers=headers,
            content=signed_transaction.bytes(),
        )
        if response.status_code >= 400:
            raise ApiError(response.text, response.status_code)
        return response.json()["hash"]

    def submit_transaction(self, sender: Account, payload: Dict[str, Any]) -> str:
        """
        1) Generates a transaction request
        2) submits that to produce a raw transaction
        3) signs the raw transaction
        4) submits the signed transaction
        """

        txn_request = {
            "sender": f"{sender.address()}",
            "sequence_number": str(self.account_sequence_number(sender.address())),
            "max_gas_amount": str(self.client_config.max_gas_amount),
            "gas_unit_price": str(self.client_config.gas_unit_price),
            "expiration_timestamp_secs": str(
                int(time.time()) + self.client_config.expiration_ttl
            ),
            "payload": payload,
        }

        response = self.client.post(
            f"{self.base_url}/transactions/encode_submission", json=txn_request
        )
        if response.status_code >= 400:
            raise ApiError(response.text, response.status_code)

        to_sign = bytes.fromhex(response.json()[2:])
        signature = sender.sign(to_sign)
        txn_request["signature"] = {
            "type": "ed25519_signature",
            "public_key": f"{sender.public_key()}",
            "signature": f"{signature}",
        }

        headers = {"Content-Type": "application/json"}
        response = self.client.post(
            f"{self.base_url}/transactions", headers=headers, json=txn_request
        )
        if response.status_code >= 400:
            raise ApiError(response.text, response.status_code)
        return response.json()["hash"]

    def transaction_pending(self, txn_hash: str) -> bool:
        response = self.client.get(f"{self.base_url}/transactions/by_hash/{txn_hash}")
        # TODO(@davidiw): consider raising a different error here, since this is an ambiguous state
        if response.status_code == 404:
            return True
        if response.status_code >= 400:
            raise ApiError(response.text, response.status_code)
        return response.json()["type"] == "pending_transaction"

    def wait_for_transaction(self, txn_hash: str) -> None:
        """
        Waits up to the duration specified in client_config for a transaction to move past pending
        state.
        """

        count = 0
        while self.transaction_pending(txn_hash):
            assert (
                count < self.client_config.transaction_wait_in_seconds
            ), f"transaction {txn_hash} timed out"
            time.sleep(1)
            count += 1
        response = self.client.get(f"{self.base_url}/transactions/by_hash/{txn_hash}")
        assert (
            "success" in response.json() and response.json()["success"]
        ), f"{response.text} - {txn_hash}"

    #
    # Transaction helpers
    #

    def create_multi_agent_bcs_transaction(
        self,
        sender: Account,
        secondary_accounts: List[Account],
        payload: TransactionPayload,
    ) -> SignedTransaction:
        raw_transaction = MultiAgentRawTransaction(
            RawTransaction(
                sender.address(),
                self.account_sequence_number(sender.address()),
                payload,
                self.client_config.max_gas_amount,
                self.client_config.gas_unit_price,
                int(time.time()) + self.client_config.expiration_ttl,
                self.chain_id,
            ),
            [x.address() for x in secondary_accounts],
        )

        keyed_txn = raw_transaction.keyed()

        authenticator = Authenticator(
            MultiAgentAuthenticator(
                Authenticator(
                    Ed25519Authenticator(sender.public_key(), sender.sign(keyed_txn))
                ),
                [
                    (
                        x.address(),
                        Authenticator(
                            Ed25519Authenticator(x.public_key(), x.sign(keyed_txn))
                        ),
                    )
                    for x in secondary_accounts
                ],
            )
        )

        return SignedTransaction(raw_transaction.inner(), authenticator)

    def create_bcs_transaction(
        self, sender: Account, payload: TransactionPayload
    ) -> RawTransaction:
        return RawTransaction(
            sender.address(),
            self.account_sequence_number(sender.address()),
            payload,
            self.client_config.max_gas_amount,
            self.client_config.gas_unit_price,
            int(time.time()) + self.client_config.expiration_ttl,
            self.chain_id,
        )

    def create_bcs_signed_transaction(
        self, sender: Account, payload: TransactionPayload
    ) -> SignedTransaction:
        raw_transaction = self.create_bcs_transaction(sender, payload)
        signature = sender.sign(raw_transaction.keyed())
        authenticator = Authenticator(
            Ed25519Authenticator(sender.public_key(), signature)
        )
        return SignedTransaction(raw_transaction, authenticator)

    #
    # Transaction wrappers
    #

    def transfer(self, sender: Account, recipient: AccountAddress, amount: int) -> str:
        """Transfer a given coin amount from a given Account to the recipient's account address.
        Returns the sequence number of the transaction used to transfer."""

        payload = {
            "type": "entry_function_payload",
            "function": "0x1::aptos_account::transfer_coins",
            "type_arguments": ["0x1::aptos_coin::AptosCoin"],
            "arguments": [
                f"{recipient}",
                str(amount),
            ],
        }
        return self.submit_transaction(sender, payload)

    # :!:>bcs_transfer
    def bcs_transfer(
        self, sender: Account, recipient: AccountAddress, amount: int
    ) -> str:
        transaction_arguments = [
            TransactionArgument(recipient, Serializer.struct),
            TransactionArgument(amount, Serializer.u64),
        ]

        payload = EntryFunction.natural(
            "0x1::aptos_account",
            "transfer_coins",
            [TypeTag(StructTag.from_str("0x1::aptos_coin::AptosCoin"))],
            transaction_arguments,
        )

        signed_transaction = self.create_bcs_signed_transaction(
            sender, TransactionPayload(payload)
        )
        return self.submit_bcs_transaction(signed_transaction)

    # <:!:bcs_transfer

    #
    # Token transaction wrappers
    #

    # :!:>create_collection
    def create_collection(
        self, account: Account, name: str, description: str, uri: str
    ) -> str:  # <:!:create_collection
        """Creates a new collection within the specified account"""

        transaction_arguments = [
            TransactionArgument(name, Serializer.str),
            TransactionArgument(description, Serializer.str),
            TransactionArgument(uri, Serializer.str),
            TransactionArgument(U64_MAX, Serializer.u64),
            TransactionArgument(
                [False, False, False], Serializer.sequence_serializer(Serializer.bool)
            ),
        ]

        payload = EntryFunction.natural(
            "0x3::token",
            "create_collection_script",
            [],
            transaction_arguments,
        )

        signed_transaction = self.create_bcs_signed_transaction(
            account, TransactionPayload(payload)
        )
        return self.submit_bcs_transaction(signed_transaction)

    # :!:>create_token
    def create_token(
        self,
        account: Account,
        collection_name: str,
        name: str,
        description: str,
        supply: int,
        uri: str,
        royalty_points_per_million: int,
    ) -> str:  # <:!:create_token
        transaction_arguments = [
            TransactionArgument(collection_name, Serializer.str),
            TransactionArgument(name, Serializer.str),
            TransactionArgument(description, Serializer.str),
            TransactionArgument(supply, Serializer.u64),
            TransactionArgument(supply, Serializer.u64),
            TransactionArgument(uri, Serializer.str),
            TransactionArgument(account.address(), Serializer.struct),
            # SDK assumes per million
            TransactionArgument(1000000, Serializer.u64),
            TransactionArgument(royalty_points_per_million, Serializer.u64),
            TransactionArgument(
                [False, False, False, False, False],
                Serializer.sequence_serializer(Serializer.bool),
            ),
            TransactionArgument([], Serializer.sequence_serializer(Serializer.str)),
            TransactionArgument(
                [], Serializer.sequence_serializer(Serializer.to_bytes)
            ),
            TransactionArgument([], Serializer.sequence_serializer(Serializer.str)),
        ]

        payload = EntryFunction.natural(
            "0x3::token",
            "create_token_script",
            [],
            transaction_arguments,
        )
        signed_transaction = self.create_bcs_signed_transaction(
            account, TransactionPayload(payload)
        )
        return self.submit_bcs_transaction(signed_transaction)

    def offer_token(
        self,
        account: Account,
        receiver: AccountAddress,
        creator: AccountAddress,
        collection_name: str,
        token_name: str,
        property_version: int,
        amount: int,
    ) -> str:
        transaction_arguments = [
            TransactionArgument(receiver, Serializer.struct),
            TransactionArgument(creator, Serializer.struct),
            TransactionArgument(collection_name, Serializer.str),
            TransactionArgument(token_name, Serializer.str),
            TransactionArgument(property_version, Serializer.u64),
            TransactionArgument(amount, Serializer.u64),
        ]

        payload = EntryFunction.natural(
            "0x3::token_transfers",
            "offer_script",
            [],
            transaction_arguments,
        )
        signed_transaction = self.create_bcs_signed_transaction(
            account, TransactionPayload(payload)
        )
        return self.submit_bcs_transaction(signed_transaction)

    def claim_token(
        self,
        account: Account,
        sender: AccountAddress,
        creator: AccountAddress,
        collection_name: str,
        token_name: str,
        property_version: int,
    ) -> str:
        transaction_arguments = [
            TransactionArgument(sender, Serializer.struct),
            TransactionArgument(creator, Serializer.struct),
            TransactionArgument(collection_name, Serializer.str),
            TransactionArgument(token_name, Serializer.str),
            TransactionArgument(property_version, Serializer.u64),
        ]

        payload = EntryFunction.natural(
            "0x3::token_transfers",
            "claim_script",
            [],
            transaction_arguments,
        )
        signed_transaction = self.create_bcs_signed_transaction(
            account, TransactionPayload(payload)
        )
        return self.submit_bcs_transaction(signed_transaction)

    def direct_transfer_token(
        self,
        sender: Account,
        receiver: Account,
        creators_address: AccountAddress,
        collection_name: str,
        token_name: str,
        property_version: int,
        amount: int,
    ) -> str:
        transaction_arguments = [
            TransactionArgument(creators_address, Serializer.struct),
            TransactionArgument(collection_name, Serializer.str),
            TransactionArgument(token_name, Serializer.str),
            TransactionArgument(property_version, Serializer.u64),
            TransactionArgument(amount, Serializer.u64),
        ]

        payload = EntryFunction.natural(
            "0x3::token",
            "direct_transfer_script",
            [],
            transaction_arguments,
        )

        signed_transaction = self.create_multi_agent_bcs_transaction(
            sender,
            [receiver],
            TransactionPayload(payload),
        )
        return self.submit_bcs_transaction(signed_transaction)

    #
    # Token accessors
    #

    def get_token(
        self,
        owner: AccountAddress,
        creator: AccountAddress,
        collection_name: str,
        token_name: str,
        property_version: int,
    ) -> Any:
        resource = self.account_resource(owner, "0x3::token::TokenStore")
        token_store_handle = resource["data"]["tokens"]["handle"]

        token_id = {
            "token_data_id": {
                "creator": creator.hex(),
                "collection": collection_name,
                "name": token_name,
            },
            "property_version": str(property_version),
        }

        try:
            return self.get_table_item(
                token_store_handle,
                "0x3::token::TokenId",
                "0x3::token::Token",
                token_id,
            )
        except ApiError as e:
            if e.status_code == 404:
                return {
                    "id": token_id,
                    "amount": "0",
                }
            raise

    def get_token_balance(
        self,
        owner: AccountAddress,
        creator: AccountAddress,
        collection_name: str,
        token_name: str,
        property_version: int,
    ) -> str:
        return self.get_token(
            owner, creator, collection_name, token_name, property_version
        )["amount"]

    # :!:>read_token_data_table
    def get_token_data(
        self,
        creator: AccountAddress,
        collection_name: str,
        token_name: str,
        property_version: int,
    ) -> Any:
        resource = self.account_resource(creator, "0x3::token::Collections")
        token_data_handle = resource["data"]["token_data"]["handle"]

        token_data_id = {
            "creator": creator.hex(),
            "collection": collection_name,
            "name": token_name,
        }

        return self.get_table_item(
            token_data_handle,
            "0x3::token::TokenDataId",
            "0x3::token::TokenData",
            token_data_id,
        )  # <:!:read_token_data_table

    def get_collection(self, creator: AccountAddress, collection_name: str) -> Any:
        resource = self.account_resource(creator, "0x3::token::Collections")
        token_data = resource["data"]["collection_data"]["handle"]

        return self.get_table_item(
            token_data,
            "0x1::string::String",
            "0x3::token::CollectionData",
            collection_name,
        )

    #
    # Package publishing
    #

    def publish_package(
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

        signed_transaction = self.create_bcs_signed_transaction(
            sender, TransactionPayload(payload)
        )
        return self.submit_bcs_transaction(signed_transaction)


class FaucetClient:
    """Faucet creates and funds accounts. This is a thin wrapper around that."""

    base_url: str
    rest_client: RestClient

    def __init__(self, base_url: str, rest_client: RestClient):
        self.base_url = base_url
        self.rest_client = rest_client

    def close(self):
        self.rest_client.close()

    def fund_account(self, address: str, amount: int):
        """This creates an account if it does not exist and mints the specified amount of
        coins into that account."""
        response = self.rest_client.client.post(
            f"{self.base_url}/mint?amount={amount}&address={address}"
        )
        if response.status_code >= 400:
            raise ApiError(response.text, response.status_code)
        for txn_hash in response.json():
            self.rest_client.wait_for_transaction(txn_hash)


class ApiError(Exception):
    """The API returned a non-success status code, e.g., >= 400"""

    def __init__(self, message: str, status_code: int):
        # Call the base class constructor with the parameters it needs
        super().__init__(message)
        self.status_code = status_code


class ResourceNotFound(Exception):
    """The underlying resource was not found"""

    def __init__(self, message: str, resource: str):
        # Call the base class constructor with the parameters it needs
        super().__init__(message)
        self.resource = resource

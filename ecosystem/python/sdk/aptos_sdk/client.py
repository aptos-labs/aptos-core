# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

import time
from typing import Any, Dict, List, Optional

import httpx

from .account import Account
from .account_address import AccountAddress
from .authenticator import Authenticator, Ed25519Authenticator, MultiAgentAuthenticator
from .bcs import Serializer
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


class RestClient:
    """A wrapper around the Aptos-core Rest API"""

    chain_id: int
    client: httpx.Client
    base_url: str

    def __init__(self, base_url: str):
        self.base_url = base_url
        self.client = httpx.Client()
        self.chain_id = int(self.info()["chain_id"])

    def close(self):
        self.client.close()

    #
    # Account accessors
    #

    def account(self, account_address: AccountAddress) -> Dict[str, str]:
        """Returns the sequence number and authentication key for an account"""

        response = self.client.get(f"{self.base_url}/accounts/{account_address}")
        if response.status_code >= 400:
            raise ApiError(f"{response.text} - {account_address}", response.status_code)
        return response.json()

    def account_balance(self, account_address: str) -> int:
        """Returns the test coin balance associated with the account"""
        return self.account_resource(
            account_address, "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>"
        )["data"]["coin"]["value"]

    def account_sequence_number(self, account_address: AccountAddress) -> int:
        account_res = self.account(account_address)
        return int(account_res["sequence_number"])

    def account_resource(
        self, account_address: AccountAddress, resource_type: str
    ) -> Optional[Dict[str, Any]]:
        response = self.client.get(
            f"{self.base_url}/accounts/{account_address}/resource/{resource_type}"
        )
        if response.status_code == 404:
            return None
        if response.status_code >= 400:
            raise ApiError(f"{response.text} - {account_address}", response.status_code)
        return response.json()

    def get_table_item(
        self, handle: str, key_type: str, value_type: str, key: Any
    ) -> Any:
        response = self.client.post(
            f"{self.base_url}/tables/{handle}/item",
            json={
                "key_type": key_type,
                "value_type": value_type,
                "key": key,
            },
        )
        if response.status_code >= 400:
            raise ApiError(response.text, response.status_code)
        return response.json()

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
            "max_gas_amount": "10000",
            "gas_unit_price": "100",
            "expiration_timestamp_secs": str(int(time.time()) + 600),
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
        if response.status_code == 404:
            return True
        if response.status_code >= 400:
            raise ApiError(response.text, response.status_code)
        return response.json()["type"] == "pending_transaction"

    def wait_for_transaction(self, txn_hash: str) -> None:
        """Waits up to 10 seconds for a transaction to move past pending state."""

        count = 0
        while self.transaction_pending(txn_hash):
            assert count < 20, f"transaction {txn_hash} timed out"
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
                100_000,
                100,
                int(time.time()) + 600,
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

    def create_single_signer_bcs_transaction(
        self, sender: Account, payload: TransactionPayload
    ) -> SignedTransaction:
        raw_transaction = RawTransaction(
            sender.address(),
            self.account_sequence_number(sender.address()),
            payload,
            100_000,
            100,
            int(time.time()) + 600,
            self.chain_id,
        )

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
            "function": "0x1::coin::transfer",
            "type_arguments": ["0x1::aptos_coin::AptosCoin"],
            "arguments": [
                f"{recipient}",
                str(amount),
            ],
        }
        return self.submit_transaction(sender, payload)

    #:!:>bcs_transfer
    def bcs_transfer(
        self, sender: Account, recipient: AccountAddress, amount: int
    ) -> str:
        transaction_arguments = [
            TransactionArgument(recipient, Serializer.struct),
            TransactionArgument(amount, Serializer.u64),
        ]

        payload = EntryFunction.natural(
            "0x1::coin",
            "transfer",
            [TypeTag(StructTag.from_str("0x1::aptos_coin::AptosCoin"))],
            transaction_arguments,
        )

        signed_transaction = self.create_single_signer_bcs_transaction(
            sender, TransactionPayload(payload)
        )
        return self.submit_bcs_transaction(signed_transaction)

    # <:!:bcs_transfer

    #
    # Token transaction wrappers
    #

    #:!:>create_collection
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

        signed_transaction = self.create_single_signer_bcs_transaction(
            account, TransactionPayload(payload)
        )
        return self.submit_bcs_transaction(signed_transaction)

    #:!:>create_token
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
            TransactionArgument([], Serializer.sequence_serializer(Serializer.bytes)),
            TransactionArgument([], Serializer.sequence_serializer(Serializer.str)),
        ]

        payload = EntryFunction.natural(
            "0x3::token",
            "create_token_script",
            [],
            transaction_arguments,
        )
        signed_transaction = self.create_single_signer_bcs_transaction(
            account, TransactionPayload(payload)
        )
        return self.submit_bcs_transaction(signed_transaction)

    def offer_token(
        self,
        account: Account,
        receiver: str,
        creator: str,
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
        signed_transaction = self.create_single_signer_bcs_transaction(
            account, TransactionPayload(payload)
        )
        return self.submit_bcs_transaction(signed_transaction)

    def claim_token(
        self,
        account: Account,
        sender: str,
        creator: str,
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
        signed_transaction = self.create_single_signer_bcs_transaction(
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
        token_store_handle = self.account_resource(owner, "0x3::token::TokenStore")[
            "data"
        ]["tokens"]["handle"]

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

    #:!:>read_token_data_table
    def get_token_data(
        self,
        creator: AccountAddress,
        collection_name: str,
        token_name: str,
        property_version: int,
    ) -> Any:
        token_data_handle = self.account_resource(creator, "0x3::token::Collections")[
            "data"
        ]["token_data"]["handle"]

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
        token_data = self.account_resource(creator, "0x3::token::Collections")["data"][
            "collection_data"
        ]["handle"]

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
            TransactionArgument(package_metadata, Serializer.bytes),
            TransactionArgument(
                modules, Serializer.sequence_serializer(Serializer.bytes)
            ),
        ]

        payload = EntryFunction.natural(
            "0x1::code",
            "publish_package_txn",
            [],
            transaction_arguments,
        )

        signed_transaction = self.create_single_signer_bcs_transaction(
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
    """Error thrown when the API returns >= 400"""

    def __init__(self, message, status_code):
        # Call the base class constructor with the parameters it needs
        super().__init__(message)
        self.status_code = status_code

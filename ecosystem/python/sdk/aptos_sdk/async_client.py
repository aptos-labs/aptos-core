# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import asyncio
import logging
import time
from typing import Any, Dict, List, Optional

import httpx

from .account import Account
from .account_address import AccountAddress
from .authenticator import Authenticator, MultiAgentAuthenticator
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

U64_MAX = 18446744073709551615


class ClientConfig:
    """Common configuration for clients, particularly for submitting transactions"""

    expiration_ttl: int = 600
    gas_unit_price: int = 100
    max_gas_amount: int = 100_000
    transaction_wait_in_seconds: int = 20
    http2: bool = False


class RestClient:
    """A wrapper around the Aptos-core Rest API"""

    _chain_id: Optional[int]
    client: httpx.AsyncClient
    client_config: ClientConfig
    base_url: str

    def __init__(self, base_url: str, client_config: ClientConfig = ClientConfig()):
        self.base_url = base_url
        # Default limits
        limits = httpx.Limits()
        # Default timeouts but do not set a pool timeout, since the idea is that jobs will wait as
        # long as progress is being made.
        timeout = httpx.Timeout(60.0, pool=None)
        # Default headers
        headers = {Metadata.APTOS_HEADER: Metadata.get_aptos_header_val()}
        self.client = httpx.AsyncClient(
            http2=client_config.http2,
            limits=limits,
            timeout=timeout,
            headers=headers,
        )
        self.client_config = client_config
        self._chain_id = None

    async def close(self):
        await self.client.aclose()

    async def chain_id(self):
        if not self._chain_id:
            info = await self.info()
            self._chain_id = int(info["chain_id"])
        return self._chain_id

    #
    # Account accessors
    #

    async def account(
        self, account_address: AccountAddress, ledger_version: Optional[int] = None
    ) -> Dict[str, str]:
        """
        Fetch the authentication key and the sequence number for an account address.

        :param account_address: Address of the account, with or without a '0x' prefix.
        :param ledger_version: Ledger version to get state of account. If not provided, it will be the latest version.
        :return: The authentication key and sequence number for the specified address.
        """
        response = await self._get(
            endpoint=f"accounts/{account_address}",
            params={"ledger_version": ledger_version},
        )
        if response.status_code >= 400:
            raise ApiError(f"{response.text} - {account_address}", response.status_code)
        return response.json()

    async def account_balance(
        self, account_address: AccountAddress, ledger_version: Optional[int] = None
    ) -> int:
        """
        Fetch the Aptos coin balance associated with the account.

        :param account_address: Address of the account, with or without a '0x' prefix.
        :param ledger_version: Ledger version to get state of account. If not provided, it will be the latest version.
        :return: The Aptos coin balance associated with the account
        """
        resource = await self.account_resource(
            account_address,
            "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>",
            ledger_version,
        )
        return int(resource["data"]["coin"]["value"])

    async def account_sequence_number(
        self, account_address: AccountAddress, ledger_version: Optional[int] = None
    ) -> int:
        """
        Fetch the current sequence number for an account address.

        :param account_address: Address of the account, with or without a '0x' prefix.
        :param ledger_version: Ledger version to get state of account. If not provided, it will be the latest version.
        :return: The current sequence number for the specified address.
        """
        account_res = await self.account(account_address, ledger_version)
        return int(account_res["sequence_number"])

    async def account_resource(
        self,
        account_address: AccountAddress,
        resource_type: str,
        ledger_version: Optional[int] = None,
    ) -> Dict[str, Any]:
        """
        Retrieves an individual resource from a given account and at a specific ledger version.

        The Aptos nodes prune account state history, via a configurable time window. If the requested ledger version
        has been pruned, the server responds with a 410.

        :param account_address: Address of the account, with or without a '0x' prefix.
        :param resource_type: Name of struct to retrieve e.g. 0x1::account::Account.
        :param ledger_version: Ledger version to get state of account. If not provided, it will be the latest version.
        :return: An individual resource from a given account and at a specific ledger version.
        """
        response = await self._get(
            endpoint=f"accounts/{account_address}/resource/{resource_type}",
            params={"ledger_version": ledger_version},
        )
        if response.status_code == 404:
            raise ResourceNotFound(resource_type, resource_type)
        if response.status_code >= 400:
            raise ApiError(f"{response.text} - {account_address}", response.status_code)
        return response.json()

    async def account_resources(
        self,
        account_address: AccountAddress,
        ledger_version: Optional[int] = None,
    ) -> List[Dict[str, Any]]:
        """
        Retrieves all account resources for a given account and a specific ledger version.

        The Aptos nodes prune account state history, via a configurable time window. If the requested ledger version
        has been pruned, the server responds with a 410.

        :param account_address: Address of the account, with or without a '0x' prefix.
        :param ledger_version: Ledger version to get state of account. If not provided, it will be the latest version.
        :return: All account resources for a given account and a specific ledger version.
        """
        response = await self._get(
            endpoint=f"accounts/{account_address}/resources",
            params={"ledger_version": ledger_version},
        )
        if response.status_code == 404:
            raise AccountNotFound(f"{account_address}", account_address)
        if response.status_code >= 400:
            raise ApiError(f"{response.text} - {account_address}", response.status_code)
        return response.json()

    async def account_module(
        self,
        account_address: AccountAddress,
        module_name: str,
        ledger_version: Optional[int] = None,
    ) -> dict:
        """
        Retrieves an individual module from a given account and at a specific ledger version.

        The Aptos nodes prune account state history, via a configurable time window. If the requested ledger version
        has been pruned, the server responds with a 410.

        :param account_address: Address of the account, with or without a '0x' prefix.
        :param module_name: Name of module to retrieve e.g. 'coin'
        :param ledger_version: Ledger version to get state of account. If not provided, it will be the latest version.
        :return: An individual module from a given account and at a specific ledger version
        """
        response = await self._get(
            endpoint=f"accounts/{account_address}/module/{module_name}",
            params={"ledger_version": ledger_version},
        )
        if response.status_code >= 400:
            raise ApiError(f"{response.text} - {account_address}", response.status_code)

        return response.json()

    async def account_modules(
        self,
        account_address: AccountAddress,
        ledger_version: Optional[int] = None,
        limit: Optional[int] = None,
        start: Optional[str] = None,
    ) -> dict:
        """
        Retrieves all account modules' bytecode for a given account at a specific ledger version.

        The Aptos nodes prune account state history, via a configurable time window. If the requested ledger version
        has been pruned, the server responds with a 410.

        :param account_address: Address of the account, with or without a '0x' prefix.
        :param ledger_version: Ledger version to get state of account. If not provided, it will be the latest version.
        :param limit: Max number of account modules to retrieve. If not provided, defaults to default page size.
        :param start: Cursor specifying where to start for pagination.
        :return: All account modules' bytecode for a given account at a specific ledger version.
        """
        response = await self._get(
            endpoint=f"accounts/{account_address}/modules",
            params={
                "ledger_version": ledger_version,
                "limit": limit,
                "start": start,
            },
        )
        if response.status_code == 404:
            raise AccountNotFound(f"{account_address}", account_address)
        if response.status_code >= 400:
            raise ApiError(f"{response.text} - {account_address}", response.status_code)

        return response.json()

    #
    # Blocks
    #

    async def blocks_by_height(
        self,
        block_height: int,
        with_transactions: bool = False,
    ) -> dict:
        """
        Fetch the transactions in a block and the corresponding block information.

        Transactions are limited by max default transactions size. If not all transactions are present, the user will
        need to query for the rest of the transactions via the get transactions API. If the block is pruned, it will
        return a 410.

        :param block_height: Block height to lookup. Starts at 0.
        :param with_transactions: If set to true, include all transactions in the block.
        :returns: Block information.
        """
        response = await self._get(
            endpoint=f"blocks/by_height/{block_height}",
            params={
                "with_transactions": with_transactions,
            },
        )
        if response.status_code >= 400:
            raise ApiError(f"{response.text}", response.status_code)

        return response.json()

    async def blocks_by_version(
        self,
        version: int,
        with_transactions: bool = False,
    ) -> dict:
        """
        Fetch the transactions in a block and the corresponding block information, given a version in the block.

        Transactions are limited by max default transactions size. If not all transactions are present, the user will
        need to query for the rest of the transactions via the get transactions API. If the block is pruned, it will
        return a 410.

        :param version: Ledger version to lookup block information for.
        :param with_transactions: If set to true, include all transactions in the block.
        :returns: Block information.
        """
        response = await self._get(
            endpoint=f"blocks/by_version/{version}",
            params={
                "with_transactions": with_transactions,
            },
        )
        if response.status_code >= 400:
            raise ApiError(f"{response.text}", response.status_code)

        return response.json()

    #
    # Events
    #

    async def event_by_creation_number(
        self,
        account_address: AccountAddress,
        creation_number: int,
        limit: Optional[int] = None,
        start: Optional[int] = None,
    ) -> List[dict]:
        """
        Retrieve events corresponding to an account address and creation number indicating the event type emitted
        to that account.

        Creation numbers are monotonically increasing for each account address.

        :param account_address: Address of the account, with or without a '0x' prefix.
        :param creation_number: Creation number corresponding to the event stream originating from the given account.
        :param limit: Max number of events to retrieve. If not provided, defaults to default page size.
        :param start: Starting sequence number of events.If unspecified, by default will retrieve the most recent.
        :returns: Events corresponding to an account address and creation number indicating the event type emitted
        to that account.
        """
        response = await self._get(
            endpoint=f"accounts/{account_address}/events/{creation_number}",
            params={
                "limit": limit,
                "start": start,
            },
        )
        if response.status_code >= 400:
            raise ApiError(f"{response.text} - {account_address}", response.status_code)

        return response.json()

    async def events_by_event_handle(
        self,
        account_address: AccountAddress,
        event_handle: str,
        field_name: str,
        limit: Optional[int] = None,
        start: Optional[int] = None,
    ) -> List[dict]:
        """
        Retrieve events corresponding to an account address, event handle (struct name) and field name.

        :param account_address: Address of the account, with or without a '0x' prefix.
        :param event_handle: Name of struct to lookup event handle e.g., '0x1::account::Account'.
        :param field_name: Name of field to lookup event handle e.g., 'withdraw_events'
        :param limit: Max number of events to retrieve. If not provided, defaults to default page size.
        :param start: Starting sequence number of events.If unspecified, by default will retrieve the most recent.
        :returns: Events corresponding to the provided account address, event handle and field name.
        """
        response = await self._get(
            endpoint=f"accounts/{account_address}/events/{event_handle}/{field_name}",
            params={
                "limit": limit,
                "start": start,
            },
        )
        if response.status_code >= 400:
            raise ApiError(f"{response.text} - {account_address}", response.status_code)

        return response.json()

    async def current_timestamp(self) -> float:
        info = await self.info()
        return float(info["ledger_timestamp"]) / 1_000_000

    async def get_table_item(
        self,
        handle: str,
        key_type: str,
        value_type: str,
        key: Any,
        ledger_version: Optional[int] = None,
    ) -> Any:
        if not ledger_version:
            request = f"{self.base_url}/tables/{handle}/item"
        else:
            request = (
                f"{self.base_url}/tables/{handle}/item?ledger_version={ledger_version}"
            )
        response = await self.client.post(
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

    async def aggregator_value(
        self,
        account_address: AccountAddress,
        resource_type: str,
        aggregator_path: List[str],
    ) -> int:
        source = await self.account_resource(account_address, resource_type)
        source_data = data = source["data"]

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
        return int(await self.get_table_item(handle, "address", "u128", key))

    #
    # Ledger accessors
    #

    async def info(self) -> Dict[str, str]:
        response = await self.client.get(self.base_url)
        if response.status_code >= 400:
            raise ApiError(response.text, response.status_code)
        return response.json()

    #
    # Transactions
    #

    async def simulate_bcs_transaction(
        self,
        signed_transaction: SignedTransaction,
        estimate_gas_usage: bool = False,
    ) -> Dict[str, Any]:
        headers = {"Content-Type": "application/x.aptos.signed_transaction+bcs"}
        params = {}
        if estimate_gas_usage:
            params = {
                "estimate_gas_unit_price": "true",
                "estimate_max_gas_amount": "true",
            }

        response = await self.client.post(
            f"{self.base_url}/transactions/simulate",
            params=params,
            headers=headers,
            content=signed_transaction.bytes(),
        )
        if response.status_code >= 400:
            raise ApiError(response.text, response.status_code)

        return response.json()

    async def simulate_transaction(
        self,
        transaction: RawTransaction,
        sender: Account,
        estimate_gas_usage: bool = False,
    ) -> Dict[str, Any]:
        # Note that simulated transactions are not signed and have all 0 signatures!
        authenticator = sender.sign_simulated_transaction(transaction)
        return await self.simulate_bcs_transaction(
            signed_transaction=SignedTransaction(transaction, authenticator),
            estimate_gas_usage=estimate_gas_usage,
        )

    async def submit_bcs_transaction(
        self, signed_transaction: SignedTransaction
    ) -> str:
        headers = {"Content-Type": "application/x.aptos.signed_transaction+bcs"}
        response = await self.client.post(
            f"{self.base_url}/transactions",
            headers=headers,
            content=signed_transaction.bytes(),
        )
        if response.status_code >= 400:
            raise ApiError(response.text, response.status_code)
        return response.json()["hash"]

    async def submit_transaction(self, sender: Account, payload: Dict[str, Any]) -> str:
        """
        1) Generates a transaction request
        2) submits that to produce a raw transaction
        3) signs the raw transaction
        4) submits the signed transaction
        """

        txn_request = {
            "sender": f"{sender.address()}",
            "sequence_number": str(
                await self.account_sequence_number(sender.address())
            ),
            "max_gas_amount": str(self.client_config.max_gas_amount),
            "gas_unit_price": str(self.client_config.gas_unit_price),
            "expiration_timestamp_secs": str(
                int(time.time()) + self.client_config.expiration_ttl
            ),
            "payload": payload,
        }

        response = await self.client.post(
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
        response = await self.client.post(
            f"{self.base_url}/transactions", headers=headers, json=txn_request
        )
        if response.status_code >= 400:
            raise ApiError(response.text, response.status_code)
        return response.json()["hash"]

    async def transaction_pending(self, txn_hash: str) -> bool:
        response = await self._get(endpoint=f"transactions/by_hash/{txn_hash}")
        # TODO(@davidiw): consider raising a different error here, since this is an ambiguous state
        if response.status_code == 404:
            return True
        if response.status_code >= 400:
            raise ApiError(response.text, response.status_code)
        return response.json()["type"] == "pending_transaction"

    async def wait_for_transaction(self, txn_hash: str) -> None:
        """
        Waits up to the duration specified in client_config for a transaction to move past pending
        state.
        """

        count = 0
        while await self.transaction_pending(txn_hash):
            assert (
                count < self.client_config.transaction_wait_in_seconds
            ), f"transaction {txn_hash} timed out"
            await asyncio.sleep(1)
            count += 1

        response = await self._get(endpoint=f"transactions/by_hash/{txn_hash}")
        assert (
            "success" in response.json() and response.json()["success"]
        ), f"{response.text} - {txn_hash}"

    async def account_transaction_sequence_number_status(
        self, address: AccountAddress, sequence_number: int
    ) -> bool:
        """Retrieve the state of a transaction by account and sequence number."""
        response = await self._get(
            endpoint=f"accounts/{address}/transactions",
            params={
                "limit": 1,
                "start": sequence_number,
            },
        )
        if response.status_code >= 400:
            logging.info(f"k {response}")
            raise ApiError(response.text, response.status_code)
        data = response.json()
        return len(data) == 1 and data[0]["type"] != "pending_transaction"

    async def transaction_by_hash(self, txn_hash: str) -> Dict[str, Any]:
        response = await self._get(endpoint=f"transactions/by_hash/{txn_hash}")
        if response.status_code >= 400:
            raise ApiError(response.text, response.status_code)
        return response.json()

    async def transactions_by_account(
        self,
        account_address: AccountAddress,
        limit: Optional[int] = None,
        start: Optional[int] = None,
    ) -> List[dict]:
        """
        Retrieves on-chain committed transactions from an account.

        If the start version is too far in the past, a 410 will be returned. If no start version is given, it will
        start at version 0.

        To retrieve a pending transaction, use /transactions/by_hash.

        :param account_address: Address of account with or without a 0x prefix.
        :param limit: Max number of transactions to retrieve. If not provided, defaults to default page size.
        :param start: Account sequence number to start list of transactions. Defaults to latest transactions.
        :returns: List of on-chain committed transactions from the specified account.
        """
        response = await self._get(
            endpoint=f"accounts/{account_address}/transactions",
            params={
                "limit": limit,
                "start": start,
            },
        )
        if response.status_code >= 400:
            raise ApiError(response.text, response.status_code)

        return response.json()

    async def transactions(
        self,
        limit: Optional[int] = None,
        start: Optional[int] = None,
    ) -> List[dict]:
        """
        Retrieve on-chain committed transactions.

        The page size and start ledger version can be provided to get a specific sequence of transactions. If the
        version has been pruned, then a 410 will be returned. To retrieve a pending transaction,
        use /transactions/by_hash.

        :param limit: Max number of transactions to retrieve. If not provided, defaults to default page size.
        :param start: Ledger version to start list of transactions. Defaults to showing the latest transactions.
        """
        response = await self._get(
            endpoint="transactions",
            params={
                "limit": limit,
                "start": start,
            },
        )
        if response.status_code >= 400:
            raise ApiError(response.text, response.status_code)

        return response.json()

    #
    # Transaction helpers
    #

    async def create_multi_agent_bcs_transaction(
        self,
        sender: Account,
        secondary_accounts: List[Account],
        payload: TransactionPayload,
    ) -> SignedTransaction:
        raw_transaction = MultiAgentRawTransaction(
            RawTransaction(
                sender.address(),
                await self.account_sequence_number(sender.address()),
                payload,
                self.client_config.max_gas_amount,
                self.client_config.gas_unit_price,
                int(time.time()) + self.client_config.expiration_ttl,
                await self.chain_id(),
            ),
            [x.address() for x in secondary_accounts],
        )

        authenticator = Authenticator(
            MultiAgentAuthenticator(
                sender.sign_transaction(raw_transaction),
                [
                    (
                        x.address(),
                        x.sign_transaction(raw_transaction),
                    )
                    for x in secondary_accounts
                ],
            )
        )

        return SignedTransaction(raw_transaction.inner(), authenticator)

    async def create_bcs_transaction(
        self,
        sender: Account,
        payload: TransactionPayload,
        sequence_number: Optional[int] = None,
    ) -> RawTransaction:
        sequence_number = (
            sequence_number
            if sequence_number is not None
            else await self.account_sequence_number(sender.address())
        )
        return RawTransaction(
            sender.address(),
            sequence_number,
            payload,
            self.client_config.max_gas_amount,
            self.client_config.gas_unit_price,
            int(time.time()) + self.client_config.expiration_ttl,
            await self.chain_id(),
        )

    async def create_bcs_signed_transaction(
        self,
        sender: Account,
        payload: TransactionPayload,
        sequence_number: Optional[int] = None,
    ) -> SignedTransaction:
        raw_transaction = await self.create_bcs_transaction(
            sender, payload, sequence_number
        )
        authenticator = sender.sign_transaction(raw_transaction)
        return SignedTransaction(raw_transaction, authenticator)

    #
    # Transaction wrappers
    #

    async def transfer(
        self, sender: Account, recipient: AccountAddress, amount: int
    ) -> str:
        """Transfer a given coin amount from a given Account to the recipient's account address.
        Returns the sequence number of the transaction used to transfer."""

        payload = {
            "type": "entry_function_payload",
            "function": "0x1::aptos_account::transfer",
            "type_arguments": [],
            "arguments": [
                f"{recipient}",
                str(amount),
            ],
        }
        return await self.submit_transaction(sender, payload)

    # :!:>bcs_transfer
    async def bcs_transfer(
        self,
        sender: Account,
        recipient: AccountAddress,
        amount: int,
        sequence_number: Optional[int] = None,
    ) -> str:
        transaction_arguments = [
            TransactionArgument(recipient, Serializer.struct),
            TransactionArgument(amount, Serializer.u64),
        ]

        payload = EntryFunction.natural(
            "0x1::aptos_account",
            "transfer",
            [],
            transaction_arguments,
        )

        signed_transaction = await self.create_bcs_signed_transaction(
            sender, TransactionPayload(payload), sequence_number=sequence_number
        )
        return await self.submit_bcs_transaction(signed_transaction)  # <:!:bcs_transfer

    async def transfer_object(
        self, owner: Account, object: AccountAddress, to: AccountAddress
    ) -> str:
        transaction_arguments = [
            TransactionArgument(object, Serializer.struct),
            TransactionArgument(to, Serializer.struct),
        ]

        payload = EntryFunction.natural(
            "0x1::object",
            "transfer_call",
            [],
            transaction_arguments,
        )

        signed_transaction = await self.create_bcs_signed_transaction(
            owner,
            TransactionPayload(payload),
        )
        return await self.submit_bcs_transaction(signed_transaction)

    async def view(
        self,
        function: str,
        type_arguments: List[str],
        arguments: List[str],
        ledger_version: Optional[int] = None,
    ) -> bytes:
        """
        Execute a view Move function with the given parameters and return its execution result.

        The Aptos nodes prune account state history, via a configurable time window. If the requested ledger version
        has been pruned, the server responds with a 410.

        :param function: Entry function id is string representation of an entry function defined on-chain.
        :param type_arguments: Type arguments of the function.
        :param arguments: Arguments of the function.
        :param ledger_version: Ledger version to get state of account. If not provided, it will be the latest version.
        :returns: Execution result.
        """
        response = await self._post(
            endpoint="view",
            params={
                "ledger_version": ledger_version,
            },
            headers={
                "Accept": "application/json",
                "Content-Type": "application/json",
            },
            data={
                "function": function,
                "type_arguments": type_arguments,
                "arguments": arguments,
            },
        )
        if response.status_code >= 400:
            raise ApiError(response.text, response.status_code)

        return response.content

    async def _post(
        self,
        endpoint: str,
        params: Optional[Dict[str, Any]] = None,
        headers: Optional[Dict[str, Any]] = None,
        data: Optional[Dict[str, Any]] = None,
    ) -> httpx.Response:
        # format params:
        params = {} if params is None else params
        params = {key: val for key, val in params.items() if val is not None}
        return await self.client.post(
            url=f"{self.base_url}/{endpoint}",
            params=params,
            headers=headers,
            json=data,
        )

    async def _get(
        self, endpoint: str, params: Optional[Dict[str, Any]] = None
    ) -> httpx.Response:
        # format params:
        params = {} if params is None else params
        params = {key: val for key, val in params.items() if val is not None}
        return await self.client.get(
            url=f"{self.base_url}/{endpoint}",
            params=params,
        )


class FaucetClient:
    """Faucet creates and funds accounts. This is a thin wrapper around that."""

    base_url: str
    rest_client: RestClient
    headers: Dict[str, str]

    def __init__(
        self, base_url: str, rest_client: RestClient, auth_token: Optional[str] = None
    ):
        self.base_url = base_url
        self.rest_client = rest_client
        self.headers = {}
        if auth_token:
            self.headers["Authorization"] = f"Bearer {auth_token}"

    async def close(self):
        await self.rest_client.close()

    async def fund_account(self, address: AccountAddress, amount: int):
        """This creates an account if it does not exist and mints the specified amount of
        coins into that account."""
        request = f"{self.base_url}/mint?amount={amount}&address={address}"
        response = await self.rest_client.client.post(request, headers=self.headers)
        if response.status_code >= 400:
            raise ApiError(response.text, response.status_code)
        for txn_hash in response.json():
            await self.rest_client.wait_for_transaction(txn_hash)

    async def healthy(self) -> bool:
        response = await self.rest_client.client.get(self.base_url)
        return "tap:ok" == response.text


class ApiError(Exception):
    """The API returned a non-success status code, e.g., >= 400"""

    status_code: int

    def __init__(self, message: str, status_code: int):
        # Call the base class constructor with the parameters it needs
        super().__init__(message)
        self.status_code = status_code


class AccountNotFound(Exception):
    """The account was not found"""

    account: AccountAddress

    def __init__(self, message: str, account: AccountAddress):
        # Call the base class constructor with the parameters it needs
        super().__init__(message)
        self.account = account


class ResourceNotFound(Exception):
    """The underlying resource was not found"""

    resource: str

    def __init__(self, message: str, resource: str):
        # Call the base class constructor with the parameters it needs
        super().__init__(message)
        self.resource = resource

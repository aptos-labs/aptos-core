# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

import time
from typing import Any, Dict, Optional

import httpx
from account import Account
from account_address import AccountAddress
from authenticator import Authenticator, Ed25519Authenticator
from bcs import Serializer
from transactions import (RawTransaction, ScriptFunction, SignedTransaction,
                          TransactionArgument, TransactionPayload)
from type_tag import StructTag, TypeTag

TESTNET_URL = "https://fullnode.devnet.aptoslabs.com"
FAUCET_URL = "https://faucet.devnet.aptoslabs.com"


class RestClient:
    """A wrapper around the Aptos-core Rest API"""

    chain_id: int
    client: httpx.Client
    base_url: str

    def __init__(self, base_url: str):
        self.base_url = base_url
        self.client = httpx.Client()
        self.chain_id = int(self.info()["chain_id"])

    def account(self, account_address: AccountAddress) -> Dict[str, str]:
        """Returns the sequence number and authentication key for an account"""

        response = self.client.get(f"{self.base_url}/accounts/{account_address}")
        assert response.status_code == 200, f"{response.text} - {account_address}"
        return response.json()

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
        assert response.status_code == 200, response.text
        return response.json()

    def info(self) -> Dict[str, str]:
        response = self.client.get(self.base_url)
        assert response.status_code == 200, f"{response.text}"
        return response.json()

    def submit_bcs_transaction(
        self, signed_transaction: SignedTransaction
    ) -> Dict[str, Any]:
        headers = {"Content-Type": "application/x.aptos.signed_transaction+bcs"}
        response = self.client.post(
            f"{self.base_url}/transactions",
            headers=headers,
            content=signed_transaction.bytes(),
        )
        assert response.status_code == 202, f"{response.text} - {signed_transaction}"
        return response.json()["hash"]

    def submit_transaction(
        self, sender: Account, payload: Dict[str, Any]
    ) -> Dict[str, Any]:
        """
        1) Generates a transaction request
        2) submits that to produce a raw transaction
        3) signs the raw transaction
        4) submits the signed transaction
        """

        txn_request = {
            "sender": f"{sender.address()}",
            "sequence_number": str(self.account_sequence_number(sender.address())),
            "max_gas_amount": "2000",
            "gas_unit_price": "1",
            "expiration_timestamp_secs": str(int(time.time()) + 600),
            "payload": payload,
        }

        res = self.client.post(
            f"{self.base_url}/transactions/signing_message", json=txn_request
        )
        assert res.status_code == 200, res.text

        to_sign = bytes.fromhex(res.json()["message"][2:])
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
        assert response.status_code == 202, f"{response.text} - {txn}"
        return response.json()

    def transaction_pending(self, txn_hash: str) -> bool:
        response = self.client.get(f"{self.base_url}/transactions/{txn_hash}")
        if response.status_code == 404:
            return True
        assert response.status_code == 200, f"{response.text} - {txn_hash}"
        return response.json()["type"] == "pending_transaction"

    def wait_for_transaction(self, txn_hash: str) -> None:
        """Waits up to 10 seconds for a transaction to move past pending state."""

        count = 0
        while self.transaction_pending(txn_hash):
            assert count < 10, f"transaction {txn_hash} timed out"
            time.sleep(1)
            count += 1
        response = self.client.get(f"{self.base_url}/transactions/{txn_hash}")
        assert "success" in response.json(), f"{response.text} - {txn_hash}"

    def account_balance(self, account_address: str) -> int:
        """Returns the test coin balance associated with the account"""
        return self.account_resource(
            account_address, "0x1::Coin::CoinStore<0x1::TestCoin::TestCoin>"
        )["data"]["coin"]["value"]

    def transfer(self, sender: Account, recipient: AccountAddress, amount: int) -> str:
        """Transfer a given coin amount from a given Account to the recipient's account address.
        Returns the sequence number of the transaction used to transfer."""

        payload = {
            "type": "script_function_payload",
            "function": "0x1::Coin::transfer",
            "type_arguments": ["0x1::TestCoin::TestCoin"],
            "arguments": [
                f"{recipient}",
                str(amount),
            ],
        }
        res = self.submit_transaction(sender, payload)
        return str(res["hash"])

    def bcs_transfer(
        self, sender: Account, recipient: AccountAddress, amount: int
    ) -> SignedTransaction:
        transaction_arguments = [
            TransactionArgument(recipient, Serializer.struct),
            TransactionArgument(amount, Serializer.u64),
        ]

        payload = ScriptFunction.natural(
            "0x1::Coin",
            "transfer",
            [TypeTag(StructTag.from_str("0x1::TestCoin::TestCoin"))],
            transaction_arguments,
        )

        raw_transaction = RawTransaction(
            sender.address(),
            self.account_sequence_number(sender.address()),
            TransactionPayload(payload),
            2000,
            1,
            int(time.time()) + 600,
            self.chain_id,
        )

        signature = sender.sign(raw_transaction.keyed())
        authenticator = Authenticator(
            Ed25519Authenticator(sender.public_key(), signature)
        )
        return SignedTransaction(raw_transaction, authenticator)


class FaucetClient:
    """Faucet creates and funds accounts. This is a thin wrapper around that."""

    base_url: str
    rest_client: RestClient

    def __init__(self, base_url: str, rest_client: RestClient):
        self.base_url = base_url
        self.rest_client = rest_client

    def fund_account(self, address: str, amount: int):
        """This creates an account if it does not exist and mints the specified amount of
        coins into that account."""
        txns = self.rest_client.client.post(
            f"{self.base_url}/mint?amount={amount}&address={address}"
        )
        assert txns.status_code == 200, txns.text
        for txn_hash in txns.json():
            self.rest_client.wait_for_transaction(txn_hash)


if __name__ == "__main__":
    rest_client = RestClient(TESTNET_URL)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)

    alice = Account.generate()
    bob = Account.generate()

    print("\n=== Addresses ===")
    print(f"Alice: {alice.address()}")
    print(f"Bob: {bob.address()}")

    faucet_client.fund_account(alice.address(), 1_000_000)
    faucet_client.fund_account(bob.address(), 0)

    print("\n=== Initial Balances ===")
    print(f"Alice: {rest_client.account_balance(alice.address())}")
    print(f"Bob: {rest_client.account_balance(bob.address())}")

    # Have Alice give Bob 1_000 coins
    txn_hash = rest_client.transfer(alice, bob.address(), 1_000)
    rest_client.wait_for_transaction(txn_hash)

    print("\n=== Intermediate Balances ===")
    print(f"Alice: {rest_client.account_balance(alice.address())}")
    print(f"Bob: {rest_client.account_balance(bob.address())}")

    # Have Alice give Bob another 1_000 coins using BCS
    signed_txn = rest_client.bcs_transfer(alice, bob.address(), 1_000)
    txn_hash = rest_client.submit_bcs_transaction(signed_txn)
    rest_client.wait_for_transaction(txn_hash)

    print("\n=== Final Balances ===")
    print(f"Alice: {rest_client.account_balance(alice.address())}")
    print(f"Bob: {rest_client.account_balance(bob.address())}")

#!/usr/bin/env python3

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

from nacl.signing import SigningKey
import hashlib
import requests
import time
from typing import Any, Dict, Optional

TESTNET_URL = "https://fullnode.devnet.aptoslabs.com"
FAUCET_URL = "https://faucet.devnet.aptoslabs.com"

#:!:>section_1
class Account:
    """Represents an account as well as the private, public key-pair for the Aptos blockchain."""

    def __init__(self, seed: bytes = None) -> None:
        if seed is None:
            self.signing_key = SigningKey.generate()
        else:
            self.signing_key = SigningKey(seed)

    def address(self) -> str:
        """Returns the address associated with the given account"""

        return self.auth_key()

    def auth_key(self) -> str:
        """Returns the auth_key for the associated account"""

        hasher = hashlib.sha3_256()
        hasher.update(self.signing_key.verify_key.encode() + b'\x00')
        return hasher.hexdigest()

    def pub_key(self) -> str:
        """Returns the public key for the associated account"""

        return self.signing_key.verify_key.encode().hex()
#<:!:section_1

#:!:>section_2
class RestClient:
    """A wrapper around the Aptos-core Rest API"""

    def __init__(self, url: str) -> None:
        self.url = url
#<:!:section_2

#:!:>section_3
    def account(self, account_address: str) -> Dict[str, str]:
        """Returns the sequence number and authentication key for an account"""

        response = requests.get(f"{self.url}/accounts/{account_address}")
        assert response.status_code == 200, f"{response.text} - {account_address}"
        return response.json()

    def account_resource(self, account_address: str, resource_type: str) -> Optional[Dict[str, Any]]:
        response = requests.get(f"{self.url}/accounts/{account_address}/resource/{resource_type}")
        if response.status_code == 404:
            return None
        assert response.status_code == 200, response.text
        return response.json()
#<:!:section_3

#:!:>section_4
    def generate_transaction(self, sender: str, payload: Dict[str, Any]) -> Dict[str, Any]:
        """Generates a transaction request that can be submitted to produce a raw transaction that
        can be signed, which upon being signed can be submitted to the blockchain. """

        account_res = self.account(sender)
        seq_num = int(account_res["sequence_number"])
        txn_request = {
            "sender": f"0x{sender}",
            "sequence_number": str(seq_num),
            "max_gas_amount": "2000",
            "gas_unit_price": "1",
            "gas_currency_code": "XUS",
            "expiration_timestamp_secs": str(int(time.time()) + 600),
            "payload": payload,
        }
        return txn_request

    def sign_transaction(self, account_from: Account, txn_request: Dict[str, Any]) -> Dict[str, Any]:
        """Converts a transaction request produced by `generate_transaction` into a properly signed
        transaction, which can then be submitted to the blockchain."""

        res = requests.post(f"{self.url}/transactions/signing_message", json=txn_request)
        assert res.status_code == 200, res.text
        to_sign = bytes.fromhex(res.json()["message"][2:])
        signature = account_from.signing_key.sign(to_sign).signature
        txn_request["signature"] = {
            "type": "ed25519_signature",
            "public_key": f"0x{account_from.pub_key()}",
            "signature": f"0x{signature.hex()}",
        }
        return txn_request

    def submit_transaction(self, txn: Dict[str, Any]) -> Dict[str, Any]:
        """Submits a signed transaction to the blockchain."""

        headers = {'Content-Type': 'application/json'}
        response = requests.post(f"{self.url}/transactions", headers=headers, json=txn)
        assert response.status_code == 202, f"{response.text} - {txn}"
        return response.json()

    def transaction_pending(self, txn_hash: str) -> bool:
        response = requests.get(f"{self.url}/transactions/{txn_hash}")
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
        response = requests.get(f"{self.url}/transactions/{txn_hash}")
        assert "success" in response.json(), f"{response.text} - {txn_hash}"
#<:!:section_4

#:!:>section_5
    def account_balance(self, account_address: str) -> Optional[int]:
        """Returns the test coin balance associated with the account"""
        return self.account_resource(account_address, "0x1::TestCoin::Balance")

    def transfer(self, account_from: Account, recipient: str, amount: int) -> str:
        """Transfer a given coin amount from a given Account to the recipient's account address.
        Returns the sequence number of the transaction used to transfer."""

        payload = {
            "type": "script_function_payload",
            "function": "0x1::TestCoin::transfer",
            "type_arguments": [],
            "arguments": [
                f"0x{recipient}",
                str(amount),
            ]
        }
        txn_request = self.generate_transaction(account_from.address(), payload)
        signed_txn = self.sign_transaction(account_from, txn_request)
        res = self.submit_transaction(signed_txn)
        return str(res["hash"])
#<:!:section_5

#:!:>section_6
class FaucetClient:
    """Faucet creates and funds accounts. This is a thin wrapper around that."""

    def __init__(self, url: str, rest_client: RestClient) -> None:
        self.url = url
        self.rest_client = rest_client

    def fund_account(self, address: str, amount: int) -> None:
        """This creates an account if it does not exist and mints the specified amount of
        coins into that account."""
        txns = requests.post(f"{self.url}/mint?amount={amount}&address={address}")
        assert txns.status_code == 200, txns.text
        for txn_hash in txns.json():
            self.rest_client.wait_for_transaction(txn_hash)
#<:!:section_6


#:!:>section_7
if __name__ == "__main__":
    rest_client = RestClient(TESTNET_URL)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)

    # Create two accounts, Alice and Bob, and fund Alice but not Bob
    alice = Account()
    bob = Account()

    print("\n=== Addresses ===")
    print(f"Alice: {alice.address()}")
    print(f"Bob: {bob.address()}")

    faucet_client.fund_account(alice.address(), 1_000_000)
    faucet_client.fund_account(bob.address(), 0)

    print("\n=== Initial Balances ===")
    print(f"Alice: {rest_client.account_balance(alice.address())}")
    print(f"Bob: {rest_client.account_balance(bob.address())}")

    # Have Alice give Bob 10 coins
    tx_hash = rest_client.transfer(alice, bob.address(), 1_000)
    rest_client.wait_for_transaction(tx_hash)

    print("\n=== Final Balances ===")
    print(f"Alice: {rest_client.account_balance(alice.address())}")
    print(f"Bob: {rest_client.account_balance(bob.address())}")
#<:!:section_7

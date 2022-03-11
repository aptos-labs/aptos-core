---
title: "Your first transaction on the Aptos Blockchain"
slug: "your-first-transaction"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Overview

This tutorial details how to generate, submit, and verify transactions submitted to the Aptos Blockchain. The steps to doing so:

* Create a representation of an account
* Prepare a wrapper around the REST interfaces
* Prepare a wrapper around the Faucet interface
* Combine them into an applicatiaon, execute and verify

The following tutorial contains example code that can be downloaded in its entirety below:
<Tabs>
  <TabItem value="python" label="Python" default>

[Download](/examples/first_transaction.py)
  </TabItem>
  <TabItem value="rust" label="Rust" default>
  </TabItem>
  <TabItem value="typescript" label="Typescript" default>
  </TabItem>
  <TabItem value="manual" label="Manually" default>
  </TabItem>
</Tabs>

## Step 1) Create a representation of an account

Each Aptos account has a unique account address. The owner of that account holds  the public, private key-pair that maps to the Aptos account address and, in turn, the authentication key stored in that account. See more in [account basics][account_basics]. The following snippets demonstrate what's described in that section.

<Tabs>
  <TabItem value="python" label="Python" default>

```python3

class Account:
    """Represents an account as well as the private, public key-pair for the Aptos blockchain."""

    def __init__(self) -> None:
        self.signing_key = SigningKey.generate()

    def address(self) -> str:
        """Returns the address associated with the given account"""

        return self.auth_key()[-32:]

    def auth_key(self) -> str:
        """Returns the auth_key for the associated account"""

        hasher = hashlib.sha3_256()
        hasher.update(self.signing_key.verify_key.encode() + b'\x00')
        return hasher.hexdigest()

    def pub_key(self) -> str:
        """Returns the public key for the associated account"""

        return self.signing_key.verify_key.encode().hex()
```
  </TabItem>
  <TabItem value="rust" label="Rust" default>
  </TabItem>
  <TabItem value="typescript" label="Typescript" default>
  </TabItem>
  <TabItem value="manual" label="Manually" default>
  </TabItem>
</Tabs>

## Step 2) Rest interface

Aptos exposes a [REST interface][rest_spec] for interacting with the blockchain. While the data from the REST interface can be read directly, the following snippets of code demonstrate a more ergonomic approach. This next set of code snippets demonstrates how to use the REST interface to retrieve ledger data from the FullNode including account and account resource data. It also demonstrates how to use the REST interface for constructing a signed transactions represented by JSON formatting.

<Tabs>
  <TabItem value="python" label="Python" default>

```python3

class RestClient:
    """A wrapper around the Aptos-core Rest API"""

    def __init__(self, url: str) -> None:
        self.url = url
```
  </TabItem>
  <TabItem value="rust" label="Rust" default>
  </TabItem>
  <TabItem value="typescript" label="Typescript" default>
  </TabItem>
  <TabItem value="manual" label="Manually" default>
  </TabItem>
</Tabs>

Step 2.1) Reading an account

The following are wrappers for querying account data.

<Tabs>
  <TabItem value="python" label="Python" default>

```python3
    def account(self, account_address: str) -> Dict[str, str]:
        """Returns the sequence number and authentication key for an account"""

        response = requests.get(f"{self.url}/accounts/{account_address}")
        assert response.status_code == 200, response.text
        return response.json()

    def account_resources(self, account_address: str) -> Dict[str, Any]:
        """Returns all resources associated with the account"""

        response = requests.get(f"{self.url}/accounts/{account_address}/resources")
        assert response.status_code == 200, response.text
        return response.json()
```
  </TabItem>
  <TabItem value="rust" label="Rust" default>
  </TabItem>
  <TabItem value="typescript" label="Typescript" default>
  </TabItem>
  <TabItem value="manual" label="Manually" default>
  </TabItem>
</Tabs>

Step 2.2) Submitting a transaction

The following demonstrate the core functionality for constructing, signing, and waiting on a transaction.

<Tabs>
  <TabItem value="python" label="Python" default>

```python3
    def generate_transaction(self, sender: str, payload: Dict[str, Any]) -> Dict[str, Any]:
        """Generates a transaction request that can be submitted to produce a raw transaction that
        can be signed, which upon being signed can be submitted to the blockchain. """

        account_res = self.account(sender)
        seq_num = int(account_res["sequence_number"])
        txn_request = {
            "sender": f"0x{sender}",
            "sequence_number": str(seq_num),
            "max_gas_amount": "1000000",
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
        to_sign = bytes.fromhex(res.json()['message'][2:])
        signature = account_from.signing_key.sign(to_sign).signature
        txn_request["signature"] = {
            "type": "ed25519_signature",
            "public_key": f"0x{account_from.pub_key()}",
            "signature": f"0x{signature.hex()}",
        }
        return txn_request

    def submit_transaction(self, txn: Dict[str, Any]) -> requests.Response:
        """Submits a signed transaction to the blockchain."""

        headers = {'Content-Type': 'application/json'}
        response = requests.post(f"{self.url}/transactions", headers=headers, json=txn)
        assert response.status_code == 202, response.text
        return response

    def transaction_pending(self, txn_hash: str) -> bool:
        response = requests.get(f"{self.url}/transactions/{txn_hash}")
        if response.status_code == 404:
            return True
        assert response.status_code == 200, response.text
        return response.json()["type"] == "pending_transaction"

    def wait_for_transaction(self, txn_hash: str) -> None:
        """Waits up to 10 seconds for a transaction to move past pending state."""

        count = 0
        while self.transaction_pending(txn_hash):
            assert count < 10
            time.sleep(1)
            count += 1
```
  </TabItem>
  <TabItem value="rust" label="Rust" default>
  </TabItem>
  <TabItem value="typescript" label="Typescript" default>
  </TabItem>
  <TabItem value="manual" label="Manually" default>
  </TabItem>
</Tabs>

### Step 2.3) Application specific logic

The following demonstrate how to read data from the blockchain and how to write to it, e.g., submit a specific transaction.

<Tabs>
  <TabItem value="python" label="Python" default>

```python3
    def account_balance(self, account_address: str) -> Optional[int]:
        """Returns the test coin balance associated with the account"""

        resources = self.account_resources(account_address)
        for resource in resources:
            if resource["type"] == "0x1::TestCoin::Balance":
                return int(resource["data"]["coin"]["value"])
        return None

    def transfer(self, account_from: Account, recipient: str, amount: int) -> (int, str):
        """Transfer a given coin amount from a given Account to the recipient's account address.
        Returns the sequence number of the transaction used to transfer."""

        payload = {
            "type": "script_function_payload",
            "function": "0x1::BasicScripts::transfer",
            "type_arguments": [],
            "arguments": [
                f"0x{recipient}",
                str(amount),
            ]
        }
        txn_request = self.generate_transaction(account_from.address(), payload)
        signed_txn = self.sign_transaction(account_from, txn_request)
        res = self.submit_transaction(signed_txn).json()
        return int(signed_txn["sequence_number"]), str(res["hash"])
```
  </TabItem>
  <TabItem value="rust" label="Rust" default>
  </TabItem>
  <TabItem value="typescript" label="Typescript" default>
  </TabItem>
  <TabItem value="manual" label="Manually" default>
  </TabItem>
</Tabs>

### Step 3) Faucet interface

A blockchain faucet provides an account some amount of tokens that can be used for paying gas fees or transferring of tokens betwen users. The Aptos faucet additionally can create an account if one does not exist yet. The Aptos faucet interface requires a public key represented in a hex-encoded string.

<Tabs>
  <TabItem value="python" label="Python" default>

```python3

class FaucetClient:
    """Faucet creates and funds accounts. This is a thin wrapper around that."""

    def __init__(self, url: str) -> None:
        self.url = url

    def fund_account(self, pub_key: str, amount: int) -> None:
        """This creates an account if it does not exist and mints the specified amount of
        coins into that account."""

        txns = requests.post(f"{self.url}/mint?amount={amount}&pub_key={pub_key}")
        assert txns.status_code == 200, txns.text
        for txn_hash in txns.json():
            rest_client.wait_for_transaction(txn_hash)
```
  </TabItem>
  <TabItem value="rust" label="Rust" default>
  </TabItem>
  <TabItem value="typescript" label="Typescript" default>
  </TabItem>
  <TabItem value="manual" label="Manually" default>
  </TabItem>
</Tabs>

## Step 4) Execute the application and verify

<Tabs>
  <TabItem value="python" label="Python" default>

```python3
rest_client = RestClient(TESTNET_URL)
faucet_client = FaucetClient(FAUCET_URL)

# Create two accounts, Alice and Bob, and fund Alice but not Bob
alice = Account()
bob = Account()

print("\n=== Addresses ===")
print(f"alice: {alice.address()}")
print(f"Bob: {bob.address()}")

faucet_client.fund_account(alice.pub_key(), 1_000_000_00)
faucet_client.fund_account(bob.pub_key(), 0)

print("\n=== Initial Balances ===")
print(f"Alice: {rest_client.account_balance(alice.address())}")
print(f"Bob: {rest_client.account_balance(bob.address())}")

# Have Alice give Bob 10 coins
seq_no, _tx_hash = rest_client.transfer(alice, bob.address(), 10)
rest_client.wait_for_transaction(alice.address(), seq_no)

print("\n=== Final Balances ===")
print(f"Alice: {rest_client.account_balance(alice.address())}")
print(f"Bob: {rest_client.account_balance(bob.address())}")
```
  </TabItem>
  <TabItem value="rust" label="Rust" default>
  </TabItem>
  <TabItem value="typescript" label="Typescript" default>
  </TabItem>
  <TabItem value="manual" label="Manually" default>
  </TabItem>
</Tabs>

The output after executing:
```
=== Addresses ===
Alice: e26d69b8d3ff12874358da6a4082a2ac
Bob: c8585f009c8a90f22c6b603f28b9ed8c

=== Initial Balances ===
Alice: 1000000000
Bob: 0

=== Final Balances ===
Alice: 999998957
Bob: 1000
```

The outcome shows that Bob received 1000 coins from Alice. Alice paid 43 coins for gas.

The data can be verified by visiting either a REST interface or the explorer:
* Alice's account via the [REST interface][alice_account_rest]
* Bob's account via the [explorer][bob_account_explorer]

[account_basics]: /basics/basics-accounts
[alice_account_rest]: https://dev.fullnode.aptoslabs.com/accounts/e26d69b8d3ff12874358da6a4082a2ac/resources
[bob_account_explorer]: https://explorer.devnet.aptos.dev/account/c8585f009c8a90f22c6b603f28b9ed8c
[rest_spec]: https://dev.fullnode.aptoslabs.com/spec.html
[python_download]: /examples/first_transaction.py

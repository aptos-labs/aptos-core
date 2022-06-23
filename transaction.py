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
    
    def execute_transaction_with_payload(self, account_from: Account, payload: Dict[str, Any]) -> Dict[str, Any]:
        """Execute a transaction for the given payload."""
        
        txn_request = self.generate_transaction(account_from.address(), payload)
        signed_txn = self.sign_transaction(account_from, txn_request)
        return self.submit_transaction(signed_txn)

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

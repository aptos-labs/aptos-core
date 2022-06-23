def account(self, account_address: str) -> dict[str, str]:
    """Returns the sequence number and authentication key for an account"""

    response = requests.get(f"{self.url}/accounts/{account_address}")
    assert response.status_code == 200, f"{response.text} - {account_address}"
    return response.json()

def account_resource(self, account_address: str, resource_type: str) -> Optional[dict[str, Any]]:
    response = requests.get(f"{self.url}/accounts/{account_address}/resource/{resource_type}")
    if response.status_code == 404:
        return None
    assert response.status_code == 200, response.text
    return response.json()

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

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

import base64
import json
import secrets
import tempfile
import unittest

from aptos_sdk import ed25519
from aptos_sdk.account_address import AccountAddress
from aptos_sdk.bcs import Serializer

from cryptography.exceptions import InvalidSignature
from cryptography.fernet import Fernet, InvalidToken
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC

def derive_password_protection_fernet(password: str, salt: bytes) -> Fernet:
    """Derive Fernet encryption key assistant from password and salt.

    For password-protecting a private key on disk.

    See also
    --------
    Account.store_private_key_password_protected
    Account.load_private_key_password_protected

    References
    ----------
    https://cryptography.io/en/latest/fernet
    https://stackoverflow.com/questions/2490334
    """
    key_derivation_function = PBKDF2HMAC(
        algorithm=hashes.SHA256(),
        length=32,
        salt=salt,
        iterations=480_000)
    derived_key = key_derivation_function.derive(password.encode())
    return Fernet(base64.urlsafe_b64encode(derived_key))

class Account:
    """Represents an account as well as the private, public key-pair for the Aptos blockchain."""

    account_address: AccountAddress
    private_key: ed25519.PrivateKey

    def __init__(
        self, account_address: AccountAddress, private_key: ed25519.PrivateKey
    ):
        self.account_address = account_address
        self.private_key = private_key

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, Account):
            return NotImplemented
        return (
            self.account_address == other.account_address
            and self.private_key == other.private_key
        )

    @staticmethod
    def generate() -> Account:
        private_key = ed25519.PrivateKey.random()
        account_address = AccountAddress.from_key(private_key.public_key())
        return Account(account_address, private_key)

    @staticmethod
    def load_key(key: str) -> Account:
        private_key = ed25519.PrivateKey.from_hex(key)
        account_address = AccountAddress.from_key(private_key.public_key())
        return Account(account_address, private_key)

    @staticmethod
    def load(path: str) -> Account:
        with open(path) as file:
            data = json.load(file)
        return Account(
            AccountAddress.from_hex(data["account_address"]),
            ed25519.PrivateKey.from_hex(data["private_key"]),
        )

    @staticmethod
    def load_private_key_password_protected(
            path: str, password: str) -> Account:
        """Load from disk a password-protected private key generated via
        `store_private_key_password_protected`.

        References
        ----------
        https://cryptography.io/en/latest/fernet
        https://stackoverflow.com/questions/2490334
        """
        with open(path, 'rb') as file:
            token = file.read()
        salt, encrypted = token[:16], token[16:]
        fernet = derive_password_protection_fernet(password, salt)
        try:
            decrypted = fernet.decrypt(encrypted)
        except (InvalidSignature, InvalidToken):
            raise ValueError("Invalid password") from None
        return Account.load_key(f"0x{decrypted.hex()}")

    def store(self, path: str):
        data = {
            "account_address": self.account_address.hex(),
            "private_key": self.private_key.hex(),
        }
        with open(path, "w") as file:
            json.dump(data, file)

    def store_private_key_password_protected(self, path: str, password: str):
        """Store password-protected private key in text file at path.

        Prepends random encryption salt to encrypted private key.

        References
        ----------
        https://cryptography.io/en/latest/fernet
        https://stackoverflow.com/questions/2490334
        """
        salt = secrets.token_bytes(16)
        fernet = derive_password_protection_fernet(password, salt)
        encrypted = fernet.encrypt(self.private_key.key.encode())
        token = salt + encrypted
        with open(path, 'wb') as file:
            file.write(token)

    def address(self) -> AccountAddress:
        """Returns the address associated with the given account"""

        return self.account_address

    def auth_key(self) -> str:
        """Returns the auth_key for the associated account"""

        return AccountAddress.from_key(self.private_key.public_key()).hex()

    def sign(self, data: bytes) -> ed25519.Signature:
        return self.private_key.sign(data)

    def public_key(self) -> ed25519.PublicKey:
        """Returns the public key for the associated account"""

        return self.private_key.public_key()

class RotationProofChallenge:
    type_info_account_address: AccountAddress = AccountAddress.from_hex('0x1')
    type_info_module_name: str = 'account'
    type_info_struct_name: str = 'RotationProofChallenge'
    sequence_number: int
    originator: AccountAddress
    current_auth_key: AccountAddress
    new_public_key: bytes

    def __init__(self,
                 sequence_number: int,
                 originator: AccountAddress,
                 current_auth_key: AccountAddress,
                 new_public_key: bytes):
        self.sequence_number = sequence_number
        self.originator = originator
        self.current_auth_key = current_auth_key
        self.new_public_key = new_public_key

    def serialize(self, serializer: Serializer):
        self.type_info_account_address.serialize(serializer)
        serializer.str(self.type_info_module_name)
        serializer.str(self.type_info_struct_name)
        serializer.u64(self.sequence_number)
        self.originator.serialize(serializer)
        self.current_auth_key.serialize(serializer)
        serializer.to_bytes(self.new_public_key)


class Test(unittest.TestCase):
    def test_load_and_store(self):
        (file, path) = tempfile.mkstemp()
        start = Account.generate()
        start.store(path)
        load = Account.load(path)

        self.assertEqual(start, load)
        # Auth key and Account address should be the same at start
        self.assertEqual(start.address().hex(), start.auth_key())

    def test_key(self):
        message = b"test message"
        account = Account.generate()
        signature = account.sign(message)
        self.assertTrue(account.public_key().verify(message, signature))

if __name__ == '__main__':
    generated = Account.generate()
    print(f"Pubkey: {generated.public_key()}")
    password = "Foobly"
    print(f"Password: {password}")
    print("Storing on disk")
    path = 'key.txt'
    generated.store_private_key_password_protected(path, password)
    print("Loading from disk")
    loaded = Account.load_private_key_password_protected(path, password)
    print(f"Pubkey: {loaded.public_key()}")
    print("Loading from disk with invalid password")
    loaded = Account.load_private_key_password_protected(path, 'foo')
    print(f"Pubkey: {loaded.public_key()}")
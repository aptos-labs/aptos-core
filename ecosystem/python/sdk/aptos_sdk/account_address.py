# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

from __future__ import annotations

import hashlib
import unittest
from dataclasses import dataclass

from . import asymmetric_crypto, asymmetric_crypto_wrapper, ed25519
from .bcs import Deserializer, Serializer


class AuthKeyScheme:
    Ed25519: bytes = b"\x00"
    MultiEd25519: bytes = b"\x01"
    SingleKey: bytes = b"\x02"
    MultiKey: bytes = b"\x03"
    DeriveObjectAddressFromGuid: bytes = b"\xFD"
    DeriveObjectAddressFromSeed: bytes = b"\xFE"
    DeriveResourceAccountAddress: bytes = b"\xFF"


class ParseAddressError(Exception):
    """
    There was an error parsing an address.
    """


class AccountAddress:
    address: bytes
    LENGTH: int = 32

    def __init__(self, address: bytes):
        self.address = address

        if len(address) != AccountAddress.LENGTH:
            raise ParseAddressError("Expected address of length 32")

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, AccountAddress):
            return NotImplemented
        return self.address == other.address

    def __str__(self):
        """
        Represent an account address in a way that is compliant with the v1 address
        standard. The standard is defined as part of AIP-40, read more here:
        https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-40.md

        In short, all special addresses SHOULD be represented in SHORT form, e.g.

        0x1

        All other addresses MUST be represented in LONG form, e.g.

        0x002098630cfad4734812fa37dc18d9b8d59242feabe49259e26318d468a99584

        For an explanation of what defines a "special" address, see `is_special`.

        All string representations of addresses MUST be prefixed with 0x.
        """
        suffix = self.address.hex()
        if self.is_special():
            suffix = suffix.lstrip("0") or "0"
        return f"0x{suffix}"

    def __repr__(self):
        return self.__str__()

    def is_special(self):
        """
        Returns whether the address is a "special" address. Addresses are considered
        special if the first 63 characters of the hex string are zero. In other words,
        an address is special if the first 31 bytes are zero and the last byte is
        smaller than `0b10000` (16). In other words, special is defined as an address
        that matches the following regex: `^0x0{63}[0-9a-f]$`. In short form this means
        the addresses in the range from `0x0` to `0xf` (inclusive) are special.

        For more details see the v1 address standard defined as part of AIP-40:
        https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-40.md
        """
        return all(b == 0 for b in self.address[:-1]) and self.address[-1] < 0b10000

    @staticmethod
    def from_str(address: str) -> AccountAddress:
        """
        NOTE: This function has strict parsing behavior. For relaxed behavior, please use
        `from_string_relaxed` function.

        Creates an instance of AccountAddress from a hex string.

        This function allows only the strictest formats defined by AIP-40. In short this
        means only the following formats are accepted:
        - LONG
        - SHORT for special addresses

        Where:
        - LONG is defined as 0x + 64 hex characters.
        - SHORT for special addresses is 0x0 to 0xf inclusive without padding zeroes.

        This means the following are not accepted:
        - SHORT for non-special addresses.
        - Any address without a leading 0x.

        Learn more about the different address formats by reading AIP-40:
        https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-40.md.

        Parameters:
        - address (str): A hex string representing an account address.

        Returns:
        - AccountAddress: An instance of AccountAddress.
        """
        # Assert the string starts with 0x.
        if not address.startswith("0x"):
            raise RuntimeError("Hex string must start with a leading 0x.")

        out = AccountAddress.from_str_relaxed(address)

        # Check if the address is in LONG form. If it is not, this is only allowed for
        # special addresses, in which case we check it is in proper SHORT form.
        if len(address) != AccountAddress.LENGTH * 2 + 2:
            if not out.is_special():
                raise RuntimeError(
                    "The given hex string is not a special address, it must be represented "
                    "as 0x + 64 chars."
                )
            else:
                # 0x + one hex char is the only valid SHORT form for special addresses.
                if len(address) != 3:
                    raise RuntimeError(
                        "The given hex string is a special address not in LONG form, "
                        "it must be 0x0 to 0xf without padding zeroes."
                    )

        # Assert that only special addresses can use short form.
        if len(address[2:]) != AccountAddress.LENGTH * 2 and not out.is_special():
            raise RuntimeError(
                "Padding zeroes are not allowed, the address must be represented as "
                "0x0 to 0xf for special addresses or 0x + 64 chars for all other addresses."
            )

        return out

    @staticmethod
    def from_str_relaxed(address: str) -> AccountAddress:
        """
        NOTE: This function has relaxed parsing behavior. For strict behavior, please use
        the `from_string` function. Where possible, use `from_string` rather than this
        function. `from_string_relaxed` is only provided for backwards compatibility.

        Creates an instance of AccountAddress from a hex string.

        This function allows all formats defined by AIP-40. In short, this means the
        following formats are accepted:
        - LONG, with or without leading 0x
        - SHORT, with or without leading 0x

        Where:
        - LONG is 64 hex characters.
        - SHORT is 1 to 63 hex characters inclusive.
        - Padding zeroes are allowed, e.g., 0x0123 is valid.

        Learn more about the different address formats by reading AIP-40:
        https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-40.md.

        Parameters:
        - address (str): A hex string representing an account address.

        Returns:
        - AccountAddress: An instance of AccountAddress.
        """
        addr = address

        # Strip 0x prefix if present.
        if address[0:2] == "0x":
            addr = address[2:]

        # Assert the address is at least one hex char long.
        if len(addr) < 1:
            raise RuntimeError(
                "Hex string is too short, must be 1 to 64 chars long, excluding the "
                "leading 0x."
            )

        # Assert the address is at most 64 hex chars long.
        if len(addr) > 64:
            raise RuntimeError(
                "Hex string is too long, must be 1 to 64 chars long, excluding the "
                "leading 0x."
            )

        if len(addr) < AccountAddress.LENGTH * 2:
            pad = "0" * (AccountAddress.LENGTH * 2 - len(addr))
            addr = pad + addr

        return AccountAddress(bytes.fromhex(addr))

    @staticmethod
    def from_key(key: asymmetric_crypto.PublicKey) -> AccountAddress:
        hasher = hashlib.sha3_256()
        hasher.update(key.to_crypto_bytes())

        if isinstance(key, ed25519.PublicKey):
            hasher.update(AuthKeyScheme.Ed25519)
        elif isinstance(key, ed25519.MultiPublicKey):
            hasher.update(AuthKeyScheme.MultiEd25519)
        elif isinstance(key, asymmetric_crypto_wrapper.PublicKey):
            hasher.update(AuthKeyScheme.SingleKey)
        else:
            raise Exception("Unsupported asymmetric_crypto.PublicKey key type.")

        return AccountAddress(hasher.digest())

    @staticmethod
    def for_resource_account(creator: AccountAddress, seed: bytes) -> AccountAddress:
        hasher = hashlib.sha3_256()
        hasher.update(creator.address)
        hasher.update(seed)
        hasher.update(AuthKeyScheme.DeriveResourceAccountAddress)
        return AccountAddress(hasher.digest())

    @staticmethod
    def for_guid_object(creator: AccountAddress, creation_num: int) -> AccountAddress:
        hasher = hashlib.sha3_256()
        serializer = Serializer()
        serializer.u64(creation_num)
        hasher.update(serializer.output())
        hasher.update(creator.address)
        hasher.update(AuthKeyScheme.DeriveObjectAddressFromGuid)
        return AccountAddress(hasher.digest())

    @staticmethod
    def for_named_object(creator: AccountAddress, seed: bytes) -> AccountAddress:
        hasher = hashlib.sha3_256()
        hasher.update(creator.address)
        hasher.update(seed)
        hasher.update(AuthKeyScheme.DeriveObjectAddressFromSeed)
        return AccountAddress(hasher.digest())

    @staticmethod
    def for_named_token(
        creator: AccountAddress, collection_name: str, token_name: str
    ) -> AccountAddress:
        collection_bytes = collection_name.encode()
        token_bytes = token_name.encode()
        return AccountAddress.for_named_object(
            creator, collection_bytes + b"::" + token_bytes
        )

    @staticmethod
    def for_named_collection(
        creator: AccountAddress, collection_name: str
    ) -> AccountAddress:
        return AccountAddress.for_named_object(creator, collection_name.encode())

    @staticmethod
    def deserialize(deserializer: Deserializer) -> AccountAddress:
        return AccountAddress(deserializer.fixed_bytes(AccountAddress.LENGTH))

    def serialize(self, serializer: Serializer):
        serializer.fixed_bytes(self.address)


"""
Tests
"""


@dataclass(init=True, frozen=True)
class TestAddresses:
    shortWith0x: str
    shortWithout0x: str
    longWith0x: str
    longWithout0x: str
    bytes: bytes


ADDRESS_ZERO = TestAddresses(
    shortWith0x="0x0",
    shortWithout0x="0",
    longWith0x="0x0000000000000000000000000000000000000000000000000000000000000000",
    longWithout0x="0000000000000000000000000000000000000000000000000000000000000000",
    bytes=bytes([0] * 32),
)

ADDRESS_F = TestAddresses(
    shortWith0x="0xf",
    shortWithout0x="f",
    longWith0x="0x000000000000000000000000000000000000000000000000000000000000000f",
    longWithout0x="000000000000000000000000000000000000000000000000000000000000000f",
    bytes=bytes([0] * 31 + [15]),
)

ADDRESS_F_PADDED_SHORT_FORM = TestAddresses(
    shortWith0x="0x0f",
    shortWithout0x="0f",
    # The rest of these below are the same as for ADDRESS_F.
    longWith0x="0x000000000000000000000000000000000000000000000000000000000000000f",
    longWithout0x="000000000000000000000000000000000000000000000000000000000000000f",
    bytes=bytes([0] * 31 + [15]),
)

ADDRESS_TEN = TestAddresses(
    shortWith0x="0x10",
    shortWithout0x="10",
    longWith0x="0x0000000000000000000000000000000000000000000000000000000000000010",
    longWithout0x="0000000000000000000000000000000000000000000000000000000000000010",
    bytes=bytes([0] * 31 + [16]),
)

ADDRESS_OTHER = TestAddresses(
    shortWith0x="0xca843279e3427144cead5e4d5999a3d0ca843279e3427144cead5e4d5999a3d0",
    shortWithout0x="ca843279e3427144cead5e4d5999a3d0ca843279e3427144cead5e4d5999a3d0",
    longWith0x="0xca843279e3427144cead5e4d5999a3d0ca843279e3427144cead5e4d5999a3d0",
    longWithout0x="ca843279e3427144cead5e4d5999a3d0ca843279e3427144cead5e4d5999a3d0",
    bytes=bytes(
        [
            202,
            132,
            50,
            121,
            227,
            66,
            113,
            68,
            206,
            173,
            94,
            77,
            89,
            153,
            163,
            208,
            202,
            132,
            50,
            121,
            227,
            66,
            113,
            68,
            206,
            173,
            94,
            77,
            89,
            153,
            163,
            208,
        ]
    ),
)


class Test(unittest.TestCase):
    def test_multi_ed25519(self):
        private_key_1 = ed25519.PrivateKey.from_str(
            "4e5e3be60f4bbd5e98d086d932f3ce779ff4b58da99bf9e5241ae1212a29e5fe"
        )
        private_key_2 = ed25519.PrivateKey.from_str(
            "1e70e49b78f976644e2c51754a2f049d3ff041869c669523ba95b172c7329901"
        )
        multisig_public_key = ed25519.MultiPublicKey(
            [private_key_1.public_key(), private_key_2.public_key()], 1
        )

        expected = AccountAddress.from_str_relaxed(
            "835bb8c5ee481062946b18bbb3b42a40b998d6bf5316ca63834c959dc739acf0"
        )
        actual = AccountAddress.from_key(multisig_public_key)
        self.assertEqual(actual, expected)

    def test_resource_account(self):
        base_address = AccountAddress.from_str_relaxed("b0b")
        expected = AccountAddress.from_str_relaxed(
            "ee89f8c763c27f9d942d496c1a0dcf32d5eacfe78416f9486b8db66155b163b0"
        )
        actual = AccountAddress.for_resource_account(base_address, b"\x0b\x00\x0b")
        self.assertEqual(actual, expected)

    def test_named_object(self):
        base_address = AccountAddress.from_str_relaxed("b0b")
        expected = AccountAddress.from_str_relaxed(
            "f417184602a828a3819edf5e36285ebef5e4db1ba36270be580d6fd2d7bcc321"
        )
        actual = AccountAddress.for_named_object(base_address, b"bob's collection")
        self.assertEqual(actual, expected)

    def test_collection(self):
        base_address = AccountAddress.from_str_relaxed("b0b")
        expected = AccountAddress.from_str_relaxed(
            "f417184602a828a3819edf5e36285ebef5e4db1ba36270be580d6fd2d7bcc321"
        )
        actual = AccountAddress.for_named_collection(base_address, "bob's collection")
        self.assertEqual(actual, expected)

    def test_token(self):
        base_address = AccountAddress.from_str_relaxed("b0b")
        expected = AccountAddress.from_str_relaxed(
            "e20d1f22a5400ba7be0f515b7cbd00edc42dbcc31acc01e31128b2b5ddb3c56e"
        )
        actual = AccountAddress.for_named_token(
            base_address, "bob's collection", "bob's token"
        )
        self.assertEqual(actual, expected)

    def test_to_standard_string(self):
        # Test special address: 0x0
        self.assertEqual(
            str(
                AccountAddress.from_str_relaxed(
                    "0x0000000000000000000000000000000000000000000000000000000000000000"
                )
            ),
            "0x0",
        )

        # Test special address: 0x1
        self.assertEqual(
            str(
                AccountAddress.from_str_relaxed(
                    "0x0000000000000000000000000000000000000000000000000000000000000001"
                )
            ),
            "0x1",
        )

        # Test special address: 0x4
        self.assertEqual(
            str(
                AccountAddress.from_str_relaxed(
                    "0x0000000000000000000000000000000000000000000000000000000000000004"
                )
            ),
            "0x4",
        )

        # Test special address: 0xf
        self.assertEqual(
            str(
                AccountAddress.from_str_relaxed(
                    "0x000000000000000000000000000000000000000000000000000000000000000f"
                )
            ),
            "0xf",
        )

        # Test special address from short no 0x: d
        self.assertEqual(
            str(AccountAddress.from_str_relaxed("d")),
            "0xd",
        )

        # Test non-special address from long:
        # 0x0000000000000000000000000000000000000000000000000000000000000010
        value = "0x0000000000000000000000000000000000000000000000000000000000000010"
        self.assertEqual(
            str(AccountAddress.from_str_relaxed(value)),
            value,
        )

        # Test non-special address from long:
        # 0x000000000000000000000000000000000000000000000000000000000000001f
        value = "0x000000000000000000000000000000000000000000000000000000000000001f"
        self.assertEqual(
            str(AccountAddress.from_str_relaxed(value)),
            value,
        )

        # Test non-special address from long:
        # 0x00000000000000000000000000000000000000000000000000000000000000a0
        value = "0x00000000000000000000000000000000000000000000000000000000000000a0"
        self.assertEqual(
            str(AccountAddress.from_str_relaxed(value)),
            value,
        )

        # Test non-special address from long no 0x:
        # ca843279e3427144cead5e4d5999a3d0ca843279e3427144cead5e4d5999a3d0
        value = "ca843279e3427144cead5e4d5999a3d0ca843279e3427144cead5e4d5999a3d0"
        self.assertEqual(
            str(AccountAddress.from_str_relaxed(value)),
            f"0x{value}",
        )

        # Test non-special address from long no 0x:
        # 1000000000000000000000000000000000000000000000000000000000000000
        value = "1000000000000000000000000000000000000000000000000000000000000000"
        self.assertEqual(
            str(AccountAddress.from_str_relaxed(value)),
            f"0x{value}",
        )

        # Demonstrate that neither leading nor trailing zeroes get trimmed for
        # non-special addresses:
        # 0f00000000000000000000000000000000000000000000000000000000000000
        value = "0f00000000000000000000000000000000000000000000000000000000000000"
        self.assertEqual(
            str(AccountAddress.from_str_relaxed(value)),
            f"0x{value}",
        )

    def test_from_str_relaxed(self):
        # Demonstrate that all formats are accepted for 0x0.
        self.assertEqual(
            str(AccountAddress.from_str_relaxed(ADDRESS_ZERO.longWith0x)),
            ADDRESS_ZERO.shortWith0x,
        )
        self.assertEqual(
            str(AccountAddress.from_str_relaxed(ADDRESS_ZERO.longWithout0x)),
            ADDRESS_ZERO.shortWith0x,
        )
        self.assertEqual(
            str(AccountAddress.from_str_relaxed(ADDRESS_ZERO.shortWith0x)),
            ADDRESS_ZERO.shortWith0x,
        )
        self.assertEqual(
            str(AccountAddress.from_str_relaxed(ADDRESS_ZERO.shortWithout0x)),
            ADDRESS_ZERO.shortWith0x,
        )

        # Demonstrate that all formats are accepted for 0xf.
        self.assertEqual(
            str(AccountAddress.from_str_relaxed(ADDRESS_F.longWith0x)),
            ADDRESS_F.shortWith0x,
        )
        self.assertEqual(
            str(AccountAddress.from_str_relaxed(ADDRESS_F.longWithout0x)),
            ADDRESS_F.shortWith0x,
        )
        self.assertEqual(
            str(AccountAddress.from_str_relaxed(ADDRESS_F.shortWith0x)),
            ADDRESS_F.shortWith0x,
        )
        self.assertEqual(
            str(AccountAddress.from_str_relaxed(ADDRESS_F.shortWithout0x)),
            ADDRESS_F.shortWith0x,
        )

        # Demonstrate that padding zeroes are allowed for 0x0f.
        self.assertEqual(
            str(
                AccountAddress.from_str_relaxed(ADDRESS_F_PADDED_SHORT_FORM.shortWith0x)
            ),
            ADDRESS_F.shortWith0x,
        )
        self.assertEqual(
            str(
                AccountAddress.from_str_relaxed(
                    ADDRESS_F_PADDED_SHORT_FORM.shortWithout0x
                )
            ),
            ADDRESS_F.shortWith0x,
        )

        # Demonstrate that all formats are accepted for 0x10.
        self.assertEqual(
            str(AccountAddress.from_str_relaxed(ADDRESS_TEN.longWith0x)),
            ADDRESS_TEN.longWith0x,
        )
        self.assertEqual(
            str(AccountAddress.from_str_relaxed(ADDRESS_TEN.longWithout0x)),
            ADDRESS_TEN.longWith0x,
        )
        self.assertEqual(
            str(AccountAddress.from_str_relaxed(ADDRESS_TEN.shortWith0x)),
            ADDRESS_TEN.longWith0x,
        )
        self.assertEqual(
            str(AccountAddress.from_str_relaxed(ADDRESS_TEN.shortWithout0x)),
            ADDRESS_TEN.longWith0x,
        )

        # Demonstrate that all formats are accepted for other addresses.
        self.assertEqual(
            str(AccountAddress.from_str_relaxed(ADDRESS_OTHER.longWith0x)),
            ADDRESS_OTHER.longWith0x,
        )
        self.assertEqual(
            str(AccountAddress.from_str_relaxed(ADDRESS_OTHER.longWithout0x)),
            ADDRESS_OTHER.longWith0x,
        )

    def test_from_str(self):
        # Demonstrate that only LONG and SHORT are accepted for 0x0.
        self.assertEqual(
            str(AccountAddress.from_str(ADDRESS_ZERO.longWith0x)),
            ADDRESS_ZERO.shortWith0x,
        )
        self.assertRaises(
            RuntimeError, AccountAddress.from_str, ADDRESS_ZERO.longWithout0x
        )
        self.assertEqual(
            str(AccountAddress.from_str(ADDRESS_ZERO.shortWith0x)),
            ADDRESS_ZERO.shortWith0x,
        )
        self.assertRaises(
            RuntimeError, AccountAddress.from_str, ADDRESS_ZERO.shortWithout0x
        )

        # Demonstrate that only LONG and SHORT are accepted for 0xf.
        self.assertEqual(
            str(AccountAddress.from_str(ADDRESS_F.longWith0x)), ADDRESS_F.shortWith0x
        )
        self.assertRaises(
            RuntimeError, AccountAddress.from_str, ADDRESS_F.longWithout0x
        )
        self.assertEqual(
            str(AccountAddress.from_str(ADDRESS_F.shortWith0x)), ADDRESS_F.shortWith0x
        )
        self.assertRaises(
            RuntimeError, AccountAddress.from_str, ADDRESS_F.shortWithout0x
        )

        # Demonstrate that padding zeroes are not allowed for 0x0f.
        self.assertRaises(
            RuntimeError,
            AccountAddress.from_str,
            ADDRESS_F_PADDED_SHORT_FORM.shortWith0x,
        )
        self.assertRaises(
            RuntimeError,
            AccountAddress.from_str,
            ADDRESS_F_PADDED_SHORT_FORM.shortWithout0x,
        )

        # Demonstrate that only LONG format is accepted for 0x10.
        self.assertEqual(
            str(AccountAddress.from_str(ADDRESS_TEN.longWith0x)), ADDRESS_TEN.longWith0x
        )
        self.assertRaises(
            RuntimeError, AccountAddress.from_str, ADDRESS_TEN.longWithout0x
        )
        self.assertRaises(
            RuntimeError, AccountAddress.from_str, ADDRESS_TEN.shortWith0x
        )
        self.assertRaises(
            RuntimeError, AccountAddress.from_str, ADDRESS_TEN.shortWithout0x
        )

        # Demonstrate that only LONG format is accepted for other addresses.
        self.assertEqual(
            str(AccountAddress.from_str(ADDRESS_OTHER.longWith0x)),
            ADDRESS_OTHER.longWith0x,
        )
        self.assertRaises(
            RuntimeError, AccountAddress.from_str, ADDRESS_OTHER.longWithout0x
        )

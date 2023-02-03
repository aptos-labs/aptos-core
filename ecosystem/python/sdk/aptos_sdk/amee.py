"""Aptos Multisig Execution Expeditor (AMEE)."""

import argparse
import base64
import getpass
import json
import secrets

from pathlib import Path

from typing import Any, Dict, Tuple

from aptos_sdk.account import Account

from cryptography.exceptions import InvalidSignature
from cryptography.fernet import Fernet, InvalidToken
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC

MIN_PASSWORD_LENGTH = 4
"""The minimum password length."""

def check_keyfile_password(path: Path) -> Tuple[Dict[Any, Any], bytes]:
    """Check keyfile password, returning JSON data/private key bytes"""
    with open(path, 'r', encoding='utf-8') as keyfile: # With file open:
        data = json.load(keyfile) # Load JSON data from keyfile.
    salt = bytes.fromhex(data['salt'][2:]) # Get salt bytes.
    # Get encrypted private key.
    encrypted_private_key = bytes.fromhex(data['encrypted_private_key'][2:])
    password = str(getpass.getpass( # Get keyfile password.
        'Enter password for encrypted private key: '))
    # Generate Fernet encryption assistant from password and salt.
    fernet = derive_password_protection_fernet(password, salt)
    try: # Try decrypting private key.
        private_key = fernet.decrypt(encrypted_private_key)
    # Assert encrypted private key can be decrypted.
    except (InvalidSignature, InvalidToken):
        raise ValueError('Invalid password.') from None
    return data, private_key

def check_password_length(password: str):
    """Verify password meets minimum length threshold."""
    if len(password) < MIN_PASSWORD_LENGTH:
        raise ValueError(
            f'Password should be at least {MIN_PASSWORD_LENGTH} characters.')

def check_outfile_exists(path: Path):
    """Verify desired outfile does not already exist."""
    if path.exists(): # Assert not overwriting file.
        raise ValueError(f'{path} already exists.')

def derive_password_protection_fernet(password: str, salt: bytes) -> Fernet:
    """Derive Fernet encryption key assistant from password and salt.

    For password-protecting a private key on disk.

    References
    ----------
    https://cryptography.io/en/latest/fernet ("Using passwords with Fernet")
    https://stackoverflow.com/questions/2490334
    """
    key_derivation_function = PBKDF2HMAC(
        algorithm=hashes.SHA256(),
        length=32,
        salt=salt,
        iterations=480_000) # Create key derivation function from salt.
    # Derive key from password.
    derived_key = key_derivation_function.derive(password.encode())
    # Return Fernet encryption assistant.
    return Fernet(base64.urlsafe_b64encode(derived_key))

def encrypt_private_key(private_key_bytes: bytes) -> Tuple[bytes, bytes]:
    """Return encrypted private key and salt."""
    password = str(getpass.getpass( # Get keyfile password.
        'Enter new password for encrypting private key: '))
    check_password_length(password)
    # Have user re-enter password.
    check = str(getpass.getpass('Re-enter password: '))
    if password != check: # Raise error if passwords do not match.
        raise ValueError('Passwords do not match.')
    # Raise error if password does not meet minimum length threshold.
    salt = secrets.token_bytes(16) # Generate encryption salt.
    # Generate Fernet encryption assistant from password and salt.
    fernet = derive_password_protection_fernet(password, salt)
    # Encrypt private key.
    encrypted_private_key = fernet.encrypt(private_key_bytes)
    return encrypted_private_key, salt

def keyfile_change_password(args):
    """Change password for a single-signer keyfile."""
    # Check password, get keyfile data and private key bytes.
    data, private_key_bytes = check_keyfile_password(args.keyfile)
    # Encrypt private key.
    encrypted_private_key_bytes, salt = encrypt_private_key(private_key_bytes)
    # Update JSON with encrypted private key.
    data['encrypted_private_key'] = f'0x{encrypted_private_key_bytes.hex()}'
    # Update JSON with new salt.
    data['salt'] = f'0x{salt.hex()}'
    keyfile_write_json(args.keyfile, data) # Write JSON to keyfile.

def keyfile_extract(args):
    """Extract private key from keyfile, store via
    aptos_sdk.account.Account.store"""
    check_outfile_exists(args.account_store) # Check if path exists.
    # Load private key bytes.
    _, private_key_bytes = check_keyfile_password(args.keyfile)
    account = Account.load_key(private_key_bytes.hex()) # Load account.
    account.store(f'{args.account_store}') # Store Aptos account file.
    with open(args.account_store, 'r', encoding='utf-8') as outfile:
        # Print contents of new account store.
        print(f'New account store at {args.account_store}: \n{outfile.read()}')

def keyfile_generate(args):
    """Generate a keyfile for a single signer."""
    signatory = ' '.join(args.signatory) # Get signatory name.
    # Assert that signatory name is not blank space.
    if len(signatory) == 0 or signatory.isspace():
        raise ValueError('Signatory name may not be blank space.')
    if args.account_store is None: # If no account store supplied:
        account = Account.generate() # Generate new account.
    else: # If account store path supplied:
        # Generate an account from it.
        account = Account.load(f"{args.account_store}")
    # Get private key bytes.
    private_key_bytes = account.private_key.key.encode()
    # Encrypt private key.
    encrypted_private_key_bytes, salt = encrypt_private_key(private_key_bytes)
    # Generate JSON data for keyfile.
    data = {'signatory': signatory,
            'public_key': f'{account.public_key()}',
            'authentication_key': f'{account.auth_key()}',
            'encrypted_private_key': f'0x{encrypted_private_key_bytes.hex()}',
            'salt': f'0x{salt.hex()}'}
    if args.keyfile is None: # If no custom filepath specified:
        # Create filepath base from signatory name.
        args.keyfile = Path('_'.join(args.signatory).casefold() +'.keyfile')
    check_outfile_exists(args.keyfile) # Check if path exists.
    keyfile_write_json(args.keyfile, data) # Write JSON to keyfile.

def keyfile_verify(args):
    """Verify password for single-signer keyfile generated via
    keyfile_generate(), show public key and authentication key."""
    data, _ = check_keyfile_password(args.keyfile) # Load JSON data.
    # Print keyfile metadata.
    print(f'Keyfile password verified for {data["signatory"]}')
    print(f'Public key:         {data["public_key"]}')
    print(f'Authentication key: {data["authentication_key"]}')

def keyfile_write_json(path: Path, data: Dict[str, str]):
    """Write JSON data to keyfile path."""
    # Dump JSON data to keyfile.
    with open(path, 'w', encoding='utf-8') as keyfile:
        json.dump(data, keyfile, indent=4)
    with open(path, 'r', encoding='utf-8') as keyfile:
        # Print contents of new keyfile.
        print(f'New keyfile at {path}: \n{keyfile.read()}')

# AMEE parser.
parser = argparse.ArgumentParser(
    description='''Aptos Multisig Execution Expeditor (AMEE): A collection of
        tools designed to expedite multisig account execution.''')
subparsers = parser.add_subparsers(required=True)

# Keyfile subcommand parser.
parser_keyfile = subparsers.add_parser(
    name='keyfile',
    aliases=['k'],
    description='Assorted single-signer keyfile operations.',
    help='Single-signer keyfile operations.')
subparsers_keyfile = parser_keyfile.add_subparsers(required=True)

# Keyfile change password subcommand parser.
parser_keyfile_change_password = subparsers_keyfile.add_parser(
    name='change-password',
    aliases=['c'],
    description='''Change password for a single-singer keyfile.''',
    help='Change keyfile password.')
parser_keyfile_change_password.set_defaults(func=keyfile_change_password)
parser_keyfile_change_password.add_argument(
    'keyfile',
    type=Path,
    help='''Relative path to keyfile.''')

# Keyfile extract subcommand parser.
parser_keyfile_extract = subparsers_keyfile.add_parser(
    name='extract',
    aliases=['e'],
    description='''Generate an `aptos_sdk.account.Account` from a single-signer
        keyfile then store on disk via `aptos_sdk.account.Account.store`.''',
    help='Extract Aptos account store from keyfile.')
parser_keyfile_extract.set_defaults(func=keyfile_extract)
parser_keyfile_extract.add_argument(
    'keyfile',
    type=Path,
    help='''Relative path to keyfile to extract from.''')
parser_keyfile_extract.add_argument(
    'account_store',
    metavar='account-store',
    type=Path,
    help='''Relative path to account file to store in.''')

# Keyfile generate subcommand parser.
parser_keyfile_generate = subparsers_keyfile.add_parser(
    name='generate',
    aliases=['g'],
    description='Generate a single-signer keyfile.',
    help='Generate new keyfile.')
parser_keyfile_generate.set_defaults(func=keyfile_generate)
parser_keyfile_generate.add_argument(
    'signatory',
    type=str,
    nargs='*',
    help='''The name of the entity acting as a signatory. For example "Aptos"
        or "The Aptos Foundation".''')
parser_keyfile_generate.add_argument(
    '-a',
    '--account-store',
    type=Path,
    help='''Relative path to Aptos account data generated via
        `aptos_sdk.account.Account.store()`.''')
parser_keyfile_generate.add_argument(
    '-k',
    '--keyfile',
    type=Path,
    help='''Custom relative path to desired keyfile.''')

# Keyfile verify subcommand parser.
parser_keyfile_verify = subparsers_keyfile.add_parser(
    name='verify',
    aliases=['v'],
    description='Verify password for a single-signer keyfile.',
    help='Verify keyfile password.')
parser_keyfile_verify.set_defaults(func=keyfile_verify)
parser_keyfile_verify.add_argument(
    'keyfile',
    type=Path,
    help='''Relative path to keyfile.''')

parsed_args = parser.parse_args() # Parse command line arguments.
parsed_args.func(parsed_args) # Call command line argument function.

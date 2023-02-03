"""Aptos Multisig Execution Expeditor (AMEE)."""

import argparse
import base64
import getpass
import json
import pathlib
import secrets

from aptos_sdk.account import Account

from cryptography.exceptions import InvalidSignature
from cryptography.fernet import Fernet, InvalidToken
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC

def derive_password_protection_fernet(password: str, salt: bytes) -> Fernet:
    """Derive Fernet encryption key assistant from password and salt.

    For password-protecting a private key on disk.

    References
    ----------
    https://cryptography.io/en/latest/fernet
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

def keyfile_generate(args):
    """Generate a keyfile for a single signer."""
    min_password_length = 4 # Declare minimum password length.
    signatory = ' '.join(args.signatory) # Get signatory name.
    # Assert that signatory name is not blank space.
    if len(signatory) == 0 or signatory.isspace():
        raise ValueError('Signatory name may not be blank space.')
    if args.account_store is None: # If no account store supplied:
        account = Account.generate() # Generate new account.
    else: # If account store path supplied:
        # Generate an account from it.
        account = Account.load(f"{args.account_store}")
    password = str(getpass.getpass( # Get keyfile password.
        'Enter password for encrypting private key: '))
    if len(password) < min_password_length:
        raise ValueError(
            f'Password should be at least {min_password_length} characters.')
    # Have user re-enter password.
    check = str(getpass.getpass('Re-enter password: '))
    if password != check: # Raise error if passwords do not match.
        raise ValueError('Passwords do not match.')
    # Raise error if password does not meet minimum length threshold.
    salt = secrets.token_bytes(16) # Generate encryption salt.
    # Generate Fernet encryption assistant from password and salt.
    fernet = derive_password_protection_fernet(password, salt)
    # Encrypt account's private key.
    encrypted_private_key = fernet.encrypt(account.private_key.key.encode())
    # Generate JSON data for keyfile.
    data = {'signatory': signatory,
            'public_key': f'{account.public_key()}',
            'authentication_key': f'{account.auth_key()}',
            'encrypted_private_key': f'0x{encrypted_private_key.hex()}',
            'salt': f'0x{salt.hex()}'}
    if args.filepath is None: # If no custom filepath specified:
        # Create filepath base from signatory name.
        args.filepath = pathlib.Path('_'.join(args.signatory).casefold())
    # Append keyfile suffix to path.
    args.filepath = args.filepath.parent / (args.filepath.name + '.keyfile')
    if args.filepath.exists(): # Assert not overwriting file.
        raise ValueError(f'{args.filepath} already exists.')
    # Dump JSON data to keyfile.
    with open(args.filepath, 'w', encoding='utf-8') as keyfile:
        json.dump(data, keyfile, indent=4)
    with open(args.filepath, 'r', encoding='utf-8') as keyfile:
        # Print contents of new keyfile.
        print(f'New keyfile at {args.filepath}: \n{keyfile.read()}')

def keyfile_verify(args):
    """Verify password for single-signer keyfile generated via
    keyfile_generate(), show public key and authentication key."""
    data = json.load(args.filepath) # Load JSON data.
    salt = bytes.fromhex(data['salt'][2:]) # Get salt bytes.
    # Get encrypted private key.
    encrypted_private_key = bytes.fromhex(data['encrypted_private_key'][2:])
    password = str(getpass.getpass( # Get keyfile password.
        'Enter password for encrypted private key: '))
    # Generate Fernet encryption assistant from password and salt.
    fernet = derive_password_protection_fernet(password, salt)
    try: # Try decrypting private key.
        fernet.decrypt(encrypted_private_key)
    # Assert encrypted private key can be decrypted.
    except (InvalidSignature, InvalidToken):
        raise ValueError('Invalid password.') from None
    # Print keyfile metadata.
    print(f'{data["signatory"]} keyfile password verified')
    print(f'Public key:         {data["public_key"]}')
    print(f'Authentication key: {data["authentication_key"]}')

# AMEE parser.
parser = argparse.ArgumentParser(
    description='''Aptos Multisig Execution Expeditor (AMEE): A collection of
        tools designed to expedite multisig account execution.''')
subparsers = parser.add_subparsers(required=True)

# Keyfile subcommand parser.
parser_keyfile = subparsers.add_parser(
    name='keyfile',
    description='Single-signer keyfile operations')
subparsers_keyfile = parser_keyfile.add_subparsers(required=True)

# Keyfile generate subcommand parser.
parser_keyfile_generate = subparsers_keyfile.add_parser(
    name='generate',
    description='Generate a single-signer keyfile.')
parser_keyfile_generate.set_defaults(func=keyfile_generate)
parser_keyfile_generate.add_argument(
    'signatory',
    type=str,
    nargs='*',
    help='''The name of the entity acting as a signatory. For example "Aptos"
        or "The Aptos Foundation".''')
parser_keyfile_generate.add_argument(
    '--account-store',
    type=pathlib.Path,
    help='''Relative path to Aptos account data generated via
        `aptos_sdk.account.Account.store()`. For example "my_account.txt".''')
parser_keyfile_generate.add_argument(
    '--filepath',
    type=pathlib.Path,
    help='''Custom relative path to desired keyfile. For example
        "keyfiles/myfile", which will result in keyfiles/myfile.keyfile.''')

# Keyfile verify subcommand parser.
parser_keyfile_verify = subparsers_keyfile.add_parser(
    name='verify',
    description='Verify password for a single-signer keyfile.')
parser_keyfile_verify.set_defaults(func=keyfile_verify)
parser_keyfile_verify.add_argument(
    'filepath',
    type=argparse.FileType('r', encoding='utf-8'),
    help='''Relative path to keyfile.''')

parsed_args = parser.parse_args() # Parse command line arguments.
parsed_args.func(parsed_args) # Call command line argument function.

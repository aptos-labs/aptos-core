"""Aptos Multisig Execution Expeditor (AMEE)."""

import argparse
import base64
import getpass
import json
import secrets
import subprocess
from datetime import datetime, timedelta
from io import TextIOWrapper
from pathlib import Path
from typing import Any, Dict, List
from typing import Optional as Option
from typing import Tuple

from aptos_sdk.account import Account, RotationProofChallenge
from aptos_sdk.account_address import AccountAddress
from aptos_sdk.authenticator import Authenticator, Ed25519Authenticator
from aptos_sdk.bcs import Serializer
from aptos_sdk.client import ClientConfig, RestClient
from aptos_sdk.ed25519 import (MultiEd25519PublicKey, MultiEd25519Signature,
                               PublicKey, Signature)
from aptos_sdk.transactions import (EntryFunction, RawTransaction,
                                    SignedTransaction, TransactionArgument,
                                    TransactionPayload)
from cryptography.exceptions import InvalidSignature
from cryptography.fernet import Fernet, InvalidToken
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC
from nacl.signing import VerifyKey

MAX_SIGNATORIES = 32
"""Maximum number of signatories on a multisig account."""

MIN_PASSWORD_LENGTH = 4
"""The minimum password length."""

MIN_SIGNATORIES = 2
"""Minimum number of signatories on a multisig account."""

MIN_THRESHOLD = 1
"""Minimum number of signatures required to for multisig transaction."""

NETWORK_URLS = {
    "devnet": "https://fullnode.devnet.aptoslabs.com/v1",
    "testnet": "https://fullnode.testnet.aptoslabs.com/v1",
    "mainnet": "https://fullnode.mainnet.aptoslabs.com/v1",
}
"""Map from network name to API node URL."""

FAUCET_URL = "https://faucet.devnet.aptoslabs.com"
"""Devnet faucet URL."""

TEST_PASSWORD = "Aptos"
"""Test password for bypassing prompts."""

USE_TEST_PASSWORD = True
"""True if using test password for bypassing prompts."""


def bytes_to_prefixed_hex(input_bytes: bytes) -> str:
    """Convert bytes to hex string with 0x prefix."""
    return f"0x{input_bytes.hex()}"


def prefixed_hex_to_bytes(prefixed_hex: str) -> bytes:
    """Convert hex string with 0x prefix to bytes."""
    return bytes.fromhex(prefixed_hex[2:])


def get_file_path(
    optional_path: Option[Path], name_tokens: List[str], extension: str
) -> Path:
    """If no path provided, generate one from tokens and extension."""
    if optional_path is not None:  # If optional path provided:
        return optional_path  # Use it as the path.
    # Otherwise return a new path from tokens and extension.
    return Path("_".join(name_tokens).casefold() + "." + extension)


def check_keyfile_password(path: Path) -> Tuple[Dict[Any, Any], Option[bytes]]:
    """Check keyfile password, returning JSON data/private key bytes."""
    with open(path, "r", encoding="utf-8") as keyfile:  # Open keyfile:
        data = json.load(keyfile)  # Load JSON data from keyfile.
    salt = prefixed_hex_to_bytes(data["salt"])  # Get salt bytes.
    encrypted_hex = data["encrypted_private_key"]  # Get encrypted key.
    # Convert encrypted private key hex to bytes.
    encrypted_private_key = prefixed_hex_to_bytes(encrypted_hex)
    if USE_TEST_PASSWORD:  # If using test password:
        print("Using test password.")  # Print user notice.
        password = TEST_PASSWORD  # Set password to test password.
    else:  # Otherwise:
        # Define password input prompt.
        prompt = "Enter password for encrypted private key: "
        password = getpass.getpass(prompt)  # Get keyfile password.
    # Generate Fernet encryption assistant from password and salt.
    fernet = derive_password_protection_fernet(password, salt)
    try:  # Try decrypting private key.
        private_key = fernet.decrypt(encrypted_private_key)
    # If exception from attempting to decrypt private key:
    except (InvalidSignature, InvalidToken):
        print("Invalid password.")  # Inform user.
        private_key = None  # Set private key to none.
    return data, private_key  # Return JSON data, private key.


def check_name(tokens: List[str]) -> str:
    """Check list of tokens for valid name, return concatenated str."""
    name = " ".join(tokens)  # Get name.
    # Assert that name is not blank space.
    assert len(name) != 0 and not name.isspace(), "Name may not be blank."
    return name  # Return name.


def check_outfile_exists(path: Path):
    """Verify desired outfile does not already exist."""
    assert not path.exists(), f"{path} already exists."


def check_password_length(password: str):
    """Verify password meets minimum length threshold."""
    assert (
        len(password) >= MIN_PASSWORD_LENGTH
    ), f"Password should be at least {MIN_PASSWORD_LENGTH} characters."


def derive_password_protection_fernet(password: str, salt: bytes) -> Fernet:
    """Derive Fernet encryption key assistant from password and salt.

    For password-protecting a private key on disk.

    References
    ----------
    https://cryptography.io/en/latest/fernet
    https://stackoverflow.com/a/55147077 (See "Fernet with password")
    """
    key_derivation_function = PBKDF2HMAC(
        algorithm=hashes.SHA256(), length=32, salt=salt, iterations=480_000
    )  # Create key derivation function from salt.
    # Derive key from password.
    derived_key = key_derivation_function.derive(password.encode())
    # Return Fernet encryption assistant.
    return Fernet(base64.urlsafe_b64encode(derived_key))


def encrypt_private_key(private_key_bytes: bytes) -> Tuple[bytes, bytes]:
    """Return encrypted private key and salt."""
    if USE_TEST_PASSWORD:  # If using test password:
        print("Using test password.")  # Print user notice.
        password = TEST_PASSWORD  # Set password to test password.
    else:  # Otherwise:
        # Define new password prompt.
        message = "Enter new password for encrypting private key: "
        password = getpass.getpass(message)  # Get keyfile password.
        check_password_length(password)  # Check password length.
        # Have user re-enter password.
        check = getpass.getpass("Re-enter password: ")
        # Assert passwords match.
        assert password == check, "Passwords do not match."
    salt = secrets.token_bytes(16)  # Generate encryption salt.
    # Generate Fernet encryption assistant from password and salt.
    fernet = derive_password_protection_fernet(password, salt)
    # Encrypt private key.
    encrypted_private_key = fernet.encrypt(private_key_bytes)
    return encrypted_private_key, salt  # Return key, salt.


def get_public_signatory_fields(data: Dict[str, Any]) -> Dict[str, Any]:
    """Extract public fields from signatory keyfile JSON data."""
    return dict(
        (field, data[field])
        for field in ["signatory", "public_key", "authentication_key"]
    )


def metafile_incorporate(args):
    """Incorporate single-signer keyfiles to multisig metafile."""
    metafile_check_update(
        metafile_json={"n_signatories": 0, "signatories": []},
        name_tokens=args.name,
        threshold=args.threshold,
        keyfiles=args.keyfiles,
        outfile=args.metafile,
    )


def metafile_append(args):
    """Append signatory/signatories to a multisig metafile."""
    metafile_check_update(
        metafile_json=json.load(args.metafile),
        name_tokens=args.name,
        threshold=args.threshold,
        keyfiles=args.keyfiles,
        outfile=args.new_metafile,
    )


def check_signatories_threshold(n_signatories: int, threshold: int):
    """Verify the number of signatories and threshold on a multisig."""
    assert MIN_SIGNATORIES <= n_signatories <= MAX_SIGNATORIES, (
        f"Number of signatories must be between {MIN_SIGNATORIES} and "
        f"{MAX_SIGNATORIES} (inclusive)."
    )  # Assert valid number of signatories.
    assert MIN_THRESHOLD <= threshold <= n_signatories, (
        f"Signature threshold must be greater than {MIN_THRESHOLD} and less "
        f"than the number of signatories."
    )  # Assert valid signature threshold.


def metafile_remove(args):
    """Remove signatories from a multisig metafile."""
    metafile_json = json.load(args.metafile)  # Load metafile JSON.
    # Sort 0-indexed signatory list indices from high to low.
    args.signatories.sort(reverse=True)
    # Loop over 0-indexed IDs to remove, high to low:
    for index in args.signatories:
        # Remove signatory at index from list.
        del metafile_json["signatories"][index]
    # Decrement signatory count.
    metafile_json["n_signatories"] -= len(args.signatories)
    metafile_check_update(  # Check and write data to disk.
        metafile_json=metafile_json,
        name_tokens=args.name,
        threshold=args.threshold,
        keyfiles=None,
        outfile=args.new_metafile,
    )


def metafile_threshold(args):
    """Update threshold for a multisig metafile."""
    metafile_json = json.load(args.metafile)  # Load metafile JSON.
    assert (  # Assert that threshold update is specified.
        metafile_json["threshold"] != args.threshold
    ), "No threshold update specified."
    metafile_check_update(  # Check and write data to disk.
        metafile_json=metafile_json,
        name_tokens=args.name,
        threshold=args.threshold,
        keyfiles=None,
        outfile=args.new_metafile,
    )


def metafile_check_update(
    metafile_json: Dict[Any, Any],
    name_tokens: List[str],
    threshold: int,
    keyfiles: Option[List[TextIOWrapper]],
    outfile: Path,
):
    """Check multisig metafile data and update fields as needed for
    inputs, including optional keyfiles for signatories to append."""
    # Get signatories list.
    signatories = metafile_json["signatories"]
    # Get signatory names list.
    signatory_names = [signatory["signatory"] for signatory in signatories]
    public_keys = [  # Get signatory public keys list.
        PublicKey(VerifyKey(prefixed_hex_to_bytes(signatory["public_key"])))
        for signatory in signatories
    ]
    if keyfiles is not None:  # If keyfiles to append:
        for keyfile in keyfiles:  # Loop over keyfiles.
            signatory = json.load(keyfile)  # Load signatory data.
            # Get signatory name.
            signatory_name = signatory["signatory"]
            assert (  # Assert signatory name not reused.
                not signatory_name in signatory_names
            ), f"{signatory_name} already in multisig."
            signatory_names.append(signatory_name)  # Append name.
            # Get signatory's public key as bytes.
            public_key_bytes = prefixed_hex_to_bytes(signatory["public_key"])
            # Append public key to list of public keys.
            public_keys.append(PublicKey(VerifyKey(public_key_bytes)))
            # Append signatory public data to list of signatories.
            signatories.append(get_public_signatory_fields(signatory))
    # Get new number of signatories on multisig.
    n_signatories = len(signatories)
    # Check number of signatories and threshold.
    check_signatories_threshold(n_signatories, threshold)
    # Initialize multisig public key.
    multisig_public_key = MultiEd25519PublicKey(public_keys, threshold)
    # Get public key as prefixed hex.
    public_key_hex = bytes_to_prefixed_hex(multisig_public_key.to_bytes())
    # Get authentication key as prefixed hex.
    auth_key_hex = bytes_to_prefixed_hex(multisig_public_key.auth_key())
    write_json_file(  # Write JSON to multisig metafile outfile.
        path=get_file_path(outfile, name_tokens, "multisig"),
        data={
            "filetype": "Multisig metafile",
            "multisig_name": check_name(name_tokens),
            "address": None,
            "threshold": threshold,
            "n_signatories": n_signatories,
            "public_key": public_key_hex,
            "authentication_key": auth_key_hex,
            "signatories": signatories,
        },
        check_if_exists=True,
    )


def keyfile_change_password(args):
    """Change password for a single-signer keyfile."""
    # Check password, get keyfile data and optional private key bytes.
    data, private_key_bytes = check_keyfile_password(args.keyfile)
    if private_key_bytes is not None:  # If able to decrypt private key:
        # Encrypt private key.
        encrypted_bytes, salt = encrypt_private_key(private_key_bytes)
        # Get encrypted private key hex.
        encrypted_private_key_hex = bytes_to_prefixed_hex(encrypted_bytes)
        # Update JSON with encrypted private key.
        data["encrypted_private_key"] = encrypted_private_key_hex
        # Update JSON with new salt.
        data["salt"] = bytes_to_prefixed_hex(salt)
        # Write JSON to keyfile, skipping check for if file exists.
        write_json_file(args.keyfile, data, False)


def keyfile_extract(args):
    """Extract private key from keyfile, store via
    aptos_sdk.account.Account.store"""
    check_outfile_exists(args.account_store)  # Check if path exists.
    # Try loading private key bytes.
    _, private_key_bytes = check_keyfile_password(args.keyfile)
    # If able to successfully decrypt:
    if private_key_bytes is not None:
        # Load account.
        account = Account.load_key(private_key_bytes.hex())
        # Store Aptos account file.
        account.store(f"{args.account_store}")
        # Open new Aptos account store:
        with open(args.account_store, "r", encoding="utf-8") as outfile:
            # Print contents of new account store.
            print(f"New account store at {args.account_store}:")
            print(f"{outfile.read()}")


def keyfile_generate(args):
    """Generate a keyfile for a single signer."""
    signatory = check_name(args.signatory)
    if args.account_store is None:  # If no account store supplied:
        account = Account.generate()  # Generate new account.
        # If vanity prefix supplied:
        if args.vanity_prefix is not None:
            to_check = args.vanity_prefix  # Define prefix to check.
            # Get number of characters in vanity prefix.
            n_chars = len(to_check)
            if n_chars % 2 == 1:  # If odd number of hex characters:
                # Append 0 to make valid hexstring.
                to_check = args.vanity_prefix + "0"
            # Check that hex can be converted to bytes.
            prefixed_hex_to_bytes(to_check)
            print("Mining vanity address...")  # Print feedback message.
            len_prefix = len(args.vanity_prefix)  # Get prefix length.
            # While account address does not have prefix:
            while account.address().hex()[0:len_prefix] != args.vanity_prefix:
                account = Account.generate()  # Generate another.
    else:  # If account store path supplied:
        # Generate an account from it.
        account = Account.load(f"{args.account_store}")
    # Get private key bytes.
    private_key_bytes = account.private_key.key.encode()
    # Encrypt private key.
    encrypted_private_key_bytes, salt = encrypt_private_key(private_key_bytes)
    # Get encrypted private key hex.
    key_hex = bytes_to_prefixed_hex(encrypted_private_key_bytes)
    write_json_file(  # Write JSON to keyfile.
        path=get_file_path(args.keyfile, args.signatory, "keyfile"),
        data={
            "filetype": "Keyfile",
            "signatory": signatory,
            "public_key": f"{account.public_key()}",
            "authentication_key": account.auth_key(),
            "encrypted_private_key": key_hex,
            "salt": bytes_to_prefixed_hex(salt),
        },
        check_if_exists=True,
    )


def keyfile_fund(args):
    """Fund account linked to keyfile using devnet faucet, assuming
    account address matches authentication key."""
    data = json.load(args.keyfile)  # Load JSON data from keyfile.
    address = data["authentication_key"]  # Get address.
    command = (  # Construct aptos CLI command.
        f"aptos account fund-with-faucet --account {address} "
        f"--faucet-url {FAUCET_URL} --url {NETWORK_URLS['devnet']}"
    )
    # Print command to run.
    print(f"Running aptos CLI command: {command}")
    # Run command.
    subprocess.run(command.split(), stdout=subprocess.PIPE, check=True)
    balance = RestClient(NETWORK_URLS["devnet"]).account_balance(
        AccountAddress(prefixed_hex_to_bytes(address))
    )  # Check balance.
    print(f"New balance: {balance}")  # Print user feedback.


def keyfile_verify(args):
    """Verify password for single-signer keyfile generated via
    keyfile_generate(), show public key and authentication key."""
    # Load JSON data and try getting private key bytes.
    data, private_key_bytes = check_keyfile_password(args.keyfile)
    if private_key_bytes is not None:  # If able to decrypt private key:
        # Print keyfile info.
        print(f'Keyfile password verified for {data["signatory"]}')
        print(f'Public key:         {data["public_key"]}')
        print(f'Authentication key: {data["authentication_key"]}')


def write_json_file(path: Path, data: Dict[str, str], check_if_exists: bool):
    """Write JSON data to path, printing contents of new file and
    optionally checking if the file already exists."""
    if check_if_exists:
        check_outfile_exists(path)  # Check if file exists.
    # With file open for writing:
    with open(path, "w", encoding="utf-8") as file:
        # Dump JSON data to file.
        json.dump(data, file, indent=4)
    filetype = data["filetype"]  # Get file type from data.
    # With file open for reading:
    with open(path, "r", encoding="utf-8") as file:
        # Print contents of file.
        print(f"{filetype} now at {path}: \n{file.read()}")


def get_sequence_number(address: bytes, network: str) -> int:
    """Return sequence number of account having address for network."""
    client = RestClient(NETWORK_URLS[network])  # Get network client.
    # Return account sequence number.
    return client.account_sequence_number(AccountAddress(address))


def rotate_challenge_propose(args):
    """Propose a rotation proof challenge, storing an output file.

    Accepts either a single-signer keyfile or multisig metafile for both
    originating and target accounts. If single-signer, assumes account
    address is identical to authentication key."""
    # Load originator data.
    originator_data = json.load(args.originator)
    target_data = json.load(args.target)  # Load target data.
    if args.from_single:  # If a single-signer originator:
        # Address is assumed to be authentication key.
        originator_address = originator_data["authentication_key"]
    else:  # If multisig originator:
        # Address is that indicated in metafile.
        originator_address = originator_data["address"]
    if args.to_single:  # If a single-signer target:
        assert target_data["authentication_key"] == originator_address, (
            "Authentication key of single-signer target account must match "
            "originating address."
        )  # Assert authentication key identical to from address.
    sequence_number = get_sequence_number(
        prefixed_hex_to_bytes(originator_address), args.network
    )  # Get originating account sequence number.
    write_json_file(  # Write JSON to proposal file.
        path=get_file_path(args.outfile, args.name, "challenge_proposal"),
        data={
            "filetype": "Rotation proof challenge proposal",
            "description": check_name(args.name),
            "from_public_key": originator_data["public_key"],
            "from_is_single_signer": args.from_single,
            "to_is_single_signer": args.to_single,
            "sequence_number": sequence_number,
            "originator": originator_address,
            "current_auth_key": originator_data["authentication_key"],
            "new_public_key": target_data["public_key"],
        },
        check_if_exists=True,
    )


def get_rotation_challenge_bcs(proposal_data: Dict[str, str]) -> bytes:
    """Convert challenge proposal data map to BCS serialization."""
    # Get proposal fields as bytes.
    (originator_address_bytes, current_auth_key_bytes, new_pubkey_bytes) = (
        prefixed_hex_to_bytes(proposal_data["originator"]),
        prefixed_hex_to_bytes(proposal_data["current_auth_key"]),
        prefixed_hex_to_bytes(proposal_data["new_public_key"]),
    )
    rotation_proof_challenge = RotationProofChallenge(
        sequence_number=int(proposal_data["sequence_number"]),
        originator=AccountAddress(originator_address_bytes),
        current_auth_key=AccountAddress(current_auth_key_bytes),
        new_public_key=new_pubkey_bytes,
    )  # Declare a rotation proof challenge.
    serializer = Serializer()  # Init BCS serializer.
    # Serialize rotation proof challenge.
    rotation_proof_challenge.serialize(serializer)
    return serializer.output()  # Return BCS.


def rotate_challenge_sign(args):
    """Sign a rotation proof challenge proposal, storing output file."""
    proposal_data = json.load(args.proposal)  # Load proposal data.
    # Check password, get keyfile data and optional private key bytes.
    keyfile_data, private_key_bytes = check_keyfile_password(args.keyfile)
    if private_key_bytes is None:  # If can't decrypt private key:
        return  # Return.
    # Get rotation proof challenged BCS bytes.
    rotation_proof_challenge_bcs = get_rotation_challenge_bcs(proposal_data)
    # Create Aptos-style account.
    account = Account.load_key(bytes_to_prefixed_hex(private_key_bytes))
    # Sign the serialized rotation proof challnege.
    signature = account.sign(rotation_proof_challenge_bcs).data()
    write_json_file(  # Write JSON to signature file.
        path=get_file_path(args.outfile, args.name, "challenge_signature"),
        data={
            "filetype": "Rotation proof challenge signature",
            "description": check_name(args.name),
            "challenge_proposal": proposal_data,
            "signatory": get_public_signatory_fields(keyfile_data),
            "signature": bytes_to_prefixed_hex(signature),
        },
        check_if_exists=True,
    )


def metafile_to_multisig_public_key(path: Path):
    """Get multisig public key instance from metafile at path."""
    # With metafile open:
    with open(path, encoding="utf-8") as metafile:
        data = json.load(metafile)  # Load JSON data.
    keys = []  # Init empty public keys list.
    for signatory in data["signatories"]:  # Loop over signatories:
        # Get public key bytes.
        public_key_bytes = prefixed_hex_to_bytes(signatory["public_key"])
        # Append public key to list
        keys.append(PublicKey(VerifyKey(public_key_bytes)))
    # Return multisig public key instance.
    return MultiEd25519PublicKey(keys, data["threshold"])


def assert_successful_transaction(
    client: RestClient, signed_transaction: SignedTransaction
):
    """Submit a signed BCS transaction, asserting that it succeeds."""
    # Submit transaction, storing its hash.
    tx_hash = client.submit_bcs_transaction(signed_transaction)
    # Wait for transaction to succeed (asserts success).
    client.wait_for_transaction(tx_hash)
    print(f"Transaction successful: {tx_hash}")


def index_rotation_challenge_signatures(
    signature_files: List[TextIOWrapper],
) -> Tuple[List[Tuple[PublicKey, Signature]], Dict[str, Any]]:
    """Index a list of rotation proof challenge signatures into a
    multisig signature map, extracting the rotation challenge proposal."""
    # Initialize signature map for multisig signature.
    signature_map = []
    proposal = None  # Initialize proposal.
    for file in signature_files:  # Loop over signature files:
        signature_data = json.load(file)  # Load data for file.
        if proposal is None:  # If challenge proposal undefined:
            # Initialize it to that from first signature file.
            proposal = signature_data["challenge_proposal"]
        else:  # If challenge proposal already defined:
            assert (  # Assert it is same across all signature files.
                signature_data["challenge_proposal"] == proposal
            ), "Signature proposal mismatch."
        # Get public key hex for signatory.
        public_key_hex = signature_data["signatory"]["public_key"]
        # Get public key class instance.
        pubkey = PublicKey(VerifyKey(prefixed_hex_to_bytes(public_key_hex)))
        # Get signature.
        sig = Signature(prefixed_hex_to_bytes(signature_data["signature"]))
        # Append public key and signature to signatures map.
        signature_map.append((pubkey, sig))
    # Return signature map and rotation proof challenge proposal.
    return signature_map, proposal


def rotate_execute_single(args):
    """Rotate authentication key for single-signer account.

    Only supports rotation to a multisig account."""
    # Check password, get keyfile data and optional private key bytes.
    keyfile_data, private_key_bytes = check_keyfile_password(args.keyfile)
    if private_key_bytes is None:  # If can't decrypt private key:
        return  # Return without rotating.
    # Create Aptos-style account for single signer.
    account = Account.load_key(bytes_to_prefixed_hex(private_key_bytes))
    # Get public key bytes for account.
    from_public_key_bytes = prefixed_hex_to_bytes(keyfile_data["public_key"])
    signature_map, proposal = index_rotation_challenge_signatures(
        args.signatures
    )  # Index signatures into signature map, extract rotation proposal.
    # Get rotation challenge BCS.
    rotation_challenge_bcs = get_rotation_challenge_bcs(proposal)
    # Get capability to update address mapping for multisig account.
    cap_update_table = MultiEd25519Signature(
        metafile_to_multisig_public_key(args.metafile), signature_map
    ).to_bytes()
    # Get REST client for network.
    client = RestClient(NETWORK_URLS[args.network])
    # Get standard time-to-live time delta.
    time_to_live = timedelta(seconds=client.client_config.expiration_ttl)
    raw_transaction = construct_raw_rotation_transaction(
        from_scheme=Authenticator.ED25519,
        from_public_key_bytes=from_public_key_bytes,
        to_scheme=Authenticator.MULTI_ED25519,
        to_public_key_bytes=prefixed_hex_to_bytes(proposal["new_public_key"]),
        cap_rotate_key=account.sign(rotation_challenge_bcs).data(),
        cap_update_table=cap_update_table,
        sender_bytes=prefixed_hex_to_bytes(proposal["originator"]),
        sequence_number=proposal["sequence_number"],
        expiry=datetime.now() + time_to_live,
        chain_id=client.chain_id,
    )  # Construct raw rotation transaction.
    authenticator = Authenticator(
        Ed25519Authenticator(
            account.public_key(), account.sign(raw_transaction.keyed())
        )
    )  # Get authenticator for signed transaction.
    # Get signed transaction.
    signed_transaction = SignedTransaction(raw_transaction, authenticator)
    # Assert successful transaction.
    assert_successful_transaction(client, signed_transaction)
    # Update multisig metafile address.
    update_multisig_address(args.metafile, proposal["originator"])


def extract_challenge_proposal_data(
    signature_files: List[TextIOWrapper],
    proposal: Option[Dict[str, Any]],
    signatures_manifest=List[Dict[str, Any]],
) -> Dict[str, Any]:
    """Extract from signature files challenge proposal data and append
    to ongoing signatures manifest."""
    for file in signature_files:  # Loop over signature files.
        signature_data = json.load(file)  # Load data for file.
        if proposal is None:  # If challenge proposal undefined:
            # Initialize it to that from first signature file.
            proposal = signature_data["challenge_proposal"]
        else:  # If challenge proposal already defined:
            assert (  # Assert it is same across all signature files.
                signature_data["challenge_proposal"] == proposal
            ), "Signature proposal mismatch."
        signatures_manifest.append(
            {  # Append signature data.
                "signatory": signature_data["signatory"],
                "signature": signature_data["signature"],
            }
        )
    return proposal  # Return proposal.


def rotate_transaction_propose(args):
    """Propose authentication key rotation transaction from multisig."""
    # Initialize empty from and to signatures for challenge proposal.
    challenge_from_signatures, challenge_to_signatures = [], []
    challenge_proposal = extract_challenge_proposal_data(
        signature_files=args.from_signatures,
        proposal=None,
        signatures_manifest=challenge_from_signatures,
    )  # Extract from challenge proposal signatures.
    challenge_proposal = extract_challenge_proposal_data(
        signature_files=args.to_signatures,
        proposal=challenge_proposal,
        signatures_manifest=challenge_to_signatures,
    )  # Extract to challenge proposal signatures.
    write_json_file(  # Write JSON to transaction proposal file.
        path=get_file_path(
            optional_path=args.outfile,
            name_tokens=args.name,
            extension="rotation_transaction_proposal",
        ),
        data={
            "filetype": "Rotation transaction proposal",
            "description": check_name(args.name),
            "challenge_proposal": challenge_proposal,
            "challenge_from_signatures": challenge_from_signatures,
            "challenge_to_signatures": challenge_to_signatures,
            "chain_id": RestClient(NETWORK_URLS[args.network]).chain_id,
            "expiry": args.expiry.isoformat(),
        },
        check_if_exists=True,
    )


def signature_json_to_map(
    manifest: List[Dict[str, Any]]
) -> List[Tuple[PublicKey, Signature]]:
    """Convert a JSON signature manifest to a classed signature map."""
    to_bytes = prefixed_hex_to_bytes  # Shorten func name for brevity.
    return [  # Return list comprehension.
        (
            PublicKey(VerifyKey(to_bytes(entry["signatory"]["public_key"]))),
            Signature(to_bytes(entry["signature"])),
        )
        for entry in manifest
    ]


def construct_raw_rotation_transaction(
    from_scheme: int,
    from_public_key_bytes: bytes,
    to_scheme: int,
    to_public_key_bytes: bytes,
    cap_rotate_key: bytes,
    cap_update_table: bytes,
    sender_bytes: bytes,
    sequence_number: int,
    expiry: datetime,
    chain_id: int,
) -> RawTransaction:
    """Return a raw authentication key rotation transaction."""
    payload = EntryFunction.natural(
        module="0x1::account",
        function="rotate_authentication_key",
        ty_args=[],
        args=[
            TransactionArgument(from_scheme, Serializer.u8),
            TransactionArgument(from_public_key_bytes, Serializer.to_bytes),
            TransactionArgument(to_scheme, Serializer.u8),
            TransactionArgument(to_public_key_bytes, Serializer.to_bytes),
            TransactionArgument(cap_rotate_key, Serializer.to_bytes),
            TransactionArgument(cap_update_table, Serializer.to_bytes),
        ],
    )  # Construct entry function payload.
    return RawTransaction(  # Return raw transaction.
        sender=AccountAddress(sender_bytes),
        sequence_number=sequence_number,
        payload=TransactionPayload(payload),
        max_gas_amount=ClientConfig.max_gas_amount,
        gas_unit_price=ClientConfig.gas_unit_price,
        expiration_timestamps_secs=int(expiry.timestamp()),
        chain_id=chain_id,
    )


def rotate_transaction_sign(args):
    """Sign an authentication key rotation transaction from a multisig
    account."""
    proposal_data = json.load(args.proposal)  # Load proposal data.
    # Check password, get keyfile data and optional private key bytes.
    keyfile_data, private_key_bytes = check_keyfile_password(args.keyfile)
    if private_key_bytes is None:  # If can't decrypt private key:
        return  # Return.
    # Create Aptos-style account for single signer.
    account = Account.load_key(bytes_to_prefixed_hex(private_key_bytes))
    # Get rotation proof challenge proposal.
    challenge_proposal = proposal_data["challenge_proposal"]
    from_public_key_bytes = prefixed_hex_to_bytes(
        challenge_proposal["from_public_key"]
    )  # Get from public key bytes.
    cap_rotate_key = MultiEd25519Signature(
        MultiEd25519PublicKey.from_bytes(from_public_key_bytes),
        signature_json_to_map(proposal_data["challenge_from_signatures"]),
    ).to_bytes()  # Get key rotation capability signature.
    to_public_key_bytes = prefixed_hex_to_bytes(
        challenge_proposal["new_public_key"]
    )  # Get public key bytes for to account.
    # If account to rotate to is a single signer:
    if challenge_proposal["to_is_single_signer"]:
        to_scheme = Authenticator.ED25519  # Scheme is single-signer.
        cap_update_table = prefixed_hex_to_bytes(
            proposal_data["challenge_to_signatures"][0]["signature"]
        )  # Update table capability signature is only one provided.
    else:  # If account to rotate to is a multisig:
        to_scheme = Authenticator.MULTI_ED25519  # Scheme is multisig.
        cap_update_table = MultiEd25519Signature(
            MultiEd25519PublicKey.from_bytes(to_public_key_bytes),
            signature_json_to_map(proposal_data["challenge_to_signatures"]),
        ).to_bytes()  # Get table update capability signature.
    raw_transaction = construct_raw_rotation_transaction(
        from_scheme=Authenticator.MULTI_ED25519,
        from_public_key_bytes=from_public_key_bytes,
        to_scheme=to_scheme,
        to_public_key_bytes=to_public_key_bytes,
        cap_rotate_key=cap_rotate_key,
        cap_update_table=cap_update_table,
        sender_bytes=prefixed_hex_to_bytes(challenge_proposal["originator"]),
        sequence_number=challenge_proposal["sequence_number"],
        expiry=datetime.fromisoformat(proposal_data["expiry"]),
        chain_id=proposal_data["chain_id"],
    )  # Construct raw rotation transaction.
    # Sign raw transaction.
    signature = account.sign(raw_transaction.keyed())
    write_json_file(  # Write JSON to signature file.
        path=get_file_path(
            optional_path=args.outfile,
            name_tokens=args.name,
            extension="rotation_transaction_signature",
        ),
        data={
            "filetype": "Rotation transaction signature",
            "description": check_name(args.name),
            "transaction_proposal": proposal_data,
            "signatory": get_public_signatory_fields(keyfile_data),
            "signature": bytes_to_prefixed_hex(signature.signature),
        },
        check_if_exists=True,
    )


def update_multisig_address(path: Path, address_prefixed_hex: str):
    """Update the address for a multisig metafile."""
    print("Updating address in multisig metafile.")
    # With multisig metafile open:
    with open(path, "r", encoding="utf-8") as metafile:
        data = json.load(metafile)  # Load JSON data from metafile.
    # Update address field.
    data["address"] = address_prefixed_hex
    # Overwrite JSON data in file.
    write_json_file(path=path, data=data, check_if_exists=False)


# AMEE parser.
parser = argparse.ArgumentParser(
    description="""Aptos Multisig Execution Expeditor (AMEE): A collection of
        tools designed to expedite multisig account execution.""",
)
subparsers = parser.add_subparsers(required=True)

# Network parent parser.
network_parser = argparse.ArgumentParser(add_help=False)
network_parser.add_argument(
    "-n",
    "--network",
    choices=["devnet", "testnet", "mainnet"],
    default="devnet",
    help="Network to use, defaults to devnet.",
)

# Keyfile subcommand parser.
parser_keyfile = subparsers.add_parser(
    name="keyfile",
    aliases=["k"],
    description="Assorted single-signer keyfile operations.",
    help="Single-signer keyfile operations.",
)
subparsers_keyfile = parser_keyfile.add_subparsers(required=True)

# Keyfile change password subcommand parser.
parser_keyfile_change_password = subparsers_keyfile.add_parser(
    name="change-password",
    aliases=["c"],
    description="Change password for a single-singer keyfile.",
    help="Change keyfile password.",
)
parser_keyfile_change_password.set_defaults(func=keyfile_change_password)
parser_keyfile_change_password.add_argument(
    "keyfile", type=Path, help="Relative path to keyfile."
)

# Keyfile extract subcommand parser.
parser_keyfile_extract = subparsers_keyfile.add_parser(
    name="extract",
    aliases=["e"],
    description="""Generate an `aptos_sdk.account.Account` from a single-signer
        keyfile then store on disk via `aptos_sdk.account.Account.store`.""",
    help="Extract Aptos account store from keyfile.",
)
parser_keyfile_extract.set_defaults(func=keyfile_extract)
parser_keyfile_extract.add_argument(
    "keyfile", type=Path, help="Relative path to keyfile to extract from."
)
parser_keyfile_extract.add_argument(
    "account_store",
    metavar="account-store",
    type=Path,
    help="Relative path to account file to store in.",
)

# Keyfile generate subcommand parser.
parser_keyfile_generate = subparsers_keyfile.add_parser(
    name="generate",
    aliases=["g"],
    description="Generate a single-signer keyfile.",
    help="Generate new keyfile.",
)
parser_keyfile_generate.set_defaults(func=keyfile_generate)
parser_keyfile_generate.add_argument(
    "signatory",
    type=str,
    nargs="+",
    help="""The name of the entity acting as a signatory. For example 'Aptos'
        or 'The Aptos Foundation'.""",
)
parser_keyfile_generate.add_argument(
    "-k", "--keyfile", type=Path, help="Relative path to desired keyfile."
)
exclusive_group = parser_keyfile_generate.add_mutually_exclusive_group()
exclusive_group.add_argument(
    "-a",
    "--account-store",
    type=Path,
    help="""Relative path to Aptos account data generated via
        `aptos_sdk.account.Account.store()`.""",
)
exclusive_group.add_argument(
    "-v",
    "--vanity-prefix",
    type=str,
    help="Vanity address prefix, for example 0xf00.",
)

# Keyfile verify subcommand parser.
parser_keyfile_verify = subparsers_keyfile.add_parser(
    name="verify",
    aliases=["v"],
    description="Verify password for a single-signer keyfile.",
    help="Verify keyfile password.",
)
parser_keyfile_verify.set_defaults(func=keyfile_verify)
parser_keyfile_verify.add_argument(
    "keyfile", type=Path, help="Relative path to keyfile."
)

# Keyfile fund subcommand parser.
parser_keyfile_fund = subparsers_keyfile.add_parser(
    name="fund",
    aliases=["f"],
    description="Fund account linked to keyfile using devnet faucet.",
    help="Fund on devnet faucet.",
)
parser_keyfile_fund.set_defaults(func=keyfile_fund)
parser_keyfile_fund.add_argument(
    "keyfile",
    type=argparse.FileType("r", encoding="utf-8"),
    help="Relative path to keyfile.",
)

# Metafile subcommand parser.
parser_metafile = subparsers.add_parser(
    name="metafile",
    aliases=["m"],
    description="Assorted multisig metafile operations.",
    help="Multisig metafile operations.",
)
subparsers_metafile = parser_metafile.add_subparsers(required=True)

# Metafile append subcommand parser.
parser_metafile_append = subparsers_metafile.add_parser(
    name="append",
    aliases=["a"],
    description="Append a signatory or signatories to multisig metafile.",
    help="Append signer(s) to a multisig.",
)
parser_metafile_append.set_defaults(func=metafile_append)
parser_metafile_append.add_argument(
    "name",
    type=str,
    nargs="+",
    help="""The name of the new multisig entity. For example 'Aptos' or 'The
        Aptos Foundation'.""",
)
parser_metafile_append.add_argument(
    "-m",
    "--metafile",
    type=argparse.FileType("r", encoding="utf-8"),
    help="Relative path to desired multisig metafile to add to.",
    required=True,
)
parser_metafile_append.add_argument(
    "-t",
    "--threshold",
    type=int,
    help="The number of single signers required to approve a transaction.",
    required=True,
)
parser_metafile_append.add_argument(
    "-k",
    "--keyfiles",
    action="extend",
    nargs="+",
    type=argparse.FileType("r", encoding="utf-8"),
    help="Relative paths to single-signer keyfiles in the multisig.",
    required=True,
)
parser_metafile_append.add_argument(
    "-n",
    "--new-metafile",
    type=Path,
    help="Custom relative path to new multisig metafile.",
)

# Metafile incorporate subcommand parser.
parser_metafile_incorporate = subparsers_metafile.add_parser(
    name="incorporate",
    aliases=["i"],
    description="""Incorporate multiple single-signer keyfiles into a multisig
        metafile.""",
    help="Incorporate single signers into a multisig.",
)
parser_metafile_incorporate.set_defaults(func=metafile_incorporate)
parser_metafile_incorporate.add_argument(
    "name",
    type=str,
    nargs="+",
    help="""The name of the multisig entity. For example 'Aptos' or 'The Aptos
        Foundation'.""",
)
parser_metafile_incorporate.add_argument(
    "-t",
    "--threshold",
    type=int,
    help="The number of single signers required to approve a transaction.",
    required=True,
)
parser_metafile_incorporate.add_argument(
    "-k",
    "--keyfiles",
    action="extend",
    nargs="+",
    type=argparse.FileType("r", encoding="utf-8"),
    help="Relative paths to single-signer keyfiles in the multisig.",
    required=True,
)
parser_metafile_incorporate.add_argument(
    "-m",
    "--metafile",
    type=Path,
    help="Custom relative path to desired multisig metafile.",
)

# Metafile remove subcommand parser.
parser_metafile_remove = subparsers_metafile.add_parser(
    name="remove",
    aliases=["r"],
    description="Remove signatory or signatories from multisig metafile.",
    help="Remove signer(s) from a multisig.",
)
parser_metafile_remove.set_defaults(func=metafile_remove)
parser_metafile_remove.add_argument(
    "name",
    type=str,
    nargs="+",
    help="""The name of the new multisig entity. For example 'Aptos' or 'The
        Aptos Foundation'.""",
)
parser_metafile_remove.add_argument(
    "-m",
    "--metafile",
    type=argparse.FileType("r", encoding="utf-8"),
    help="Relative path to desired multisig metafile to add to.",
    required=True,
)
parser_metafile_remove.add_argument(
    "-t",
    "--threshold",
    type=int,
    help="The number of single signers required to approve a transaction.",
    required=True,
)
parser_metafile_remove.add_argument(
    "-s",
    "--signatories",
    action="extend",
    nargs="+",
    type=int,
    help="""Signatory or signatories to remove, indicated by 0-indexed position
        in signatories list.""",
    required=True,
)
parser_metafile_remove.add_argument(
    "-n",
    "--new-metafile",
    type=Path,
    help="Custom relative path to new multisig metafile.",
)

# Metafile threshold subcommand parser.
parser_metafile_threshold = subparsers_metafile.add_parser(
    name="threshold",
    aliases=["t"],
    description="Change signer threshold for multisig metafile.",
    help="Change multisig threshold.",
)
parser_metafile_threshold.set_defaults(func=metafile_threshold)
parser_metafile_threshold.add_argument(
    "name",
    type=str,
    nargs="+",
    help="""The name of the new multisig entity. For example 'Aptos' or 'The
        Aptos Foundation'.""",
)
parser_metafile_threshold.add_argument(
    "-m",
    "--metafile",
    type=argparse.FileType("r", encoding="utf-8"),
    help="Relative path to desired multisig metafile to add to.",
    required=True,
)
parser_metafile_threshold.add_argument(
    "-t",
    "--threshold",
    type=int,
    help="The number of single signers required to approve a transaction.",
    required=True,
)
parser_metafile_threshold.add_argument(
    "-n",
    "--new-metafile",
    type=Path,
    help="Custom relative path to new multisig metafile.",
)

# Rotate subcommand parser.
parser_rotate = subparsers.add_parser(
    name="rotate",
    aliases=["r"],
    description="Assorted authentication key rotation operations.",
    help="Authentication key rotation operations.",
)
subparsers_rotate = parser_rotate.add_subparsers(required=True)

# Rotate challenge subcommand parser.
parser_rotate_challenge = subparsers_rotate.add_parser(
    name="challenge",
    aliases=["c"],
    description="Authentication key rotation proof challenge operations.",
    help="Authentication key rotation proof challenges.",
)
tmp = parser_rotate_challenge.add_subparsers(required=True)
subparsers_rotate_challenge = tmp  # Temp variable for line breaking.

# Rotate challenge propose subcommand parser.
parser_rotate_challenge_propose = subparsers_rotate_challenge.add_parser(
    name="propose",
    aliases=["p"],
    description="Propose a rotation proof challenge.",
    help="Rotation proof challenge proposal.",
    parents=[network_parser],
)
parser_rotate_challenge_propose.set_defaults(func=rotate_challenge_propose)
parser_rotate_challenge_propose.add_argument(
    "originator",
    type=argparse.FileType("r", encoding="utf-8"),
    help="""Relative file path for either single-signer keyfile or multisig
        metafile for originating account. If a single-signer keyfile, assumes
        account address is identical to its authentication key.""",
)
parser_rotate_challenge_propose.add_argument(
    "target",
    type=argparse.FileType("r", encoding="utf-8"),
    help="""Relative file path for either single-signer keyfile or multisig
        metafile for target account. If a single-signer keyfile, assumes
        account address is identical to its authentication key.""",
)
parser_rotate_challenge_propose.add_argument(
    "name",
    type=str,
    nargs="+",
    help="Description for rotation. For example 'Setup' or 'Add signer'.",
)
parser_rotate_challenge_propose.add_argument(
    "-f",
    "--from-single",
    action="store_true",
    help="If originator is a single signer.",
)
parser_rotate_challenge_propose.add_argument(
    "-t",
    "--to-single",
    action="store_true",
    help="If authentication key to rotate to is for single signer.",
)
parser_rotate_challenge_propose.add_argument(
    "-o",
    "--outfile",
    type=Path,
    help="Relative path to rotation proof challenge proposal outfile.",
)

# Rotate challenge sign subcommand parser.
parser_rotate_challenge_sign = subparsers_rotate_challenge.add_parser(
    name="sign",
    aliases=["s"],
    description="Sign a rotation proof challenge proposal.",
    help="Rotation proof challenge proposal signing.",
)
parser_rotate_challenge_sign.set_defaults(func=rotate_challenge_sign)
parser_rotate_challenge_sign.add_argument(
    "proposal",
    type=argparse.FileType("r", encoding="utf-8"),
    help="Rotation proof challenge proposal file.",
)
parser_rotate_challenge_sign.add_argument(
    "keyfile",
    type=Path,
    help="Single-signer keyfile for signing challenge proposal.",
)
parser_rotate_challenge_sign.add_argument(
    "name",
    type=str,
    nargs="+",
    help="Description for rotation signature.",
)
parser_rotate_challenge_sign.add_argument(
    "-o",
    "--outfile",
    type=Path,
    help="Relative path to rotation proof challenge signature outfile.",
)

# Rotate execute subcommand parser.
parser_rotate_execute = subparsers_rotate.add_parser(
    name="execute",
    aliases=["e"],
    description="Authentication key rotation execution operations.",
    help="Execute an authentication key rotation.",
)
tmp = parser_rotate_execute.add_subparsers(required=True)
subparsers_rotate_execute = tmp  # Temp variable for line breaking.

# Rotate execute single subcommand parser.
parser_rotate_execute_single = subparsers_rotate_execute.add_parser(
    name="single",
    aliases=["s"],
    description="""Rotate the authentication key of a single-signer account to
        the authentication key of a multisig account. Assumes single-signer
        account address is identical to its authentication key. Requires
        single-signer password approval.""",
    help="Rotate single-signer account to multisig account.",
    parents=[network_parser],
)
parser_rotate_execute_single.set_defaults(func=rotate_execute_single)
parser_rotate_execute_single.add_argument(
    "keyfile",
    type=Path,
    help="Single-signer keyfile for account to convert.",
)
parser_rotate_execute_single.add_argument(
    "metafile",
    type=Path,
    help="Relative path to metafile for multisig to rotate to.",
)
parser_rotate_execute_single.add_argument(
    "signatures",
    action="extend",
    nargs="+",
    type=argparse.FileType("r", encoding="utf-8"),
    help="""Relative paths to rotation proof challenge signature files from
        threshold number of signatories.""",
)

# Rotate transaction subcommand parser.
parser_rotate_transaction = subparsers_rotate.add_parser(
    name="transaction",
    aliases=["t"],
    description="""Authentication key rotation transaction operations for when
        originating account is a multisig.""",
    help="Authentication key rotation prep for multisig originator.",
)
tmp = parser_rotate_transaction.add_subparsers(required=True)
subparsers_rotate_transaction = tmp  # Temp variable for line breaking.

# Rotate transaction propose subcommand parser.
parser_rotate_transaction_propose = subparsers_rotate_transaction.add_parser(
    name="propose",
    aliases=["p"],
    description="""Propose an authentication key rotation from a multisig
        account originator.""",
    help="Propose authentication key rotation for multisig account.",
    parents=[network_parser],
)
parser_rotate_transaction_propose.set_defaults(func=rotate_transaction_propose)
parser_rotate_transaction_propose.add_argument(
    "name",
    type=str,
    nargs="+",
    help="Description for rotation transaction proposal.",
)
parser_rotate_transaction_propose.add_argument(
    "-f",
    "--from-signatures",
    action="extend",
    nargs="+",
    type=argparse.FileType("r", encoding="utf-8"),
    help="""Relative paths to rotation proof challenge signatures for multisig
        signatories at from account.""",
    required=True,
)
parser_rotate_transaction_propose.add_argument(
    "-t",
    "--to-signatures",
    action="extend",
    nargs="+",
    type=argparse.FileType("r", encoding="utf-8"),
    help="""Relative paths to rotation proof challenge signatures for requisite
        signatories at to account. Can be a for a single signer account or for
        a multisig account.""",
    required=True,
)
parser_rotate_transaction_propose.add_argument(
    "-e",
    "--expiry",
    required=True,
    help="Transaction expiry, in ISO 8601 format. For example '2023-02-15'.",
    type=datetime.fromisoformat,
)
parser_rotate_transaction_propose.add_argument(
    "-o",
    "--outfile",
    type=Path,
    help="Relative path to rotation transaction proposal outfile.",
)

# Rotate transaction sign subcommand parser.
parser_rotate_transaction_sign = subparsers_rotate_transaction.add_parser(
    name="sign",
    aliases=["s"],
    description="Sign an authentication key rotation transaction.",
    help="Authentication key rotation transaction signing.",
)
parser_rotate_transaction_sign.set_defaults(func=rotate_transaction_sign)
parser_rotate_transaction_sign.add_argument(
    "proposal",
    type=argparse.FileType("r", encoding="utf-8"),
    help="Rotation transaction proposal file.",
)
parser_rotate_transaction_sign.add_argument(
    "keyfile",
    type=Path,
    help="Relative path to single-signer keyfile for signing proposal.",
)
parser_rotate_transaction_sign.add_argument(
    "name",
    type=str,
    nargs="+",
    help="Description for transaction signature.",
)
parser_rotate_transaction_sign.add_argument(
    "-o",
    "--outfile",
    type=Path,
    help="Relative path to rotation transaction signature outfile.",
)


parsed_args = parser.parse_args()  # Parse command line arguments.
parsed_args.func(parsed_args)  # Call command line argument function.

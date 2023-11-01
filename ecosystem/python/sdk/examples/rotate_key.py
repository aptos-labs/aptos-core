import asyncio
from typing import List, cast

import aptos_sdk.asymmetric_crypto as asymmetric_crypto
import aptos_sdk.ed25519 as ed25519
from aptos_sdk.account import Account, RotationProofChallenge
from aptos_sdk.account_address import AccountAddress
from aptos_sdk.async_client import FaucetClient, RestClient
from aptos_sdk.authenticator import Authenticator
from aptos_sdk.bcs import Serializer
from aptos_sdk.transactions import (
    EntryFunction,
    TransactionArgument,
    TransactionPayload,
)

from .common import FAUCET_URL, NODE_URL

WIDTH = 19


def truncate(address: str) -> str:
    return address[0:6] + "..." + address[-6:]


def format_account_info(account: Account) -> str:
    vals = [
        str(account.address()),
        account.auth_key(),
        account.private_key.hex(),
        str(account.public_key()),
    ]
    return "".join([truncate(v).ljust(WIDTH, " ") for v in vals])


async def rotate_auth_key_ed_25519_payload(
    rest_client: RestClient, from_account: Account, private_key: ed25519.PrivateKey
) -> TransactionPayload:
    to_account = Account.load_key(private_key.hex())
    rotation_proof_challenge = RotationProofChallenge(
        sequence_number=await rest_client.account_sequence_number(
            from_account.address()
        ),
        originator=from_account.address(),
        current_auth_key=AccountAddress.from_str_relaxed(from_account.auth_key()),
        new_public_key=to_account.public_key(),
    )

    serializer = Serializer()
    rotation_proof_challenge.serialize(serializer)
    rotation_proof_challenge_bcs = serializer.output()

    from_signature = from_account.sign(rotation_proof_challenge_bcs)
    to_signature = to_account.sign(rotation_proof_challenge_bcs)

    return rotation_payload(
        from_account.public_key(), to_account.public_key(), from_signature, to_signature
    )


async def rotate_auth_key_multi_ed_25519_payload(
    rest_client: RestClient,
    from_account: Account,
    private_keys: List[ed25519.PrivateKey],
) -> TransactionPayload:
    to_accounts = list(
        map(lambda private_key: Account.load_key(private_key.hex()), private_keys)
    )
    public_keys = list(map(lambda account: account.public_key(), to_accounts))
    public_key = ed25519.MultiPublicKey(cast(List[ed25519.PublicKey], public_keys), 1)

    rotation_proof_challenge = RotationProofChallenge(
        sequence_number=await rest_client.account_sequence_number(
            from_account.address()
        ),
        originator=from_account.address(),
        current_auth_key=AccountAddress.from_str(from_account.auth_key()),
        new_public_key=public_key,
    )

    serializer = Serializer()
    rotation_proof_challenge.serialize(serializer)
    rotation_proof_challenge_bcs = serializer.output()

    from_signature = from_account.sign(rotation_proof_challenge_bcs)
    to_signature = cast(
        ed25519.Signature, to_accounts[0].sign(rotation_proof_challenge_bcs)
    )
    multi_to_signature = ed25519.MultiSignature.from_key_map(
        public_key,
        [(cast(ed25519.PublicKey, to_accounts[0].public_key()), to_signature)],
    )

    return rotation_payload(
        from_account.public_key(), public_key, from_signature, multi_to_signature
    )


def rotation_payload(
    from_key: asymmetric_crypto.PublicKey,
    to_key: asymmetric_crypto.PublicKey,
    from_signature: asymmetric_crypto.Signature,
    to_signature: asymmetric_crypto.Signature,
) -> TransactionPayload:
    from_scheme = Authenticator.from_key(from_key)
    to_scheme = Authenticator.from_key(to_key)

    entry_function = EntryFunction.natural(
        module="0x1::account",
        function="rotate_authentication_key",
        ty_args=[],
        args=[
            TransactionArgument(from_scheme, Serializer.u8),
            TransactionArgument(from_key, Serializer.struct),
            TransactionArgument(to_scheme, Serializer.u8),
            TransactionArgument(to_key, Serializer.struct),
            TransactionArgument(from_signature, Serializer.struct),
            TransactionArgument(to_signature, Serializer.struct),
        ],
    )

    return TransactionPayload(entry_function)


async def main():
    # Initialize the clients used to interact with the blockchain
    rest_client = RestClient(NODE_URL)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)

    # Generate random accounts Alice and Bob
    alice = Account.generate()
    bob = Account.generate()

    # Fund Alice's account, since we don't use Bob's
    await faucet_client.fund_account(alice.address(), 100_000_000)

    # Display formatted account info
    print(
        "\n"
        + "Account".ljust(WIDTH, " ")
        + "Address".ljust(WIDTH, " ")
        + "Auth Key".ljust(WIDTH, " ")
        + "Private Key".ljust(WIDTH, " ")
        + "Public Key".ljust(WIDTH, " ")
    )
    print(
        "-------------------------------------------------------------------------------------------"
    )
    print("Alice".ljust(WIDTH, " ") + format_account_info(alice))
    print("Bob".ljust(WIDTH, " ") + format_account_info(bob))

    print("\n...rotating...\n")

    # :!:>rotate_key
    # Create the payload for rotating Alice's private key to Bob's private key
    payload = await rotate_auth_key_ed_25519_payload(
        rest_client, alice, bob.private_key
    )
    # Have Alice sign the transaction with the payload
    signed_transaction = await rest_client.create_bcs_signed_transaction(alice, payload)
    # Submit the transaction and wait for confirmation
    tx_hash = await rest_client.submit_bcs_transaction(signed_transaction)
    await rest_client.wait_for_transaction(tx_hash)  # <:!:rotate_key

    # Check the authentication key for Alice's address on-chain
    alice_new_account_info = await rest_client.account(alice.address())
    # Ensure that Alice's authentication key matches bob's
    assert (
        alice_new_account_info["authentication_key"] == bob.auth_key()
    ), "Authentication key doesn't match Bob's"

    # Construct a new Account object that reflects alice's original address with the new private key
    original_alice_key = alice.private_key
    alice = Account(alice.address(), bob.private_key)

    # Display formatted account info
    print("Alice".ljust(WIDTH, " ") + format_account_info(alice))
    print("Bob".ljust(WIDTH, " ") + format_account_info(bob))
    print()

    print("\n...rotating...\n")
    payload = await rotate_auth_key_multi_ed_25519_payload(
        rest_client, alice, [bob.private_key, original_alice_key]
    )
    signed_transaction = await rest_client.create_bcs_signed_transaction(alice, payload)
    tx_hash = await rest_client.submit_bcs_transaction(signed_transaction)
    await rest_client.wait_for_transaction(tx_hash)

    alice_new_account_info = await rest_client.account(alice.address())
    auth_key = alice_new_account_info["authentication_key"]
    print(f"Rotation to MultiPublicKey complete, new authkey: {auth_key}")

    await rest_client.close()


if __name__ == "__main__":
    asyncio.run(main())

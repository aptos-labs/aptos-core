# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import asyncio
import subprocess
import time

from aptos_sdk.account import Account, RotationProofChallenge
from aptos_sdk.account_address import AccountAddress
from aptos_sdk.async_client import FaucetClient, RestClient
from aptos_sdk.authenticator import Authenticator, MultiEd25519Authenticator
from aptos_sdk.bcs import Serializer
from aptos_sdk.ed25519 import MultiPublicKey, MultiSignature
from aptos_sdk.transactions import (
    EntryFunction,
    RawTransaction,
    Script,
    ScriptArgument,
    SignedTransaction,
    TransactionArgument,
    TransactionPayload,
)
from aptos_sdk.type_tag import StructTag, TypeTag

from .common import FAUCET_URL, NODE_URL


def wait():
    """Wait for user to press Enter before starting next section."""
    input("\nPress Enter to continue...")


async def main():
    rest_client = RestClient(NODE_URL)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)

    # :!:>section_1
    alice = Account.generate()
    bob = Account.generate()
    chad = Account.generate()

    print("\n=== Account addresses ===")
    print(f"Alice: {alice.address()}")
    print(f"Bob:   {bob.address()}")
    print(f"Chad:  {chad.address()}")

    print("\n=== Authentication keys ===")
    print(f"Alice: {alice.auth_key()}")
    print(f"Bob:   {bob.auth_key()}")
    print(f"Chad:  {chad.auth_key()}")

    print("\n=== Public keys ===")
    print(f"Alice: {alice.public_key()}")
    print(f"Bob:   {bob.public_key()}")
    print(f"Chad:  {chad.public_key()}")  # <:!:section_1

    wait()

    # :!:>section_2
    threshold = 2

    multisig_public_key = MultiPublicKey(
        [alice.public_key(), bob.public_key(), chad.public_key()], threshold
    )

    multisig_address = AccountAddress.from_multi_ed25519(multisig_public_key)

    print("\n=== 2-of-3 Multisig account ===")
    print(f"Account public key: {multisig_public_key}")
    print(f"Account address:    {multisig_address}")  # <:!:section_2

    wait()

    # :!:>section_3
    print("\n=== Funding accounts ===")
    alice_start = 10_000_000
    bob_start = 20_000_000
    chad_start = 30_000_000
    multisig_start = 40_000_000

    alice_fund = faucet_client.fund_account(alice.address(), alice_start)
    bob_fund = faucet_client.fund_account(bob.address(), bob_start)
    chad_fund = faucet_client.fund_account(chad.address(), chad_start)
    multisig_fund = faucet_client.fund_account(multisig_address, multisig_start)
    await asyncio.gather(*[alice_fund, bob_fund, chad_fund, multisig_fund])

    alice_balance = rest_client.account_balance(alice.address())
    bob_balance = rest_client.account_balance(bob.address())
    chad_balance = rest_client.account_balance(chad.address())
    multisig_balance = rest_client.account_balance(multisig_address)
    [alice_balance, bob_balance, chad_balance, multisig_balance] = await asyncio.gather(
        *[alice_balance, bob_balance, chad_balance, multisig_balance]
    )

    print(f"Alice's balance:  {alice_balance}")
    print(f"Bob's balance:    {bob_balance}")
    print(f"Chad's balance:   {chad_balance}")
    print(f"Multisig balance: {multisig_balance}")  # <:!:section_3

    wait()

    # :!:>section_4
    entry_function = EntryFunction.natural(
        module="0x1::coin",
        function="transfer",
        ty_args=[TypeTag(StructTag.from_str("0x1::aptos_coin::AptosCoin"))],
        args=[
            TransactionArgument(chad.address(), Serializer.struct),
            TransactionArgument(100, Serializer.u64),
        ],
    )

    chain_id = await rest_client.chain_id()
    raw_transaction = RawTransaction(
        sender=multisig_address,
        sequence_number=0,
        payload=TransactionPayload(entry_function),
        max_gas_amount=rest_client.client_config.max_gas_amount,
        gas_unit_price=rest_client.client_config.gas_unit_price,
        expiration_timestamps_secs=(
            int(time.time()) + rest_client.client_config.expiration_ttl
        ),
        chain_id=chain_id,
    )

    alice_signature = alice.sign(raw_transaction.keyed())
    bob_signature = bob.sign(raw_transaction.keyed())

    assert raw_transaction.verify(alice.public_key(), alice_signature)
    assert raw_transaction.verify(bob.public_key(), bob_signature)

    print("\n=== Individual signatures ===")
    print(f"Alice: {alice_signature}")
    print(f"Bob:   {bob_signature}")  # <:!:section_4

    wait()

    # :!:>section_5
    sig_map = [  # Map from signatory public key to signature.
        (alice.public_key(), alice_signature),
        (bob.public_key(), bob_signature),
    ]

    multisig_signature = MultiSignature(multisig_public_key, sig_map)

    authenticator = Authenticator(
        MultiEd25519Authenticator(multisig_public_key, multisig_signature)
    )

    signed_transaction = SignedTransaction(raw_transaction, authenticator)

    print("\n=== Submitting transfer transaction ===")

    tx_hash = await rest_client.submit_bcs_transaction(signed_transaction)
    await rest_client.wait_for_transaction(tx_hash)
    print(f"Transaction hash: {tx_hash}")  # <:!:section_5

    wait()

    # :!:>section_6
    print("\n=== New account balances===")

    alice_balance = rest_client.account_balance(alice.address())
    bob_balance = rest_client.account_balance(bob.address())
    chad_balance = rest_client.account_balance(chad.address())
    multisig_balance = rest_client.account_balance(multisig_address)
    [alice_balance, bob_balance, chad_balance, multisig_balance] = await asyncio.gather(
        *[alice_balance, bob_balance, chad_balance, multisig_balance]
    )

    print(f"Alice's balance:  {alice_balance}")
    print(f"Bob's balance:    {bob_balance}")
    print(f"Chad's balance:   {chad_balance}")
    print(f"Multisig balance: {multisig_balance}")  # <:!:section_6

    wait()

    # :!:>section_7
    print("\n=== Funding vanity address ===")

    deedee = Account.generate()

    while str(deedee.address())[2:4] != "dd":
        deedee = Account.generate()

    print(f"Deedee's address:    {deedee.address()}")
    print(f"Deedee's public key: {deedee.public_key()}")

    deedee_start = 50_000_000

    await faucet_client.fund_account(deedee.address(), deedee_start)
    deedee_balance = await rest_client.account_balance(deedee.address())
    print(f"Deedee's balance:    {deedee_balance}")  # <:!:section_7

    wait()

    # :!:>section_8
    print("\n=== Signing rotation proof challenge ===")

    rotation_proof_challenge = RotationProofChallenge(
        sequence_number=0,
        originator=deedee.address(),
        current_auth_key=deedee.address(),
        new_public_key=multisig_public_key.to_bytes(),
    )

    serializer = Serializer()
    rotation_proof_challenge.serialize(serializer)
    rotation_proof_challenge_bcs = serializer.output()

    cap_rotate_key = deedee.sign(rotation_proof_challenge_bcs).data()

    cap_update_table = MultiSignature(
        multisig_public_key,
        [
            (bob.public_key(), bob.sign(rotation_proof_challenge_bcs)),
            (chad.public_key(), chad.sign(rotation_proof_challenge_bcs)),
        ],
    ).to_bytes()

    cap_rotate_key_hex = f"0x{cap_rotate_key.hex()}"
    cap_update_table_hex = f"0x{cap_update_table.hex()}"

    print(f"cap_rotate_key:   {cap_rotate_key_hex}")
    print(f"cap_update_table: {cap_update_table_hex}")  # <:!:section_8

    wait()

    # :!:>section_9
    print("\n=== Submitting authentication key rotation transaction ===")

    from_scheme = Authenticator.ED25519
    from_public_key_bytes = deedee.public_key().key.encode()
    to_scheme = Authenticator.MULTI_ED25519
    to_public_key_bytes = multisig_public_key.to_bytes()

    entry_function = EntryFunction.natural(
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
    )

    signed_transaction = await rest_client.create_bcs_signed_transaction(
        deedee, TransactionPayload(entry_function)
    )

    account_data = await rest_client.account(deedee.address())
    print(f"Auth key pre-rotation: {account_data['authentication_key']}")

    tx_hash = await rest_client.submit_bcs_transaction(signed_transaction)
    await rest_client.wait_for_transaction(tx_hash)
    print(f"Transaction hash:      {tx_hash}")

    account_data = await rest_client.account(deedee.address())
    print(f"New auth key:          {account_data['authentication_key']}")
    print(f"1st multisig address:  {multisig_address}")  # <:!:section_9

    wait()

    # :!:>section_10
    print("\n=== Genesis publication ===")

    packages_dir = "../../../aptos-move/move-examples/upgrade_and_govern/"

    command = (
        f"aptos move compile "
        f"--save-metadata "
        f"--package-dir {packages_dir}genesis "
        f"--named-addresses upgrade_and_govern={str(deedee.address())}"
    )

    print(f"Running aptos CLI command: {command}\n")
    subprocess.run(command.split(), stdout=subprocess.PIPE)

    build_path = f"{packages_dir}genesis/build/UpgradeAndGovern/"

    with open(f"{build_path}package-metadata.bcs", "rb") as f:
        package_metadata = f.read()

    with open(f"{build_path}bytecode_modules/parameters.mv", "rb") as f:
        parameters_module = f.read()

    modules_serializer = Serializer.sequence_serializer(Serializer.to_bytes)

    payload = EntryFunction.natural(
        module="0x1::code",
        function="publish_package_txn",
        ty_args=[],
        args=[
            TransactionArgument(package_metadata, Serializer.to_bytes),
            TransactionArgument([parameters_module], modules_serializer),
        ],
    )

    raw_transaction = RawTransaction(
        sender=deedee.address(),
        sequence_number=1,
        payload=TransactionPayload(payload),
        max_gas_amount=rest_client.client_config.max_gas_amount,
        gas_unit_price=rest_client.client_config.gas_unit_price,
        expiration_timestamps_secs=(
            int(time.time()) + rest_client.client_config.expiration_ttl
        ),
        chain_id=chain_id,
    )

    alice_signature = alice.sign(raw_transaction.keyed())
    chad_signature = chad.sign(raw_transaction.keyed())

    sig_map = [  # Map from signatory public key to signature.
        (alice.public_key(), alice_signature),
        (chad.public_key(), chad_signature),
    ]

    multisig_signature = MultiSignature(multisig_public_key, sig_map)

    authenticator = Authenticator(
        MultiEd25519Authenticator(multisig_public_key, multisig_signature)
    )

    signed_transaction = SignedTransaction(raw_transaction, authenticator)

    tx_hash = await rest_client.submit_bcs_transaction(signed_transaction)
    await rest_client.wait_for_transaction(tx_hash)
    print(f"\nTransaction hash: {tx_hash}")

    registry = await rest_client.account_resource(
        deedee.address(), "0x1::code::PackageRegistry"
    )

    package_name = registry["data"]["packages"][0]["name"]
    n_upgrades = registry["data"]["packages"][0]["upgrade_number"]

    print(f"Package name from on-chain registry: {package_name}")
    print(f"On-chain upgrade number: {n_upgrades}")  # <:!:section_10

    wait()

    # :!:>section_11
    print("\n=== Upgrade publication ===")

    command = (
        f"aptos move compile "
        f"--save-metadata "
        f"--package-dir {packages_dir}upgrade "
        f"--named-addresses upgrade_and_govern={str(deedee.address())}"
    )

    print(f"Running aptos CLI command: {command}\n")
    subprocess.run(command.split(), stdout=subprocess.PIPE)

    build_path = f"{packages_dir}upgrade/build/UpgradeAndGovern/"

    with open(f"{build_path}package-metadata.bcs", "rb") as f:
        package_metadata = f.read()

    with open(f"{build_path}bytecode_modules/parameters.mv", "rb") as f:
        parameters_module = f.read()

    with open(f"{build_path}bytecode_modules/transfer.mv", "rb") as f:
        transfer_module = f.read()

    payload = EntryFunction.natural(
        module="0x1::code",
        function="publish_package_txn",
        ty_args=[],
        args=[
            TransactionArgument(package_metadata, Serializer.to_bytes),
            TransactionArgument(  # Transfer module listed second.
                [parameters_module, transfer_module],
                Serializer.sequence_serializer(Serializer.to_bytes),
            ),
        ],
    )

    raw_transaction = RawTransaction(
        sender=deedee.address(),
        sequence_number=2,
        payload=TransactionPayload(payload),
        max_gas_amount=rest_client.client_config.max_gas_amount,
        gas_unit_price=rest_client.client_config.gas_unit_price,
        expiration_timestamps_secs=(
            int(time.time()) + rest_client.client_config.expiration_ttl
        ),
        chain_id=chain_id,
    )

    alice_signature = alice.sign(raw_transaction.keyed())
    bob_signature = bob.sign(raw_transaction.keyed())
    chad_signature = chad.sign(raw_transaction.keyed())

    sig_map = [  # Map from signatory public key to signature.
        (alice.public_key(), alice_signature),
        (bob.public_key(), bob_signature),
        (chad.public_key(), chad_signature),
    ]

    multisig_signature = MultiSignature(multisig_public_key, sig_map)

    authenticator = Authenticator(
        MultiEd25519Authenticator(multisig_public_key, multisig_signature)
    )

    signed_transaction = SignedTransaction(raw_transaction, authenticator)

    tx_hash = await rest_client.submit_bcs_transaction(signed_transaction)
    await rest_client.wait_for_transaction(tx_hash)
    print(f"\nTransaction hash: {tx_hash}")

    registry = await rest_client.account_resource(
        deedee.address(), "0x1::code::PackageRegistry"
    )

    n_upgrades = registry["data"]["packages"][0]["upgrade_number"]

    print(f"On-chain upgrade number: {n_upgrades}")  # <:!:section_11

    wait()

    # :!:>section_12
    print("\n=== Invoking Move script ===")

    with open(f"{build_path}bytecode_scripts/set_and_transfer.mv", "rb") as f:
        script_code = f.read()

    payload = Script(
        code=script_code,
        ty_args=[],
        args=[
            ScriptArgument(ScriptArgument.ADDRESS, alice.address()),
            ScriptArgument(ScriptArgument.ADDRESS, bob.address()),
        ],
    )

    raw_transaction = RawTransaction(
        sender=deedee.address(),
        sequence_number=3,
        payload=TransactionPayload(payload),
        max_gas_amount=rest_client.client_config.max_gas_amount,
        gas_unit_price=rest_client.client_config.gas_unit_price,
        expiration_timestamps_secs=(
            int(time.time()) + rest_client.client_config.expiration_ttl
        ),
        chain_id=chain_id,
    )

    alice_signature = alice.sign(raw_transaction.keyed())
    bob_signature = bob.sign(raw_transaction.keyed())

    sig_map = [  # Map from signatory public key to signature.
        (alice.public_key(), alice_signature),
        (bob.public_key(), bob_signature),
    ]

    multisig_signature = MultiSignature(multisig_public_key, sig_map)

    authenticator = Authenticator(
        MultiEd25519Authenticator(multisig_public_key, multisig_signature)
    )

    signed_transaction = SignedTransaction(raw_transaction, authenticator)

    tx_hash = await rest_client.submit_bcs_transaction(signed_transaction)
    await rest_client.wait_for_transaction(tx_hash)
    print(f"Transaction hash: {tx_hash}")

    alice_balance = rest_client.account_balance(alice.address())
    bob_balance = rest_client.account_balance(bob.address())
    chad_balance = rest_client.account_balance(chad.address())
    multisig_balance = rest_client.account_balance(multisig_address)
    [alice_balance, bob_balance, chad_balance, multisig_balance] = await asyncio.gather(
        *[alice_balance, bob_balance, chad_balance, multisig_balance]
    )

    print(f"Alice's balance:  {alice_balance}")
    print(f"Bob's balance:    {bob_balance}")
    print(f"Chad's balance:   {chad_balance}")
    print(f"Multisig balance: {multisig_balance}")  # <:!:section_12


if __name__ == "__main__":
    asyncio.run(main())

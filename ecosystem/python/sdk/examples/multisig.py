import time

from aptos_sdk.account import Account
from aptos_sdk.account_address import AccountAddress
from aptos_sdk.authenticator import Authenticator, MultiEd25519Authenticator
from aptos_sdk.bcs import Serializer
from aptos_sdk.client import FaucetClient, RestClient
from aptos_sdk.ed25519 import MultiEd25519PublicKey, MultiEd25519Signature
from aptos_sdk.transactions import (
    EntryFunction,
    RawTransaction,
    SignedTransaction,
    TransactionArgument,
    TransactionPayload
)
from aptos_sdk.type_tag import TypeTag, StructTag

from common import NODE_URL, FAUCET_URL

wait_for_user = True

if __name__ == '__main__':

    # :!:>section_1
    alice = Account.generate()
    bob   = Account.generate()
    chad  = Account.generate()

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
    print(f"Chad:  {chad.public_key()}") # <:!:section_1

    if wait_for_user: input("\nPress Enter to continue...")

    # :!:>section_2
    threshold = 2
    multisig_public_key = MultiEd25519PublicKey(
        [alice.public_key(), bob.public_key(), chad.public_key()], threshold)
    multisig_address = AccountAddress.from_multisig_schema(
        [alice.public_key(), bob.public_key(), chad.public_key()], threshold)

    print("\n=== 2-of-3 Multisig account ===")
    print(f"Account public key: {multisig_public_key}")
    print(f"Account address:    {multisig_address}") # <:!:section_2

    if wait_for_user: input("\nPress Enter to continue...")

    # :!:>section_3
    rest_client = RestClient(NODE_URL)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)

    print("\n=== Funding accounts ===")
    alice_start    = 10_000_000
    bob_start      = 20_000_000
    chad_start     = 30_000_000
    multisig_start = 40_000_000
    faucet_client.fund_account(alice.address() , alice_start)
    alice_balance = rest_client.account_balance(alice.address())
    print(f"Alice's balance:  {alice_balance}")
    faucet_client.fund_account(bob.address()   , bob_start)
    bob_balance = rest_client.account_balance(bob.address())
    print(f"Bob's balance:    {bob_balance}")
    faucet_client.fund_account(chad.address()  , chad_start)
    chad_balance = rest_client.account_balance(chad.address())
    print(f"Chad's balance:   {chad_balance}")
    faucet_client.fund_account(multisig_address, multisig_start)
    multisig_balance = rest_client.account_balance(multisig_address)
    print(f"Multisig balance: {multisig_balance}") # <:!:section_3

    if wait_for_user: input("\nPress Enter to continue...")

    # :!:>section_4
    entry_function = EntryFunction.natural(
        module="0x1::coin",
        function="transfer",
        ty_args=[TypeTag(StructTag.from_str("0x1::aptos_coin::AptosCoin"))],
        args=[TransactionArgument(chad.address(), Serializer.struct),
              TransactionArgument(100, Serializer.u64)])

    raw_transaction = RawTransaction(
        sender=multisig_address,
        sequence_number=0,
        payload=TransactionPayload(entry_function),
        max_gas_amount=rest_client.client_config.max_gas_amount,
        gas_unit_price=rest_client.client_config.gas_unit_price,
        expiration_timestamps_secs=(
            int(time.time()) + rest_client.client_config.expiration_ttl),
        chain_id=rest_client.chain_id)

    alice_signature = alice.sign(raw_transaction.keyed())
    bob_signature = bob.sign(raw_transaction.keyed())

    assert raw_transaction.verify(alice.public_key(), alice_signature)
    assert raw_transaction.verify(bob.public_key(), bob_signature)

    print("\n=== Individual signatures ===")
    print(f"Alice: {alice_signature}")
    print(f"Bob:   {bob_signature}") # <:!:section_4

    if wait_for_user: input("\nPress Enter to continue...")

    # :!:>section_5
    signatures_map = [(alice.public_key(), alice_signature),
                      (bob.public_key(),   bob_signature)]
    multisig_signature = MultiEd25519Signature(multisig_public_key,
                                               signatures_map)
    authenticator = Authenticator(MultiEd25519Authenticator(
        multisig_public_key, multisig_signature))
    signed_transaction = SignedTransaction(raw_transaction, authenticator)
    print("\n=== Submitting transaction ===")
    tx_hash = rest_client.submit_bcs_transaction(signed_transaction)
    print(f"Transaction hash: {tx_hash}") # <:!:section_5

    if wait_for_user: input("\nPress Enter to continue...")

    print(f"\nWaiting for client to update...")
    time.sleep(2.5)

    # :!:>section_6
    print("\n=== New account balances===")
    alice_balance = rest_client.account_balance(alice.address())
    print(f"Alice's balance:  {alice_balance}")
    bob_balance = rest_client.account_balance(bob.address())
    print(f"Bob's balance:    {bob_balance}")
    chad_balance = rest_client.account_balance(chad.address())
    print(f"Chad's balance:   {chad_balance}")
    multisig_balance = rest_client.account_balance(multisig_address)
    print(f"Multisig balance: {multisig_balance}") # <:!:section_6
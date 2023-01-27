import time

from aptos_sdk.account import Account
from aptos_sdk.account_address import AccountAddress
from aptos_sdk.authenticator import MultiEd25519Authenticator
from aptos_sdk.bcs import Serializer
from aptos_sdk.client import FaucetClient, RestClient
from aptos_sdk.ed25519 import MultiEd25519PublicKey, MultiEd25519Signature
from aptos_sdk.transactions import (
    EntryFunction,
    RawTransaction,
    TransactionArgument,
    TransactionPayload
)
from aptos_sdk.type_tag import TypeTag, StructTag

from common import NODE_URL, FAUCET_URL

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

    input("\nPress Enter to continue...")

    # :!:>section_2
    threshold = 2
    multisig_public_key = MultiEd25519PublicKey(
        [alice.public_key(), bob.public_key(), chad.public_key()], threshold)
    multisig_address = AccountAddress.from_multisig_schema(
        [alice.public_key(), bob.public_key(), chad.public_key()], threshold)

    print("\n=== 2-of-3 Multisig account ===")
    print(f"Account public key: {multisig_public_key}")
    print(f"Account address:    {multisig_address}") # <:!:section_2

    input("\nPress Enter to continue...")

    # :!:>section_3
    rest_client = RestClient(NODE_URL)
    faucet_client = FaucetClient(FAUCET_URL, rest_client)

    print("\n=== Funding accounts ===")
    faucet_client.fund_account(alice.address() , 10_000_000)
    print(f"Alice's balance:  {rest_client.account_balance(alice.address())}")
    faucet_client.fund_account(bob.address()   , 20_000_000)
    print(f"Bob's balance:    {rest_client.account_balance(bob.address())}")
    faucet_client.fund_account(chad.address()  , 30_000_000)
    print(f"Chad's balance:   {rest_client.account_balance(chad.address())}")
    faucet_client.fund_account(multisig_address, 40_000_000)
    print(f"Multisig balance: {rest_client.account_balance(multisig_address)}") # <:!:section_3

    input("\nPress Enter to continue...")

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

    alice_signature = raw_transaction.sign(alice.private_key)
    bob_signature   = raw_transaction.sign(bob.private_key)

    print("\n=== Individual signatures ===")
    print(f"Alice: {alice_signature}")
    print(f"Bob:   {bob_signature}") # <:!:section_4

    input("\nPress Enter to continue...")

    # :!:>section_5
    multisig_signature = MultiEd25519Signature(
        [(alice.public_key(), alice_signature),
         (bob.public_key()  , bob_signature)])
    authenticator = MultiEd25519Authenticator(
        multisig_public_key, multisig_signature) # <:!:section_5
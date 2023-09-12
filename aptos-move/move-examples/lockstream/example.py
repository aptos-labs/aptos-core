from aptos_sdk.account import Account
from aptos_sdk.account_address import AccountAddress
from aptos_sdk.async_client import RestClient
from aptos_sdk.bcs import Serializer
from aptos_sdk.transactions import (
    EntryFunction,
    RawTransaction,
    SignedTransaction,
    TransactionArgument,
    TransactionPayload,
)
from aptos_sdk.type_tag import StructTag, TypeTag

from decimal import Decimal

from pathlib import Path

import subprocess

# Client setup
NODE_URL = "http://0.0.0.0:8080"
client = RestClient(NODE_URL)

# Coin amounts.
DECIMALS_DEE_COIN = 8
DECIMALS_USDC = 6
DEE_COIN_MINT_NOMINAL = 100_000_000

def coin_nominal_to_subunit(amount: str, decimals: int) -> int:
    int(Decimal(amount) * Decimal(10 ** decimals))


# Accounts.


#ace = Account.load("accounts/ace.key")
#bee = Account.load("accounts/bee.key")
#cad = Account.load("accounts/cad.key")
#dee = Account.load("accounts/dee.key")
aptos_framework = AccountAddress.from_str("0x1")

# Mint to Dee coin to Dee.
#client.submit_transaction(
    #dee,
    #entry_function = EntryFunction.natural(
        #module=f"{dee.address()}::dee_coin",
        #function="mint",
        #ty_args=[],
        #args=[
            #TransactionArgument(
                #coin_nominal_to_subunit(DEE_COIN_MINT_NOMINAL, DECIMALS_DEE_COIN),
                #Serializer.u64),
        #],
    #)
#)

# Mint USDC to Ace, Bee, and Cad.

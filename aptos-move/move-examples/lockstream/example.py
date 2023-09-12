import asyncio

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

import time
import subprocess

# Client setup
NODE_URL = "http://0.0.0.0:8080"
client = RestClient(NODE_URL)

# Coin amounts.
DECIMALS_DEE_COIN = 8
DECIMALS_USDC = 6
DEE_COIN_MINT_NOMINAL = 100_000_000

def coin_nominal_to_subunit(amount: str, decimals: int) -> int:
    return int(Decimal(amount) * Decimal(10 ** decimals))


# Accounts.
ace = Account.load_key(Path("accounts/ace.key").read_text())
bee = Account.load_key(Path("accounts/bee.key").read_text())
cad = Account.load_key(Path("accounts/cad.key").read_text())
dee = Account.load_key(Path("accounts/dee.key").read_text())

# Mint to Dee coin to Dee.
#time.sleep(30)
#ace_balance = asyncio.run(client.account_balance(ace.address()))

print(asyncio.run(client.account_resources(dee.address())))


#tx_hash = await client.submit_transaction(
    #dee,
    #EntryFunction.natural(
        #module=f"{dee.address()}::dee_coin",
        #function="mint",
        #ty_args=[],
        #args=[
            #TransactionArgument(
                #coin_nominal_to_subunit(DEE_COIN_MINT_NOMINAL, DECIMALS_DEE_COIN),
                #Serializer.u64
            #),
        #],
    #)
#)

# Mint USDC to Ace, Bee, and Cad.

from aptos_sdk.account import Account
from aptos_sdk.account_address import AccountAddress
from aptos_sdk.client import RestClient
from aptos_sdk.transactions import EntryFunction, ModuleId

from pathlib import Path

import subprocess

# Client setup
NODE_URL = "http://0.0.0.0:8080"
client = RestClient(NODE_URL)

# Coin amounts.
DECIMALS_DEE_COIN = 8
DECIMALS_USDC = 6
DEE_COIN_MINT_NOMINAL = 100_000_000

# Accounts.
ace = Account.load("accounts/ace.key")
bee = Account.load("accounts/bee.key")
cad = Account.load("accounts/cad.key")
dee = Account.load("accounts/dee.key")
aptos_framework = AccountAddress.from_str("0x1")

# Modules.
lockstream_module = ModuleId(aptos_framework, "lockstream")
dee_coin_module = ModuleId(dee.address(), "dee_coin")
usdc_module = ModuleId(dee.address(), "usdc")

print(ace.address())
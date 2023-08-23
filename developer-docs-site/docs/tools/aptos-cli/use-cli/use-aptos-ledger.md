---
title: "Use Aptos CLI with Ledger"
id: "use-aptos-ledger"
---

# Use the Aptos CLI with Ledger

The `aptos` tool can be used with your Ledger device to sign any transaction. This is the most secure way to sign transactions, as your private key never leaves your device.

NOTE: It is highly recommended to use `Ledger Nano S Plus` or `Ledger Nano X` devices. The `Ledger Nano S` device has very limited memory and may not be able to sign most of the transactions. If you are trying to sign a transaction that is too big for your device to handle, you will get an error `Wrong raw transaction length`.

## Blind Signing

Before you begin, make sure you have `Blind Signing` enabled on your Ledger device. Otherwise you will not be able to sign transactions.
`Blind Signing` - confirming a smart contract interaction you can’t verify through a human readable language.

## Create a new Ledger profile

In order to interact with your Ledger device, you must first create a new profile. This profile will be used to store your Ledger public key, and will be used to sign transactions.

```bash
$ aptos init --profile myprofile --ledger
Configuring for profile myprofile
Choose network from [devnet, testnet, mainnet, local, custom | defaults to devnet]

No network given, using devnet...
Please choose an index from the following 5 ledger accounts, or choose an arbitrary index that you want to use:
[0] Derivation path: m/44'/637'/0'/0'/0' (Address: 59836ba1dd0c845713bdab34346688d6f1dba290dbf677929f2fc20593ba0cfb)
[1] Derivation path: m/44'/637'/1'/0'/0' (Address: 21563230cf6d69ee72a51d21920430d844ee48235e708edbafbc69708075a86e)
[2] Derivation path: m/44'/637'/2'/0'/0' (Address: 667446181b3b980ef29f5145a7a2cc34d433fc3ee8c97fc044fd978435f2cb8d)
[3] Derivation path: m/44'/637'/3'/0'/0' (Address: 2dcf037a9f31d93e202c074229a1b69ea8ee4d2f2d63323476001c65b0ec4f31)
[4] Derivation path: m/44'/637'/4'/0'/0' (Address: 23c579a9bdde1a59f1c9d36d8d379aeefe7a5997b5b58bd5a5b0c12a4f170431)
0
Account 59836ba1dd0c845713bdab34346688d6f1dba290dbf677929f2fc20593ba0cfb has been already found onchain

---
Aptos CLI is now set up for account 59836ba1dd0c845713bdab34346688d6f1dba290dbf677929f2fc20593ba0cfb as profile myprofile!  Run `aptos --help` for more information about commands
{
  "Result": "Success"
}
```
In the above, we have created a new profile called `myprofile` and have chosen to use the first Ledger account (index 0) to sign transactions. If there is a certain index account you would like to use, you are welcome to use it.


After the above command, a new profile will be created in `~/.aptos/config.yml` and will look like the following:
```yaml
  myprofile:
    public_key: "0x05a8ace09d1136181029be3e817de3619562b0da2eedbff210e2b2f92c71be70"
    account: 59836ba1dd0c845713bdab34346688d6f1dba290dbf677929f2fc20593ba0cfb
    rest_url: "https://fullnode.devnet.aptoslabs.com"
    faucet_url: "https://faucet.devnet.aptoslabs.com"
    derivation_path: "m/44'/637'/0'/0'/0'"
```
Notice that the above stores the derivation path instead of private key. This is because the private key is stored on your Ledger device, and is never exposed to the `aptos` tool.

## Publish a package with Ledger
Once you have created a profile, you can use it to publish a package. The `aptos` tool will prompt you to confirm the transaction on your Ledger device.
Note: Make sure that you are on the same directory as where your move module is located:
```bash
$ aptos move publish --profile myprofile --named-addresses hello_blockchain=myprofile
Compiling, may take a little while to download git dependencies...
INCLUDING DEPENDENCY AptosFramework
INCLUDING DEPENDENCY AptosStdlib
INCLUDING DEPENDENCY MoveStdlib
BUILDING Examples
package size 1755 bytes
Do you want to submit a transaction for a range of [139600 - 209400] Octas at a gas unit price of 100 Octas? [yes/no] >
yes
{
  "Result": {
    "transaction_hash": "0xd5a12594f85284cfd5518d547d084030b178ee926fa3d8cbf699cc0596eff538",
    "gas_used": 1396,
    "gas_unit_price": 100,
    "sender": "59836ba1dd0c845713bdab34346688d6f1dba290dbf677929f2fc20593ba0cfb",
    "sequence_number": 0,
    "success": true,
    "timestamp_us": 1689887104333038,
    "version": 126445,
    "vm_status": "Executed successfully"
  }
}
```

After the above command, you will be prompted to confirm the transaction on your Ledger device. Once you confirm, the transaction will be submitted to the network. Note: Make sure you have `Blind Signing` enabled on your Ledger device. Otherwise you will not be able to sign transactions.
`Blind Signing` - confirming a smart contract interaction you can’t verify through a human readable language.

## Common Errors

### Error: Wrong raw transaction length
Your raw transaction or package size is too big. Currently the Aptos ledger app can only support up to 20kb transaction. If you are using a `Ledger Nano S`, the supported transaction size will be even smaller.
```bash
{
  "Error": "Unexpected error: Error - Wrong raw transaction length"
}
```

### Error: Ledger device is locked
Make sure your Ledger device is unlocked and you have Aptos app opened
```bash
{
  "Error": "Unexpected error: Error - Ledger device is locked"
}
```
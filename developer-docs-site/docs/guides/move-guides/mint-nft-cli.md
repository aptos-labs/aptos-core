---
title: "Mint an NFT with Aptos CLI"
slug: "mint-nft-cli"
---

# Mint an NFT

## Prerequisites

This tutorial assumes you have:

* a GitHub account
* the GitHub CLI
* the Aptos CLI (installed below)

## Mint with the Aptos CLI

Now that you are starting to write smart contracts with Move, let's create our first NFT with the Aptos CLI.

1. [Install the Aptos CLI](../../cli-tools/aptos-cli-tool/install-aptos-cli.md) and note its [many uses](../../cli-tools/aptos-cli-tool/use-aptos-cli.md) for later if you haven't experienced its goodness already.

1. Create an account on Aptos testnet by running the following command and selecting `testnet`:
```shell
aptos init --profile nft-receiver
```

1. When prompted, select `testnet` by entering it:

```shell
Configuring for profile nft-receiver
Choose network from [devnet, testnet, mainnet, local, custom | defaults to devnet]
testnet
```

1. When prompted for your private key, hit enter to generate a new key:
```shell
Enter your private key as a hex literal (0x...) [Current: None | No input: Generate new key (or keep one if present)]
```

1. Receive output resembling:
```shell
No key given, generating key...
Account blah does not exist, you will need to create and fun the account through a community faucet e.g. https://aptoslabs.com/testnet-faucet, or by transferring funds from another account

---
Aptos CLI is now set up for account blah as profile nft-receiver!  Run `aptos --help` for more information about commands
{
  "Result": "Success"
}
➜  devel
```

1. Note *blah* above is a placeholder for your private key. Record it someplace safe.

1. Mint the NFT:

```shell
aptos move run --function-id 8cdf69c8c93fee36ed83f8882908060c1335ed39a827c08dbb506b46237e88fb::minting::mint_nft --profile nft-receiver
```

1. When asked, `Do you want to submit a transaction for a range of...?`, enter: `yes`

1. Receive results resembling:

```shell
{
  "Result": {
    "transaction_hash": "0x9dc5c20f45a06d0cc621bf12610caa5b3c0797ac181c3339248b48ab0f0fcba2",
    "gas_used": 3917,
    "gas_unit_price": 100,
    "sender": "7d69283af198b1265d17a305ff0cca6da1bcee64d499ce5b35b659098b3a82dc",
    "sequence_number": 13,
    "success": true,
    "timestamp_us": 1668045908262170,
    "version": 341215563,
    "vm_status": "Executed successfully"
  }
}
*/
```

## Find the NFT in your Petra wallet

1. Run `more ~/.aptos/config.yaml` to see the `nft-receiver` private key and then copy it.

1. Install the [Petra Wallet](../../guides/install-petra-wallet.md) Chrome extension.

1. Select the [Testnet network](https://petra.app/docs/use) in the wallet via *Petra settings > Network > Testnet*.

1. Use your testnet coins to send a transaction and mint an NFT. Obtain more and connect your wallet to the faucet at: https://aptoslabs.com/testnet-faucet

1. Go to *Petra > Settings > Switch account > Add Account > Import private key*.

1. Paste the `nft-receiver` private key there.

1. Go to *Petra > Settings > Network > Testnet*.

1. Click **Library** at bottom.

1. See the NFT in your wallet.

## Deploy the NFT contract

Now you can add this smart contract to the Aptos network:

1. Check out and review the [NFT Tutorial]([https://github.com/aptos-labs/nft-tutorial](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/mint_nft) source code.

1. Explore the `mint_event_ticket` function in [`minting.move`](https://github.com/aptos-labs/nft-tutorial/blob/main/sources/minting.move) within each of the folders.

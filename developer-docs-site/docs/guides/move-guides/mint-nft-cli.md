---
title: "Mint an NFT with Aptos CLI"
slug: "mint-nft-cli"
---

# Mint an NFT 

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

TODO: Find out why I get "No collectibles yet." Confirm the private key is emitted in the output above as it says account. If not from there, where do they derive this essential ingredient?
---
title: "Mint an NFT with Aptos CLI"
slug: "mint-nft-cli"
---

# Mint an NFT

This tutorial lets you use the Aptos CLI to mint NFTs in Aptos testnet so you can see how the process works and employ related functions in your code.

## Prerequisites

This tutorial assumes you have:

* a GitHub account
* the GitHub CLI
* the Aptos CLI (installed below, or you can run from source via `cargo run`)

## Understand the minting workflow

In short, when you are minting an NFT, Aptos ensures no one else can alter your collection. This is why a private key is required to obtain signer capabilities. When you submit a transaction, you sign the transaction. Creating a [resource account](../resource-accounts.md) grants the signer capability that can be stored in a new resource on the same account. And that capability is protected as no one has access to the private key for the resource account. Resource accounts allow the delegation of signing transactions.

Check out and review the [NFT Tutorial](https://github.com/aptos-labs/nft-tutorial/tree/main/tutorial) source code.

Explore the `mint_nft` function in [`minting.move`](https://github.com/aptos-labs/nft-tutorial/blob/main/sources/minting.move).

Note the `mint_nft` function gets a collection and creates a token.

With the resulting TokenData ID, the function uses the resource signer of the module to mint the token to the `nft-receiver`.

For example:
```shell
    public entry fun mint_nft(receiver: &signer) acquires ModuleData {
        let receiver_addr = signer::address_of(receiver);
```

The only argument taken is `receiver`. Any `entry fun` will take as the first parameter the type `&signer`. In both Move and Aptos, whenever you submit a transaction, whatever private key you sign the transaction with, the associated account automatically becomes the first parameter of the signer.

You can go from the signer to an address but normally not the reverse. So when claiming an NFT, both the private keys of the minter and receiver are needed, as shown below.

Also in [`minting.move`](https://github.com/aptos-labs/nft-tutorial/blob/main/sources/minting.move), see this `init_module`:

```shell
    fun init_module(resource_account: &signer) {
        let resource_signer_cap = resource_account::retrieve_resource_account_cap(resource_account, @source_addr);
        let resource_signer = account::create_signer_with_capability(&resource_signer_cap);
```

This `init_module` always gets called and run when the module is published. Here, the signer is always the account uploading the contract. This gets combined with:

```shell
        token::create_collection(&resource_signer, collection, description, collection_uri, maximum_supply, mutate_setting);

```
Where the `&resource_signer` is the first parameter, defined previously as a new address that has all of the attributes of the original account plus signer capability. See:

```shell
        signer_cap: account::SignerCapability,
```

The signer capability prevents anyone from getting the private key from the resource acount. The [resource account](../resource-accounts.md) is entirely controlled by the contract. So later in the same file:

```shell
        move_to(resource_account, ModuleData {
```

The `ModuleData` is initialized and *moved* to the resource account, which has the signer capability. So in the `mint_nft` function, you see the first step is borrowing the `ModuleData` struct:

```shell
        let module_data = borrow_global_mut<ModuleData>(@mint_nft);
```

And then uses the reference to the signer capability in the  `ModuleData` struct to create the `resource_signer`:

```shell
        let resource_signer = account::create_signer_with_capability(&module_data.signer_cap);
```

In this manner, you can later use the signer capability already stored in module. When you move a module and its structs into an account, they become visible in [Aptos Explorer](https://explorer.aptoslabs.com/) associated with the account.

## Mint with the Aptos CLI

Now that you are starting to write smart contracts with Move, let's create our first testnet NFT with the Aptos CLI.

### Install Aptos CLI and create an account

1. [Install the Aptos CLI](../../cli-tools/aptos-cli-tool/install-aptos-cli.md) and note its [many uses](../../cli-tools/aptos-cli-tool/use-aptos-cli.md) for later if you haven't experienced its goodness already.

1. Create an account on Aptos testnet to receive the NFT by running the following command and selecting `testnet`:
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
Account blah does not exist, you will need to create and fund the account through a community faucet e.g. https://aptoslabs.com/testnet-faucet, or by transferring funds from another account

---
Aptos CLI is now set up for account blah as profile nft-receiver!  Run `aptos --help` for more information about commands
{
  "Result": "Success"
}
➜  devel
```

1. Note your configuration information can be found in ~/.aptos/config.yaml`. Read that file to see your private and public keys, account address, and REST API URL per network.

### Install wallet and import account

1. Run `more ~/.aptos/config.yaml` to see the `nft-receiver` private key and then copy it.

1. Install the wallet of your choice. We use the [Petra Wallet](../../guides/install-petra-wallet.md) Chrome extension.

1. Open the wallet and select the [Testnet network](https://petra.app/docs/use) in the wallet via *Petra settings > Network > Testnet*.

1. Go to *Petra > Settings > Switch account > Add Account > Import private key*.

1. Paste the `nft-receiver` private key there.

1. Click **Submit** to add the previously created account to the wallet.

1. You are switched into that account automatically.

### Get coins from faucet

1. Go to *Petra > Settings > Network > Testnet*.

1. Open the extension and connect your wallet to the Aptos faucet at: https://aptoslabs.com/testnet-faucet

1. Approve the connection request.

1. Now when you load your wallet, you will see a **Faucet** button next to **Send**. Click **Faucet** to receive one APT to use when minting.

### Mint the NFT

1. Mint the NFT by calling the `mint_nft` function and an existing contract using the Aptos CLI:

```shell
aptos move run --function-id 8cdf69c8c93fee36ed83f8882908060c1335ed39a827c08dbb506b46237e88fb::minting::mint_nft --profile nft-receiver
```

1. When asked, `Do you want to submit a transaction for a range of...?`, enter: `yes`

1. Receive results resembling:

```shell
{
  "Result": {
    "transaction_hash": "0x6e022532fb8d802324829d5ec85fd32c05a58a6f826751f63cdbf9bf313ff991",
    "gas_used": 3944,
    "gas_unit_price": 150,
    "sender": "b9d394a7bc582a54e8610d6a7b973f62c8d9595c54c35cdbb95965aa8e5cd111",
    "sequence_number": 0,
    "success": true,
    "timestamp_us": 1670969779029341,
    "version": 385901038,
    "vm_status": "Executed successfully"
  }
}
*/
```

## Find the NFT in your Petra wallet

1. Open the Petra Wallet Chrome extension.

1. Go to *Petra > Settings > Network > Testnet*.

1. Click **Library** at bottom.

1. See the NFT in your wallet.

## Deploy the NFT contract

Now you can add this smart contract to the Aptos network.

### Create and fund admin and source account

Create two accounts on testnet for deploying and managing this contract:
  * The source account will be used to create the resource account that will deploy this smart contract.
  * The admin account is in charge of updating the config of the module (e.g. whether or not we enable minting for this collection).

1. Run these commands to create the accounts, selecting `testnet` when prompted:

```shell
aptos init --profile source-account
```
```shell
aptos init --profile admin-account
```
1. Open `~/.aptos/config.yaml` to find the private keys for the `admin-account` and `source-account` profiles and copy them.

1. Fund these accounts by adding them to your wallet via importing their private keys into testnet and using the **Faucet** function as you did for the `nft-receiver` profile.

### Create a resource account from source account

In this section, we will create a [resource account](../resource-accounts.md) from the source-account and publish the module on testnet under the resource account’s address. A resource account is used here to programmatically signed for transactions and avoids the need for multiple signatures.

In the [NFT Tutorial](https://github.com/aptos-labs/nft-tutorial/tree/main/tutorial) smart contract, we store the resource account’s signer capability in the `ModuleData` resource so that it can automatically sign for transactions in the contract. If we don’t store the signer capability within the module, we’d need to provide the resource account’s signer; but we no longer have access to the resource account’s signer because the resource account is meant to be autonomous, and the contracts published under a resource account are immutable. Once the contract is published, the resource account no longer has access to the signer.

1. Clone the NFT Tutorial:
```shell
git clone https://github.com/aptos-labs/nft-tutorial
```

1. Navigate into the tutorial directory:
```shell
cd nft-tutorial/tutorial
```

1. Open `Move.toml` in that directory for editing.

1. Update the `source_addr` and `admin_addr` with the `account` values for the `source-account` and `admin-account` profiles you just created (found in `~/.aptos/config.yaml`).

1. Run the following command to create the resource account and publish the package all at once. The seed is just an example - feel free to provide a different seed if the resource account created by the specified seed already exists:

```shell
aptos move create-resource-account-and-publish-package --seed hex_array:1234 --address-name mint_nft --profile source-account
```

1. Receive output indicating success and resembling:

```shell
/* expected output
Compiling, may take a little while to download git dependencies...
INCLUDING DEPENDENCY AptosFramework
INCLUDING DEPENDENCY AptosStdlib
INCLUDING DEPENDENCY AptosToken
INCLUDING DEPENDENCY MoveStdlib
BUILDING NFT-tutorial
Do you want to publish this package under the resource account's address 8cdf69c8c93fee36ed83f8882908060c1335ed39a827c08dbb506b46237e88fb? [yes/no] >
yes
package size 4550 bytes
Do you want to submit a transaction for a range of [513300 - 769900] Octas at a gas unit price of 100 Octas? [yes/no] >
yes
{
  "Result": "Success"
}
*/
```

1. Mint another NFT using the `nft-receiver` profile, sustituting in the resource account's address:

```shell
aptos move run --function-id <resource-account-address>::minting::mint_nft --profile nft-receiver
```

1. Again answer `yes` when prompted to submit the transaction.

1. Receive output resembling:

```shell
{
  "Result": {
    "transaction_hash": "0x62660973b1a94e620c863899a157b0b46c02dcfdb0c9261a34ed4d2391550fc7",
    "gas_used": 6691,
    "gas_unit_price": 100,
    "sender": "aaf015db7b6dacb1db4637ef017e68e689a40797fe8fd3897cee08808979bb91",
    "sequence_number": 0,
    "success": true,
    "timestamp_us": 1667434233137811,
    "version": 27685944,
    "vm_status": "Executed successfully"
  }
}
*/
```

1. Disable NFT minting in this module using the `admin-account` profile so that random folks cannot mint this NFT from your module:

```shell
aptos move run --function-id <resource-account-address>::minting::set_minting_enabled --args bool:false --profile admin-account
```

Now you can include your own artwork once you are ready to customize your NFTs by replacing our defaults in `minting.move`:
https://slwdaeeko5tz5hx46c6zwqhmh3c6je4sbdbjsdjzbntme5dxarxa.arweave.net/kuwwEIp3Z56e_PC9m0DsPsXkk5IIwpkNOQtmwnR3BG4
https://lty5vdw4cl6yczbpz2rnm2732rbtnk3jeiutyqd644wojmkyt2hq.arweave.net/XPHajtwS_YFkL86i1mv71EM2q2kiKTxAfucs5LFYno8
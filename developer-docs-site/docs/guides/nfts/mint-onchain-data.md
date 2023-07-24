---
title: "Mint FTs with On-Chain Data"
---
import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Mint Fungible Tokens with On-Chain Data

You, as a developer, maybe also a game fancier, recently starts to develop a game and hope to
integrate it with some web3 experience. You spontaneously think Aptos supports fungible tokens
with abundant APIs in the token standard. So you wanna give a try to issue an in-game currency
called *silver coin* by using the [Aptos command line interface (CLI)](../../tools/aptos-cli/install-cli/index.md). There is only two requirements:
- The token is fungible
- The token is associated with a URL that points to the coin image.

## Create a collection
Token must belong to a collection. First, you can create a collection by running Aptos CLI:
```bash
aptos move run --function-id 0x3::token::create_collection_script --args string:"Game Coins" string:"The collection of all in-game coins" string:"" u64:0 'bool:[false,false,false]'
```
Parameters:
- collection name: "Game Coins".
- collection description: "The collection of all in-game coins".
- uri: It's empty because we don't intend to have an url for this collection. In general, you
  could put an image or a JSON url here representing the collection or providing rich metadata.
- maximum: 0. 0 means there is no maximum limit for the size of the collections.
- mutability vector:
  1. description: false
  2. uri: false
  3. maximum: false

All the parameters could be adjusted as appropriate.

## Create tokens
The fungible token in Aptos has an unique `TokenId`, a set of creator, collection name, token name and property version.
For fungible tokens, the property version is always 0. To create a token named "Silver Coin", you run:
```bash
aptos move run --function-id 0x3::token::create_token_script --args string:"Game Coins" string:"Silver Coin" string:"The currency in my game" u64:100 u64:0 string:$TOKEN_URL address:$YOUR_ADDRESS u64:0 u64:0 'bool:[false,false,false,false,false]' raw:00 raw:00 raw:00
```
Parameters:
- collection name: "Game Coins", same as the previous step.
- token name: "Silver Coin".
- token description: "The currency in my game".
- maximum: 0. 0 means there is no maximum supply of this token.
- uri: Your token url can point to anything. Usually it is an arweave or ipfs url pointing to an token image or JSON
  file containing the required metadata. 
- royalty_payee_address: Used for NFT. For FT, it is recommended to set the creator address. 
- royalty_points_denominator: Used for NFT. Set to 0 as N/A. 
- royalty_points_numerator: Used for NFT. Set to 0 as N/A
- token_mutate_config
    1. maximum: false
    2. uri: false
    3. royalty: false
    4. description: false
    5. property map: false
- property_keys: empty as we don't need any special property.
- property_values: ditto.
- property_types: ditto.

:::tip Customized Property Map
The last three CLI arguments passed into `move run` is "raw" type because current CLI doesn't support
`vector<string>` type as pass-in arguments. So for empty vectors, we can easily hack it using the raw serialized
format (`0x00` is BCS-ed binary format of any empty vector). We are working hard to support it later. Therefore,
if you need customized properties to be set, please use our Rust/Typescript/Python SDK instead for now.
:::

After run this command, you, the creator account, will have created your "silver coin" and mint 100 silver coins to your
account.

## Mint, Transfer and Burn
After silver coin is created, As the creator, you can `mint` or `burn` at your own discretion and anyone who owns silver
coin can `transfer` or `burn` any amount of tokens not exceeding the balance.

### Mint
To mint 1000 tokens, the creator can run:
```bash
aptos move run --function-id 0x3::token::mint_script --args address:$YOUR_ADDRESS string:"Game Coins" string:"Silver Coin" u64:1000
```
Parameters:
- creator address: put your creator address here.
- collection name: "Game Coins", same as before.
- token name: "Silver Coin", same as before.
- mint amount: 1000.

### Transfer
To transfer 20 tokens to a receiver, the owner can run:
```bash
aptos move run --function-id 0x3::token::transfer_with_opt_in --args address:$CREATOR_ADDRESS string:"Game Coins" string:"Silver Coin" u64:0 address:$RECEIVER_ADDRESS u64:20
```

:::tip Opt-in direct transfer
Aptos token contract, to avoid receiving spam tokens, by default disable direct transfer of any token, which means if the receiver
does not opt-in direct transfer of a specific token, the sender calling `transfer_with_opt_in` will get an error. Thus,
if the receiver wish to accept this token, she has to run the following command first.
```bash
aptos move run --function-id 0x3::token::opt_in_direct_transfer --args bool:true
```
:::

### Burn
If you are the creator, you can burn silver coins from any account. If you are an owner, you are only allowed to burn
your own silver coins.

To burn 10 tokens as a creator from any owner's account, you can run:
```bash
aptos move run --function-id 0x3::token::burn_by_creator --args address:$OWNER_ADDRESS string:"Game Coins" string:"Silver Coin" u64:0 u64:10
```
Parameters:
- owner address: put the address of any owner of silver coin here.
- collection name: "Game Coins", same as before.
- token name: "Silver Coin", same as before.
- property version: 0. FT will always have 0 as property version.
- burn amount: 10.

To burn 10 tokens as an owner, you can run:
```bash
aptos move run --function-id 0x3::token::burn --args address:$CREATOR_ADDRESS string:"Game Coins" string:"Silver Coin" u64:0 u64:10
```
Since owner will burn silver coin from her own address, owner address is not required as a parameter.

Parameters:
- owner address: put the address of the creator of silver coin here.
- collection name: "Game Coins", same as before.
- token name: "Silver Coin", same as before.
- property version: 0. FT will always have 0 as property version.
- burn amount: 10.

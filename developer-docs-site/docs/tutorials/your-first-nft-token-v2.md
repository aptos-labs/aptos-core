--- 
title: "Mint NFT with the token v2 SDKs"
slug: "your-first-nft-token-v2"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Mint Token v2 NFTs with the Aptos SDKs

This tutorial describes how to create and transfer NFTs on the Aptos blockchain. The Aptos implementation for core NFTs can be found in [aptos_token.move](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token-objects/sources/aptos_token.move) Move module.

## Step 1: Pick an SDK

Install your preferred SDK from the below list:

* [TypeScript SDK](../sdks/ts-sdk/index.md)
* [Python SDK](../sdks/python-sdk.md)

---

## Step 2: Run the example

Each SDK provides an `examples` directory. This tutorial covers the `simple-nft-v2` example. 

Clone the `aptos-core` repo: 
```bash 
git clone git@github.com:aptos-labs/aptos-core.git ~/aptos-core
```

<Tabs groupId="sdk-examples">
    <TabItem value="python" label="Python">

Navigate to the Python SDK directory:
  ```bash
  cd ~/aptos-core/ecosystem/python/sdk
  ```

Install the necessary dependencies:
  ```bash
  curl -sSL https://install.python-poetry.org | python3
  poetry update
  ```

Run the Python [`simple-nft`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/python/sdk/examples/simple-nft-v2.py) example:
  ```bash
  poetry run python -m examples.simple-nft-v2 main
  ```
  </TabItem>
</Tabs>

---

## Step 3: Understand the output

The following output should appear after executing the `simple-nft-v2` example, though some values will be different:

```yaml
=== Addresses ===
Alice: 0x5ce75334cd71793f417cc76cbb0b191660b8868538d6590086f4f231c73ed073
Bob: 0x443b51f29aa0beb2387e24752710e3e4edabd69b4bf336db333e802ac5a59f97

  === Initial Coin Balances ===
Alice: 100000000
Bob: 100000000

  === Creating Collection and Minting Token ===
Alice's collection: ReadObject
  0x1::object::ObjectCore: Object[allow_ungated_transfer: False, owner: 0x5ce75334cd71793f417cc76cbb0b191660b8868538d6590086f4f231c73ed073]
  0x4::collection::Collection: AccountAddress[creator: 0x5ce75334cd71793f417cc76cbb0b191660b8868538d6590086f4f231c73ed073, description: Alice's simple collection, name: Alice's, ur: https://aptos.dev]
  0x4::royalty::Royalty: Royalty[numerator: 0, denominator: 1, payee_address: 0x5ce75334cd71793f417cc76cbb0b191660b8868538d6590086f4f231c73ed073]
Alice's token data: ReadObject
  0x1::object::ObjectCore: Object[allow_ungated_transfer: True, owner: 0x5ce75334cd71793f417cc76cbb0b191660b8868538d6590086f4f231c73ed073]
  0x4::property_map::PropertyMap: PropertyMap[Property[string, 0x1::string::String, string value]]
  0x4::token::Token: Token[collection: 0xac104d89b8a8a99a48c07be0edce2664ac830cb1ae921e170fa7e08a5ed18a6c, index: 1, description: Alice's simple token, name: Alice's first token, uri: https://aptos.dev/img/nyan.jpeg]

  === Transferring the token to Bob ===
Bob's token: ReadObject
  0x1::object::ObjectCore: Object[allow_ungated_transfer: True, owner: 0x443b51f29aa0beb2387e24752710e3e4edabd69b4bf336db333e802ac5a59f97]
  0x4::property_map::PropertyMap: PropertyMap[Property[string, 0x1::string::String, string value]]
  0x4::token::Token: Token[collection: 0xac104d89b8a8a99a48c07be0edce2664ac830cb1ae921e170fa7e08a5ed18a6c, index: 1, description: Alice's simple token, name: Alice's first token, uri: https://aptos.dev/img/nyan.jpeg]
```

This example demonstrates:

* Initializing the REST and faucet clients.
* The creation of two accounts: Alice and Bob.
* The funding and creation of Alice and Bob's accounts.
* The creation of a collection using Alice's account.
* The minting of a token from the collection to Alice's account. 
* Alice transferring a token to Bob. 

---

## Step 4: The SDK in depth

<Tabs groupId="sdk-examples">
  <TabItem value="python" label="Python">

:::tip See the full code
See [`simple-nft-v2`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/python/sdk/examples/simple-nft-v2.py) for the complete code as you follow the below steps.
:::
</TabItem>
</Tabs>

---

### Step 4.1: Initializing the clients

In the first step, the example initializes both the API and faucet clients.

- The API client interacts with the REST API.
- The faucet client interacts with the devnet Faucet service for creating and funding accounts.

<Tabs groupId="sdk-examples">
    <TabItem value="python" label="Python">
    
```python
:!: static/sdks/python/examples/simple-nft.py section_1
```

[`common.py`](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/python/sdk/examples/common.py) initializes these values as follows:

```python
:!: static/sdks/python/examples/common.py section_1
```
  </TabItem>
</Tabs>

:::tip

By default, the URLs for both the services point to Aptos devnet services. However, they can be configured with the following environment variables:
- `APTOS_NODE_URL`
- `APTOS_FAUCET_URL`
  :::

---

### Step 4.2: Creating local accounts

The next step is to create two accounts locally. [Accounts](../concepts/accounts.md) represent both on and off-chain state. Off-chain state consists of an address and the public/private key pair used to authenticate ownership. This step demonstrates how to generate that off-chain state.

<Tabs groupId="sdk-examples">
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/simple-nft.py section_2
```
  </TabItem>
</Tabs>

---

### Step 4.3: Creating blockchain accounts

In Aptos, each account must have an on-chain representation in order to receive tokens and coins and interact with other dApps. An account represents a medium for storing assets; hence, it must be explicitly created. This example leverages the Faucet to create Alice and Bob's accounts:

<Tabs groupId="sdk-examples">
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/simple-nft.py section_3
```
  </TabItem>
</Tabs>

---

### Step 4.4: Creating a collection

Now begins the process of creating tokens. First, the creator must create a collection to mint tokens. A collection can contain zero, one, or many distinct tokens within it. 
The collection does not restrict the attributes of the tokens, as it is only a container.

<Tabs groupId="sdk-examples">
  <TabItem value="python" label="Python">

Your application will call `create_collection`:
```python
:!: static/sdks/python/examples/simple-nft.py section_4
```

The is the function signature of `create_collection`. It returns a transaction hash:
```python
:!: static/sdks/python/aptos_sdk/client.py create_collection
```
  </TabItem>
</Tabs>

---

### Step 4.5: Minting a token

To mint a token, the creator must specify an associated collection. 


<Tabs groupId="sdk-examples">
  <TabItem value="python" label="Python">

Your application will call `mint_token`:
```python
:!: static/sdks/python/examples/simple-nft.py section_5
```

The is the function signature of `mint_token`. It returns a transaction hash:
```python
:!: static/sdks/python/aptos_sdk/client.py mint_token
```
  </TabItem>
</Tabs>

---

### Step 4.6: Reading token and collection metadata

Under the token v2 framework, a collection and each token within the collection will be an object that has its own address. 

<Tabs groupId="sdk-examples">
<TabItem value="python" label="Python">

We can get the collection's address from the creator's address and the collection name by calling the 
`AccountAddress.for_named_collection` function. 
We can then read the collection object by calling `token_client.read_object` function with the collection's address. 
```python
:!: static/sdks/python/examples/simple-nft.py section_6
```

Similarly, to read a token's metadata, we first get its address and then call `token_client.read_object` with the token's
address. 
```python
:!: static/sdks/python/examples/simple-nft.py section_7
```
  </TabItem>
</Tabs>

---

### Step 4.7: Transferring a token

In token v1, when transferring a token, the creator needs to initiate an offer transaction, and the recipient needs to claim the token. 
This is to prevent recipients from having tokens that they do not want. 

In token v2, this works differently. As long as the token object itself allows ungated transfer, the current owner of the token can transfer 
the token to another account. In the future, we might allow users to dictate which resources can be transferred to them by either 1) supporting
a user object store that represents objects that the user has accepted or 2) having a flag in the account structure that indicates whether the user 
is willing to accept objects. 
<Tabs groupId="sdk-examples">
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/simple-nft.py section_8
```
  </TabItem>
</Tabs>

---

## Supporting documentation
* [Object AIP](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-10.md)
* [Token V2 AIP](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-11.md)
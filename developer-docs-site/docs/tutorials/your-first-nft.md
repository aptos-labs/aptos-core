---
title: "Your First NFT"
slug: "your-first-nft"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Your First NFT

This tutorial describes how to create and transfer non-fungible assets on the Aptos blockchain. The Aptos no-code implementation for non-fungible digital assets can be found in the [aptos_token.move](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token-objects/sources/aptos_token.move) Move module.


## Step 1: Pick an SDK

Install your preferred SDK from the below list:

* [TypeScript SDK](../sdks/new-ts-sdk/index.md)
* [Python SDK](../sdks/python-sdk.md)

---

## Step 2: Run the example

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

Clone the `aptos-ts-sdk` repo:
```bash
git clone git@github.com:aptos-labs/aptos-ts-sdk.git 
```

  Navigate to the Typescript SDK examples directory:
  ```bash
  cd aptos-ts-sdk/examples/typescript-esm
  ```

  Install the necessary dependencies:
  ```bash
  pnpm install
  ```

  Run the Typescript [`simple_digital_asset`](https://github.com/aptos-labs/aptos-ts-sdk/blob/main/examples/typescript-esm/simple_digital_asset.ts) example:
  ```bash
  pnpm run simple_digital_asset
  ```
  </TabItem>
  <TabItem value="python" label="Python">

  Clone the `aptos-core` repo:
```bash
git clone https://github.com/aptos-labs/aptos-core.git
```

  Navigate to the Python SDK directory:
  ```bash
  cd ~/aptos-core/ecosystem/python/sdk
  ```

  Install the necessary dependencies:
  ```bash
  curl -sSL https://install.python-poetry.org | python3
  poetry install
  ```

  Run the Python [`simple_aptos_token`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/python/sdk/examples/simple_aptos_token.py) example:
  ```bash
  poetry run python -m examples.simple_aptos_token
  ```
  </TabItem>
</Tabs>

---

## Step 3: Understand the output

<Tabs groupId="sdk-output">
<TabItem value="typescript" label="Typescript">

The following output should appear after executing the `simple_digital_asset` example, though some values will be different:

```yaml
=== Addresses ===

Alice's address is: 0x770dbeb6101056eac5a19de9a73ad72fac512e0de909e7bcb13a9d9241d1d162

=== Create the collection ===

Alice's collection: {
    "collection_id": "0x23ece6c35415f5c5a720dc4de2820cabece0a6f1768095db479f657ad2c05753",
    "collection_name": "Example Collection",
    "creator_address": "0x770dbeb6101056eac5a19de9a73ad72fac512e0de909e7bcb13a9d9241d1d162",
    "current_supply": 0,
    "description": "Example description.",
    "last_transaction_timestamp": "2023-11-29T21:26:03.204874",
    "last_transaction_version": 8001101,
    "max_supply": 18446744073709552000,
    "mutable_description": true,
    "mutable_uri": true,
    "table_handle_v1": null,
    "token_standard": "v2",
    "total_minted_v2": 0,
    "uri": "aptos.dev"
}

=== Alice Mints the digital asset ===

Alice's digital assets balance: 1
Alice's digital asset: {
    "token_standard": "v2",
    "token_properties_mutated_v1": null,
    "token_data_id": "0x9f4460e29a66b4e41cef1671767dc8a5e8c52a2291e36f84b8596e0d1205fd8c",
    "table_type_v1": null,
    "storage_id": "0x9f4460e29a66b4e41cef1671767dc8a5e8c52a2291e36f84b8596e0d1205fd8c",
    "property_version_v1": 0,
    "owner_address": "0x770dbeb6101056eac5a19de9a73ad72fac512e0de909e7bcb13a9d9241d1d162",
    "last_transaction_version": 8001117,
    "last_transaction_timestamp": "2023-11-29T21:26:04.521624",
    "is_soulbound_v2": false,
    "is_fungible_v2": false,
    "amount": 1,
    "current_token_data": {
        "collection_id": "0x23ece6c35415f5c5a720dc4de2820cabece0a6f1768095db479f657ad2c05753",
        "description": "Example asset description.",
        "is_fungible_v2": false,
        "largest_property_version_v1": null,
        "last_transaction_timestamp": "2023-11-29T21:26:04.521624",
        "last_transaction_version": 8001117,
        "maximum": null,
        "supply": 0,
        "token_data_id": "0x9f4460e29a66b4e41cef1671767dc8a5e8c52a2291e36f84b8596e0d1205fd8c",
        "token_name": "Example Asset",
        "token_properties": {},
        "token_standard": "v2",
        "token_uri": "aptos.dev/asset",
        "current_collection": {
            "collection_id": "0x23ece6c35415f5c5a720dc4de2820cabece0a6f1768095db479f657ad2c05753",
            "collection_name": "Example Collection",
            "creator_address": "0x770dbeb6101056eac5a19de9a73ad72fac512e0de909e7bcb13a9d9241d1d162",
            "current_supply": 1,
            "description": "Example description.",
            "last_transaction_timestamp": "2023-11-29T21:26:04.521624",
            "last_transaction_version": 8001117,
            "max_supply": 18446744073709552000,
            "mutable_description": true,
            "mutable_uri": true,
            "table_handle_v1": null,
            "token_standard": "v2",
            "total_minted_v2": 1,
            "uri": "aptos.dev"
        }
    }
}

=== Transfer the digital asset to Bob ===

Alices's digital assets balance: 0
Bob's digital assets balance: 1
```

This example demonstrates:

* Initializing the Aptos client.
* The creation of two accounts: Alice and Bob.
* The funding and creation of Alice and Bob's accounts.
* The creation of a collection and a token using Alice's account.
* Alice sending a token to Bob.


  </TabItem>
  <TabItem value="python" label="Python">

The following output should appear after executing the `simple_aptos_token` example, though some values will be different:

```yaml
=== Addresses ===
Alice: 0x391f8b07439768674023fb87ae5740e90fb8508600486d8ee9cc411b4365fe89
Bob: 0xfbca055c91d12989dc6a2c1a5e41ae7ba69a35454b04c69f03094bbccd5210b4

=== Initial Coin Balances ===
Alice: 100000000
Bob: 100000000

=== Creating Collection and Token ===

Collection data: {
    "address": "0x38f5310a8f6f3baef9a54daea8a356d807438d3cfe1880df563fb116731b671c",
    "creator": "0x391f8b07439768674023fb87ae5740e90fb8508600486d8ee9cc411b4365fe89",
    "name": "Alice's",
    "description": "Alice's simple collection",
    "uri": "https://aptos.dev"
}

Token owner: Alice
Token data: {
    "address": "0x57710a3887eaa7062f96967ebf966a83818017b8f3a8a613a09894d8465e7624",
    "owner": "0x391f8b07439768674023fb87ae5740e90fb8508600486d8ee9cc411b4365fe89",
    "collection": "0x38f5310a8f6f3baef9a54daea8a356d807438d3cfe1880df563fb116731b671c",
    "description": "Alice's simple token",
    "name": "Alice's first token",
    "uri": "https://aptos.dev/img/nyan.jpeg",
    "index": "1"
}

=== Transferring the token to Bob ===
Token owner: Bob

=== Transferring the token back to Alice ===
Token owner: Alice
```

This example demonstrates:

* Initializing the REST and faucet clients.
* The creation of two accounts: Alice and Bob.
* The funding and creation of Alice and Bob's accounts.
* The creation of a collection and a token using Alice's account.
* Alice sending a token to Bob.
* Bob sending the token back to Alice.


  </TabItem>
</Tabs>

---

## Step 4: The SDK in depth

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

:::tip See the full code
See [`simple_digital_asset`](https://github.com/aptos-labs/aptos-ts-sdk/blob/main/examples/typescript-esm/simple_digital_asset.ts) for the complete code as you follow the below steps.
:::
  </TabItem>
  <TabItem value="python" label="Python">

:::tip See the full code
See [`simple_aptos_token`](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/python/sdk/examples/simple_aptos_token.py) for the complete code as you follow the below steps.
:::
  </TabItem>
</Tabs>

---

### Step 4.1: Initializing the clients

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

In the first step, the `simple_digital_asset` example initializes the Aptos client:

```ts
const APTOS_NETWORK: Network = NetworkToNetworkName[process.env.APTOS_NETWORK] || Network.DEVNET;
const config = new AptosConfig({ network: APTOS_NETWORK });
const aptos = new Aptos(config);
```

:::tip
By default, the Aptos client points to Aptos devnet services. However, it can be configured with the `network` input argument
:::

  </TabItem>
  <TabItem value="python" label="Python">

In the first step, the example initializes both the API and faucet clients.

- The API client interacts with the REST API.
- The faucet client interacts with the devnet Faucet service for creating and funding accounts.

```python
:!: static/sdks/python/examples/simple_aptos_token.py section_1a
```

Using the API client we can create a `TokenClient` that we use for common token operations such as creating collections and tokens, transferring them, claiming them, and so on.

```python
:!: static/sdks/python/examples/simple_aptos_token.py section_1b
```

[`common.py`](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/python/sdk/examples/common.py) initializes these values as follows:

```python
:!: static/sdks/python/examples/common.py section_1
```

:::tip

By default, the URLs for both the services point to Aptos devnet services. However, they can be configured with the following environment variables:
  - `APTOS_NODE_URL`
  - `APTOS_FAUCET_URL`
:::
  </TabItem>
</Tabs>


---

### Step 4.2: Creating local accounts

The next step is to create two accounts locally. [Accounts](../concepts/accounts.md) consist of a public address and the public/private key pair used to authenticate ownership of the account. This step demonstrates how to generate an Account and store its key pair and address in a variable.


<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

```ts
const alice = Account.generate();
const bob = Account.generate();
```
  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/simple_aptos_token.py section_2
```
  </TabItem>
</Tabs>

:::info
Note that this only generates the local keypair. After generating the keypair and public address, the account still does not exist on-chain.
:::

---

### Step 4.3: Creating blockchain accounts

In order to actually instantiate the Account on-chain, it must be explicitly created somehow. On the devnet network, you can request free coins with the Faucet API to use for testing purposes. This example leverages the faucet to fund and inadvertently create Alice and Bob's accounts:

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

```ts
await aptos.fundAccount({
    accountAddress: alice.accountAddress,
    amount: 100_000_000,
  });
await aptos.faucet.fundAccount({
  accountAddress: bob.accountAddress,
  amount: 100_000_000,
});
```
  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/sdks/python/examples/simple_aptos_token.py section_3
```
  </TabItem>
</Tabs>

---

### Step 4.4: Creating a collection

Now begins the process of creating the digital, non-fungible assets. First, as the creator, you must create a collection that groups the assets. A collection can contain zero, one, or many distinct fungible or non-fungible assets within it. The collection is simply a container, intended only to group assets for a creator.

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

Your application will call `createCollectionTransaction` and then `signAndSubmitTransaction` to chain:
```ts
const createCollectionTransaction = await aptos.createCollectionTransaction({
    creator: alice,
    description: collectionDescription,
    name: collectionName,
    uri: collectionURI,
  });

const committedTxn = await aptos.signAndSubmitTransaction({ signer: alice, transaction: createCollectionTransaction });
```

This is the function signature of `createCollectionTransaction`. It returns a `SingleSignerTransaction` that can be simulated or submitted to chain:
```ts
export async function createCollectionTransaction(
  args: {
    creator: Account;
    description: string;
    name: string;
    uri: string;
    options?: InputGenerateTransactionOptions;
  } & CreateCollectionOptions,
): Promise<SingleSignerTransaction>
```
  </TabItem>
  <TabItem value="python" label="Python">

Your application will call `create_collection`:
```python
:!: static/sdks/python/examples/simple_aptos_token.py section_4
```

This is the function signature of `create_collection`. It returns a transaction hash:
```python
:!: static/sdks/python/aptos_sdk/aptos_token_client.py create_collection
```
  </TabItem>
</Tabs>

---

### Step 4.5: Creating a token

To create a token, the creator must specify an associated collection. A token must be associated with a collection, and that collection must have remaining tokens that can be minted. There are many attributes associated with a token, but the helper API exposes only the minimal amount required to create static content.

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

Your application will call `mintTokenTransaction`:
```ts
const mintTokenTransaction = await aptos.mintTokenTransaction({
  creator: alice,
  collection: collectionName,
  description: tokenDescription,
  name: tokenName,
  uri: tokenURI,
});

const committedTxn = await aptos.signAndSubmitTransaction({ signer: alice, transaction: mintTokenTransaction });
```

This is the function signature of `mintTokenTransaction`. It returns a `SingleSignerTransaction` that can be simulated or submitted to chain:
```ts
async mintTokenTransaction(args: {
    creator: Account;
    collection: string;
    description: string;
    name: string;
    uri: string;
    options?: InputGenerateTransactionOptions;
  }): Promise<SingleSignerTransaction>
```
  </TabItem>
  <TabItem value="python" label="Python">

Your application will call `mint_token`:
```python
:!: static/sdks/python/examples/simple_aptos_token.py section_5
```

This is the function signature of `mint_token`. It returns a transaction hash:
```python
:!: static/sdks/python/aptos_sdk/aptos_token_client.py mint_token
```
  </TabItem>
</Tabs>

---

### Step 4.6: Reading token and collection metadata

Both the collection and token assets are [Objects](../standards/aptos-object) on-chain with unique addresses. Their metadata is stored at the object address. The SDKs provide convenience wrappers around querying this data:

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

To read a collection's metadata:
```ts
const alicesCollection = await aptos.getCollectionData({
  creatorAddress: alice.accountAddress,
  collectionName,
  minimumLedgerVersion: BigInt(pendingTxn.version),
});
console.log(`Alice's collection: ${JSON.stringify(alicesCollection, null, 4)}`);
```

To read a owned token's metadata:
```ts
const alicesDigitalAsset = await aptos.getOwnedTokens({
  ownerAddress: alice.accountAddress,
  minimumLedgerVersion: BigInt(pendingTxn.version),
});

console.log(`Alice's digital asset: ${JSON.stringify(alicesDigitalAsset[0], null, 4)}`);
```

  </TabItem>
  <TabItem value="python" label="Python">

To read a collection's metadata:
```python
:!: static/sdks/python/examples/simple_aptos_token.py section_6
```

To read a token's metadata:
```python
:!: static/sdks/python/examples/simple_aptos_token.py get_token_data
```

  </TabItem>
</Tabs>

---

### Step 4.7: Reading an object's owner

Each object created from the `aptos_token.move` contract is a distinct asset. The assets owned by a user are stored separately from the user's account. To check if a user owns an object, check the object's owner:

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

```ts
const alicesDigitalAsset = await aptos.getOwnedTokens({
  ownerAddress: alice.accountAddress,
  minimumLedgerVersion: BigInt(pendingTxn.version),
});

console.log(`Alice's digital asset: ${JSON.stringify(alicesDigitalAsset[0], null, 4)}`);
```

```ts title="Making the query to get the data"

export async function getOwnedTokens(args: {
  aptosConfig: AptosConfig;
  ownerAddress: AccountAddressInput;
  options?: PaginationArgs & OrderByArg<GetTokenActivityResponse[0]>;
}): Promise<GetOwnedTokensResponse> {
  const { aptosConfig, ownerAddress, options } = args;

  const whereCondition: CurrentTokenOwnershipsV2BoolExp = {
    owner_address: { _eq: AccountAddress.from(ownerAddress).toStringLong() },
    amount: { _gt: 0 },
  };

  const graphqlQuery = {
    query: GetCurrentTokenOwnership,
    variables: {
      where_condition: whereCondition,
      offset: options?.offset,
      limit: options?.limit,
      order_by: options?.orderBy,
    },
  };

  const data = await queryIndexer<GetCurrentTokenOwnershipQuery>({
    aptosConfig,
    query: graphqlQuery,
    originMethod: "getOwnedTokens",
  });

  return data.current_token_ownerships_v2;
}
```
  </TabItem>
  <TabItem value="python" label="Python">

```python title="Get the object's resources and parse the owner"
:!: static/sdks/python/examples/simple_aptos_token.py section_7
```

```python title="How the owners dictionary is defined"
:!: static/sdks/python/examples/simple_aptos_token.py owners
```

  </TabItem>
</Tabs>

---

### Step 4.8: Transfer the object back and forth

Each object created from the `aptos_token.move` contract is a distinct asset. The assets owned by a user are stored separately from the user's account. To check if a user owns an object, check the object's owner:

<Tabs groupId="sdk-examples">
  <TabItem value="typescript" label="Typescript">

```ts
const alicesDigitalAsset = await aptos.getOwnedTokens({
  ownerAddress: alice.accountAddress,
  minimumLedgerVersion: BigInt(pendingTxn.version),
});

console.log(`Alice's digital asset: ${JSON.stringify(alicesDigitalAsset[0], null, 4)}`);
```

```ts title="Transfer the token from Alice to Bob"
const transferTransaction = await aptos.transferDigitalAsset({
  sender: alice,
  digitalAssetAddress: alicesDigitalAsset[0].token_data_id,
  recipient: bob.accountAddress,
});
const committedTxn = await aptos.signAndSubmitTransaction({ signer: alice, transaction: transferTransaction });
const pendingTxn = await aptos.waitForTransaction({ transactionHash: committedTxn.hash });
```

```ts title="Print each user's queried token amount"
const alicesDigitalAssetsAfter = await aptos.getOwnedTokens({
  ownerAddress: alice.accountAddress,
  minimumLedgerVersion: BigInt(pendingTxn.version),
});
console.log(`Alices's digital assets balance: ${alicesDigitalAssetsAfter.length}`);

const bobDigitalAssetsAfter = await aptos.getOwnedTokens({
  ownerAddress: bob.accountAddress,
  minimumLedgerVersion: BigInt(pendingTxn.version),
});
console.log(`Bob's digital assets balance: ${bobDigitalAssetsAfter.length}`);
```

  </TabItem>
  <TabItem value="python" label="Python">

```python title="Transfer the token to Bob"
:!: static/sdks/python/examples/simple_aptos_token.py section_8
```

```python title="How the transfer_token function is defined"
:!: static/sdks/python/aptos_sdk/aptos_token_client.py transfer_token
```

```python title="Read the owner"
:!: static/sdks/python/examples/simple_aptos_token.py section_9
```

```python title="Transfer the token back to Alice"
:!: static/sdks/python/examples/simple_aptos_token.py section_10
```

```python title="Read the owner again"
:!: static/sdks/python/examples/simple_aptos_token.py section_11
```


  </TabItem>
</Tabs>

---


## Supporting documentation

* [Account basics](../concepts/accounts.md)
* [TypeScript SDK](../sdks/new-ts-sdk/index.md)
* [Python SDK](../sdks/python-sdk.md)
* [REST API specification](https://aptos.dev/nodes/aptos-api-spec#/)

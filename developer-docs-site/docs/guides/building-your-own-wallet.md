---
title: "Building Your Own Wallet"
slug: "building-your-own-wallet"
---
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Building Your Own Wallet

In order to allow for Aptos' wallet interoperability, the following is required:
1. Mnemonics - a set of words that can derive account private keys
2. dApp API - entry points into the wallet to support access to identity managed by the wallet
3. Key rotation - handling both the relationship around mnemonics and the recovery of accounts in different wallets

## Mnemonics
While [Petra wallet](../guides/install-petra-wallet.md) recommends 1 mnemonic <-> 1 account, we recognize that some wallets may want to support 1 mnemonic <-> n accounts coming from other chains. To support both of these use cases we are using a [BIP44](https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki) derive path for mnemonics to accounts.

### Creating an Aptos Account
1. Generate mnemonic using something like BIP39
2. Get a master seed from that mnemonic using BIP39
3. Use the BIP44 derive path to retrieve an account address (e.g. `m/44'/637'/0'/0'/0'`)
    - See Aptos' [typescript SDK implementation for the derive path](https://github.com/aptos-labs/aptos-core/blob/1bc5fd1f5eeaebd2ef291ac741c0f5d6f75ddaef/ecosystem/typescript/sdk/src/aptos_account.ts#L49-L69))
    - In the case of Petra, we will always use the path `m/44'/637'/0'/0'/0'` since we have 1 mnemonic <-> 1 account


```typescript
/**
   * Creates new account with bip44 path and mnemonics,
   * @param path. (e.g. m/44'/637'/0'/0'/0')
   * Detailed description: {@link https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki}
   * @param mnemonics.
   * @returns AptosAccount
   */
  static fromDerivePath(path: string, mnemonics: string): AptosAccount {
    if (!AptosAccount.isValidPath(path)) {
      throw new Error("Invalid derivation path");
    }

    const normalizeMnemonics = mnemonics
      .trim()
      .split(/\s+/)
      .map((part) => part.toLowerCase())
      .join(" ");

    const { key } = derivePath(path, bytesToHex(bip39.mnemonicToSeedSync(normalizeMnemonics)));

    return new AptosAccount(new Uint8Array(key));
  }
```

### Supporting 1 Mnemonic <-> N Account Wallets
Again, because the 1 mnemonic <-> n accounts paradigm doesn't fit well with key rotation. We don't recommend this approach currently. But to support importing these type of accounts we will follow this standard.

1. Same as above steps 1-3
2. Use the BIP44 derive path to retrieve private keys (e.g. m/44'/637'/i'/0'/0') where i is the account index
3. Now we will iterate i until we get all the accounts the user wants to import
    - We don't want to iterate to infinity to we will be checking if the accounts exist on chain. If an account doesn't exist during iteration we will keep iterating for a constant `address_gap_limit` (10 for now) to see if there are any other accounts. If an account is found we will continue to iterate as normal.


ie.
```typescript
const gapLimit = 10;
let currentGap = 0;

for (let i = 0; currentGap < gapLimit; i += 1) {
    const derivationPath = `m/44'/637'/${i}'/0'/0'`;
    const account = fromDerivePath(derivationPath, mnemonic);
    const response = account.getResources();
    if (response.status !== 404) {
        wallet.addAccount(account);
        currentGap = 0;
    } else {
        currentGap += 1;
    }
}
```

## dApp API
**[Forum post with discussion](https://forum.aptoslabs.com/t/wallet-dapp-api-standards/11765/33)**
There will be some APIs that certain wallets may add but there should be a few apis that are standard across wallets. This will make mass adoption easier and will make dApp developers' lives easier.

- `connect()`, `disconnect()`, and `isConnected()`
- `account()`
- `signAndSubmitTransaction(transaction: EntryFunctionPayload)`
- `signMessage(payload: SignMessagePayload)`
- Event listening (`onAccountChanged(listener)`, `onNetworkChanged(listener)`)

```typescript
// Common Args and Responses

// For single-signer account, there is one publicKey and minKeysRequired is null.
// For multi-signer account, there are multiple publicKeys and minKeysRequired value.
interface PublicAccount {
    address: string;
    publicKey: string | string[];
    minKeysRequired?: number; // for multi-signer account
}

// The important thing to return here is the transaction hash the dApp can wait for it
type [PendingTransaction](https://github.com/aptos-labs/aptos-core/blob/1bc5fd1f5eeaebd2ef291ac741c0f5d6f75ddaef/ecosystem/typescript/sdk/src/generated/models/PendingTransaction.ts)

type [EntryFunctionPayload](https://github.com/aptos-labs/aptos-core/blob/1bc5fd1f5eeaebd2ef291ac741c0f5d6f75ddaef/ecosystem/typescript/sdk/src/generated/models/EntryFunctionPayload.ts)


```

### connect(), disconnect(), isConnected()
It is important that dApps, aren't allow to send requests to the wallet until the user acknowledges that they want to see these requests.

- `connect()` will prompt the user 
    - return `Promise<PublicAccount>`
- `disconnect()` allows the user to stop giving access to a dApp and also helps the dApp with state management
    - return `Promise<void>`
- `isConnected()` able to make requests to the wallet to get current state of connection
    - return `Promise<boolean>`


### account()
**Needs to be connected**
The dApp may want to query for the current connected account to get the address or public key.

- `account()` no prompt to the user
    - returns `Promise<PublicAccount>`

### signAndSubmitTransaction(transaction: EntryFunctionPayload)
We will be generate a transaction from payload(simple JSON) using the [SDK](https://github.com/aptos-labs/aptos-core/blob/1bc5fd1f5eeaebd2ef291ac741c0f5d6f75ddaef/ecosystem/typescript/sdk/src/aptos_client.ts#L217-L221) and then sign and submit it to the wallet's node.

- `signAndSubmitTransaction(transaction: EntryFunctionPayload)` will prompt the user with the transaction they are signing
    - returns `Promise<PendingTransaction>`

### signMessage(payload: SignMessagePayload)
The most common usecase for this function is to verify identity, but there are a few other possible use cases. You may notice some wallets from other chains just provide an interface to sign arbitrary strings. This can be susceptible to man-in-the-middle attacks, signing string transactions, etc.

Types:
```typescript
export interface SignMessagePayload {
  address?: boolean; // Should we include the address of the account in the message
  application?: boolean; // Should we include the domain of the dApp
  chainId?: boolean; // Should we include the current chain id the wallet is connected to
  message: string; // The message to be signed and displayed to the user
  nonce: string; // A nonce the dApp should generate
}

export interface SignMessageResponse {
  address?: string;
  application?: string;
  chainId?: number;
  fullMessage: string; // The message that was generated to sign
  message: string; // The message passed in by the user
  nonce: string,
  prefix: string, // Should always be APTOS
  signature: string | string[]; // The signed full message
  bitmap?: Uint8Array; // a 4-byte (32 bits) bit-vector of length N
}
```

- `signMessage(payload: SignMessagePayload)` prompts the user with the `payload.message` to be signed
    - returns `Promise<SignMessageResponse>`

An example:
`signMessage({nonce: 1234034, message: "Welcome to dApp!", address: true, application: true, chainId: true })`

This would generate the `fullMessage` to be signed and returned as the `signature`:
```yaml
APTOS
address: 0x000001
chain_id: 7
application: badsite.firebase.google.com
nonce: 1234034
message: Welcome to dApp!
```

If the wallet is single-signer account, there is one signature and `bitmap` is null.

If the wallet is multi-signers account, there are multiple `signature` and `bitmap` value. `bitmap` masks which public key has signed message.

### Event listening (In progress)

## Key Rotation (In Progress)

Mapping has been [implemented](https://github.com/aptos-labs/aptos-core/pull/2972) but SDK integration is in progress. This will be updated soon.
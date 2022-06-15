# API Documentation

## Table of Contents

  - [API Documentation](#api-documentation)
    - [class AptosAccount](#class-aptosaccount)
    - [class AptosClient](#class-aptosclient)
    - [class HexString](#class-hexstring)
    - [class FaucetClient](#class-faucetclient)
    - [class TokenClient](#class-tokenclient)

## __class AptosAccount__
Class for creating and managing Aptos account

Methods
### `new AptosAccount(privateKeyBytes?: Uint8Array | undefined, address?: MaybeHexString)`
Creates new account instance
- `privateKeyBytes` - private key from which account key pair will be generated. If not specified, new key pair is going to be created.
- `address` - account address. If not specified, a new one will be generated from public key.
### `address()`
Returns the address associated with the given account. It is the 32-byte of the SHA-3 256 cryptographic hash of the public key(s) concatenated with a signature scheme identifier byte. Example of account key: `0xe8012714cd17606cee7188a2a365eef3fe760be598750678c8c5954eb548a591`
### `authKey()`
Returns the authentication key associated with the given account. This key enables account owners to rotate their private key(s) associated with the account without changing the address that hosts their account. Example of auth key: `0xa38c6887ec9b808dd10a819fba3f92d14a80a86ac2ca644a320c84d5b2b394d2`
### `pubKey()`
Returns the public key associated with the given account. This key is generated with Ed25519 scheme. Public key is used to check a signature of transaction, signed by given account.
### `signBuffer(buffer: Buffer)`
Signs specified `buffer` and returns a `HexString` of signature
### `signHexString(hexString: MaybeHexString)`
Signs specified `HexString` instance and returns signature.
- `hexString` - a `HexString` instance or just regular string to sign
### `toPrivateKeyObject()`
Returns an `AptosAccountObject` that contains account address, public key and private key
```bash
AptosAccountObject {
  address: "0xe8012714cd17606cee7188a2a365eef3fe760be598750678c8c5954eb548a591",
  publicKeyHex: "0xf56d8524faf79fbc0f48c13aeed3b0ce5dd376b4db93b8130a107c0a5e04ba04 ",
  privateKeyHex: "0x009c9f7c992a06cfafe916f125d8adb7a395fca243e264a8e56a4b3e6accf940d2b11e9ece3049ce60e3c7b4a1c58aebfa9298e29a30a58a67f1998646135204"
}
```
### `static fromAptosAccountObject(obj: AptosAccountObject)`
Creates new `AptosAccount` instance from `AptosAccountObject`

Fields
### `readonly signingKey: Nacl.SignKeyPair`
A private key and public key, associated with the given account

## __class AptosClient__
Provides methods for retrieving data from Aptos node. For more detailed API specification see https://fullnode.devnet.aptoslabs.com/spec.html#/

Methods
### `new AptosClient(nodeUrl: string, config?: AptosClientConfig)`
Establishes a connection to Aptos node
### `async getAccount(accountAddress: MaybeHexString)`
Queries an Aptos account by address
- `accountAddress` - address of queried account
### `async getAccountTransactions(accountAddress: MaybeHexString, query?: { start?: number; limit?: number })`
Queries transactions sent by given account
- `accountAddress` - address of account, which transactions will be queried
- `query` - optional pagination object. If limit is not specified, returns 25 transactions by default
### `async getAccountModules(accountAddress: MaybeHexString, query?: { version?: Types.LedgerVersion })`
Queries modules associated with given account
- `accountAddress` - address of account, which modules will be queried
- `query.version` - specifies ledger version of transactions. By default latest version will be used
### `async getAccountModule(accountAddress: MaybeHexString, moduleName: string, query?: { version?: Types.LedgerVersion })`
Queries module associated with given account by module name
- `accountAddress` - address of account, which module will be queried
- `moduleName` - name of a queried module
- `query.version` - specifies ledger version of transactions. By default latest version will be used
### `async getAccountResources(accountAddress: MaybeHexString, query?: { version?: Types.LedgerVersion })`
Queries all resources associated with given account
- `accountAddress` - address of account, which resources will be queried
- `query.version` - specifies ledger version of transactions. By default latest version will be used
### `async getAccountResource(accountAddress: MaybeHexString, resourceType: string, query?: { version?: Types.LedgerVersion})`
Queries resource associated with given account by resource type
- `accountAddress` - address of account, which resource will be queried
- `resourceType` - queried resource type
- `query.version` - specifies ledger version of transactions. By default latest version will be used
### `async generateTransaction(sender: MaybeHexString, payload: Types.TransactionPayload, options?: Partial<Types.UserTransactionRequest>)`
Generates a raw transaction, that can then be signed and submitted to the blockchain
- `sender` - transaction sender address
- `payload` - transaction payload
- `options` - options allow to overwrite default transaction options, which are:
```bash
  {
    sender: senderAddress.hex(),
    sequence_number: account.sequence_number,
    max_gas_amount: "1000",
    gas_unit_price: "1",
    gas_currency_code: "XUS",
    // Unix timestamp, in seconds + 10 seconds
    expiration_timestamp_secs: (Math.floor(Date.now() / 1000) + 10).toString(),
  }
```
### `async createSigningMessage(txnRequest: Types.UserTransactionRequest)`
Converts raw transaction into it's binary hex BCS representation, ready for signing and submitting. Generally you may want to use `signTransaction`, as it takes care of this step + signing
- `txnRequest` - raw transaction to convert
### `async signTransaction(accountFrom: AptosAccount, txnRequest: Types.UserTransactionRequest)`
Signs raw transaction, which then can be submitted to the blockchain
- `accountFrom` - sender's AptosAccount which needs to sign a transaction
- `txnRequest` - raw transaction to sign
### `async getEventsByEventKey(eventKey: Types.HexEncodedBytes)`
Queries events by event key
### `async getEventsByEventHandle(address: MaybeHexString, eventHandleStruct: Types.MoveStructTagId, fieldName: string, query?: { start?: number; limit?: number })`
This API extracts event key from the account resource identified by the `eventHandleStruct` and `fieldName`, then returns events identified by the event key
### `async submitTransaction(signedTxnRequest: Types.SubmitTransactionRequest)`
Submits signed transaction to the blockchain
### `async getTransactions(query?: { start?: number; limit?: number })`
Queries transactions
- `query` - optional pagination object. If limit is not specified, returns 25 transaction by default
### `async getTransaction(txnHashOrVersion: string)`
Queries transaction by hash or version. When given transaction hash, server first looks up on-chain transaction by hash; if no on-chain transaction found, then look up transaction by hash in the mempool (pending) transactions. When given a transaction version, server looks up the transaction on-chain by version.
### `async transactionPending(txnHash: Types.HexEncodedBytes)`
Retuns `true` if specified transaction is currently in pending state
### `async waitForTransaction(txnHash: Types.HexEncodedBytes)`
Waits up to 10 seconds for a transaction to move past pending state
### `async getLedgerInfo(params: RequestParams = {})`
Queries the latest ledger information
### `async getTableItem(handle: string, data: Types.TableItemRequest, params?: RequestParams)`
Gets a table item for a table identified by the handle and the key for the item. Key and value types need to be passed in to help with key serialization and value deserialization.
- `handle` - table handle
- `data` - object with next fields:
```bash
{
  key_type: MoveTypeId, // type of table key
  value_type: MoveTypeId, // type of table value
  key: MoveValue // value of table key
}
```
- `params` - request params

## __class HexString__
A util class for working with hex strings

Methods
### `new HexString(hexString: string | Types.HexEncodedBytes)`
Creates new hex string or wraps regular string
### `hex()`
Returns hex string
### `noPrefix()`
Returns hex string without `0x` prefix
### `toString()`
Returns hex string
### `toShortString()`
Trims extra zeroes in the begining of a string. For example:
```bash
new HexString("0x000000string").toShortString(); // result = "0xstring"
```
### `toBuffer()`
Returns `new Buffer` from inner hex string
### `toUint8Array()`
Returns `new Uint8Array` from inner hex string
### `static fromBuffer(buffer: Buffer)`
Creates new hex string from buffer
### `static fromUint8Array(arr: Uint8Array)`
Creates new hex string from Uint8Array
### `static ensure(hexString: MaybeHexString)`
Checks if `hexString` is instance of `HexString` class. If not, creates new `HexString` and returns it and just returns `hexString` in other case

Fields
### `readonly hexString`
## __class FaucetClient__
Class for working with faucet

Methods
### `async fundAccount(address: MaybeHexString, amount: number)`
This creates an account if it does not exist and mints the specified amount of coins into that account. Returns hashes of submitted transactions
## __class TokenClient__
Class for creating and managing NFT collections and tokens

Methods
### `new TokenClient(aptosClient: AptosClient)`
Creates new TokenClient instance
### `async submitTransactionHelper(account: AptosAccount, payload: Types.TransactionPayload)`
This method brings together methods for generating, signing and submitting transaction. Returns transaction hash
- `account` - account which wants to send a transaction
- `payload` - transaction payload
### `async createCollection( account: AptosAccount, name: string, description: string, uri: string)`
Creates a new NFT collection within the specified account. Returns transaction hash
- `account` - account associated with collection
- `name` - collection name
- `description` - collection description
- `uri` - URL to additional info about collection
### `async createToken(account: AptosAccount, collectionName: string, name: string, description: string, supply: number, uri: string)`
Creates a new NFT within the specified account. Returns transaction hash
- `account` - account associated with token
- `collectionName` - collection name
- `name` - token name
- `description` - token description
- `supply` - token supply
- `uri` - URL to additional info about token
### `async offerToken(account: AptosAccount, receiver: MaybeHexString, creator: MaybeHexString, collectionName: string, name: string, amount: number)`
Transfers specified amount of tokens from account to receiver. Returns transaction hash
- `account` - account from which tokens will be transfered
- `receiver` - account which will receive tokens
- `creator` - account which creted a token
- `collectionName` - collection where token is stored
- `name` - token name
- `amount` - amount of tokens which will be transfered
### `async claimToken(account: AptosAccount, sender: MaybeHexString, creator: MaybeHexString, collectionName: string, name: string)`
Claims a token on specified account. Returns transaction hash
- `account` - account which will claim token
- `sender` - account which holds a token
- `creator` - account which created a token
- `collectionName` - collection where token is stored
- `name` - token name
### `async cancelTokenOffer(account: AptosAccount, receiver: MaybeHexString, creator: MaybeHexString,collectionName: string, name: string)`
Removes token from pending claims list. Returns transaction hash
- `account` - account which pending claims list will be changed
- `receiver` - account which had to claim token
- `creator` - account which created a token
- `collectionName`- collection where token is stored
- `name`- token name
### `async getCollectionData(creator: MaybeHexString, collectionName: string)`
Queries collection data
- `creator` - account which created a collection
- `collectionName` - collection name
### `async getTokenData(creator: MaybeHexString, collectionName: string, tokenName: string)`
Queries token data from collection
- `creator` - account which created a token
- `collectionName` - collection name
- `tokenName` - token name
### `async getTokenBalance(creator: MaybeHexString, collectionName: string, tokenName: string)`
Queries specific token from account's TokenStore
- `creator` - account which created a token
- `collectionName` - collection name
- `tokenName` - token name

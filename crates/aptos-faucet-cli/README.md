# Faucet

Faucet is a service for creating and funding accounts on the Aptos Network. It is meant to be used for devnets and testnets. By default, the Faucet takes the provided account, creates a new account, mints a lot of Coin<AptosCoin> into that account, and delegates minting capability to that account. That account is then used to provide mint services via the faucet.


## Mint API

The Mint API can create and fund your account.

* Base URL: `http://faucet.testnet.aptoslabs.com/`
* Path: `/mint`
* Method: POST

URL Query Params:

| param name             | type   | required? | description                                                 |
|------------------------|--------|-----------|-------------------------------------------------------------|
| `amount`               | int    | Y         | Amount of coins to mint. This is not always enabled.        |
| `pub_key`              | string | Y         | Your account public key (ed25519)                           |
| `return_txns`          | bool   | N         | Returns the transactions for creating / funding the account |

Notes:
* Type bool means you set value to a string "true" or "false"
* For existing accounts as defined by the pub_key, the service submits 1 transfer funds transaction.
* For new accounts as defined by the pub_key, the service first issues a transaction for creating the account and another for transferring funds.
* All funds transferred come from the account 0xa550c18.
* Clients should retry their request if the requests or the transaction execution failed. One reason for failure is that, under load, the service may issue transactions with duplicate sequence numbers. Only one of those transactions will be executed, the rest will fail.

### Response

If the query param `return_txns` is not provided, or it is not "true", the server returns a json-encoded list of transaction hash values. These can be used to monitor the status of submitted transactions.

If the query param `return_txns` is set, the server will respond with the transactions for creating and funding your account.
The response HTTP body is hex encoded bytes of BCS encoded `Vec<aptos_types::transaction::SignedTransaction>`.

Decode Example ([source code generator](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/aptos-sdk-builder)):

``` python
  de = bcs.BcsDeserializer(bytes.fromhex(response.text))
  length = de.deserialize_len()

  txns = []
  for i in range(length):
    txns.push(de.deserialize_any(aptos_types.SignedTransaction))

```

You should retry the mint API call if the transaction execution fails.


## Example

```bash
curl -X POST http://faucet.testnet.aptoslabs.com/mint\?amount\=1000000\&pub_key\=459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d\&return_txns\=true
01000000000000000000000000000000dd05a600000000000001e001a11ceb0b010000000701000202020403061004160205181d0735600895011000000001010000020001000003020301010004010300010501060c0108000506080005030a020a020005060c05030a020a020109000b4469656d4163636f756e741257697468647261774361706162696c6974791b657874726163745f77697468647261775f6361706162696c697479087061795f66726f6d1b726573746f72655f77697468647261775f6361706162696c69747900000000000000000000000000000001010104010c0b0011000c050e050a010a020b030b0438000b051102020107000000000000000000000000000000010358555303585553000403a74fd7c46952c497e75afb0a7932586d0140420f00000000000400040040420f00000000000000000000000000035855532a610f6000000000020020056244e7bf776e471d818dc18fdf7b8833c5439ac9a96e126f8f32c7bc7c14b64026a2c45c8e4066c661dc4f36baa6ad61499999b548b9f63ad15853660c408cedec3078b7773a829ec48de8b04291cd11530734b2f91d5e42f35a4c6378cb7c09
```

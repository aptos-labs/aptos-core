---
title: "Key Rotation"
id: "key-rotation"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

Aptos Move accounts have authentication keys that are separate from their public address. This means you can rotate an account's private key to give control of it to another account without changing the initial account's public address.

After rotating the key, the controlling account can sign transactions for the initial account.

In this guide, we show examples for how to rotate an account's authentication key using the various Aptos SDKs.

If you haven't installed the SDKs, you can do so at the following links:

* [Aptos CLI](../../tools/install-cli/)
* [Typescript SDK](../../sdks/ts-sdk/index)
* [Python SDK](../../sdks/python-sdk/index)
* [Rust SDK](../../sdks/rust-sdk/index)

:::warning
Some of the following examples use private keys. Do not share your private keys with anyone.
:::

## How to rotate an account's authentication key
<Tabs groupId="examples">
  <TabItem value="CLI" label="CLI">

```shell title="Initialize two test profiles on devnet"
echo "devnet" | aptos init --profile rotate-1
echo "devnet" | aptos init --profile rotate-2
```
```shell title="Rotate the authentication key for rotate-1 to rotate-2's authentication key"
aptos account rotate-key --profile rotate-1 --new-private-key <ROTATE_2_PRIVATE_KEY>
```
:::tip Where do I get the private key for a profile?
Public, private, and authentication keys for Aptos CLI profiles are stored in `~/.aptos/config.yaml`.
:::

```shell title="Confirm yes and create a new profile so that you can continue to sign for the resource account"
Do you want to submit a transaction for a range of [52000 - 78000] Octas at a gas unit price of 100 Octas? [yes/no] >
yes
...

Do you want to create a profile for the new key? [yes/no] >
yes
...

Enter the name for the profile
rotate-1-new

Profile rotate-1-new is saved.
```
You can now use the profile like any other account. The private key for `rotate-1-new` will match `rotate-2` in the `~/.aptos/config.yaml` file and the `Authentication Key` on the account will match the `Authentication Key` of `rotate-2` on-chain.

```shell title="Verify the authentication keys are now equal with view functions"
# View the authentication key of `rotate-1-new`
aptos move view --function-id 0x1::account::get_authentication_key --args address:rotate-1-new

# View the authentication key of `rotate-2`, it should equal the above.
aptos move view --function-id 0x1::account::get_authentication_key --args address:rotate-2
```

```json title="Example output from the previous two commands"
{
  "Result": [
    "0x458fba533b84717c91897cab05047c1dd7ac2ea73e75c77281781f5b7fec180c"
  ]
}
{
  "Result": [
    "0x458fba533b84717c91897cab05047c1dd7ac2ea73e75c77281781f5b7fec180c"
  ]
}
```
  </TabItem>

  <TabItem value="typescript" label="Typescript">

View the full example for this code [here](https://github.com/aptos-labs/aptos-core/ecosystem/typescript/sdk/examples/typescript/rotate_key.ts).

This program creates two accounts on devnet, Alice and Bob, funds them, then rotates the Alice's authentication key to that of Bob's.

The function to rotate is very simple:
```typescript title="Typescript SDK rotate authentication key function"
:!: static/sdks/typescript/examples/typescript-esm/rotate_key.ts rotate_key
```
```shell title="Navigate to the typescript SDK directory, install dependencies and run rotate_key.ts"
cd ~/aptos-core/ecosystem/typescript/sdk/examples/typescript-esm
pnpm install && pnpm rotate_key
```
```shell title="rotate_key.ts output"
Account          Address           Auth Key           Private Key     
-------------------------------------------------------------------
Alice            0x8dcc...7dbe    '0x8dcc...7dbe'    '0x1cec...cc88'
Bob              0x36eb...9b6c     0x36eb...9b6c      0x9d7c...0610   
 
...rotating...

Alice            0x8dcc...7dbe    '0x36eb...9b6c'    '0x9d7c...0610'
Bob              0x36eb...9b6c     0x36eb...9b6c      0x9d7c...0610   
```
  </TabItem>
  <TabItem value="python" label="Python">

```python title="Python SDK rotate authentication key function"
:!: static/sdks/python/examples/rotate-key.py rotate_key
```
```shell title="Navigate to the python SDK directory, install dependencies and run rotate_key.ts"
cd ~/aptos-core/ecosystem/python/sdk
poetry install && poetry run python -m examples.rotate-key
```
```shell title="rotate_key.ts output"
Account            Address             Auth Key             Private Key        
------------------------------------------------------------------------
Alice              0x3e58...2b6243    '0x3e58...2b6243'    '0xb927...f4264d'    
Bob                0xe39d...adf91f     0xe39d...adf91f      0xb2cd...a5c415    

...rotating...

Alice              0x3e58...2b6243    '0xe39d...adf91f'    '0xb2cd...a5c415'    
Bob                0xe39d...adf91f     0xe39d...adf91f      0xb2cd...a5c415  
```

  </TabItem>

  <TabItem value="rust" label="Rust">

    Coming soon.

  </TabItem>
</Tabs>
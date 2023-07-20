---
title: "Rotating an authentication key"
id: "key-rotation"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

Aptos Move accounts have a public address, an authentication key, a public key, and a private key. The public address is permanent, always matching the account's initial authentication key.

The Aptos account model facilitates the unique ability to rotate an account's private key. Since an account's address is the *initial* authentication key, the ability to sign for an account can be transferred to another private key without changing its public address.

In this guide, we show examples for how to rotate an account's authentication key using a few of the various Aptos SDKs.

Here are the installation links for the SDKs we will cover in this example:

* [Aptos CLI](../../tools/aptos-cli)
* [Typescript SDK](../../sdks/ts-sdk/index)
* [Python SDK](../../sdks/python-sdk)

:::warning
Some of the following examples use private keys. Do not share your private keys with anyone.
:::

## How to rotate an account's authentication key
<Tabs groupId="examples">
  <TabItem value="CLI" label="CLI">

Run the following to initialize two test profiles. Leave the inputs blank both times you're prompted for a private key.

```shell title="Initialize two test profiles on devnet"
aptos init --profile test_profile_1 --network devnet --assume-yes
aptos init --profile test_profile_2 --network devnet --assume-yes
```
```shell title="Rotate the authentication key for test_profile_1 to test_profile_2's authentication key"
aptos account rotate-key --profile test_profile_1 --new-private-key <TEST_PROFILE_2_PRIVATE_KEY>
```
:::info Where do I view the private key for a profile?
Public, private, and authentication keys for Aptos CLI profiles are stored in `~/.aptos/config.yaml` if your config is set to `Global` and `<local_directory>/.aptos/config.yaml` if it's set to `Workspace`.

To see your config settings, run `aptos config show-global-config`.
:::

```shell title="Confirm yes and create a new profile so that you can continue to sign for the resource account"
Do you want to submit a transaction for a range of [52000 - 78000] Octas at a gas unit price of 100 Octas? [yes/no] >
yes
...

Do you want to create a profile for the new key? [yes/no] >
yes
...

Enter the name for the profile
test_profile_1_rotated

Profile test_profile_1_rotated is saved.
```
You can now use the profile like any other account.

In your `config.yaml` file, `test_profile_1_rotated` will retain its original public address but have a new public and private key that matches `test_profile_2`.

The authentication keys aren't shown in the `config.yaml` file, but we can verify the change with the following commands:

```shell title="Verify the authentication keys are now equal with view functions"
# View the authentication key of `test_profile_1_rotated`
aptos move view --function-id 0x1::account::get_authentication_key --args address:test_profile_1_rotated

# View the authentication key of `test_profile_2`, it should equal the above.
aptos move view --function-id 0x1::account::get_authentication_key --args address:test_profile_2
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

This program creates two accounts on devnet, Alice and Bob, funds them, then rotates the Alice's authentication key to that of Bob's.

View the full example for this code [here](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/typescript/sdk/examples/typescript/rotate_key.ts).

The function to rotate is very simple:
```typescript title="Typescript SDK rotate authentication key function"
:!: static/sdks/typescript/examples/typescript-esm/rotate_key.ts rotate_key
```
Commands to run the example script:
```shell title="Navigate to the typescript SDK directory, install dependencies and run rotate_key.ts"
cd ~/aptos-core/ecosystem/typescript/sdk/examples/typescript-esm
pnpm install && pnpm rotate_key
```
```shell title="rotate_key.ts output"
Account            Address             Auth Key             Private Key          Public Key         
------------------------------------------------------------------------------------------------
Alice              0x213d...031013    '0x213d...031013'    '0x00a4...b2887b'    '0x859e...08d2a9'
Bob                0x1c06...ac3bb3     0x1c06...ac3bb3      0xf2be...9486aa      0xbbc1...abb808    

...rotating...

Alice              0x213d...031013    '0x1c06...ac3bb3'    '0xf2be...9486aa'    '0xbbc1...abb808'
Bob                0x1c06...ac3bb3     0x1c06...ac3bb3      0xf2be...9486aa      0xbbc1...abb808 
```
  </TabItem>
  <TabItem value="python" label="Python">

This program creates two accounts on devnet, Alice and Bob, funds them, then rotates the Alice's authentication key to that of Bob's.

View the full example for this code [here](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/python/sdk/examples/rotate-key.py).

Here's the relevant code that rotates Alice's keys to Bob's:
```python title="Python SDK rotate authentication key function"
:!: static/sdks/python/examples/rotate-key.py rotate_key
```
Commands to run the example script:
```shell title="Navigate to the python SDK directory, install dependencies and run rotate_key.ts"
cd ~/aptos-core/ecosystem/python/sdk
poetry install && poetry run python -m examples.rotate-key
```
```shell title="rotate_key.py output"
Account            Address             Auth Key             Private Key          Public Key         
------------------------------------------------------------------------------------------------
Alice              0x213d...031013    '0x213d...031013'    '0x00a4...b2887b'    '0x859e...08d2a9'
Bob                0x1c06...ac3bb3     0x1c06...ac3bb3      0xf2be...9486aa      0xbbc1...abb808    

...rotating...

Alice              0x213d...031013    '0x1c06...ac3bb3'    '0xf2be...9486aa'    '0xbbc1...abb808'
Bob                0x1c06...ac3bb3     0x1c06...ac3bb3      0xf2be...9486aa      0xbbc1...abb808 
```

  </TabItem>
</Tabs>
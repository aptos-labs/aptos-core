---
title: "Your First Multisig"
slug: "your-first-multisig"
---

# Your First Multisig

This tutorial introduces assorted [K-of-N multi-signer authentication](../concepts/accounts.md#multi-signer-authentication) operations and supplements content from the following tutorials:

* [Your First Transaction](./first-transaction.md)
* [Your First Coin](./first-coin.md)
* [Your First Move Module](./first-move-module.md)

:::tip
Try out the above tutorials (which include dependency installations) before moving on to multisig operations.
:::

## Step 1: Pick an SDK

This tutorial, a community contribution, was created for the [Python SDK](../sdks/python-sdk.md).

Other developers are invited to add support for the [TypeScript SDK](../sdks/ts-sdk/index.md), [Rust SDK](../sdks/rust-sdk.md), and [Unity SDK](../sdks/unity-sdk.md)!

## Step 2: Start the example

Navigate to the Python SDK directory:

```zsh
cd <aptos-core-parent-directory>/aptos-core/ecosystem/python/sdk/
```

Run the `multisig.py` example:

```zsh
poetry run python -m examples.multisig
```

:::tip
This example uses the Aptos devnet, which has historically been reset each Thursday.
Make sure devnet is live when you try running the example!
:::

## Step 3: Generate single signer accounts

First, we will generate single signer accounts for Alice, Bob, and Chad:

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_1
```

Fresh accounts are generated for each example run, but the output should resemble:

```zsh title=Output
=== Account addresses ===
Alice: 0x93c1b7298d53dd0d517f503f2d3188fc62f6812ab94a412a955720c976fecf96
Bob:   0x85eb913e58d0885f6a966d98c76e4d00714cf6627f8db5903e1cd38cc86d1ce0
Chad:  0x14cf8dc376878ac268f2efc7ba65a2ee0ac13ceb43338b6106dd88d8d23e087a

=== Authentication keys ===
Alice: 0x93c1b7298d53dd0d517f503f2d3188fc62f6812ab94a412a955720c976fecf96
Bob:   0x85eb913e58d0885f6a966d98c76e4d00714cf6627f8db5903e1cd38cc86d1ce0
Chad:  0x14cf8dc376878ac268f2efc7ba65a2ee0ac13ceb43338b6106dd88d8d23e087a

=== Public keys ===
Alice: 0x3f23f869632aaa4378f3d68560e08d18b1fc2e80f91d6f9d1b39d720b0ef7a3f
Bob:   0xcf21e85337a313bdac33d068960a3e52d22ce0e6190e9acc03a1c9930e1eaf3e
Chad:  0xa1a2aef8525eb20655387d3ed50b9a3ea1531ef6117f579d0da4bcf5a2e1f76d
```

For each user, note the [account address](../concepts/accounts.md#account-address) and [authentication key](../concepts/accounts.md#single-signer-authentication) are identical, but the [public key](../concepts/accounts.md#creating-an-account) is different.

## Step 4: Generate a multisig account

Next generate a [K-of-N multi-signer](../concepts/accounts.md#multi-signer-authentication) public key and account address for a multisig account requiring two of the three signatures:

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_2
```

The multisig account address depends on the public keys of the single signers. (Hence, it will be different for each example.) But the output should resemble:

```zsh title=Output
=== 2-of-3 Multisig account ===
Account public key: 2-of-3 Multi-Ed25519 public key
Account address:    0x08cac3b7b7ce4fbc5b18bc039279d7854e4c898cbf82518ac2650b565ad4d364
```

## Step 5: Fund all accounts

Next fund all accounts:

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_3
```

```zsh title=Output
=== Funding accounts ===
Alice's balance:  10000000
Bob's balance:    20000000
Chad's balance:   30000000
Multisig balance: 40000000
```

## Step 6: Send coins from the multisig

This transaction will send 100 octas from the multisig account to Chad's account.
Since it is a two-of-three multisig account, signatures are required from only two individual signers.

### Step 6.1: Gather individual signatures

First generate a raw transaction, signed by Alice and Bob, but not by Chad.

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_4
```

Again, signatures vary for each example run:

```zsh title=Output
=== Individual signatures ===
Alice: 0x41b9dd65857df2d8d8fba251336357456cc9f17974de93292c13226f560102eac1e70ddc7809a98cd54ddee9b79853e8bf7d18cfef23458f23e3a335c3189e0d
Bob:   0x6305101f8f3ad5a75254a8fa74b0d9866756abbf359f9e4f2b54247917caf8c52798a36c5a81c77505ebc1dc9b80f2643e8fcc056bcc4f795e80b229fa41e509
```

### Step 6.2: Submit the multisig transaction

Next generate a [multisig authenticator](../guides/sign-a-transaction.md#multisignature-transactions) and submit the transaction:


```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_5
```

```zsh title=Output
=== Submitting transfer transaction ===
Transaction hash: 0x3ff2a848bf6145e6df3abb3ccb8b94fefd07ac16b4acb0c694fa7fa30b771f8c
```

### Step 6.3: Check balances

Check the new account balances:

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_6
```

```zsh title=Output
=== New account balances===
Alice's balance:  10000000
Bob's balance:    20000000
Chad's balance:   30000100
Multisig balance: 39999300
```

Note that even though Alice and Bob signed the transaction, their account balances have not changed.
Chad, however, has received 100 octas from the multisig account, which assumed the gas costs of the transaction and thus has had more than 100 octas deducted.

## Step 7: Create a vanity address multisig

In this section, a fourth user named Deedee will generate a vanity address, then rotate her account to the two-of-three multisig.

### Step 7.1 Generate a vanity address

A fourth user, Deedee, wants her account address to start with `0xdd..`, so she generates random accounts until she finds one with a matching account address:

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_7
```

```zsh title=Output
=== Funding vanity address ===
Deedee's address:    0xdd86860ae7f77f58d08188e1c39fbc6a2f7cec782f09f6767f8367d84357ed57
Deedee's public key: 0xdbf02311c45903f0217e9ab76ca64007c2876363118bb422922410d4cfe9964c
Deedee's balance:    50000000
```

### Step 7.2 Sign a rotation proof challenge

Deedee and the two-of-three multisig must both sign a `RotationProofChallenge`, yielding two signatures.
Deedee's signature, `cap_rotate_key`, verifies that she approves of the authentication key rotation.
The multisig signature, `cap_update_table`, verifies that the multisig approves of the authentication key rotation.
Here, Bob and Chad provide individual signatures for the multisig:

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_8
```

```zsh title=Output
=== Signing rotation proof challenge ===
cap_rotate_key:   0x3b2906df78bb79f210051e910985c358572c2ec7cdd03f688480fb6adf8d538df48a52787d5651d85f2959dcca88d58da49709c9c0dc9c3c58b67169ec1e1c01
cap_update_table: 0x965fd11d7afe14396e5af40b8ffb78e6eb6f7caa1f1b1d8c7b819fdd6045864e70258788ec1670a3989c85f8cc604f4b54e43e1ce173a59aa0a6f7cf124fd902dcbb2ad53467d05c144260b2be1814777c082d8db437698b00e6a2109a015a565ff5783e827a21a4c07ae332b56398b69dfdbcc08b8ad5585dc1ac649b74730760000000
```

### Step 7.3 Rotate the authentication key

Now that the relevant signatures have been gathered, the authentication key rotation transaction can be submitted.
After it executes, the rotated authentication key matches the address of the first multisig account (the one that sent octas to Chad):

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_9
```

```zsh title=Output
=== Submitting authentication key rotation transaction ===
Auth key pre-rotation: 0xdd86860ae7f77f58d08188e1c39fbc6a2f7cec782f09f6767f8367d84357ed57
Transaction hash:      0x57c66089a1b81e2895a2d6919ab19eb90c4d3c3cbe9fecab8169eaeedff2c6e6
New auth key:          0x08cac3b7b7ce4fbc5b18bc039279d7854e4c898cbf82518ac2650b565ad4d364
1st multisig address:  0x08cac3b7b7ce4fbc5b18bc039279d7854e4c898cbf82518ac2650b565ad4d364
```

In other words, Deedee generated an account with a vanity address so that Alice, Bob, and Chad could use it as a multisig account.
Then Deedee and the Alice/Bob/Chad group (under the authority of Bob and Chad) approved to rotate the vanity account's authentication key to the authentication key of the first multisig account.

## Step 8: Perform Move package governance

In this section, the multisig vanity account will publish a simple package, upgrade it, then invoke a Move script.

Move source code for this section is found in the [`upgrade_and_govern`](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/upgrade_and_govern) directory.

### Step 8.1: Review genesis package

The `UpgradeAndGovern` genesis package (version `1.0.0`) contains a simple `.toml` manifest and a single Move source file:

```toml title="Move.toml"
:!: static/move-examples/upgrade_and_govern/genesis/Move.toml manifest
```

```rust title="parameters.move"
:!: static/move-examples/upgrade_and_govern/genesis/sources/parameters.move module
```

As soon as the package is published, a `GovernanceParameters` resource is moved to the `upgrade_and_govern` package account with the values specified by `GENESIS_PARAMETER_1` and `GENESIS_PARAMETER_2`.
Then, the `get_parameters()` function can be used to look up the governance parameters, but note that in this version there is no setter function.
The setter function will be added later.

### Step 8.2: Publish genesis package

Here, Alice and Chad will sign off on the publication transaction.

All compilation and publication operations are handled via the ongoing Python script:

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_10
```

```zsh title=Output
=== Genesis publication ===
Running aptos CLI command: aptos move compile --save-metadata --package-dir ../../../../aptos-move/move-examples/upgrade_and_govern/genesis --named-addresses upgrade_and_govern=0xdd86860ae7f77f58d08188e1c39fbc6a2f7cec782f09f6767f8367d84357ed57

Compiling, may take a little while to download git dependencies...
INCLUDING DEPENDENCY AptosFramework
INCLUDING DEPENDENCY AptosStdlib
INCLUDING DEPENDENCY MoveStdlib
BUILDING UpgradeAndGovern

Transaction hash: 0x3c65c681194d6c64d73dc5d0cbcbad62e99a997b8600b8edad6847285e580c13
Package name from on-chain registry: UpgradeAndGovern
On-chain upgrade number: 0
```

### Step 8.3: Review package upgrades

The `UpgradeAndGovern` upgrade package adds the following parameter setter functionality at the end of `parameters.move`:

```rust title=parameters.move
:!: static/move-examples/upgrade_and_govern/upgrade/sources/parameters.move appended
```

Here, the account that the package is published under, `upgrade_and_govern`, has the authority to change the `GovernanceParameter` values from the genesis values to the new `parameter_1` and `parameter_2` values.

There is also a new module, `transfer.move`:

```rust title=transfer.move
:!: static/move-examples/upgrade_and_govern/upgrade/sources/transfer.move module
```

This module simply looks up the `GovernanceParameter` values, and treats them as the amount of octas to send to two recipients.

Lastly, the manifest has been updated with the new version number `1.1.0`:

```toml title=Move.toml
:!: static/move-examples/upgrade_and_govern/upgrade/Move.toml manifest
```

### Step 8.4: Upgrade the package

Alice, Bob, and Chad will all sign off on this publication transaction, which results in an upgrade.
This process is almost identical to that of the genesis publication, with the new `transfer` module listed after the `parameters` module:

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_11
```

:::tip
Modules that `use` other modules must be listed *after* the modules they use.
:::

```zsh title=Output
=== Upgrade publication ===
Running aptos CLI command: aptos move compile --save-metadata --package-dir ../../../../aptos-move/move-examples/upgrade_and_govern/upgrade --named-addresses upgrade_and_govern=0xdd86860ae7f77f58d08188e1c39fbc6a2f7cec782f09f6767f8367d84357ed57

Compiling, may take a little while to download git dependencies...
INCLUDING DEPENDENCY AptosFramework
INCLUDING DEPENDENCY AptosStdlib
INCLUDING DEPENDENCY MoveStdlib
BUILDING UpgradeAndGovern

Transaction hash: 0x0f0ea3bb7271ddeaceac5b49ff5503d6c652d4746c1510e47665ceee5a89aaf0
On-chain upgrade number: 1
```

Note that the on-chain upgrade number has been incremented by 1.

### Step 8.6: Review the governance script

The `UpgradeAndGovern` upgrade package also includes a Move script at `set_and_transfer.move`:

```rust title=set_and_transfer.move
:!: static/move-examples/upgrade_and_govern/upgrade/scripts/set_and_transfer.move script
```

This script calls the governance parameter setter using hard-coded values defined at the top of the script, then calls the octa transfer function.
The script accepts as arguments the signature of the account hosting the package, as well as two target addresses for the transfer operation.

Note that both functions in the script are `public entry fun` functions, which means that everything achieved in the script could be performed without a script.
However, a non-script approach would require two transactions instead of just one, and would complicate the signature aggregation process:
in practical terms, Alice, Bob, and/or Chad would likely have to send single-signer transaction signatures around through off-chain communication channels, and a *scribe* for the group would then have to submit a multisig `Authenticator` (for *each* `public entry fun` call).
Hence in a non-script approach, extra operational complexity can quickly introduce opportunities for consensus failure.

A Move script, by contrast, collapses multiple governance function calls into a single transaction; and moreover, Move scripts can be published in a public forum like GitHub so that all signatories can review the actual function calls before they sign the script.

### Step 8.5: Execute the governance script

Alice and Bob sign off on the Move script, which sends coins from the vanity multisig account to their personal accounts.
Here, the amounts sent to each account are specified in the hard-coded values from the script.

```python title="multisig.py snippet"
:!: static/sdks/python/examples/multisig.py section_12
```

```zsh title=Output
=== Invoking Move script ===
Transaction hash: 0xd06de4bd9fb12a9f3cbd8ce1b9a9fd47ea2b923a8cfac21f9788869430e4149b
Alice's balance:  10000300
Bob's balance:    20000200
Chad's balance:   30000100
```

## Step 9: Expedite execution with AMEE

The above code snippets demonstrate concepts relevant to multisig operations, but are impractical for realistic workflows:
all of the private keys are stored in memory on the same machine, the function calls do not generalize to other multisig operations, etc.
As a result, there is a significant amount of overhead required to implement a bespoke solution that ports the above concepts to one's particular use case, which almost necessarily involves signers coordinating across space and time through an off-chain social consensus strategy (e.g. Have enough signatories signed yet? How do we compile their signatures?)

To expedite this process, the Python SDK thus provides the Aptos Multisig Execution Expeditor (AMEE), a command-line tool that facilitates general multisig workflows through straightforward data structures and function calls.

To use AMEE, navigate to the Python SDK directory:

```zsh
cd <aptos-core-parent-directory>/aptos-core/ecosystem/python/sdk
```

Then call up the help menu from the command line:

```python title=Command
:!: static/sdks/python/examples/multisig.sh help
```

<details>
<summary>Output</summary>

```zsh
usage: amee.py [-h] {keyfile,k,metafile,m,publish,p,rotate,r,script,s} ...

Aptos Multisig Execution Expeditor (AMEE): A collection of tools designed to expedite multisig account execution.

positional arguments:
  {keyfile,k,metafile,m,publish,p,rotate,r,script,s}
    keyfile (k)         Single-signer keyfile operations.
    metafile (m)        Multisig metafile operations.
    publish (p)         Move package publication.
    rotate (r)          Authentication key rotation operations.
    script (s)          Move script invocation.

options:
  -h, --help            show this help message and exit
```

</details>

AMEE offers a rich collection of useful subcommands, and to access their all of their help menus recursively, simply call the `multisig.sh` shell script file with the `menus` argument (still from inside the `sdk` directory):

```zsh title=Command
sh examples/multisig.sh menus
```

:::tip
This shell script file will be used for several other examples throughout the remainder of this tutorial, so try running it now!
:::

<details>
<summary>Output</summary>

```zsh

<Top-level help menu>

...

usage: amee.py keyfile [-h] {change-password,c,extract,e,fund,f,generate,g,verify,v} ...

Assorted single-signer keyfile operations.

positional arguments:
  {change-password,c,extract,e,fund,f,generate,g,verify,v}
    change-password (c)
                        Change keyfile password.
    extract (e)         Extract Aptos account store from keyfile.
    fund (f)            Fund on devnet faucet.
    generate (g)        Generate new keyfile.
    verify (v)          Verify keyfile password.

options:
  -h, --help            show this help message and exit





usage: amee.py keyfile change-password [-h] [-u] keyfile

Change password for a single-singer keyfile.

positional arguments:
  keyfile               Relative path to keyfile.

options:
  -h, --help            show this help message and exit
  -u, --use-test-password
                        Flag to use test password.

...

<More menus>

```

</details>

### Step 9.1 Keyfiles

Unlike the `aptos` CLI which stores private keys in plain text on disk, AMEE encrypts single-signer private keys in a [JSON](https://docs.python.org/3/library/json.html) keyfile format with password protection:

```zsh title=Command
:!: static/sdks/python/examples/multisig.sh generate_keyfile
```

This initiates a hidden password prompt and creates a new file on disk:

<details>
<summary>Output</summary>

```zsh
Enter new password for encrypting private key:
Re-enter password:
Keyfile now at the_aptos_foundation.keyfile:
{
    "filetype": "Keyfile",
    "signatory": "The Aptos Foundation",
    "public_key": "0x8b10b1b680e0e8734e58fe8466625fd7edff62a2cbbb9d83ddee2a593360b922",
    "authentication_key": "0x0e8b6be1755cd65e50ebd3300a18287ab2e96af1fbd4298f49b66ee2a8c5ac06",
    "encrypted_private_key": "0x674141414141426b474f6238743031312d53574874454e397370497a752d6f7470363675434647414f4c6669566e49597739426449676d655f7941732d79637a34332d5a526a4f754e3478547253434c595f45574245457a76466d62584a515143783762374a34726f374c62735f307861326d45734541637254462d714f38336d72656f7051484168713478",
    "salt": "0x3a66e3df50e32c9b43115c016afbef70"
}
```

</details>

This keyfile can be decrypted using the password to produce an unprotected account store (via `aptos_sdk.account.Account.store()`), which may be useful when trying to fund via the testnet faucet. Note here the abbreviation of `keyfile` to `k`:

```zsh title=Command
:!: static/sdks/python/examples/multisig.sh extract_keyfile
```

<details>
<summary>Output</summary>

```zsh
Enter password for encrypted private key:
New account store at the_aptos_foundation.account_store:
{"account_address": "0x0e8b6be1755cd65e50ebd3300a18287ab2e96af1fbd4298f49b66ee2a8c5ac06", "private_key": "0x55fde61334143fa6dcbbab9ce1367bc82fb46d75c32be8989a31e548594c50ce"}
```

</details>

Similarly, AMEE can generate keyfiles from an unprotected account store format. Note here the abbreviation of `generate` to `g` and the optional `outfile` positional argument:

:::tip
AMEE supports abbreviations for all commands and subcommands!
:::

```zsh title=Command
:!: static/sdks/python/examples/multisig.sh generate_from_store
```

<details>
<summary>Output</summary>

```zsh
Enter new password for encrypting private key:
Re-enter password:
Keyfile now at from_store.keyfile:
{
    "filetype": "Keyfile",
    "signatory": "The Aptos Foundation",
    "public_key": "0x8b10b1b680e0e8734e58fe8466625fd7edff62a2cbbb9d83ddee2a593360b922",
    "authentication_key": "0x0e8b6be1755cd65e50ebd3300a18287ab2e96af1fbd4298f49b66ee2a8c5ac06",
    "encrypted_private_key": "0x674141414141426b474f636a3275633358474e5f333676346978584945634a3661793647706c354472716e7574465f63435f69384b67504e6a4f74724c3238356f416b5732526d3438546c634742416c796e64554767556a62656d4a6d6479656d35484d33555f696867476b6873656b587458566230377173436c436535344c474145667045446743415053",
    "salt": "0x601841caf964c95f730d90751f9550a9"
}
```

</details>

To change the password on a keyfile:


```zsh title=Command
:!: static/sdks/python/examples/multisig.sh change_password
```

<details>
<summary>Output</summary>

```zsh
Enter password for encrypted private key:
Enter new password for encrypting private key:
Re-enter password:
Keyfile now at from_store.keyfile:
{
    "filetype": "Keyfile",
    "signatory": "The Aptos Foundation",
    "public_key": "0x8b10b1b680e0e8734e58fe8466625fd7edff62a2cbbb9d83ddee2a593360b922",
    "authentication_key": "0x0e8b6be1755cd65e50ebd3300a18287ab2e96af1fbd4298f49b66ee2a8c5ac06",
    "encrypted_private_key": "0x674141414141426b474f632d3842546c4b75476d543853486a4355465333744871365f4964383570446d4b523276703563397867305734765a496579736e5739363835413068316857795f744d4c6e545248475a6b4c69535a4d435f7145616b5576504a6c4847616172676f7477734d5231614637564c39623345493439774d39755a67363955743675746a",
    "salt": "0xe8ef8643f5bc714f7ade7bd6798e04a4"
}
```

</details>

Now verify the new password:

```zsh title=Command
:!: static/sdks/python/examples/multisig.sh verify_password
```

<details>
<summary>Output</summary>

```zsh
Enter password for encrypted private key:
Keyfile password verified for The Aptos Foundation
Public key:         0x8b10b1b680e0e8734e58fe8466625fd7edff62a2cbbb9d83ddee2a593360b922
Authentication key: 0x0e8b6be1755cd65e50ebd3300a18287ab2e96af1fbd4298f49b66ee2a8c5ac06
```

</details>

Note that all of these commands can be run in a scripted fashion simply by calling the `multisig.sh` shell script with the `keyfiles` argument.

```zsh title=Command
sh examples/multisig.sh keyfiles
```

:::tip
Try running the shell script yourself, then experiment with variations on the commands!
:::

### Step 9.2 Metafiles

AMEE manages multisig account metadata through metafiles, which assimilate information from multiple single-signer keyfiles.

The below demo script, also in `multisig.sh`, demonstrates assorted metafile operations:

| Command                | Use                                                              |
|------------------------|------------------------------------------------------------------|
| `metafile incorporate` | Incorporate multiple signers into a multisig metafile            |
| `metafile threshold`   | Modify the threshold, outputting a new metafile                  |
| `metafile append`      | Append a new signatory or signatories, outputting a new metafile |
| `metafile remove`      | Remove a signatory or signatories, outputting a new metafile     |

```zsh title=Command
sh examples/multisig.sh metafiles
```

The first part of the demo generates a vanity account for both Ace and Bee, via the `--vanity-prefix` argument, which mines an account having a matching authentication key prefix. Note also the use of the `--use-test-password` command to reduce password inputs for the demo process:

```zsh title="multisig.sh snippet"
:!: static/sdks/python/examples/multisig.sh metafiles_ace_bee
```

Here, each keyfile's authentication key begins with the specified vanity prefix:

<details>
<summary>Output</summary>

```zsh
=== Generate vanity account for Ace ===


Mining vanity address...
Using test password.
Keyfile now at ace.keyfile:
{
    "filetype": "Keyfile",
    "signatory": "Ace",
    "public_key": "0x7fdb582d1d36ebddb0f6bd3f34abda788f8dc75beb5028f2ccf00044d970540d",
    "authentication_key": "0xace4edf64e9db130f9bf3b38fdaa9e3a4d0a63f748ef87552f77e6dd860465c7",
    "encrypted_private_key": "0x674141414141426b474f6539334d356c6444694475725645764a71354f44412d6746685450416f4472634d386d6156725547664d333269495379616773496635675274794664524335627546494f4e637567727858594974614d4e4c4e65724f617052472d45576f5374797a7144516a7542675467474541554f45794d6b75756631656156376b4e5349572d",
    "salt": "0x01dbce2e058247bfd1cb6f5b2a24074b"
}


=== Generate vanity account for Bee ===


Mining vanity address...
Using test password.
Keyfile now at bee.keyfile:
{
    "filetype": "Keyfile",
    "signatory": "Bee",
    "public_key": "0x11434bffb4366cd39d49a04998d7d6cfb9d821c72ed78ead1134d4709f054278",
    "authentication_key": "0xbee4336547dc92c70b02841426fe9ec59ad9de3e5866686b9e9967151ae1d2c8",
    "encrypted_private_key": "0x674141414141426b474f652d31743633747265797975364e45704b476771665562386d694639584e7764576b316f4e596b4343785f395634636a55782d3950684d4354394b61323156457038674e612d714d5578594566746c36797a48584a5259467843556944513679674c4176746168747a71314b7553336f53784476304332424f72345a4b545652705a",
    "salt": "0xd502e9251c85d469616b30d77c6dc557"
}

```

</details>

Next, Ace and Bee are incorporated into a 1-of-2 multisig via `metafile incorporate`:

```zsh title="multisig.sh snippet"
:!: static/sdks/python/examples/multisig.sh metafiles_incorporate
```

<details>
<summary>Output</summary>

```zsh
=== Incorporate Ace and Bee into 1-of-2 multisig ===


Multisig metafile now at ace_and_bee.multisig:
{
    "filetype": "Multisig metafile",
    "multisig_name": "Ace and Bee",
    "address": null,
    "threshold": 1,
    "n_signatories": 2,
    "public_key": "0x7fdb582d1d36ebddb0f6bd3f34abda788f8dc75beb5028f2ccf00044d970540d11434bffb4366cd39d49a04998d7d6cfb9d821c72ed78ead1134d4709f05427801",
    "authentication_key": "0xe7dc07203970203cdb2ebd3db3dc520c73281fc70a396c1e7b594addc0828840",
    "signatories": [
        {
            "signatory": "Ace",
            "public_key": "0x7fdb582d1d36ebddb0f6bd3f34abda788f8dc75beb5028f2ccf00044d970540d",
            "authentication_key": "0xace4edf64e9db130f9bf3b38fdaa9e3a4d0a63f748ef87552f77e6dd860465c7"
        },
        {
            "signatory": "Bee",
            "public_key": "0x11434bffb4366cd39d49a04998d7d6cfb9d821c72ed78ead1134d4709f054278",
            "authentication_key": "0xbee4336547dc92c70b02841426fe9ec59ad9de3e5866686b9e9967151ae1d2c8"
        }
    ]
}
```

</details>

The `metafile threshold` command is used to increase the threshold to two signatures:

```zsh title="multisig.sh snippet"
:!: static/sdks/python/examples/multisig.sh metafiles_threshold
```

<details>
<summary>Output</summary>

```zsh
=== Increase threshold to two signatures ===


Multisig metafile now at ace_and_bee_increased.multisig:
{
    "filetype": "Multisig metafile",
    "multisig_name": "Ace and Bee increased",
    "address": null,
    "threshold": 2,
    "n_signatories": 2,
    "public_key": "0x7fdb582d1d36ebddb0f6bd3f34abda788f8dc75beb5028f2ccf00044d970540d11434bffb4366cd39d49a04998d7d6cfb9d821c72ed78ead1134d4709f05427802",
    "authentication_key": "0xd56a906e88170c2566236467c7dfd34341efd1f436300e0b77f2cface20d65f3",
    "signatories": [
        {
            "signatory": "Ace",
            "public_key": "0x7fdb582d1d36ebddb0f6bd3f34abda788f8dc75beb5028f2ccf00044d970540d",
            "authentication_key": "0xace4edf64e9db130f9bf3b38fdaa9e3a4d0a63f748ef87552f77e6dd860465c7"
        },
        {
            "signatory": "Bee",
            "public_key": "0x11434bffb4366cd39d49a04998d7d6cfb9d821c72ed78ead1134d4709f054278",
            "authentication_key": "0xbee4336547dc92c70b02841426fe9ec59ad9de3e5866686b9e9967151ae1d2c8"
        }
    ]
}
```

</details>

Now Cad and Dee have vanity accounts generated as well:

```zsh title="multisig.sh snippet"
:!: static/sdks/python/examples/multisig.sh metafiles_cad_dee
```

<details>
<summary>Output</summary>

```zsh
=== Generate vanity account for Cad ===


Mining vanity address...
Using test password.
Keyfile now at cad.keyfile:
{
    "filetype": "Keyfile",
    "signatory": "Cad",
    "public_key": "0xa6f3b47f68ceb96c1934568413dcf2044d5bd5a5e443817cfff15a478bf5df36",
    "authentication_key": "0xcad363212d405dd6029706271d7e8f5de1556b0be020a64df6b9453f5a9d297f",
    "encrypted_private_key": "0x674141414141426b474f66756f76703959396358325965326b5f74706b6c6b30776e7166774a4e505a3954384768336273746654676b4a39777a6d315775746553315832325157525a586c32736c66524f4b546268726454777a6c73584b7258374d546d5a4854766345507037794c476e5458504b335744436f3749626c70655235583032694f77792d6b46",
    "salt": "0xbad21fa2de923ecdb071799329496703"
}


=== Generate vanity account for Dee ===


Mining vanity address...
Using test password.
Keyfile now at dee.keyfile:
{
    "filetype": "Keyfile",
    "signatory": "Dee",
    "public_key": "0xb3988b79feb04eae7264389617f958e2e73069fbcaba912f4a1c630261481dc8",
    "authentication_key": "0xdeeda0b7894cbc6fdf14fc9310907fb3ba3f9f670b714960286040b50a5d4dc4",
    "encrypted_private_key": "0x674141414141426b474f6675734e6f4d34705f59586c526139715454705279333455746b343052502d7231557a4c4b343562744470707159436956574b4d3076334650396662707331685a4847645571553067584856696c6b445162524e684257566a64494d6537356164663337706f464437565f70523846707063316d4e4a48684a516639574252474f48",
    "salt": "0xcac1fc0024c95e9d3637b9b5ed090b19"
}
```

</details>

Now Cad and Dee are appended to the first multisig metafile via `metafile append`:

```zsh title="multisig.sh snippet"
:!: static/sdks/python/examples/multisig.sh metafiles_append
```

<details>
<summary>Output</summary>

```zsh
=== Append Cad and Dee to 3-of-4 multisig ===


Multisig metafile now at cad_and_dee_added.multisig:
{
    "filetype": "Multisig metafile",
    "multisig_name": "Cad and Dee added",
    "address": null,
    "threshold": 3,
    "n_signatories": 4,
    "public_key": "0x7fdb582d1d36ebddb0f6bd3f34abda788f8dc75beb5028f2ccf00044d970540d11434bffb4366cd39d49a04998d7d6cfb9d821c72ed78ead1134d4709f054278a6f3b47f68ceb96c1934568413dcf2044d5bd5a5e443817cfff15a478bf5df36b3988b79feb04eae7264389617f958e2e73069fbcaba912f4a1c630261481dc803",
    "authentication_key": "0x60e4a2ed780e6735b4c5851db670cca7ee11a1e263bc62e70044c403c412b064",
    "signatories": [
        {
            "signatory": "Ace",
            "public_key": "0x7fdb582d1d36ebddb0f6bd3f34abda788f8dc75beb5028f2ccf00044d970540d",
            "authentication_key": "0xace4edf64e9db130f9bf3b38fdaa9e3a4d0a63f748ef87552f77e6dd860465c7"
        },
        {
            "signatory": "Bee",
            "public_key": "0x11434bffb4366cd39d49a04998d7d6cfb9d821c72ed78ead1134d4709f054278",
            "authentication_key": "0xbee4336547dc92c70b02841426fe9ec59ad9de3e5866686b9e9967151ae1d2c8"
        },
        {
            "signatory": "Cad",
            "public_key": "0xa6f3b47f68ceb96c1934568413dcf2044d5bd5a5e443817cfff15a478bf5df36",
            "authentication_key": "0xcad363212d405dd6029706271d7e8f5de1556b0be020a64df6b9453f5a9d297f"
        },
        {
            "signatory": "Dee",
            "public_key": "0xb3988b79feb04eae7264389617f958e2e73069fbcaba912f4a1c630261481dc8",
            "authentication_key": "0xdeeda0b7894cbc6fdf14fc9310907fb3ba3f9f670b714960286040b50a5d4dc4"
        }
    ]
}
```

</details>

Finally, Ace and Dee are removed from the resultant multisig via `metafile remove` to produce another 1-of-2 multisig:

```zsh title="multisig.sh snippet"
:!: static/sdks/python/examples/multisig.sh metafiles_remove
```

<details>
<summary>Output</summary>

```zsh
=== Remove Ace and Dee for 1-of-2 multisig ===


Multisig metafile now at ace_and_dee_removed.multisig:
{
    "filetype": "Multisig metafile",
    "multisig_name": "Ace and Dee removed",
    "address": null,
    "threshold": 1,
    "n_signatories": 2,
    "public_key": "0x11434bffb4366cd39d49a04998d7d6cfb9d821c72ed78ead1134d4709f054278a6f3b47f68ceb96c1934568413dcf2044d5bd5a5e443817cfff15a478bf5df3601",
    "authentication_key": "0x30967fd887c25c9e98f2bac3ac0eec2046b060f627c9660af0dac585c1c5dcd8",
    "signatories": [
        {
            "signatory": "Bee",
            "public_key": "0x11434bffb4366cd39d49a04998d7d6cfb9d821c72ed78ead1134d4709f054278",
            "authentication_key": "0xbee4336547dc92c70b02841426fe9ec59ad9de3e5866686b9e9967151ae1d2c8"
        },
        {
            "signatory": "Cad",
            "public_key": "0xa6f3b47f68ceb96c1934568413dcf2044d5bd5a5e443817cfff15a478bf5df36",
            "authentication_key": "0xcad363212d405dd6029706271d7e8f5de1556b0be020a64df6b9453f5a9d297f"
        }
    ]
}
```

</details>

Thus far all AMEE operations have been conducted off-chain, because the relevant keyfile and metafile operations have simply involved public keys, private keys, and authentication keys.

As such, all multisig metafiles have `"address": null`, since an on-chain account address has not yet been linked with any of the multisig accounts.

### Step 9.3 Authentication key rotation

In this section, the authentication key for Ace's vanity account will be rotated to a 1-of-2 multisig including Ace and Bee, then to a 2-of-2 multisig, and finally back to Ace as a single signer.
Here the demo script uses devnet to automatically fund Ace's account from the faucet, but note that Bee's account does not need to be funded, because only her *signature* is required throughout operations.

In general, authentication key rotation can be used to "convert" a single-signer account to a multisig account, to modify signatories or the threshold of a multisig account, and to convert a multisig account back to a single-signer account.

| Command                      | Use                                                              |
|------------------------------|------------------------------------------------------------------|
| `rotate challenge propose`   | Propose a rotation proof challenge                               |
| `rotate challenge sign`      | Sign a rotation proof challenge                                  |
| `rotate transaction propose` | Propose key rotation transaction for multisig account            |
| `rotate transaction sign`    | Sign key rotation transaction for multisig account               |
| `rotate execute single`      | Execute key rotation transaction from single-signer account      |
| `rotate execute multisig`    | Execute key rotation transaction from multisig account           |

:::tip
The next few demos use the Aptos devnet, which has historically been reset each Thursday.
Make sure devnet is live when you try running the examples!
:::

```zsh title=Command
sh examples/multisig.sh rotate
```

First, generate a vanity account for Ace and Bee, funding Ace since his account will need to pay for authentication key rotation transactions:

```zsh title="multisig.sh snippet"
:!: static/sdks/python/examples/multisig.sh rotate_prep_accounts
```

Note that the `keyfile fund` command is used to wrap a call to the `aptos` CLI:

<details>
<summary>Output</summary>

```zsh
=== Generate vanity account for Ace ===


Mining vanity address...
Using test password.
Keyfile now at ace.keyfile:
{
    "filetype": "Keyfile",
    "signatory": "Ace",
    "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
    "authentication_key": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39",
    "encrypted_private_key": "0x674141414141426b474f684d3969526c525f3171475830466863472d556731766777612d4d4f5536764b43706c574b5441336853516731526a424b4f67364943514933473469724e5049497a5041463578723275325a664d774f6450733969414f7268564d54795347514c4944375064637779506a62512d625074626243554743327a696c64766b67796b43",
    "salt": "0x406c12de7f5edc8e2ed001dab7549259"
}


=== Generate vanity account for Bee ===


Mining vanity address...
Using test password.
Keyfile now at bee.keyfile:
{
    "filetype": "Keyfile",
    "signatory": "Bee",
    "public_key": "0x03d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad84",
    "authentication_key": "0xbee172929a9e4d9722064a802aca27895c314fd351cee9ffe198bbb45f400912",
    "encrypted_private_key": "0x674141414141426b474f684e563976683642546d324258564776694230546443534362516f4f74676c744651574e6d554b324c5773754634627a5651582d78764b63574e66702d72646352726950395056324b6b417466765a5a77584743702d3078427833416e744c47594a45793964587964786178514f5332646c7379493442765f5353595f5445464b67",
    "salt": "0xcb20c2620cee80744ca5bfa2e532938e"
}


=== Fund Ace on devnet ===


Running aptos CLI command: aptos account fund-with-faucet --account 0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39 --faucet-url https://faucet.devnet.aptoslabs.com --url https://fullnode.devnet.aptoslabs.com/v1
New balance: 100000000
```

</details>

Next incorporate Ace and Bee into a multisig account, proposing a rotation proof challenge for rotation to the multisig account:

```zsh title="multisig.sh snippet"
:!: static/sdks/python/examples/multisig.sh rotate_convert_multisig
```

Here, since the multisig account has a threshold of 1, only Ace needs to sign the rotation proof challenge.
Then he can initiate the authentication key rotation transaction from his account:

<details>
<summary>Output</summary>

```zsh
=== Incorporate to 1-of-2 multisig ===


Multisig metafile now at initial.multisig:
{
    "filetype": "Multisig metafile",
    "multisig_name": "Initial",
    "address": null,
    "threshold": 1,
    "n_signatories": 2,
    "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8401",
    "authentication_key": "0xe6d1f9f6d4a4cc571b94b13a9a0e8e96e0673bd3a98bb28df6fdeb0068cd4982",
    "signatories": [
        {
            "signatory": "Ace",
            "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
            "authentication_key": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39"
        },
        {
            "signatory": "Bee",
            "public_key": "0x03d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad84",
            "authentication_key": "0xbee172929a9e4d9722064a802aca27895c314fd351cee9ffe198bbb45f400912"
        }
    ]
}


=== Propose rotation challenge for rotating to multisig ===


Rotation proof challenge proposal now at initial.challenge_proposal:
{
    "filetype": "Rotation proof challenge proposal",
    "description": "Initial",
    "from_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
    "from_is_single_signer": true,
    "to_is_single_signer": false,
    "sequence_number": 0,
    "originator": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39",
    "current_auth_key": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39",
    "new_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8401",
    "chain_id": 47,
    "expiry": "2030-01-01T00:00:00"
}


=== Have Ace sign challenge proposal ===


Using test password.
Rotation proof challenge signature now at ace_initial.challenge_signature:
{
    "filetype": "Rotation proof challenge signature",
    "description": "Ace initial",
    "challenge_proposal": {
        "filetype": "Rotation proof challenge proposal",
        "description": "Initial",
        "from_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
        "from_is_single_signer": true,
        "to_is_single_signer": false,
        "sequence_number": 0,
        "originator": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39",
        "current_auth_key": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39",
        "new_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8401",
        "chain_id": 47,
        "expiry": "2030-01-01T00:00:00"
    },
    "signatory": {
        "signatory": "Ace",
        "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
        "authentication_key": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39"
    },
    "signature": "0x6b849a3f7151e3b5442c23ae59c4b0e2b960965e365e2d14535ddf6c02c9f69e9680cb458cea094beb6d61c072d02be94935e0a75abacff97cf0fa01632a3e06"
}


=== Have Ace execute rotation from single-signer account ===


Using test password.
Transaction successful: 0x97070240cae4ccad7b2fe90f6e69d9aa527a882bd1ed5dd0553f9a942cca5c01
Updating address in multisig metafile.
Multisig metafile now at initial.multisig:
{
    "filetype": "Multisig metafile",
    "multisig_name": "Initial",
    "address": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39",
    "threshold": 1,
    "n_signatories": 2,
    "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8401",
    "authentication_key": "0xe6d1f9f6d4a4cc571b94b13a9a0e8e96e0673bd3a98bb28df6fdeb0068cd4982",
    "signatories": [
        {
            "signatory": "Ace",
            "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
            "authentication_key": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39"
        },
        {
            "signatory": "Bee",
            "public_key": "0x03d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad84",
            "authentication_key": "0xbee172929a9e4d9722064a802aca27895c314fd351cee9ffe198bbb45f400912"
        }
    ]
}
```

</details>

Note that after the successful rotation transaction, the `"address"` field of the multisig metafile has been updated to the vanity address starting with `0xace...`.

Now, propose a threshold increase to 2 signatories:

```zsh title="multisig.sh snippet"
:!: static/sdks/python/examples/multisig.sh rotate_increase_propose
```

In this case, Ace and Bee both need to sign the rotation proof challenge since the account is rotating to a 2-of-2 multisig:

<details>
<summary>Output</summary>

```zsh
=== Increase metafile threshold to two signatures ===


Multisig metafile now at increased.multisig:
{
    "filetype": "Multisig metafile",
    "multisig_name": "Increased",
    "address": null,
    "threshold": 2,
    "n_signatories": 2,
    "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8402",
    "authentication_key": "0x77bb183beceb174bd645eac6370d0dd37e4fba80b60d7f70f071ac4c5ac95839",
    "signatories": [
        {
            "signatory": "Ace",
            "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
            "authentication_key": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39"
        },
        {
            "signatory": "Bee",
            "public_key": "0x03d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad84",
            "authentication_key": "0xbee172929a9e4d9722064a802aca27895c314fd351cee9ffe198bbb45f400912"
        }
    ]
}


=== Propose rotation challenge for increasing threshold ===


Rotation proof challenge proposal now at increase.challenge_proposal:
{
    "filetype": "Rotation proof challenge proposal",
    "description": "Increase",
    "from_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8401",
    "from_is_single_signer": false,
    "to_is_single_signer": false,
    "sequence_number": 1,
    "originator": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39",
    "current_auth_key": "0xe6d1f9f6d4a4cc571b94b13a9a0e8e96e0673bd3a98bb28df6fdeb0068cd4982",
    "new_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8402",
    "chain_id": 47,
    "expiry": "2030-01-01T00:00:00"
}


=== Have Ace sign challenge proposal ===


Using test password.
Rotation proof challenge signature now at ace_increase.challenge_signature:
{
    "filetype": "Rotation proof challenge signature",
    "description": "Ace increase",
    "challenge_proposal": {
        "filetype": "Rotation proof challenge proposal",
        "description": "Increase",
        "from_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8401",
        "from_is_single_signer": false,
        "to_is_single_signer": false,
        "sequence_number": 1,
        "originator": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39",
        "current_auth_key": "0xe6d1f9f6d4a4cc571b94b13a9a0e8e96e0673bd3a98bb28df6fdeb0068cd4982",
        "new_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8402",
        "chain_id": 47,
        "expiry": "2030-01-01T00:00:00"
    },
    "signatory": {
        "signatory": "Ace",
        "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
        "authentication_key": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39"
    },
    "signature": "0xb8e12eaa11bae54639723faa50c3d21afd88adf407b8d301d5a141bfe41146f236a7d3989987684d647999c04aa5aab38ec49b64373ffec7c89c939ff91aa801"
}


=== Have Bee sign challenge proposal ===


Using test password.
Rotation proof challenge signature now at bee_increase.challenge_signature:
{
    "filetype": "Rotation proof challenge signature",
    "description": "Bee increase",
    "challenge_proposal": {
        "filetype": "Rotation proof challenge proposal",
        "description": "Increase",
        "from_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8401",
        "from_is_single_signer": false,
        "to_is_single_signer": false,
        "sequence_number": 1,
        "originator": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39",
        "current_auth_key": "0xe6d1f9f6d4a4cc571b94b13a9a0e8e96e0673bd3a98bb28df6fdeb0068cd4982",
        "new_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8402",
        "chain_id": 47,
        "expiry": "2030-01-01T00:00:00"
    },
    "signatory": {
        "signatory": "Bee",
        "public_key": "0x03d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad84",
        "authentication_key": "0xbee172929a9e4d9722064a802aca27895c314fd351cee9ffe198bbb45f400912"
    },
    "signature": "0xebc14833629362d64508f06b4a67bdf99fff3b753521c127d66e70a5de03d2f7c9bc204763963a3b78c0202949d800b55eff9bbdc5ac28138cd32b27accdbf0d"
}
```

</details>

Now that the rotation proof challenge has been signed, the rotation transaction can be proposed.
Note that even though Ace and Bee both needed to sign the challenge (since the account to rotate to requires two signatures), only one of them needs to sign the transaction proposal (since the account undergoing rotation is originally 1-of-2).
Here, only Bee signs the transaction proposal, then the transaction can be executed.

```zsh title="multisig.sh snippet"
:!: static/sdks/python/examples/multisig.sh rotate_increase_execute
```

<details>
<summary>Output</summary>

```zsh
=== Propose rotation transaction ===


Rotation transaction proposal now at increase.rotation_transaction_proposal:
{
    "filetype": "Rotation transaction proposal",
    "description": "Increase",
    "challenge_proposal": {
        "filetype": "Rotation proof challenge proposal",
        "description": "Increase",
        "from_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8401",
        "from_is_single_signer": false,
        "to_is_single_signer": false,
        "sequence_number": 1,
        "originator": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39",
        "current_auth_key": "0xe6d1f9f6d4a4cc571b94b13a9a0e8e96e0673bd3a98bb28df6fdeb0068cd4982",
        "new_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8402",
        "chain_id": 47,
        "expiry": "2030-01-01T00:00:00"
    },
    "challenge_from_signatures": [
        {
            "signatory": {
                "signatory": "Ace",
                "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
                "authentication_key": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39"
            },
            "signature": "0xb8e12eaa11bae54639723faa50c3d21afd88adf407b8d301d5a141bfe41146f236a7d3989987684d647999c04aa5aab38ec49b64373ffec7c89c939ff91aa801"
        }
    ],
    "challenge_to_signatures": [
        {
            "signatory": {
                "signatory": "Ace",
                "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
                "authentication_key": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39"
            },
            "signature": "0xb8e12eaa11bae54639723faa50c3d21afd88adf407b8d301d5a141bfe41146f236a7d3989987684d647999c04aa5aab38ec49b64373ffec7c89c939ff91aa801"
        },
        {
            "signatory": {
                "signatory": "Bee",
                "public_key": "0x03d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad84",
                "authentication_key": "0xbee172929a9e4d9722064a802aca27895c314fd351cee9ffe198bbb45f400912"
            },
            "signature": "0xebc14833629362d64508f06b4a67bdf99fff3b753521c127d66e70a5de03d2f7c9bc204763963a3b78c0202949d800b55eff9bbdc5ac28138cd32b27accdbf0d"
        }
    ]
}


=== Have Bee only sign rotation transaction proposal ===


Using test password.
Rotation transaction signature now at bee_increase.rotation_transaction_signature:
{
    "filetype": "Rotation transaction signature",
    "description": "Bee increase",
    "transaction_proposal": {
        "filetype": "Rotation transaction proposal",
        "description": "Increase",
        "challenge_proposal": {
            "filetype": "Rotation proof challenge proposal",
            "description": "Increase",
            "from_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8401",
            "from_is_single_signer": false,
            "to_is_single_signer": false,
            "sequence_number": 1,
            "originator": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39",
            "current_auth_key": "0xe6d1f9f6d4a4cc571b94b13a9a0e8e96e0673bd3a98bb28df6fdeb0068cd4982",
            "new_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8402",
            "chain_id": 47,
            "expiry": "2030-01-01T00:00:00"
        },
        "challenge_from_signatures": [
            {
                "signatory": {
                    "signatory": "Ace",
                    "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
                    "authentication_key": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39"
                },
                "signature": "0xb8e12eaa11bae54639723faa50c3d21afd88adf407b8d301d5a141bfe41146f236a7d3989987684d647999c04aa5aab38ec49b64373ffec7c89c939ff91aa801"
            }
        ],
        "challenge_to_signatures": [
            {
                "signatory": {
                    "signatory": "Ace",
                    "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
                    "authentication_key": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39"
                },
                "signature": "0xb8e12eaa11bae54639723faa50c3d21afd88adf407b8d301d5a141bfe41146f236a7d3989987684d647999c04aa5aab38ec49b64373ffec7c89c939ff91aa801"
            },
            {
                "signatory": {
                    "signatory": "Bee",
                    "public_key": "0x03d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad84",
                    "authentication_key": "0xbee172929a9e4d9722064a802aca27895c314fd351cee9ffe198bbb45f400912"
                },
                "signature": "0xebc14833629362d64508f06b4a67bdf99fff3b753521c127d66e70a5de03d2f7c9bc204763963a3b78c0202949d800b55eff9bbdc5ac28138cd32b27accdbf0d"
            }
        ]
    },
    "signatory": {
        "signatory": "Bee",
        "public_key": "0x03d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad84",
        "authentication_key": "0xbee172929a9e4d9722064a802aca27895c314fd351cee9ffe198bbb45f400912"
    },
    "signature": "0xf9420a6a8504f9f77a55de4d4e4a2e44cf26aedb1ef0509b8ef95e5f26fe00ee0bbdfdb28f870baa98077214e816c6bd34a86611155d626b6c9f8979f45fb302"
}


=== Submit rotation transaction ===


Transaction successful: 0xe8799316d6aebe0ca66120a9ccb0eb9b51b659026805535aa37bea18254f6922
Updating address in multisig metafile.
Multisig metafile now at initial.multisig:
{
    "filetype": "Multisig metafile",
    "multisig_name": "Initial",
    "address": null,
    "threshold": 1,
    "n_signatories": 2,
    "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8401",
    "authentication_key": "0xe6d1f9f6d4a4cc571b94b13a9a0e8e96e0673bd3a98bb28df6fdeb0068cd4982",
    "signatories": [
        {
            "signatory": "Ace",
            "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
            "authentication_key": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39"
        },
        {
            "signatory": "Bee",
            "public_key": "0x03d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad84",
            "authentication_key": "0xbee172929a9e4d9722064a802aca27895c314fd351cee9ffe198bbb45f400912"
        }
    ]
}
Updating address in multisig metafile.
Multisig metafile now at increased.multisig:
{
    "filetype": "Multisig metafile",
    "multisig_name": "Increased",
    "address": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39",
    "threshold": 2,
    "n_signatories": 2,
    "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8402",
    "authentication_key": "0x77bb183beceb174bd645eac6370d0dd37e4fba80b60d7f70f071ac4c5ac95839",
    "signatories": [
        {
            "signatory": "Ace",
            "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
            "authentication_key": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39"
        },
        {
            "signatory": "Bee",
            "public_key": "0x03d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad84",
            "authentication_key": "0xbee172929a9e4d9722064a802aca27895c314fd351cee9ffe198bbb45f400912"
        }
    ]
}
```

</details>

Note that the `"address"` field of `initial.multisig` has been set to `null`, and `increased.multisig` now reflects the vanity address starting with `0xace...`.

Next, propose a rotation proof challenge for rotating the account back to have Ace as a single signer:

```zsh title="multisig.sh snippet"
:!: static/sdks/python/examples/multisig.sh rotate_convert_single_propose
```

Here, Ace and Bee both need to sign the proposal since the account undergoing rotation is a 2-of-2 multisig:

<details>
<summary>Output</summary>

```zsh
=== Propose rotation challenge for rotating back to Ace ===


Rotation proof challenge proposal now at return.challenge_proposal:
{
    "filetype": "Rotation proof challenge proposal",
    "description": "Return",
    "from_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8402",
    "from_is_single_signer": false,
    "to_is_single_signer": true,
    "sequence_number": 2,
    "originator": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39",
    "current_auth_key": "0x77bb183beceb174bd645eac6370d0dd37e4fba80b60d7f70f071ac4c5ac95839",
    "new_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
    "chain_id": 47,
    "expiry": "2030-01-01T00:00:00"
}


=== Have Ace sign challenge proposal ===


Using test password.
Rotation proof challenge signature now at ace_return.challenge_signature:
{
    "filetype": "Rotation proof challenge signature",
    "description": "Ace return",
    "challenge_proposal": {
        "filetype": "Rotation proof challenge proposal",
        "description": "Return",
        "from_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8402",
        "from_is_single_signer": false,
        "to_is_single_signer": true,
        "sequence_number": 2,
        "originator": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39",
        "current_auth_key": "0x77bb183beceb174bd645eac6370d0dd37e4fba80b60d7f70f071ac4c5ac95839",
        "new_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
        "chain_id": 47,
        "expiry": "2030-01-01T00:00:00"
    },
    "signatory": {
        "signatory": "Ace",
        "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
        "authentication_key": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39"
    },
    "signature": "0x3603b39d1fabb753f88015bafdad8c0cc87668b32434d6c449d4b9b30506af0336ff565397d9ed73cbd4b2cfea339b2555cb8de66f36bf37298c179582553807"
}


=== Have Bee sign challenge proposal ===


Using test password.
Rotation proof challenge signature now at bee_return.challenge_signature:
{
    "filetype": "Rotation proof challenge signature",
    "description": "Bee return",
    "challenge_proposal": {
        "filetype": "Rotation proof challenge proposal",
        "description": "Return",
        "from_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8402",
        "from_is_single_signer": false,
        "to_is_single_signer": true,
        "sequence_number": 2,
        "originator": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39",
        "current_auth_key": "0x77bb183beceb174bd645eac6370d0dd37e4fba80b60d7f70f071ac4c5ac95839",
        "new_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
        "chain_id": 47,
        "expiry": "2030-01-01T00:00:00"
    },
    "signatory": {
        "signatory": "Bee",
        "public_key": "0x03d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad84",
        "authentication_key": "0xbee172929a9e4d9722064a802aca27895c314fd351cee9ffe198bbb45f400912"
    },
    "signature": "0x61f8e2ca2d7559ce99a15b9a7d3bebfd75469030eb34417cce4e451ec184b6582340974e284efad087b659782f80ba555ca1b98b0c4c81549309fce5e588ad05"
}
```

</details>

Now that both challenge signatures are available, a transaction from the multisig account can be proposed and executed:

```zsh title="multisig.sh snippet"
:!: static/sdks/python/examples/multisig.sh rotate_convert_single_execute
```

In this case, both Ace and Bee have to sign the transaction since the account undergoing rotation starts off as a 2-of-2 multisig:

<details>
<summary>Output</summary>

```zsh
=== Propose rotation transaction ===


Rotation transaction proposal now at return.rotation_transaction_proposal:
{
    "filetype": "Rotation transaction proposal",
    "description": "Return",
    "challenge_proposal": {
        "filetype": "Rotation proof challenge proposal",
        "description": "Return",
        "from_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8402",
        "from_is_single_signer": false,
        "to_is_single_signer": true,
        "sequence_number": 2,
        "originator": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39",
        "current_auth_key": "0x77bb183beceb174bd645eac6370d0dd37e4fba80b60d7f70f071ac4c5ac95839",
        "new_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
        "chain_id": 47,
        "expiry": "2030-01-01T00:00:00"
    },
    "challenge_from_signatures": [
        {
            "signatory": {
                "signatory": "Ace",
                "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
                "authentication_key": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39"
            },
            "signature": "0x3603b39d1fabb753f88015bafdad8c0cc87668b32434d6c449d4b9b30506af0336ff565397d9ed73cbd4b2cfea339b2555cb8de66f36bf37298c179582553807"
        },
        {
            "signatory": {
                "signatory": "Bee",
                "public_key": "0x03d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad84",
                "authentication_key": "0xbee172929a9e4d9722064a802aca27895c314fd351cee9ffe198bbb45f400912"
            },
            "signature": "0x61f8e2ca2d7559ce99a15b9a7d3bebfd75469030eb34417cce4e451ec184b6582340974e284efad087b659782f80ba555ca1b98b0c4c81549309fce5e588ad05"
        }
    ],
    "challenge_to_signatures": [
        {
            "signatory": {
                "signatory": "Ace",
                "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
                "authentication_key": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39"
            },
            "signature": "0x3603b39d1fabb753f88015bafdad8c0cc87668b32434d6c449d4b9b30506af0336ff565397d9ed73cbd4b2cfea339b2555cb8de66f36bf37298c179582553807"
        }
    ]
}


=== Have Ace sign rotation transaction proposal ===


Using test password.
Rotation transaction signature now at ace_return.rotation_transaction_signature:
{
    "filetype": "Rotation transaction signature",
    "description": "Ace return",
    "transaction_proposal": {
        "filetype": "Rotation transaction proposal",
        "description": "Return",
        "challenge_proposal": {
            "filetype": "Rotation proof challenge proposal",
            "description": "Return",
            "from_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8402",
            "from_is_single_signer": false,
            "to_is_single_signer": true,
            "sequence_number": 2,
            "originator": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39",
            "current_auth_key": "0x77bb183beceb174bd645eac6370d0dd37e4fba80b60d7f70f071ac4c5ac95839",
            "new_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
            "chain_id": 47,
            "expiry": "2030-01-01T00:00:00"
        },
        "challenge_from_signatures": [
            {
                "signatory": {
                    "signatory": "Ace",
                    "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
                    "authentication_key": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39"
                },
                "signature": "0x3603b39d1fabb753f88015bafdad8c0cc87668b32434d6c449d4b9b30506af0336ff565397d9ed73cbd4b2cfea339b2555cb8de66f36bf37298c179582553807"
            },
            {
                "signatory": {
                    "signatory": "Bee",
                    "public_key": "0x03d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad84",
                    "authentication_key": "0xbee172929a9e4d9722064a802aca27895c314fd351cee9ffe198bbb45f400912"
                },
                "signature": "0x61f8e2ca2d7559ce99a15b9a7d3bebfd75469030eb34417cce4e451ec184b6582340974e284efad087b659782f80ba555ca1b98b0c4c81549309fce5e588ad05"
            }
        ],
        "challenge_to_signatures": [
            {
                "signatory": {
                    "signatory": "Ace",
                    "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
                    "authentication_key": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39"
                },
                "signature": "0x3603b39d1fabb753f88015bafdad8c0cc87668b32434d6c449d4b9b30506af0336ff565397d9ed73cbd4b2cfea339b2555cb8de66f36bf37298c179582553807"
            }
        ]
    },
    "signatory": {
        "signatory": "Ace",
        "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
        "authentication_key": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39"
    },
    "signature": "0xe83937397995a3ef0f2d6c75ae75f1e92fc7b1adb01a7e66a18fc6cf67dc547c2fda42812f7a8fcea61c23579e688a1a13ac51190a3f2a65e39e42626a21470c"
}


=== Have Bee sign rotation transaction proposal ===


Using test password.
Rotation transaction signature now at bee_return.rotation_transaction_signature:
{
    "filetype": "Rotation transaction signature",
    "description": "Bee return",
    "transaction_proposal": {
        "filetype": "Rotation transaction proposal",
        "description": "Return",
        "challenge_proposal": {
            "filetype": "Rotation proof challenge proposal",
            "description": "Return",
            "from_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8402",
            "from_is_single_signer": false,
            "to_is_single_signer": true,
            "sequence_number": 2,
            "originator": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39",
            "current_auth_key": "0x77bb183beceb174bd645eac6370d0dd37e4fba80b60d7f70f071ac4c5ac95839",
            "new_public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
            "chain_id": 47,
            "expiry": "2030-01-01T00:00:00"
        },
        "challenge_from_signatures": [
            {
                "signatory": {
                    "signatory": "Ace",
                    "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
                    "authentication_key": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39"
                },
                "signature": "0x3603b39d1fabb753f88015bafdad8c0cc87668b32434d6c449d4b9b30506af0336ff565397d9ed73cbd4b2cfea339b2555cb8de66f36bf37298c179582553807"
            },
            {
                "signatory": {
                    "signatory": "Bee",
                    "public_key": "0x03d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad84",
                    "authentication_key": "0xbee172929a9e4d9722064a802aca27895c314fd351cee9ffe198bbb45f400912"
                },
                "signature": "0x61f8e2ca2d7559ce99a15b9a7d3bebfd75469030eb34417cce4e451ec184b6582340974e284efad087b659782f80ba555ca1b98b0c4c81549309fce5e588ad05"
            }
        ],
        "challenge_to_signatures": [
            {
                "signatory": {
                    "signatory": "Ace",
                    "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
                    "authentication_key": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39"
                },
                "signature": "0x3603b39d1fabb753f88015bafdad8c0cc87668b32434d6c449d4b9b30506af0336ff565397d9ed73cbd4b2cfea339b2555cb8de66f36bf37298c179582553807"
            }
        ]
    },
    "signatory": {
        "signatory": "Bee",
        "public_key": "0x03d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad84",
        "authentication_key": "0xbee172929a9e4d9722064a802aca27895c314fd351cee9ffe198bbb45f400912"
    },
    "signature": "0xc43866daf8c1963822d24c36c4774353d1816bc89ac66aa2cb91a5bd0bbd8da4a990eb83d313de55a228e90bbf22dec43b6c7005ddfd2a94d99041a04cc1e607"
}


=== Submit rotation transaction ===


Transaction successful: 0x5d255734cb8c48d68c4b0686b0e672d0624ab43282361d036b3200db73364287
Updating address in multisig metafile.
Multisig metafile now at increased.multisig:
{
    "filetype": "Multisig metafile",
    "multisig_name": "Increased",
    "address": null,
    "threshold": 2,
    "n_signatories": 2,
    "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f836442603d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad8402",
    "authentication_key": "0x77bb183beceb174bd645eac6370d0dd37e4fba80b60d7f70f071ac4c5ac95839",
    "signatories": [
        {
            "signatory": "Ace",
            "public_key": "0xddaf20b0c4bc71538087b70f3f346c39565abd2941c59a85a9bab212f8364426",
            "authentication_key": "0xace97430e0b2cf78048fb6587603b15dbba6e240f526b0b2402409480f50cf39"
        },
        {
            "signatory": "Bee",
            "public_key": "0x03d68f252ce02170971c4ce6e1a5691dd9da02ab1830166df23f4e36091bad84",
            "authentication_key": "0xbee172929a9e4d9722064a802aca27895c314fd351cee9ffe198bbb45f400912"
        }
    ]
}
```

</details>

Note that after the rotation, the metafile has been updated with `"address": null`

In practice, note that the consensus mechanism will probably entail something like the following:

1. Ace and Bee independently generate single-signer keyfiles.
2. One of them, for example Ace, acts as a "scribe", so Bee sends her keyfile to Ace.
3. Ace uses the `metafile incorporate` command to generate a multisig metafile, and sends a copy to Bee for her records.
4. Ace then uses the appropriate `metafile` and `rotate` subcommands to propose rotation proof challenges, rotation transactions, etc. (note that Bee's private key is encrypted so this is not a security threat).
5. Ace sends proposals over to Bee, then Bee signs them and sends her signature files back to Ace.
6. Ace signs locally, then executes transactions using his and Bee's signature files.

Theoretically this can be scaled to as many as 32 independent signatories, but note that higher numbers of signatories introduce logistical complexities (e.g. sending signature files back and forth in a group chat, or running shell commands with 32 arguments).

### Step 9.4 Protocol governance

In this section AMEE will be used to [publish and upgrade the same `UpgradeAndGovern` package as above](#step-8-perform-move-package-governance), then to invoke a different governance script, all under the authority of a 1-of-2 multisig account:

| Command                      | Use                                                              |
|------------------------------|------------------------------------------------------------------|
| `publish propose`            | Propose Move package publication                                 |
| `publish sign`               | Sign a Move package publication proposal                         |
| `publish execute`            | Execute Move package publication from proposal signature file(s) |
| `script propose`             | Propose Move script invocation                                   |
| `script sign`                | Sign a Move script invocation proposal                           |
| `script execute`             | Execute Move script invocation from proposal signature file(s)   |

```zsh title=Command
sh examples/multisig.sh govern
```

As before, this example begins with a vanity account for both Ace and Bee:

```zsh title="multisig.sh snippet"
:!: static/sdks/python/examples/multisig.sh govern_prep_accounts
```

<details>
<summary>Output</summary>

```zsh
=== Generate vanity account for Ace ===


Mining vanity address...
Using test password.
Keyfile now at ace.keyfile:
{
    "filetype": "Keyfile",
    "signatory": "Ace",
    "public_key": "0x466bc39951c1655f3a069e3aa8008fea8ce06b1d4369453dae27ef350156e688",
    "authentication_key": "0xace0c53165faa60e289b06a1b331acda8730aeecfe071896778b4a9dcb56c3fc",
    "encrypted_private_key": "0x674141414141426b474f3451376343457438384f303450737a553633495672377137664f6e4e7145466b623930694c5a4835305874493651304155534151345931375364466c4b683978595553716d45694e3774786b66375a376537776f4e76754c676935554b36384a7a763748574f5a38425f313859515f7a33736576305144734a37345347772d643856",
    "salt": "0x32ca9953b85b899bc1266e5f52d72ec8"
}


=== Generate vanity account for Bee ===


Mining vanity address...
Using test password.
Keyfile now at bee.keyfile:
{
    "filetype": "Keyfile",
    "signatory": "Bee",
    "public_key": "0x4c975513021598e108d7fe494116b8177a4562781984d5135904ee324a99e66a",
    "authentication_key": "0xbee654e2c91e02a6c6bd98b99c0d6a0b84b314c5d90d36c6c96f85daef90eb29",
    "encrypted_private_key": "0x674141414141426b474f34516331397270536645686369466c5a63315a5f2d66484454516e74765562495a5877414a775f366879326c63366b4643794e774c545557564a62486e78772d69783478636a673552676b516b345a377430504f4b61784863556e5a704d6337776354685a375679532d7a42744a325f6c47662d633452527562614e7935686b3263",
    "salt": "0x95678e49f59d1c498ee7f0b83b893a12"
}
```

</details>

Ace and Bee are then incorporated in a multisig, which is funded on devnet.
Note here that neither Ace nor Bee need to be funded, since the multisig account is linked with an on-chain account through direct funding, rather than through authentication key rotation.
Here, the multisig account address is identical to its authentication key, so the devnet faucet can simply be used to fund the corresponding address.
On testnet or mainnet, this process would probably entail sending `APT` to the account in question.

```zsh title="multisig.sh snippet"
:!: static/sdks/python/examples/multisig.sh govern_prep_multisig
```

Note that the multisig metafile has `"address": null` before but not after the faucet funding operation:

<details>
<summary>Output</summary>

```zsh
=== Incorporate to 1-of-2 multisig ===


Multisig metafile now at protocol.multisig:
{
    "filetype": "Multisig metafile",
    "multisig_name": "Protocol",
    "address": null,
    "threshold": 1,
    "n_signatories": 2,
    "public_key": "0x466bc39951c1655f3a069e3aa8008fea8ce06b1d4369453dae27ef350156e6884c975513021598e108d7fe494116b8177a4562781984d5135904ee324a99e66a01",
    "authentication_key": "0x214c3bae3231c134b59036daa22b518a007f398cf71aa2b5bc873c781c10ccec",
    "signatories": [
        {
            "signatory": "Ace",
            "public_key": "0x466bc39951c1655f3a069e3aa8008fea8ce06b1d4369453dae27ef350156e688",
            "authentication_key": "0xace0c53165faa60e289b06a1b331acda8730aeecfe071896778b4a9dcb56c3fc"
        },
        {
            "signatory": "Bee",
            "public_key": "0x4c975513021598e108d7fe494116b8177a4562781984d5135904ee324a99e66a",
            "authentication_key": "0xbee654e2c91e02a6c6bd98b99c0d6a0b84b314c5d90d36c6c96f85daef90eb29"
        }
    ]
}


=== Fund multisig ===


Running aptos CLI command: aptos account fund-with-faucet --account 0x214c3bae3231c134b59036daa22b518a007f398cf71aa2b5bc873c781c10ccec --faucet-url https://faucet.devnet.aptoslabs.com --url https://fullnode.devnet.aptoslabs.com/v1
New balance: 100000000
Updating address in multisig metafile.
Multisig metafile now at protocol.multisig:
{
    "filetype": "Multisig metafile",
    "multisig_name": "Protocol",
    "address": "0x214c3bae3231c134b59036daa22b518a007f398cf71aa2b5bc873c781c10ccec",
    "threshold": 1,
    "n_signatories": 2,
    "public_key": "0x466bc39951c1655f3a069e3aa8008fea8ce06b1d4369453dae27ef350156e6884c975513021598e108d7fe494116b8177a4562781984d5135904ee324a99e66a01",
    "authentication_key": "0x214c3bae3231c134b59036daa22b518a007f398cf71aa2b5bc873c781c10ccec",
    "signatories": [
        {
            "signatory": "Ace",
            "public_key": "0x466bc39951c1655f3a069e3aa8008fea8ce06b1d4369453dae27ef350156e688",
            "authentication_key": "0xace0c53165faa60e289b06a1b331acda8730aeecfe071896778b4a9dcb56c3fc"
        },
        {
            "signatory": "Bee",
            "public_key": "0x4c975513021598e108d7fe494116b8177a4562781984d5135904ee324a99e66a",
            "authentication_key": "0xbee654e2c91e02a6c6bd98b99c0d6a0b84b314c5d90d36c6c96f85daef90eb29"
        }
    ]
}
```

</details>

Next a Move package publication proposal is constructed, signed, and the package is published. Here, only Ace's signature is necessary because the multisig threshold is 1:

```zsh title="multisig.sh snippet"
:!: static/sdks/python/examples/multisig.sh govern_publish
```

Note that the publication proposal includes information required to download and publish the package from GitHub:

* GitHub user
* GitHub project
* Commit
* Path to package's `Move.toml` inside the repository
* Named address to substitute inside `Move.toml`
* Sequence to publish modules in

:::tip
Modules should be listed from the bottom of the dependency hierarchy up, with modules that are used listed before the modules that use them, and modules that declare friends listed before the friends they declare.
:::

For this example, the `Move.toml` file in question is as follows:

```toml title="Move.toml"
:!: static/move-examples/upgrade_and_govern/genesis/Move.toml manifest
```

Here, `Move.toml` contains the named address `upgrade_and_govern`, which is defined generically as `_`:
AMEE expects a named address of this format, corresponding to the multisig account address to publish under.

Note that the repository is downloaded and recompiled before signing, and before transaction execution.
This is to ensure that all signatories, as well as the transaction submitter, are referring to the same transaction payload (as defined by the GitHub information from the proposal file):

<details>
<summary>Output</summary>


```zsh
=== Propose publication ===


Publication proposal now at genesis.publication_proposal:
{
    "filetype": "Publication proposal",
    "description": "Genesis",
    "github_user": "aptos-labs",
    "github_project": "aptos-core",
    "commit": "965b6f5",
    "manifest_path": "aptos-move/move-examples/upgrade_and_govern/genesis/Move.toml",
    "named_address": "upgrade_and_govern",
    "module_sequence": [
        "parameters"
    ],
    "multisig": {
        "filetype": "Multisig metafile",
        "multisig_name": "Protocol",
        "address": "0x214c3bae3231c134b59036daa22b518a007f398cf71aa2b5bc873c781c10ccec",
        "threshold": 1,
        "n_signatories": 2,
        "public_key": "0x466bc39951c1655f3a069e3aa8008fea8ce06b1d4369453dae27ef350156e6884c975513021598e108d7fe494116b8177a4562781984d5135904ee324a99e66a01",
        "authentication_key": "0x214c3bae3231c134b59036daa22b518a007f398cf71aa2b5bc873c781c10ccec",
        "signatories": [
            {
                "signatory": "Ace",
                "public_key": "0x466bc39951c1655f3a069e3aa8008fea8ce06b1d4369453dae27ef350156e688",
                "authentication_key": "0xace0c53165faa60e289b06a1b331acda8730aeecfe071896778b4a9dcb56c3fc"
            },
            {
                "signatory": "Bee",
                "public_key": "0x4c975513021598e108d7fe494116b8177a4562781984d5135904ee324a99e66a",
                "authentication_key": "0xbee654e2c91e02a6c6bd98b99c0d6a0b84b314c5d90d36c6c96f85daef90eb29"
            }
        ]
    },
    "sequence_number": 0,
    "chain_id": 47,
    "expiry": "2030-12-31T00:00:00"
}


=== Sign publication proposal ===


Extracting https://github.com/aptos-labs/aptos-core/archive/965b6f5.zip to temporary directory /var/folders/4c/rtts9qpj3yq0f5_f_gbl6cn40000gn/T/tmpxzwx1kip.
Running aptos CLI command: aptos move compile --save-metadata --included-artifacts none --package-dir /var/folders/4c/rtts9qpj3yq0f5_f_gbl6cn40000gn/T/tmpxzwx1kip/aptos-core-965b6f54aa0664da885a0858f2c42e15a58ab79f/aptos-move/move-examples/upgrade_and_govern/genesis --named-addresses upgrade_and_govern=0x214c3bae3231c134b59036daa22b518a007f398cf71aa2b5bc873c781c10ccec

Compiling, may take a little while to download git dependencies...
INCLUDING DEPENDENCY AptosFramework
INCLUDING DEPENDENCY AptosStdlib
INCLUDING DEPENDENCY MoveStdlib
BUILDING UpgradeAndGovern
Using test password.
Publication signature now at genesis.publication_signature:
{
    "filetype": "Publication signature",
    "description": "Genesis",
    "transaction_proposal": {
        "filetype": "Publication proposal",
        "description": "Genesis",
        "github_user": "aptos-labs",
        "github_project": "aptos-core",
        "commit": "965b6f5",
        "manifest_path": "aptos-move/move-examples/upgrade_and_govern/genesis/Move.toml",
        "named_address": "upgrade_and_govern",
        "module_sequence": [
            "parameters"
        ],
        "multisig": {
            "filetype": "Multisig metafile",
            "multisig_name": "Protocol",
            "address": "0x214c3bae3231c134b59036daa22b518a007f398cf71aa2b5bc873c781c10ccec",
            "threshold": 1,
            "n_signatories": 2,
            "public_key": "0x466bc39951c1655f3a069e3aa8008fea8ce06b1d4369453dae27ef350156e6884c975513021598e108d7fe494116b8177a4562781984d5135904ee324a99e66a01",
            "authentication_key": "0x214c3bae3231c134b59036daa22b518a007f398cf71aa2b5bc873c781c10ccec",
            "signatories": [
                {
                    "signatory": "Ace",
                    "public_key": "0x466bc39951c1655f3a069e3aa8008fea8ce06b1d4369453dae27ef350156e688",
                    "authentication_key": "0xace0c53165faa60e289b06a1b331acda8730aeecfe071896778b4a9dcb56c3fc"
                },
                {
                    "signatory": "Bee",
                    "public_key": "0x4c975513021598e108d7fe494116b8177a4562781984d5135904ee324a99e66a",
                    "authentication_key": "0xbee654e2c91e02a6c6bd98b99c0d6a0b84b314c5d90d36c6c96f85daef90eb29"
                }
            ]
        },
        "sequence_number": 0,
        "chain_id": 47,
        "expiry": "2030-12-31T00:00:00"
    },
    "signatory": {
        "signatory": "Ace",
        "public_key": "0x466bc39951c1655f3a069e3aa8008fea8ce06b1d4369453dae27ef350156e688",
        "authentication_key": "0xace0c53165faa60e289b06a1b331acda8730aeecfe071896778b4a9dcb56c3fc"
    },
    "signature": "0x77124d930ad708838183782bc974d5c0ae8459bb1962204de484fd6adb47fdf1ac25a38373e8147914671a0381fc12f866aabc8033192d2b7d122e918875030f"
}


=== Execute publication ===


Extracting https://github.com/aptos-labs/aptos-core/archive/965b6f5.zip to temporary directory /var/folders/4c/rtts9qpj3yq0f5_f_gbl6cn40000gn/T/tmp77o1ui3g.
Running aptos CLI command: aptos move compile --save-metadata --included-artifacts none --package-dir /var/folders/4c/rtts9qpj3yq0f5_f_gbl6cn40000gn/T/tmp77o1ui3g/aptos-core-965b6f54aa0664da885a0858f2c42e15a58ab79f/aptos-move/move-examples/upgrade_and_govern/genesis --named-addresses upgrade_and_govern=0x214c3bae3231c134b59036daa22b518a007f398cf71aa2b5bc873c781c10ccec

Compiling, may take a little while to download git dependencies...
INCLUDING DEPENDENCY AptosFramework
INCLUDING DEPENDENCY AptosStdlib
INCLUDING DEPENDENCY MoveStdlib
BUILDING UpgradeAndGovern
Transaction successful: 0x3e6fa333901a8a9933a8e01da8e547da74321947486f510b9d10671bf599b6a0
```

</details>

Next, the package is upgraded to `v1.1.0`, which involves the same workflow albeit with a different manifest path and a new module to publish (`transfer.move` uses `parameters.move`, so it is listed second):

```zsh title="multisig.sh snippet"
:!: static/sdks/python/examples/multisig.sh govern_upgrade
```

<details>
<summary>Output</summary>

```zsh
=== Propose upgrade ===


Publication proposal now at upgrade.publication_proposal:
{
    "filetype": "Publication proposal",
    "description": "Upgrade",
    "github_user": "aptos-labs",
    "github_project": "aptos-core",
    "commit": "965b6f5",
    "manifest_path": "aptos-move/move-examples/upgrade_and_govern/upgrade/Move.toml",
    "named_address": "upgrade_and_govern",
    "module_sequence": [
        "parameters",
        "transfer"
    ],
    "multisig": {
        "filetype": "Multisig metafile",
        "multisig_name": "Protocol",
        "address": "0x214c3bae3231c134b59036daa22b518a007f398cf71aa2b5bc873c781c10ccec",
        "threshold": 1,
        "n_signatories": 2,
        "public_key": "0x466bc39951c1655f3a069e3aa8008fea8ce06b1d4369453dae27ef350156e6884c975513021598e108d7fe494116b8177a4562781984d5135904ee324a99e66a01",
        "authentication_key": "0x214c3bae3231c134b59036daa22b518a007f398cf71aa2b5bc873c781c10ccec",
        "signatories": [
            {
                "signatory": "Ace",
                "public_key": "0x466bc39951c1655f3a069e3aa8008fea8ce06b1d4369453dae27ef350156e688",
                "authentication_key": "0xace0c53165faa60e289b06a1b331acda8730aeecfe071896778b4a9dcb56c3fc"
            },
            {
                "signatory": "Bee",
                "public_key": "0x4c975513021598e108d7fe494116b8177a4562781984d5135904ee324a99e66a",
                "authentication_key": "0xbee654e2c91e02a6c6bd98b99c0d6a0b84b314c5d90d36c6c96f85daef90eb29"
            }
        ]
    },
    "sequence_number": 1,
    "chain_id": 47,
    "expiry": "2030-12-31T00:00:00"
}


=== Sign upgrade proposal ===


Extracting https://github.com/aptos-labs/aptos-core/archive/965b6f5.zip to temporary directory /var/folders/4c/rtts9qpj3yq0f5_f_gbl6cn40000gn/T/tmpsnj4d5f2.
Running aptos CLI command: aptos move compile --save-metadata --included-artifacts none --package-dir /var/folders/4c/rtts9qpj3yq0f5_f_gbl6cn40000gn/T/tmpsnj4d5f2/aptos-core-965b6f54aa0664da885a0858f2c42e15a58ab79f/aptos-move/move-examples/upgrade_and_govern/upgrade --named-addresses upgrade_and_govern=0x214c3bae3231c134b59036daa22b518a007f398cf71aa2b5bc873c781c10ccec

Compiling, may take a little while to download git dependencies...
INCLUDING DEPENDENCY AptosFramework
INCLUDING DEPENDENCY AptosStdlib
INCLUDING DEPENDENCY MoveStdlib
BUILDING UpgradeAndGovern
Using test password.
Publication signature now at upgrade.publication_signature:
{
    "filetype": "Publication signature",
    "description": "Upgrade",
    "transaction_proposal": {
        "filetype": "Publication proposal",
        "description": "Upgrade",
        "github_user": "aptos-labs",
        "github_project": "aptos-core",
        "commit": "965b6f5",
        "manifest_path": "aptos-move/move-examples/upgrade_and_govern/upgrade/Move.toml",
        "named_address": "upgrade_and_govern",
        "module_sequence": [
            "parameters",
            "transfer"
        ],
        "multisig": {
            "filetype": "Multisig metafile",
            "multisig_name": "Protocol",
            "address": "0x214c3bae3231c134b59036daa22b518a007f398cf71aa2b5bc873c781c10ccec",
            "threshold": 1,
            "n_signatories": 2,
            "public_key": "0x466bc39951c1655f3a069e3aa8008fea8ce06b1d4369453dae27ef350156e6884c975513021598e108d7fe494116b8177a4562781984d5135904ee324a99e66a01",
            "authentication_key": "0x214c3bae3231c134b59036daa22b518a007f398cf71aa2b5bc873c781c10ccec",
            "signatories": [
                {
                    "signatory": "Ace",
                    "public_key": "0x466bc39951c1655f3a069e3aa8008fea8ce06b1d4369453dae27ef350156e688",
                    "authentication_key": "0xace0c53165faa60e289b06a1b331acda8730aeecfe071896778b4a9dcb56c3fc"
                },
                {
                    "signatory": "Bee",
                    "public_key": "0x4c975513021598e108d7fe494116b8177a4562781984d5135904ee324a99e66a",
                    "authentication_key": "0xbee654e2c91e02a6c6bd98b99c0d6a0b84b314c5d90d36c6c96f85daef90eb29"
                }
            ]
        },
        "sequence_number": 1,
        "chain_id": 47,
        "expiry": "2030-12-31T00:00:00"
    },
    "signatory": {
        "signatory": "Ace",
        "public_key": "0x466bc39951c1655f3a069e3aa8008fea8ce06b1d4369453dae27ef350156e688",
        "authentication_key": "0xace0c53165faa60e289b06a1b331acda8730aeecfe071896778b4a9dcb56c3fc"
    },
    "signature": "0xf10efc093a1a7ae62f1939bddbb83d3951668fab1af5361412fb7b3ddc438d0703aa2cfdd4007e3c8f684600b42d6f5e941e21b6c0615dc12079230e8c665906"
}


=== Execute upgrade ===


Extracting https://github.com/aptos-labs/aptos-core/archive/965b6f5.zip to temporary directory /var/folders/4c/rtts9qpj3yq0f5_f_gbl6cn40000gn/T/tmp7_vh_ja0.
Running aptos CLI command: aptos move compile --save-metadata --included-artifacts none --package-dir /var/folders/4c/rtts9qpj3yq0f5_f_gbl6cn40000gn/T/tmp7_vh_ja0/aptos-core-965b6f54aa0664da885a0858f2c42e15a58ab79f/aptos-move/move-examples/upgrade_and_govern/upgrade --named-addresses upgrade_and_govern=0x214c3bae3231c134b59036daa22b518a007f398cf71aa2b5bc873c781c10ccec

Compiling, may take a little while to download git dependencies...
INCLUDING DEPENDENCY AptosFramework
INCLUDING DEPENDENCY AptosStdlib
INCLUDING DEPENDENCY MoveStdlib
BUILDING UpgradeAndGovern
Transaction successful: 0x9800e6bffe17072c63bb4c5b23763e61135b3957e38181223879445454d95064
```

</details>

Lastly, the `set_only.move` governance script is invoked from the multisig account:

```rust title=set_only.move
:!: static/move-examples/upgrade_and_govern/upgrade/scripts/set_only.move script
```

Note here that the main function in this script, `set_only`, accepts only a `&signer` as an argument, with constants like `PARAMETER_1` and `PARAMETER_2` defined inside the script.
AMEE expects scripts of this format, having only a single `&signer` argument in the main function call, such that all inner function arguments other than the governance signature can be easily inspected on GitHub.

```zsh title="multisig.sh snippet"
:!: static/sdks/python/examples/multisig.sh govern_script
```

Note here that a script proposal is similar in form to a publication proposal, except for an additional `script_name` field (which specifies the name of the main function call), and no `module_sequence` field.
Similarly, the Move script in question is downloaded and recompiled during signing and submission, to ensure the same transaction payload:

<details>
<summary>Output</summary>

```zsh
=== Propose script invocation ===


Script proposal now at invoke.script_proposal:
{
    "filetype": "Script proposal",
    "description": "Invoke",
    "github_user": "aptos-labs",
    "github_project": "aptos-core",
    "commit": "965b6f5",
    "manifest_path": "aptos-move/move-examples/upgrade_and_govern/upgrade/Move.toml",
    "named_address": "upgrade_and_govern",
    "script_name": "set_only",
    "multisig": {
        "filetype": "Multisig metafile",
        "multisig_name": "Protocol",
        "address": "0x214c3bae3231c134b59036daa22b518a007f398cf71aa2b5bc873c781c10ccec",
        "threshold": 1,
        "n_signatories": 2,
        "public_key": "0x466bc39951c1655f3a069e3aa8008fea8ce06b1d4369453dae27ef350156e6884c975513021598e108d7fe494116b8177a4562781984d5135904ee324a99e66a01",
        "authentication_key": "0x214c3bae3231c134b59036daa22b518a007f398cf71aa2b5bc873c781c10ccec",
        "signatories": [
            {
                "signatory": "Ace",
                "public_key": "0x466bc39951c1655f3a069e3aa8008fea8ce06b1d4369453dae27ef350156e688",
                "authentication_key": "0xace0c53165faa60e289b06a1b331acda8730aeecfe071896778b4a9dcb56c3fc"
            },
            {
                "signatory": "Bee",
                "public_key": "0x4c975513021598e108d7fe494116b8177a4562781984d5135904ee324a99e66a",
                "authentication_key": "0xbee654e2c91e02a6c6bd98b99c0d6a0b84b314c5d90d36c6c96f85daef90eb29"
            }
        ]
    },
    "sequence_number": 2,
    "chain_id": 47,
    "expiry": "2030-12-31T00:00:00"
}


=== Sign invocation proposal ===


Extracting https://github.com/aptos-labs/aptos-core/archive/965b6f5.zip to temporary directory /var/folders/4c/rtts9qpj3yq0f5_f_gbl6cn40000gn/T/tmpout_mpw2.
Running aptos CLI command: aptos move compile --save-metadata --included-artifacts none --package-dir /var/folders/4c/rtts9qpj3yq0f5_f_gbl6cn40000gn/T/tmpout_mpw2/aptos-core-965b6f54aa0664da885a0858f2c42e15a58ab79f/aptos-move/move-examples/upgrade_and_govern/upgrade --named-addresses upgrade_and_govern=0x214c3bae3231c134b59036daa22b518a007f398cf71aa2b5bc873c781c10ccec

Compiling, may take a little while to download git dependencies...
INCLUDING DEPENDENCY AptosFramework
INCLUDING DEPENDENCY AptosStdlib
INCLUDING DEPENDENCY MoveStdlib
BUILDING UpgradeAndGovern
Using test password.
Script signature now at invoke.script_signature:
{
    "filetype": "Script signature",
    "description": "Invoke",
    "transaction_proposal": {
        "filetype": "Script proposal",
        "description": "Invoke",
        "github_user": "aptos-labs",
        "github_project": "aptos-core",
        "commit": "965b6f5",
        "manifest_path": "aptos-move/move-examples/upgrade_and_govern/upgrade/Move.toml",
        "named_address": "upgrade_and_govern",
        "script_name": "set_only",
        "multisig": {
            "filetype": "Multisig metafile",
            "multisig_name": "Protocol",
            "address": "0x214c3bae3231c134b59036daa22b518a007f398cf71aa2b5bc873c781c10ccec",
            "threshold": 1,
            "n_signatories": 2,
            "public_key": "0x466bc39951c1655f3a069e3aa8008fea8ce06b1d4369453dae27ef350156e6884c975513021598e108d7fe494116b8177a4562781984d5135904ee324a99e66a01",
            "authentication_key": "0x214c3bae3231c134b59036daa22b518a007f398cf71aa2b5bc873c781c10ccec",
            "signatories": [
                {
                    "signatory": "Ace",
                    "public_key": "0x466bc39951c1655f3a069e3aa8008fea8ce06b1d4369453dae27ef350156e688",
                    "authentication_key": "0xace0c53165faa60e289b06a1b331acda8730aeecfe071896778b4a9dcb56c3fc"
                },
                {
                    "signatory": "Bee",
                    "public_key": "0x4c975513021598e108d7fe494116b8177a4562781984d5135904ee324a99e66a",
                    "authentication_key": "0xbee654e2c91e02a6c6bd98b99c0d6a0b84b314c5d90d36c6c96f85daef90eb29"
                }
            ]
        },
        "sequence_number": 2,
        "chain_id": 47,
        "expiry": "2030-12-31T00:00:00"
    },
    "signatory": {
        "signatory": "Ace",
        "public_key": "0x466bc39951c1655f3a069e3aa8008fea8ce06b1d4369453dae27ef350156e688",
        "authentication_key": "0xace0c53165faa60e289b06a1b331acda8730aeecfe071896778b4a9dcb56c3fc"
    },
    "signature": "0x2dfdadc1465f079c5106504acc0ca63723807b9b53689ca564b7bdd858f00f7639f5443bcce0fd69e22981109e4385e398a55892afd4c2aea739a6449382ce05"
}


=== Execute script invocation ===


Extracting https://github.com/aptos-labs/aptos-core/archive/965b6f5.zip to temporary directory /var/folders/4c/rtts9qpj3yq0f5_f_gbl6cn40000gn/T/tmpxcr6g_8r.
Running aptos CLI command: aptos move compile --save-metadata --included-artifacts none --package-dir /var/folders/4c/rtts9qpj3yq0f5_f_gbl6cn40000gn/T/tmpxcr6g_8r/aptos-core-965b6f54aa0664da885a0858f2c42e15a58ab79f/aptos-move/move-examples/upgrade_and_govern/upgrade --named-addresses upgrade_and_govern=0x214c3bae3231c134b59036daa22b518a007f398cf71aa2b5bc873c781c10ccec

Compiling, may take a little while to download git dependencies...
INCLUDING DEPENDENCY AptosFramework
INCLUDING DEPENDENCY AptosStdlib
INCLUDING DEPENDENCY MoveStdlib
BUILDING UpgradeAndGovern
Transaction successful: 0xd4220b0bf55608df4d00ead8fc07ee19c9b7cc4b1a8a1aec4b4bf0e7f503b83f
```

</details>

Again, in practice note that the consensus mechanism will probably entail something like the following, in the case of a 2-of-2 multisig (unlike a 1-of-2 in the above example):

1. Ace and Bee independently generate single-signer keyfiles.
2. One of them, for example Bee, acts as a "scribe", so Ace sends his keyfile to Bee.
3. Bee uses the `metafile incorporate` command to generate a multisig metafile, and sends a copy to Ace for his records.
4. Bee then uses the appropriate `publish` and `script` subcommands to propose package publications, package upgrades, and script invocations from the multisig account.
5. Bee sends proposals over to Ace, then Ace reviews the corresponding package on GitHub before signing and sending a signature files back to Bee.
6. Bee signs locally, then executes transactions using her and Ace's signature files.

Theoretically this can be scaled to as many as 32 independent signatories, but note that higher numbers of signatories introduce logistical complexities (e.g. sending signature files back and forth in a group chat, or running shell commands with 32 arguments).

---

Congratulations on completing the tutorial on K-of-N multi-signer authentication operations!

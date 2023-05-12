---
title: "Move Scripts"
slug: "move-scripts"
---

# Move Scripts

This tutorial explains how to write and execute a [Move script](../book/modules-and-scripts.md). You can use Move scripts to execute a series of commands across published Move module interfaces.

## Example use case

The following example calls functions on the [aptos_coin.move](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/aptos_coin.move) module to confirm the balance of the destination account is less than `desired_balance`, and if so, tops it up to `desired_balance`.

```move
script {
    use std::signer;
    use aptos_framework::aptos_account;
    use aptos_framework::aptos_coin;
    use aptos_framework::coin;

    fun main(src: &signer, dest: address, desired_balance: u64) {
        let src_addr = signer::address_of(src);

        let balance = coin::balance<aptos_coin::AptosCoin>(src_addr);
        if (balance < desired_balance) {
            aptos_account::transfer(src, dest, desired_balance - balance);
        };
    }
}
```

## Execution

Now that you know what you would like to accomplish, you need to determine:

- Where do I put these files?
- What do I name them?
- Do I need a `Move.toml`?
- How do I run my script with the CLI?

Let us run through how to execute a Move script with a step-by-step example using the [Aptos CLI](../../tools/aptos-cli-tool/use-aptos-cli.md).

1. Make a new directory for your work:
```sh
mkdir testing
cd testing
```

2. Set up the Aptos CLI and [create an account](../../tools/aptos-cli-tool/use-aptos-cli#initialize-local-configuration-and-create-an-account):
```sh
aptos init
```
The CLI will ask you which network you want to work with (e.g. `devnet`, `testnet`, `mainnet`). Enter: `devnet`

You may reuse an existing private key (which looks like this: `0xf1adc8d01c1a890f17efc6b08f92179e6008d43026dd56b71e7b0d9b453536be`), or it can generate a new one for you, as part of setting up your account.

3. From this same directory, initialize a new Move project:
```sh
aptos move init --name my_script
```

4. Create a `top_up.move` file containing the example script above in a `sources/` subdirectory of your `testing/` directory.

5. Create a `Move.toml` file in the root of your `testing/` directory containing:

```
[package]
name = 'my_script'
version = '1.0.0'

[dependencies.AptosFramework]
git = 'https://github.com/aptos-labs/aptos-core.git'
rev = 'devnet'
subdir = 'aptos-move/framework/aptos-framework'
```

This results in a file structure of:
```
testing/
   Move.toml
   sources/
      top_up.move
      my_module.move
```

6. Compile the script:
```
$ aptos move compile --named-addresses addr=81e2e2499407693c81fe65c86405ca70df529438339d9da7a6fc2520142b591e
Compiling, may take a little while to download git dependencies...
INCLUDING DEPENDENCY AptosFramework
INCLUDING DEPENDENCY AptosStdlib
INCLUDING DEPENDENCY MoveStdlib
BUILDING my_script
{
  "Result": []
}
```
Note how we use the `--named-addresses` argument. This is necessary because in the code we refer to this named address called `addr`. The compiler needs to know what this refers to. Instead of using this CLI argument, you could put something like this in your `Move.toml`:
```
[addresses]
addr = "b078d693856a65401d492f99ca0d6a29a0c5c0e371bc2521570a86e40d95f823"
```

7. Run the compiled script:
```
$ aptos move run-script --compiled-script-path build/my_script/bytecode_scripts/main.mv --args address:b078d693856a65401d492f99ca0d6a29a0c5c0e371bc2521570a86e40d95f823 --args u64:5
Do you want to submit a transaction for a range of [17000 - 25500] Octas at a gas unit price of 100 Octas? [yes/no] >
yes
{
  "Result": {
    "transaction_hash": "0x655f839a45c5f14ba92590c321f97c3c3f9aba334b9152e994fb715d5648db4b",
    "gas_used": 178,
    "gas_unit_price": 100,
    "sender": "81e2e2499407693c81fe65c86405ca70df529438339d9da7a6fc2520142b591e",
    "sequence_number": 53,
    "success": true,
    "timestamp_us": 1669811892262502,
    "version": 370133122,
    "vm_status": "Executed successfully"
  }
}
```

Note that the path of the compiled script is under `build/my_script/`, not `build/top_up/`. This is because it uses the name of the project contained in `Move.toml`, which is `my_script` from when we ran `aptos move init --name my_script`.

See the code used for this document at: https://github.com/banool/move-examples/tree/main/run_script. The full example explains how to use a Move script that relies on a user-created Move module as well.

See also how to do this with the [Rust SDK](https://stackoverflow.com/questions/74452702/how-do-i-execute-a-move-script-on-aptos-using-the-rust-sdk) instead of the Aptos CLI in Stack Overflow.

## Advanced

You may execute a script in a more streamlined fashion; instead of running `aptos move compile` and then `aptos move run-script --compiled-script-path` separately, you can just do this:
```
$ aptos move run-script --script-path sources/my_script.move --args address:b078d693856a65401d492f99ca0d6a29a0c5c0e371bc2521570a86e40d95f823 --args u64:5
```
This will conduct both steps with a single CLI command yet has [issues](https://github.com/aptos-labs/aptos-core/issues/5733). For this reason, we recommend using the previous two-step approach for now.

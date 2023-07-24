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
        
        addr::my_module::do_nothing();

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

Let us run through how to execute a Move script with a step-by-step example using the [Aptos CLI](../../tools/aptos-cli/use-cli/use-aptos-cli.md).

1. Make a new directory for your work:
    ```sh
    mkdir testing
    cd testing
    ```

2. Set up the Aptos CLI and [create an account](../../tools/aptos-cli/use-cli/use-aptos-cli#initialize-local-configuration-and-create-an-account):
    ```sh
    aptos init --network devnet
    ```
    
    You may reuse an existing private key (which looks like this: `0xbd944102bf5b5dfafa7fe865d8fa719da6a1f0eafa3cd600f93385482d2c37a4`), or it can generate a new one for you, as part of setting up your account. Let's say your account looks like the example below:
    ```sh
    ---
    profiles:
      default:
        private_key: "0xbd944102bf5b5dfafa7fe865d8fa719da6a1f0eafa3cd600f93385482d2c37a4"
        public_key: "0x47673ec83bb254cc9a8bfdb31846daacd0c96fe41f81855462f5fc5306312b1b"
        account: cb265645385819f3dbe71aac266e319e7f77aed252cacf2930b68102828bf615
        rest_url: "https://fullnode.devnet.aptoslabs.com"
        faucet_url: "https://faucet.devnet.aptoslabs.com"
    ```

3. From this same directory, initialize a new Move project:
    ```sh
    aptos move init --name run_script
    ```

4. Create a `my_script.move` file containing the example script above in a `sources/` subdirectory of your `testing/` directory. Also, create a `my_module.move` file as seen in the example below:
    ```
    module addr::my_module {
        public entry fun do_nothing() { }
    }
    ```

    This results in the following file structure:
    ```
    testing/
       Move.toml
       sources/
          my_script.move
          my_module.move
    ```

5. Compile the script:
    ```
    $ aptos move compile --named-addresses addr=cb265645385819f3dbe71aac266e319e7f77aed252cacf2930b68102828bf615
    Compiling, may take a little while to download git dependencies...
    INCLUDING DEPENDENCY AptosFramework
    INCLUDING DEPENDENCY AptosStdlib
    INCLUDING DEPENDENCY MoveStdlib
    BUILDING run_script
    {
      "Result": [
        "cb265645385819f3dbe71aac266e319e7f77aed252cacf2930b68102828bf615::my_module"
      ]
    }
    ```

    Note how we use the `--named-addresses` argument. This is necessary because in the code we refer to this named address called `addr`. The compiler needs to know what this refers to. Instead of using this CLI argument, you could put something like this in your `Move.toml`:

    ```
    [addresses]
    addr = "cb265645385819f3dbe71aac266e319e7f77aed252cacf2930b68102828bf615"
    ```

6. Run the compiled script:
    ```
    $ aptos move run-script --compiled-script-path build/my_script/bytecode_scripts/main.mv --args address:b078d693856a65401d492f99ca0d6a29a0c5c0e371bc2521570a86e40d95f823 --args u64:5
    Do you want to submit a transaction for a range of [17000 - 25500] Octas at a gas unit price of 100 Octas? [yes/no] >
    yes
    {
      "Result": {
        "transaction_hash": "0xa6ca6275c73f82638b88a830015ab81734a533aebd36cc4647b48ff342434cdf",
        "gas_used": 3,
        "gas_unit_price": 100,
        "sender": "cb265645385819f3dbe71aac266e319e7f77aed252cacf2930b68102828bf615",
        "sequence_number": 4,
        "success": true,
        "timestamp_us": 1683030933803632,
        "version": 3347495,
        "vm_status": "Executed successfully"
      }
    }
    ```

Note that the path of the compiled script is under `build/run_script/`, not `build/my_script/`. This is because it uses the name of the project contained in `Move.toml`, which is `run_script` from when we ran `aptos move init --name run_script`.

See the [code](https://github.com/banool/move-examples/tree/main/run_script) used for this document. The full example explains how to use a Move script that relies on a user-created Move module as well.

See also how to do this with the [Rust SDK](https://stackoverflow.com/questions/74452702/how-do-i-execute-a-move-script-on-aptos-using-the-rust-sdk) instead of the Aptos CLI in Stack Overflow.

## Advanced

You may execute a script in a more streamlined fashion; instead of running `aptos move compile` and then `aptos move run-script --compiled-script-path` separately, you can just do this:
```
$ aptos move run-script --script-path sources/my_script.move --args address:b078d693856a65401d492f99ca0d6a29a0c5c0e371bc2521570a86e40d95f823 --args u64:5
```
This will conduct both steps with a single CLI command yet has [issues](https://github.com/aptos-labs/aptos-core/issues/5733). For this reason, we recommend using the previous two-step approach for now.

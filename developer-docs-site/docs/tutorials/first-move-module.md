---
title: "Your first Move Module on the Aptos Blockchain"
slug: "your-first-move-module"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Overview

This tutorial details how to write, compile, test, publish, and interact with Move Modules on the Aptos Blockchain. The steps to doing so:

* Write, compile, and test the Move Module
* Publish the Move Module to the Aptos Blockchain
* Initialize and interact with resources of the Move Module

This tutorial builds on "[Your first transaction](/tutorials/your-first-transaction)" and borrows that code as a library for this example. The following tutorial contains example code that can be downloaded in its entirety below:
<Tabs>
  <TabItem value="python" label="Python" default>

[Download Library](/examples/first_transaction.py)
[Download Example](/examples/hello_blockchain.py)
  </TabItem>
  <TabItem value="rust" label="Rust">
  </TabItem>
  <TabItem value="typescript" label="Typescript">
  </TabItem>
  <TabItem value="manual" label="Manually">
  </TabItem>
</Tabs>

## Step 1) Write and test the Move Module

### Step 1.1) Download Aptos-core

For the simplicity of this exercise, Aptos-core has a `move-examples` directory that makes it easy to build and test Move modules without downloading additional resources. Over time, we will expand this section to describe how to leverage [Move][move_url] tools for development.

For now, download and prepare Aptos-core:

```bash
git clone https://github.com/aptos-labs/aptos-core.git
cd aptos-core
./scripts/dev_setup.sh
source ~/.cargo/env
```

### Step 1.2) Review the Module

Change the path to `aptos-move/move-examples`. The rest of this section will review the file `sources/HelloBlockchain.move`.

This module enables users to create a `String` resource under their account and set it. Users are only able to set their resource and cannot set other's resources.

```rust
module HelloBlockchain::Message {
    use Std::ASCII;
    use Std::Errors;
    use Std::Signer;

    struct MessageHolder has key {
        message: ASCII::String,
    }

    public(script) fun set_message(account: signer, message_bytes: vector<u8>)
    acquires MessageHolder {
        let message = ASCII::string(message_bytes);
        let account_addr = Signer::address_of(&account);
        if (!exists<MessageHolder>(account_addr)) {
            move_to(&account, MessageHolder {
                message,
            })
        } else {
            let old_message_holder = borrow_global_mut<MessageHolder>(account_addr);
            old_message_holder.message = message;
        }
    }
}
```

In the previous code, the two important sections are the struct `MessageHolder` and the function `set_message`. `set_message` is a `script` function allowing it to be called directly by transactions. Upon calling it, the function will determine if the current account has a `MessageHolder` resource and creates and stores the `message` it if it does not exist. If the resource exists, the `message` in the `MessageHolder` is overwritten.

### Step 1.3) Testing the Module

Move allows for inline tests, so we add `get_message` to make retrieving the `message` convenient and a test function `sender_can_set_message` to validate an end-to-end flow. This can be validated by running `cargo test`. There is another test under `sources/HelloBlockchainTest.move` that demonstrates another method for writing tests.

Note: `sender_can_set_message` is `script` function in order to call the `script` function `set_message`.

```rust
    const ENO_MESSAGE: u64 = 0;

    public fun get_message(addr: address): ASCII::String acquires MessageHolder {
        assert!(exists<MessageHolder>(addr), Errors::not_published(ENO_MESSAGE));
        *&borrow_global<MessageHolder>(addr).message
    }

    #[test(account = @0x1)]
    public(script) fun sender_can_set_message(account: signer) acquires MessageHolder {
        let addr = Signer::address_of(&account);
        set_message(account,  b"Hello, Blockchain");

        assert!(
          get_message(addr) == ASCII::string(b"Hello, Blockchain"),
          0
        );
    }
```

## Step 2) Publishing and Interacting with the Move Module

Now we return to our application to deploy and interact with the module on the Atpos blockchain. As mentioned earlier, this tutorial builds upon the earlier tutorial and shares the common code. As a result, this tutorial only discusses new features for that library including the ability to publish, send the `set_message` transaction, and reading `MessageHolder::message`. The only difference from publishing a module and submitting a transaction is the payload type. See the following:


### Step 2.1) Publishing the Move Module

<Tabs>
  <TabItem value="python" label="Python" default>

```python3
    def publish_module(self, account_from: Account, module: str) -> str:
        """Publish a new module to the blockchain within the specified account"""

        payload = {
            "type": "module_bundle_payload",
            "modules": [
                {"bytecode": f"0x{module}"},
            ],
        }
        txn_request = self.generate_transaction(account_from.address(), payload)
        signed_txn = self.sign_transaction(account_from, txn_request)
        res = self.submit_transaction(signed_txn).json()
        return str(res["hash"])
```
  </TabItem>
  <TabItem value="rust" label="Rust">
  </TabItem>
  <TabItem value="typescript" label="Typescript">
  </TabItem>
  <TabItem value="manual" label="Manually">
  </TabItem>
</Tabs>

### Step 2.2) Reading a resource

The module is published at an address. This is the `contract_address` below. This is similar to the previous example, where the `TestCoin` is at `0x1`. The `contract_address` will be the same as the account that publishes it.

<Tabs>
  <TabItem value="python" label="Python" default>

```python3
    def get_message(self, contract_address: str, account_address: str) -> str:
        """ Retrieve the resource Message::MessageHolder::message """

        resources = self.account_resources(account_address)
        for resource in resources:
            if resource["type"] == f"0x{contract_address}::Message::MessageHolder":
                return resource["data"]["message"]
        return None
```
  </TabItem>
  <TabItem value="rust" label="Rust">
  </TabItem>
  <TabItem value="typescript" label="Typescript">
  </TabItem>
  <TabItem value="manual" label="Manually">
  </TabItem>
</Tabs>

### Step 2.3) Modifying a resource

Move modules must expose `script` functions for initializing and manipulating resources. The `script` can then be called from a transaction.

Note: while the REST interface can display strings, due to limitations of JSON and Move, it cannot determine if an argument is a string or a hex-encoded string. So the transaction arguments always assume the latter. Hence, in this example, the message is encoded as a hex-string.

<Tabs>
  <TabItem value="python" label="Python" default>

```python3
    def set_message(self, contract_address: str, account_from: Account, message: string) -> str:
        """ Potentially initialize and set the resource Message::MessageHolder::message """
        payload = {
            "type": "script_function_payload",
            "function": f"0x{contract_address}::Message::set_message",
            "type_arguments": [],
            "arguments": [
                message.encode("utf-8").hex(),
            ]
        }
        txn_request = self.generate_transaction(account_from.address(), payload)
        signed_txn = self.sign_transaction(account_from, txn_request)
        res = self.submit_transaction(signed_txn).json()
        return str(res["hash"])
```
  </TabItem>
  <TabItem value="rust" label="Rust">
  </TabItem>
  <TabItem value="typescript" label="Typescript">
  </TabItem>
  <TabItem value="manual" label="Manually">
  </TabItem>
</Tabs>

### Step 3) Initialize and interact with the Move module

<Tabs>
<TabItem value="python" label="Python" default>
For Python:

* Download both the [library](/examples/first_transaction.py) and [example](/examples/hello_blockchain.py) into the same directory.
* Enter your favorite cli tool and navigate to the location of library and example.
* Install nacl and requests, if necessary.
* Execute `python3 hello_blockchain.py Message.mv`
* At a certain point, it will mention that "Update the modules path to Alice's address, build, copy to the provided path, and press enter."
* Switch to the CLI where you were testing the Move module:
  * Edit `Move.toml` and under `[addresses]` replace `e110` with Alice's address stated in the other prompt.
  * `cargo run -- sources`
  * Copy `build/Examples/bytecode_modules/Message.mv` to the same folder as `hello_blockchain.py`
* Return to the prompt and press enter.

</TabItem>
  <TabItem value="rust" label="Rust">
  </TabItem>
  <TabItem value="typescript" label="Typescript">
  </TabItem>
  <TabItem value="manual" label="Manually">
  </TabItem>
</Tabs>

The output should look like the following:

```
=== Addresses ===
Alice: a52671f10dc3479b09d0a11ce47694c0
Bob: ec6ec14e4abe10aaa6ad53b0b63a1806

=== Initial Balances ===
Alice: 10000000
Bob: 10000000

Update the modules path to Alice's address, build, copy to the provided path, and press enter.

=== Testing Alice ===
Publishing...
Initial value: None
Setting the message to "Hello, Blockchain"
New value: Hello, Blockchain

=== Testing Bob ===
Initial value: None
Setting the message to "Hello, Blockchain"
New value: Hello, Blockchain
```

The outcome shows that Alice and Bob went from having no resource to one with a `message` set to "Hello, Blockchain".

The data can be verified by visiting either a REST interface or the explorer:
* Alice's account via the [REST interface][alice_account_rest]
* Bob's account via the [explorer][bob_account_explorer]

[account_basics]: /basics/basics-accounts
[alice_account_rest]: https://dev.fullnode.aptoslabs.com/accounts/a52671f10dc3479b09d0a11ce47694c0/
[bob_account_explorer]: https://aptos-explorer.netlify.app/account/ec6ec14e4abe10aaa6ad53b0b63a1806/
[rest_spec]: https://dev.fullnode.aptoslabs.com/spec.html

---
title: "Your First Move Module"
slug: "your-first-move-module"
sidebar_position: 2
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Your First Move Module

This tutorial details how to write, compile, test, publish and interact with Move Modules on the Aptos Blockchain. The steps are:

1. Write, compile, and test the Move Module
2. Publish the Move Module to the Aptos Blockchain
3. Initialize and interact with resources of the Move Module

This tutorial builds on [Your first transaction](/tutorials/your-first-transaction) as a library for this example. The following tutorial contains example code that can be downloaded in its entirety below:

<Tabs>
  <TabItem value="typescript" label="Typescript" default>

For this tutorial, will be focusing on `hello_blockchain.ts` and re-using the `first_transaction.ts` library from the previous tutorial.

You can find the typescript project [here](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/examples/typescript)

  </TabItem>
  <TabItem value="python" label="Python">

For this tutorial, will be focusing on `hello_blockchain.py` and re-using the `first_transaction.py` library from the previous tutorial.

You can find the python project [here](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/examples/python)

  </TabItem>
  <TabItem value="rust" label="Rust">

For this tutorial, will be focusing on `hello_blockchain/src` and re-using the `first_transaction/src` library from the previous tutorial.

You can find the rust project [here](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/examples/rust)

The key is that you need have two terminals open at the same time:  One is Move terminal, the other is Python terminal, and the sequence of events of switching these two terminals is very important.


  </TabItem>
</Tabs>

## Step 1) Write and test the Move Module

### Step 1.1) Download Aptos-core

For the simplicity of this exercise, Aptos-core has a `move-examples` directory that makes it easy to build and test Move modules without downloading additional resources. Over time, we will expand this section to describe how to leverage [Move](https://github.com/move-language/move/tree/main/language/documentation/tutorial) tools for development.

For now, download and prepare Aptos-core:

```bash
git clone https://github.com/aptos-labs/aptos-core.git
cd aptos-core
./scripts/dev_setup.sh
source ~/.cargo/env
git checkout origin/devnet
```

Install Aptos Commandline tool. Learn more about the [Aptos command line tool](https://github.com/aptos-labs/aptos-core/tree/main/crates/aptos)

```bash
cargo install --git https://github.com/aptos-labs/aptos-core.git aptos
```

### Step 1.2) Review the Module

In this terminal, change directories to `aptos-move/move-examples/hello_blockchain`. Keep this terminal window for the rest of this tutorial- we will refer to it later as the "Move Window". The rest of this section will review the file `sources/HelloBlockchain.move`.

This module enables users to create a `String` resource under their account and set it. Users are only able to set their resource and cannot set other's resources.

```rust
module HelloBlockchain::Message {
    use std::string;
    use std::error;
    use std::signer;

    struct MessageHolder has key {
        message: string::String,
    }

    public entry fun set_message(account: signer, message_bytes: vector<u8>)
    acquires MessageHolder {
        let message = string::utf8(message_bytes);
        let account_addr = signer::address_of(&account);
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

In the code above, the two important sections are the struct `MessageHolder` and the function `set_message`. `set_message` is a `script` function allowing it to be called directly by transactions. Upon calling it, the function will determine if the current account has a `MessageHolder` resource and creates and stores the `message` if it does not exist. If the resource exists, the `message` in the `MessageHolder` is overwritten.

### Step 1.3) Testing the Module

Move allows for inline tests, so we add `get_message` to make retrieving the `message` convenient and a test function `sender_can_set_message` to validate an end-to-end flow. This can be validated by running `cargo test`. There is another test under `sources/HelloBlockchainTest.move` that demonstrates another method for writing tests.

This can be tested by entering `cargo test test_hello_blockchain -p move-examples -- --exact` at the terminal.

Note: `sender_can_set_message` is a `script` function in order to call the `script` function `set_message`.

```rust
    const ENO_MESSAGE: u64 = 0;

    public fun get_message(addr: address): string::String acquires MessageHolder {
        assert!(exists<MessageHolder>(addr), Errors::not_published(ENO_MESSAGE));
        *&borrow_global<MessageHolder>(addr).message
    }

    #[test(account = @0x1)]
    public(script) fun sender_can_set_message(account: signer) acquires MessageHolder {
        let addr = Signer::address_of(&account);
        set_message(account,  b"Hello, Blockchain");

        assert!(
          get_message(addr) == string::utf8(b"Hello, Blockchain"),
          0
        );
    }
```

## Step 2) Publishing and Interacting with the Move Module

Now we return to our application to deploy and interact with the module on the Aptos blockchain. As mentioned earlier, this tutorial builds upon the earlier tutorial and shares the common code. As a result, this tutorial only discusses new features for that library including the ability to publish, send the `set_message` transaction, and reading `MessageHolder::message`. The only difference from publishing a module and submitting a transaction is the payload type. See the following:

### Step 2.1) Publishing the Move Module

<Tabs>
  <TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/hello_blockchain.ts section_1
```

  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/examples/python/hello_blockchain.py section_1
```

  </TabItem>
  <TabItem value="rust" label="Rust">

```rust
:!: static/examples/rust/hello_blockchain/src/lib.rs section_1
```

  </TabItem>
</Tabs>

:::tip
To initialize the module, you can write a `init_module` function. This private function is executed automatically when the module is published. This `init_module` function must be private, it must only take signer or signer reference as a parameter, and it must not return any value. Here is an example:

```asm
 fun init_module(creator: &signer) {
        move_to(
            creator,
            ModuleData { global_counter: 0 }
        );
    }
```

:::

### Step 2.2) Reading a resource

The module is published at an address. This is the `contract_address` below. This is similar to the previous example, where the `Coin` is at `0x1`. The `contract_address` will be the same as the account that publishes it.

<Tabs>
  <TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/hello_blockchain.ts section_2
```

  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/examples/python/hello_blockchain.py section_2
```

  </TabItem>
  <TabItem value="rust" label="Rust">

```rust
:!: static/examples/rust/hello_blockchain/src/lib.rs section_2
```

  </TabItem>
</Tabs>

### Step 2.3) Modifying a resource

Move modules must expose `script` functions for initializing and manipulating resources. The `script` can then be called
from a transaction.

Note: while the REST interface can display strings, due to limitations of JSON and Move, it cannot determine if an argument is a string or a hex-encoded string. So the transaction arguments always assume the latter. Hence, in this example, the message is encoded as a hex-string.

<Tabs>
  <TabItem value="typescript" label="Typescript" default>

```typescript
:!: static/examples/typescript/hello_blockchain.ts section_3
```

  </TabItem>
  <TabItem value="python" label="Python">

```python
:!: static/examples/python/hello_blockchain.py section_3
```

  </TabItem>
  <TabItem value="rust" label="Rust">

```rust
:!: static/examples/rust/hello_blockchain/src/lib.rs section_3
```

  </TabItem>
</Tabs>

### Step 3) Initialize and interact with the Move module

<Tabs>
<TabItem value="typescript" label="Typescript" default>
For Typescript:

- Download the [example project](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/examples/typescript)
- Open your favorite terminal and navigate to where you downloaded the above example project
- Install the required libraries: `yarn install`
- Execute the example: `yarn hello_blockchain Message.mv`

</TabItem>
<TabItem value="python" label="Python">
For Python3:

- Download the [example project](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/examples/python)
- Open your favorite terminal and navigate to where you downloaded the above example project
- Install the required libraries: `pip3 install -r requirements.txt`.
- Execute the example: `python3 hello_blockchain.py Message.mv`

</TabItem>
<TabItem value="rust" label="Rust">
For Rust:

- Download the [example project](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/examples/rust)
- Open your favorite terminal and navigate to where you downloaded the above example project
- Execute the example: `cargo run --bin hello-blockchain -- Message.mv`

</TabItem>
</Tabs>

On Python terminal:
 python git:(c6841bd6b0) python3 hello_blockchain.py Message.mv

=== Addresses ===
Alice: 10bf3a5f10a4dcd6b387221559ca4cf6ead4850ae3499816494fd94845938d7b
Bob: 43f9269af65f97e24d71d874de7e070ad889c7da7254d14307e7c887b4e4ee7c

=== Initial Balances ===
Alice: {'type': '0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>', 'data': {'coin': {'value': '5000'}, 'deposit_events': {'counter': '1', 'guid': {'id': {'addr': '0x10bf3a5f10a4dcd6b387221559ca4cf6ead4850ae3499816494fd94845938d7b', 'creation_num': '1'}}}, 'withdraw_events': {'counter': '0', 'guid': {'id': {'addr': '0x10bf3a5f10a4dcd6b387221559ca4cf6ead4850ae3499816494fd94845938d7b', 'creation_num': '2'}}}}}
Bob: {'type': '0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>', 'data': {'coin': {'value': '5000'}, 'deposit_events': {'counter': '1', 'guid': {'id': {'addr': '0x43f9269af65f97e24d71d874de7e070ad889c7da7254d14307e7c887b4e4ee7c', 'creation_num': '1'}}}, 'withdraw_events': {'counter': '0', 'guid': {'id': {'addr': '0x43f9269af65f97e24d71d874de7e070ad889c7da7254d14307e7c887b4e4ee7c', 'creation_num': '2'}}}}}

Update the module with Alice's address, build, copy to the provided path, and press enter.  


AT the point, move to the other Move terminal: 

On the Move terminal
/Users/feizheng/aptos-core/aptos-move/move-examples/hello_blockchain
➜  hello_blockchain git:(c6841bd6b0) aptos move compile --package-dir . --named-addresses HelloBlockchain=0x10bf3a5f10a4dcd6b387221559ca4cf6ead4850ae3499816494fd94845938d7b
{
  "Result": [
    "10BF3A5F10A4DCD6B387221559CA4CF6EAD4850AE3499816494FD94845938D7B::message"
  ]
}

➜  bytecode_modules git:(c6841bd6b0) cp message.mv /Users/feizheng/aptos-core/developer-docs-site/static/examples/python/


AT this point, move to the other Python terminal: 

Press “Enter”


=== Testing Alice ===
Publishing...
Initial value: None
Setting the message to "Hello, Blockchain"
New value: {'type': '0x10bf3a5f10a4dcd6b387221559ca4cf6ead4850ae3499816494fd94845938d7b::message::MessageHolder', 'data': {'message': 'Hello, Blockchain', 'message_change_events': {'counter': '0', 'guid': {'id': {'addr': '0x10bf3a5f10a4dcd6b387221559ca4cf6ead4850ae3499816494fd94845938d7b', 'creation_num': '3'}}}}}

=== Testing Bob ===
Initial value: None
Setting the message to "Hello, Blockchain"
New value: {'type': '0x10bf3a5f10a4dcd6b387221559ca4cf6ead4850ae3499816494fd94845938d7b::message::MessageHolder', 'data': {'message': 'Hello, Blockchain', 'message_change_events': {'counter': '0', 'guid': {'id': {'addr': '0x43f9269af65f97e24d71d874de7e070ad889c7da7254d14307e7c887b4e4ee7c', 'creation_num': '3'}}}}}



The outcome shows that Alice and Bob went from having no resource to one with a `message` set to "Hello, Blockchain".

The data can be verified by visiting either a REST interface or the explorer:

- Alice's account via the [REST interface][alice_account_rest]
- Bob's account on the [explorer][bob_account_explorer]

[account_basics]: /concepts/basics-accounts
[alice_account_rest]: https://fullnode.devnet.aptoslabs.com/v1/accounts/a52671f10dc3479b09d0a11ce47694c0/
[bob_account_explorer]: https://explorer.devnet.aptos.dev/account/ec6ec14e4abe10aaa6ad53b0b63a1806
[rest_spec]: https://fullnode.devnet.aptoslabs.com/v1/spec#/

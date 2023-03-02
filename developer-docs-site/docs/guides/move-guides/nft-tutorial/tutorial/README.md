# Aptos NFT Tutorial
This tutorial is forked from an existing tutorial from the Move repo (https://github.com/move-language/move/tree/main/language/documentation/tutorial).
Because Aptos toolings and Move toolings are slightly different, we made some small modifications that changed all Move commands to Aptos Move commands. 

# Move Tutorial
Welcome to the Move Tutorial! In this tutorial, we are going to go through some steps of developing Move code
including design, implementation, unit testing and formal verification of Move modules. 

There are seven steps in total:

- [Step 0: Installation](#step-0-installation)
- [Step 1: Writing my first Move module](#step-1-writing-my-first-move-module)
- [Step 2: Adding unit tests to my first Move module](#step-2-adding-unit-tests-to-my-first-move-module)
- [Step 3: Designing my `BasicCoin` module](#step-3-designing-my-basiccoin-module)
- [Step 4: Implementing my `BasicCoin` module](#step-4-implementing-my-basiccoin-module)
- [Step 5: Adding and using unit tests with the `BasicCoin` module](#step-5-adding-and-using-unit-tests-with-the-basiccoin-module)
- [Step 6: Making my `BasicCoin` module generic](#step-6-making-my-basiccoin-module-generic)

Each step is designed to be self-contained in the corresponding `step_x` folder. For example, if you would
like to skip the contents in step 1 through 4, feel free to jump to step 5 since all the code we have written
before step 5 will be in `step_5` folder. At the end of some steps, we also include
additional material on more advanced topics.

Now let's get started!

## Step 0: Installation

If you haven't already, install the:

* [Aptos CLI](../../../../cli-tools/aptos-cli-tool/index.md)
* [Move Prover](../../../../cli-tools/install-move-prover.md)

If you want to find what commands are available and what they do, running
a command or subcommand with the `--help` flag will print documentation.

Before running the next steps, `cd` to the tutorial directory:
```bash
cd tutorial
```

## Step 1: Writing my first Move module

Change directory into the [`step_1/BasicCoin`](./step_1/BasicCoin) directory.
You should see a directory called `sources` -- this is the place where all
the Move code for this package lives. You should also see a
`Move.toml` file as well. This file specifies dependencies and other information about
the package; if you're familiar with Rust and Cargo, the `Move.toml` file
is similar to the `Cargo.toml` file, and the `sources` directory similar to
the `src` directory.

Let's take a look at some Move code! Open up
[`sources/FirstModule.move`](./step_1/BasicCoin/sources/FirstModule.move) in
your editor of choice. The first thing you'll see is this:

```
// sources/FirstModule.move
module 0xCAFE::BasicCoin {
    ...
}
```

This is defining a Move
[module](https://move-language.github.io/move/modules-and-scripts.html). Modules are the
building blocks of Move code, and are defined with a specific address -- the
address that the module can be published under. In this case, the `BasicCoin`
module can only be published under `0xCAFE`.

Let's now take a look at the next part of this file where we define a
[struct](https://move-language.github.io/move/structs-and-resources.html)
to represent a `Coin` with a given `value`:

```
module 0xCAFE::BasicCoin {
    struct Coin has key {
        value: u64,
    }
    ...
}
```

Looking at the rest of the file, we see a function definition that creates a `Coin` struct and stores it under an account:

```
module 0xCAFE::BasicCoin {
    struct Coin has key {
        value: u64,
    }

    public fun mint(account: signer, value: u64) {
        move_to(&account, Coin { value })
    }
}
```

Let's take a look at this function and what it's saying:
* It takes a [`signer`](https://move-language.github.io/move/signer.html) -- an
  unforgeable token that represents control over a particular address, and
  a `value` to mint.
* It creates a `Coin` with the given value and stores it under the
  `account` using the `move_to` operator.

Let's make sure it compiles! This can be done with the `compile` command:

```bash
aptos move compile --package-dir step_1/BasicCoin
```

<details>
<summary>Advanced concepts and references</summary>

* Move supports the idea of [named
  addresses](https://move-language.github.io/move/address.html#named-addresses), Named
  addresses are a way to parametrize Move source code so that you can compile
  the module using different values for `NamedAddr` to get different bytecode
  that you can deploy, depending on what address(es) you control. They are used quite frequently, and can be defined in the `Move.toml` file in the `[addresses]` section, e.g.,
    ```
    [addresses]
    SomeNamedAddress = "0xC0FFEE"
    ```
* [Structures](https://move-language.github.io/move/structs-and-resources.html) in Move
  can be given different
  [abilities](https://move-language.github.io/move/abilities.html) that describe what
  can be done with that type. There are four different abilities:
    - `copy`: Allows values of types with this ability to be copied.
    - `drop`: Allows values of types with this ability to be popped/dropped.
    - `store`: Allows values of types with this ability to exist inside a struct in global storage.
    - `key`: Allows the type to serve as a key for global storage operations.

    So in the `BasicCoin` module we are saying that the `Coin` struct can be used as a key
    in global storage and, because it has no other abilities, it cannot be
    copied, dropped, or stored as a non-key value in storage. So you can't copy
    coins, and you also can't lose coins by accident!
* [Functions](https://move-language.github.io/move/functions.html) are default
    private, and can also be `public`,
    [`public(friend)`](https://move-language.github.io/move/friends.html), or
    `public(script)`. The last of these states that this function can be
    called from a transaction script. `public(script)` functions can also be
    called by other `public(script)` functions.
* `move_to` is one of the [five different global storage operators](https://move-language.github.io/move/global-storage-operators.html).

</details>

## Step 2: Adding unit tests to my first Move module

Now that we've taken a look at our first Move module, we'll take a look at a
test to make sure minting works the way we expect it to by changing directory
to [`step_2/BasicCoin`](./step_2/BasicCoin).  Unit tests in Move are similar to
unit tests in Rust if you're familiar with them -- tests are annotated with
`#[test]` and written like normal Move functions.

You can run the tests with the `aptos move test` command:

```bash
aptos move test --package-dir step_2/BasicCoin
```

Let's now take a look at the contents of the [`FirstModule.move`
file](./step_2/BasicCoin/sources/FirstModule.move). The first new thing you'll
see is this test:

```
module 0xCAFE::BasicCoin {
    ...
    // Declare a unit test. It takes a signer called `account` with an
    // address value of `0xC0FFEE`.
    #[test(account = @0xC0FFEE)]
    fun test_mint_10(account: signer) acquires Coin {
        let addr = 0x1::signer::address_of(&account);
        mint(account, 10);
        // Make sure there is a `Coin` resource under `addr` with a value of `10`.
        // We can access this resource and its value since we are in the
        // same module that defined the `Coin` resource.
        assert!(borrow_global<Coin>(addr).value == 11, 0);
    }
}
```

This is declaring a unit test called `test_mint_10` that mints a `Coin` struct
under the `account` with a `value` of `11`. It is then checking that the minted
coin in storage has the value that is expected with the `assert!` call. If the
assertion fails the unit test will fail.
This test is expected to fail - by changing line 24 to `assert!(borrow_global<Coin>(addr).value == 10, 0),`
the test will pass. 

<details>
<summary>Advanced concepts and exercises</summary>

* There are a number of test-related annotations that are worth exploring, they
  can be found
  [here](https://github.com/move-language/move/blob/main/language/changes/4-unit-testing.md#testing-annotations-their-meaning-and-usage).
  You'll see some of these used in Step 5.


</details>

## Step 3: Designing my `BasicCoin` module

In this section, we are going to design a module implementing a basic coin and balance interface, where coins can
be minted and transferred between balances held under different addresses.

The signatures of the public Move function are the following:

```
/// Publish an empty balance resource under `account`'s address. This function must be called before
/// minting or transferring to the account.
public fun publish_balance(account: &signer) { ... }

/// Mint `amount` tokens to `mint_addr`. Mint must be approved by the module owner.
public fun mint(module_owner: &signer, mint_addr: address, amount: u64) acquires Balance { ... }

/// Returns the balance of `owner`.
public fun balance_of(owner: address): u64 acquires Balance { ... }

/// Transfers `amount` of tokens from `from` to `to`.
public fun transfer(from: &signer, to: address, amount: u64) acquires Balance { ... }
```

Next we look at the data structs we need for this module.

A Move module doesn't have its own storage. Instead, Move "global storage" (what we call our
blockchain state) is indexed by addresses. Under each address there are Move modules (code) and Move resources (values).

The global storage looks roughly like this in Rust syntax:

```rust
struct GlobalStorage {
    resources: Map<address, Map<ResourceType, ResourceValue>>
    modules: Map<address, Map<ModuleName, ModuleBytecode>>
}
```

The Move resource storage under each address is a map from types to values. (An observant reader might observe that
this means each address can only have one value of each type.) This conveniently provides us a native mapping indexed
by addresses. In our `BasicCoin` module, we define the following `Balance` resource representing the number of coins
each address holds:

```
/// Struct representing the balance of each address.
struct Balance has key {
    coin: Coin // same Coin from Step 1
}
```

Roughly the Move blockchain state should look like this:

![](diagrams/move_state.png)

#### Advanced topics:
<details>
<summary><code>public(script)</code> functions</summary>

Only functions with `public(script)` visibility can be invoked directly in transactions. So if you would like to call the `transfer`
method directly from a transaction, you'll want to change its signature to:
```
public(script) fun transfer(from: signer, to: address, amount: u64) acquires Balance { ... }
```
Read more on Move function visibilities [here](https://move-language.github.io/move/functions.html#visibility).
</details>
<details>
<summary>Comparison with Ethereum/Solidity</summary>

In most Ethereum [ERC-20]((https://ethereum.org/en/developers/docs/standards/tokens/erc-20/)) contracts, the balance of each address is stored in a _state variable_ of type
<code>mapping(address => uint256)</code>. This state variable is stored in the storage of a particular smart contract.

The Ethereum blockchain state might look like this:

![](diagrams/solidity_state.png)
</details>

## Step 4: Implementing my `BasicCoin` module

We have created a Move package for you in folder `step_4` called `BasicCoin`. The `sources` folder contains source code for
all your Move modules in the package, including `BasicCoin.move`. In this section, we will take a closer look at the
implementation of the methods inside [`BasicCoin.move`](./step_4/sources/BasicCoin.move).

### Compiling our code

Let's first try building the code using Move package by running the following command
in [`step_4/BasicCoin`](./step_4/BasicCoin) folder:
```bash
aptos move compile --package-dir step_4/BasicCoin
```

### Implementation of methods
Now let's take a closer look at the implementation of the methods inside [`BasicCoin.move`](./step_4/BasicCoin/sources/BasicCoin.move).

<details>
<summary>Method <code>publish_balance</code></summary>

This method publishes a `Balance` resource to a given address. Since this resource is needed to receive coins through
minting or transferring, `publish_balance` method must be called by a user before they can receive money, including the
module owner.

This method uses a `move_to` operation to publish the resource:

```
let empty_coin = Coin { value: 0 };
move_to(account, Balance { coin:  empty_coin });
```
</details>
<details>
<summary>Method <code>mint</code></summary>

`mint` method mints coins to a given account. Here we require that `mint` must be approved
by the module owner. We enforce this using the assert statement:
```
assert!(signer::address_of(&module_owner) == MODULE_OWNER, ENOT_MODULE_OWNER);
```
Assert statements in Move can be used in this way: `assert!(<predicate>, <abort_code>);`. This means that if the `<predicate>`
is false, then abort the transaction with `<abort_code>`. Here `MODULE_OWNER` and `ENOT_MODULE_OWNER` are both constants
defined at the beginning of the module. The standard library's [`error` module] also defines common error categories we can use.
It is important to note that Move is transactional in its execution -- so
if an [abort](https://move-language.github.io/move/abort-and-assert.html) is raised no unwinding of state
needs to be performed, as no changes from that transaction will be persisted to the blockchain.

[`error` module]: https://github.com/move-language/move/blob/main/language/move-stdlib/docs/error.md

We then deposit a coin with value `amount` to the balance of `mint_addr`.
```
deposit(mint_addr, Coin { value: amount });
```
</details>

<details>
<summary>Method <code>balance_of</code></summary>

We use `borrow_global`, one of the global storage operators, to read from the global storage.
```
borrow_global<Balance>(owner).coin.value
                 |       |       \    /
        resource type  address  field names
```
</details>

<details>
<summary>Method <code>transfer</code></summary>

This function withdraws tokens from `from`'s balance and deposits the tokens into `to`s balance. We take a closer look
at `withdraw` helper function:
```
fun withdraw(addr: address, amount: u64) : Coin acquires Balance {
    let balance = balance_of(addr);
    assert!(balance >= amount, EINSUFFICIENT_BALANCE);
    let balance_ref = &mut borrow_global_mut<Balance>(addr).coin.value;
    *balance_ref = balance - amount;
    Coin { value: amount }
}
```
At the beginning of the method, we assert that the withdrawing account has enough balance. We then use `borrow_global_mut`
to get a mutable reference to the global storage, and `&mut` is used to create a [mutable reference](https://move-language.github.io/move/references.html) to a field of a
struct. We then modify the balance through this mutable reference and return a new coin with the withdrawn amount.
</details>

### Exercises
There are two `TODO`s in our module, left as exercises for the reader:
- Finish implementing the `publish_balance` method.
- Implement the `deposit` method.

The solution to this exercise can be found in [`step_4_sol`](./step_4_sol) folder.

**Bonus exercise**
- What would happen if we deposit too many tokens to a balance?

## Step 5: Adding and using unit tests with the `BasicCoin` module

In this step we're going to take a look at all the different unit tests
we've written to cover the code we wrote in step 4. We're also going to
take a look at some tools we can use to help us write tests.

To get started, run the `package test` command in the [`step_5/BasicCoin`](./step_5/BasicCoin) folder

```bash
aptos move test --package-dir step_5/BasicCoin
```

You should see something like this:

```
INCLUDING DEPENDENCY MoveStdlib
BUILDING BasicCoin
Running Move unit tests
[ PASS    ] 0xcafe::BasicCoin::can_withdraw_amount
[ PASS    ] 0xcafe::BasicCoin::init_check_balance
[ PASS    ] 0xcafe::BasicCoin::init_non_owner
[ PASS    ] 0xcafe::BasicCoin::publish_balance_already_exists
[ PASS    ] 0xcafe::BasicCoin::publish_balance_has_zero
[ PASS    ] 0xcafe::BasicCoin::withdraw_dne
[ PASS    ] 0xcafe::BasicCoin::withdraw_too_much
Test result: OK. Total tests: 7; passed: 7; failed: 0
```

Taking a look at the tests in the
[`BasicCoin` module](./step_5/BasicCoin/sources/BasicCoin.move) we've tried
to keep each unit test to testing one particular behavior.

<details>
<summary>Exercise</summary>

After taking a look at the tests, try and write a unit test called
`balance_of_dne` in the `BasicCoin` module that tests the case where a
`Balance` resource doesn't exist under the address that `balance_of` is being
called on. It should only be a couple lines!

The solution to this exercise can be found in [`step_5_sol`](./step_5_sol)

</details>

## Step 6: Making my `BasicCoin` module generic

In Move, we can use generics to define functions and structs over different input data types. Generics are a great
building block for library code. In this section, we are going to make our simple `BasicCoin` module generic so that it can
serve as a library module that can be used by other user modules.

First, we add type parameters to our data structs:
```
struct Coin<phantom CoinType> has store {
    value: u64
}

struct Balance<phantom CoinType> has key {
    coin: Coin<CoinType>
}
```

We also add type parameters to our methods in the same manner. For example, `withdraw` becomes the following:
```
fun withdraw<CoinType>(addr: address, amount: u64) : Coin<CoinType> acquires Balance {
    let balance = balance_of<CoinType>(addr);
    assert!(balance >= amount, EINSUFFICIENT_BALANCE);
    let balance_ref = &mut borrow_global_mut<Balance<CoinType>>(addr).coin.value;
    *balance_ref = balance - amount;
    Coin<CoinType> { value: amount }
}
```
Take a look at [`step_6/BasicCoin/sources/BasicCoin.move`](./step_6/BasicCoin/sources/BasicCoin.move) to see the full implementation.

At this point, readers who are familiar with Ethereum might notice that this module serves a similar purpose as
the [ERC20 token standard](https://ethereum.org/en/developers/docs/standards/tokens/erc-20/), which provides an
interface for implementing fungible tokens in smart contracts. One key advantage of using generics is the ability
to reuse code since the generic library module already provides a standard implementation and the instantiating module
can provide customizations by wrapping the standard implementation.

We provide a little module called [`MyOddCoin`](./step_6/BasicCoin/sources/MyOddCoin.move) that instantiates
the `Coin` type and customizes its transfer policy: only odd number of coins can be transferred. We also include two
[tests](./step_6/BasicCoin/sources/MyOddCoin.move) to test this behavior. You can use the commands you learned in step 2 and step 5 to run the tests.

#### Advanced topics:
<details>
<summary><code>phantom</code> type parameters</summary>

In definitions of both `Coin` and `Balance`, we declare the type parameter `CoinType`
to be phantom because `CoinType` is not used in the struct definition or is only used as a phantom type
parameter.

Read more about phantom type parameters <a href="https://move-language.github.io/move/generics.html#phantom-type-parameters">here</a>.
</details>
    

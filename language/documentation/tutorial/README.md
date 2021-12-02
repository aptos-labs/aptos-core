# Move Tutorial

Welcome to the Move Tutorial! In this tutorial, we are going to go through some steps of developing Move code
including design, implementation, unit testing and formal verification of Move modules.

There are eight steps in total:

- [Step 0: Installation](#Step0)
- [Step 1: Writing my first Move module](#Step1)
- [Step 2: Adding unit tests to my first Move module](#Step2)
- [Step 3: Designing my `BasicCoin` module](#Step3)
- [Step 4: Implementing my `BasicCoin` module](#Step4)
- [Step 5: Adding and using unit tests with the `BasicCoin` module](#Step5)
- [Step 6: Making my `BasicCoin` module generic](#Step6)
- [Step 7: Writing formal specifications for my `BasicCoin` module](#Step7)

Each step is designed to be self-contained in the corresponding `step_x` folder. For example, if you would
like to skip the contents in step 1 through 4, feel free to jump to step 5 since all the code we have written
before step 5 will be in `step_5` folder. At the end of some steps, we also include
additional material on more advanced topics.

Now let's get started!

## Step 0: Installation<span id="Step0"><span>


- Open up your terminal of choice and clone [`diem`](https://github.com/diem/diem) repo if you don't already have it:
```bash
$ git clone https://github.com/diem/diem.git
```

- Go to `diem` folder and run the `dev_setup.sh` script:
```bash
$ cd diem && sh scripts/dev_setup.sh -ypt
```

Follow the prompt to install all the necessary dependencies.

- Include environment variable definitions in `~/.profile` by running this command:
```bash
$ . ~/.profile
````
- Install Move CLI by running this command in diem repo:
```bash
$ cargo build -p df-cli
```

Once this is done, you can alias the `move` command to point the `df-cli`
binary:

```bash
$ alias move="<path_to_diem_repo>/target/debug/df-cli"
```

You can check that it is working by running

```bash
$ move package -h
```

You should see something like this along with a list and description of a
number of commands:

```
move-package 0.1.0
Package and build system for Move code.

USAGE:
    move package [FLAGS] [OPTIONS] <SUBCOMMAND>
...
```

There is official Move support for VSCode, you can install this extension
by opening VSCode and searching for the "move-analyzer" package and
installing it. More detailed instructions can be found
[here](https://github.com/diem/diem/tree/main/language/move-analyzer/editors/code).

Before running the next steps, `cd` to the tutorial directory:
```bash
$ cd <path_to_diem_repo>/language/documentation/tutorial/
```

## Step 1: Writing my first Move module<span id="Step1"><span>

To create your first Move module, we first need to create a Move package by
calling

```bash
$ move package new <pkg_name>
```

Now change directory into the package you just created

```bash
$ cd <pkg_name>
```

You should see a directory called `sources` -- this is the place where all
the Move code for this package will live[1]. You should also see a
`Move.toml` file as well. This file specifies dependencies and other information about
the package; if you're familiar with Rust and Cargo, the `Move.toml` file
is similar to the `Cargo.toml` file, and the `sources` directory similar to
the `src` directory. More information on the `Move.toml` file can be found
[here](https://diem.github.io/move/packages.html#movetoml).

Let's write some Move code! Open up `sources/FirstModule.move` in your
editor of choice.

[Modules](https://diem.github.io/move/modules-and-scripts.html) are the
building block of Move code, and they are always defined relative to a
specific address -- the address that they can be published under. So let's
start out by defining our first module, and look at the different parts:

```rust
// sources/FirstModule.move
module NamedAddr::BasicCoin {
}
```

This is defining the module `BasicCoin` that can be published under the
[named address](https://diem.github.io/move/address.html#named-addresses)
`NamedAddr`. Named addresses are a way to parametrize Move source code so
that you can compile the module using different values for `NamedAddr` to
get different bytecode that you can deploy, depending on what address(es)
you control.

Define and assign the named address `NamedAddr` the value `0xDEADBEEF`.
You can do this by opening the `Move.toml` in your favorite editor and adding the
following to the bottom of it:

```
[addresses]
NamedAddr = "0xDEADBEEF"
```

Let's now see if it works by building it!

```bash
$ move package build
```

We're now now going to define a
[structure](https://diem.github.io/move/structs-and-resources.html) in this
module to represent a `Coin` with a given `value`:

```
module NamedAddr::BasicCoin {
    struct Coin has key {
        value: u64,
    }
}
```

Structures in Move can be given different
[abilities](https://diem.github.io/move/abilities.html) that describe what
can be done with that type. There are four different abilities:
* `copy`: Allows values of types with this ability to be copied.
* `drop`: Allows values of types with this ability to be popped/dropped.
* `store`: Allows values of types with this ability to exist inside a struct in global storage.
* `key`: Allows the type to serve as a key for global storage operations.

So in this module we are saying that the `Coin` struct can be used as a key
in global storage and, because it has no other abilities, it cannot be
copied, dropped, or stored as a non-key value in storage. So you can't copy
coins, and you also can't lose coins by accident!

Check that it can build again:

```bash
$ move package build
```

Let's now add a function to this module that mints a `Coin` and stores it
under an account.

```
module NamedAddr::BasicCoin {
    struct Coin has key {
        value: u64,
    }

    public fun mint(account: signer, value: u64) {
        move_to(&account, Coin { value })
    }
}
```

Let's take a look at this function and what it's saying:
* It takes a [`signer`](https://diem.github.io/move/signer.html) -- an
  unforgeable token that represents control over a particular address, and
  a `value` to mint.
* It creates a `Coin` with the given value and stores it under the
  `account` using one of the [five different global storage
  operators](https://diem.github.io/move/global-storage-operators.html)
  `move_to`. This is where the `key` ability is important -- we couldn't
  call `move_to` on `Coin` unless it had the `key` ability!

Let's make sure it compiles again:

```bash
$ move package build
```

## Step 2: Adding unit tests to my first Move module<span id="Step2"><span>

Now that we've written our first Move module, we'll write a test to
make sure minting works the way we expect it to.

Unit tests in Move are similar to unit tests in Rust if you're familiar with
them. There are a number of [test-related
annotations that are worth exploring](https://github.com/diem/diem/blob/main/language/changes/4-unit-testing.md#testing-annotations-their-meaning-and-usage).
Unit tests can be run with the `move package test` command. We'll see how
they're used shortly, but we first need to bring in a dependency.

#### Adding dependencies

Before running unit tests, we need to add a dependency on the Move standard
library. This can be done by adding an entry to the `[dependencies]`
section of the `Move.toml`. Add the following to the bottom of the
`Move.toml` file:

```toml
[dependencies]
MoveStdlib = { local = "../../../../move-stdlib/", addr_subst = { "Std" = "0x1" } }
```

Note that you may need to alter the path to point to the `move-stdlib` directory under
`$DIEM_HOME/language`.

You can read more on Move package dependencies
[here](https://diem.github.io/move/packages.html#movetoml).

Once you've added this to the `Move.toml` file you should be able to run
`move package test`. You'll see something like this:

```
BUILDING MoveStdlib
BUILDING BasicCoin
Running Move unit tests
Test result: OK. Total tests: 0; passed: 0; failed: 0
```

Let's now add a test to make sure that `BasicCoin::mint(account, 10)`
stores a `Coin` resource with a value of `10` under `account`. We can do
this by adding the following to our `BasicCoin` module:

```
module NamedAddr::BasicCoin {
    // Only included in compilation for testing. Similar to #[cfg(test)] in Rust.
    #[test_only]
    use Std::Signer;
    ...
    // Declare a unit test. It takes a signer called `account` with an
    // address value of `0xCAFE`.
    #[test(account = @0xCAFE)]
    fun test_mint_10(account: signer) acquires Coin {
        let addr = Signer::address_of(&account);
        mint(account, 10);
        // Make sure there is a `Coin` resource under `addr` with a value of `10`.
        assert!(borrow_global<Coin>(addr).value == 10, 0);
    }
}
```

#### Exercise
* Change the assertion to `11` so that the test fails. Find a flag that you can
  pass to the `move package test` command that will show you the global state when
  the test fails. It should look something like this:
  ```
    ┌── test_mint_10 ──────
    │ error[E11001]: test failure
    │    ┌─ step_2/BasicCoin/sources/FirstModule.move:22:9
    │    │
    │ 18 │     fun test_mint_10(account: signer) acquires Coin {
    │    │         ------------ In this function in 0xdeadbeef::BasicCoin
    │    ·
    │ 22 │         assert!(borrow_global<Coin>(addr).value == 11, 0);
    │    │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Test was not expected to abort but it aborted with 0 here
    │
    │
    │ ────── Storage state at point of failure ──────
    │ 0xcafe:
    │       => key 0xdeadbeef::BasicCoin::Coin {
    │           value: 10
    │       }
    │
    └──────────────────
  ```
* [Bonus] Find a flag that allows you to gather test coverage information, and
  then play around with using the `move package coverage` command to look at
  coverage statistics and source coverage.

## Step 3: Designing my `BasicCoin` module<span id="Step3"><span>

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

Only functions with `public(script)` can be invoked directly in transactions. So if you would like to call the `transfer`
method directly from a transaction, you'll want to change its signature to:
```
public(script) fun transfer(from: signer, to: address, amount: u64) acquires Balance { ... }
```
Read more on Move function visibilities [here](https://diem.github.io/move/functions.html#visibility).
</details>
<details>
<summary>Comparison with Ethereum/Solidity</summary>

In most Ethereum [ERC-20]((https://ethereum.org/en/developers/docs/standards/tokens/erc-20/)) contracts, the balance of each address is stored in a _state variable_ of type
<code>mapping(address => uint256)</code>. This state variable is stored in the storage of a particular smart contract.

The Ethereum blockchain state might look like this:

![](diagrams/solidity_state.png)
</details>

## Step 4: Implementing my `BasicCoin` module<span id="Step4"><span>

We have created a Move package for you in folder `step_4` called `BasicCoin`. The `sources` folder contains source code for
all your Move modules. `BasicCoin.move` lives inside this folder. In this section, we will take a closer look at the
implementation of the methods inside [`BasicCoin.move`](./step_4/BasicCoin/sources/BasicCoin.move).

### Compiling our code

Let's first try building the code using Move package by running the following command
in [`step_4/BasicCoin`](./step_4/BasicCoin) folder:
```bash
$ move package build
```

### Implementation of methods
Now let's take a closer look at the implementation of the methods inside `BasicCoin.move`.

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
assert!(Signer::address_of(&module_owner) == MODULE_OWNER, Errors::requires_address(ENOT_MODULE_OWNER));
```
Assert statements in Move can be used in this way: `assert!(<predicate>, <abort_code>);`. This means that if the `<predicate>`
is false, then abort the transaction with `<abort_code>`. Here `MODULE_OWNER` and `ENOT_MODULE_OWNER` are both constants
defined at the beginning of the module. And `Errors` module defines common error categories we can use.
It is important to note that Move is transactional in its execution -- so
if an [abort](https://diem.github.io/move/abort-and-assert.html) is raised no unwinding of state
needs to be performed, as no changes from that transaction will be persisted to the blockchain.

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
to get a mutable reference to the global storage, and `&mut` is used to create a [mutable reference](https://diem.github.io/move/references.html) to a field of a
struct. We then modify the balance through this mutable reference and return a new coin with the withdrawn amount.
</details>

### Exercises
There are two `TODO`s in our module, left as exercises for the reader:
- Finish implementing the `publish_balance` method.
- Implement the `deposit` method.

The solution to this exercise can be found in [`step_4_sol`](./step_4_sol) folder.

**Bonus exercise**
- What would happen if we deposit too many tokens to a balance?

## Step 5: Adding and using unit tests with the `BasicCoin` module<span id="Step5"><span>

In this step we're going to take a look at all the different unit tests
we've written to cover the code we wrote in step 4. We're also going to
take a look at some tools we can use to help us write tests.

To get started, run `move package test` in the [`step_5/BasicCoin`](./step_5/BasicCoin) folder. You should see
something like this:

```
BUILDING MoveStdlib
BUILDING BasicCoin
Running Move unit tests
[ PASS    ] 0xdeadbeef::BasicCoin::can_withdraw_amount
[ PASS    ] 0xdeadbeef::BasicCoin::init_check_balance
[ PASS    ] 0xdeadbeef::BasicCoin::init_non_owner
[ PASS    ] 0xdeadbeef::BasicCoin::publish_balance_already_exists
[ PASS    ] 0xdeadbeef::BasicCoin::publish_balance_has_zero
[ PASS    ] 0xdeadbeef::BasicCoin::withdraw_dne
[ PASS    ] 0xdeadbeef::BasicCoin::withdraw_too_much
Test result: OK. Total tests: 7; passed: 7; failed: 0
```

Taking a look at the tests in the
[`BasicCoin` module](./step_5/BasicCoin/sources/BasicCoin.move) we've tried
to keep each unit test to testing one particular behavior.

After taking a look at the tests, try and complete the following exercise, it
should only be a couple lines!

### Exercise
* Write a unit test called `balance_of_dne` in the `BasicCoin` module that tests
  the case where a `Balance` resource doesn't exist under the address that
  `balance_of` is being called on.

The solution to this exercise can be found in [`step_5_sol`](./step_5_sol)

## Step 6: Making my `BasicCoin` module generic<span id="Step6"><span>

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
the `Coin` type and customizes its transfer policy: only odd number of coins can be transferred.

#### Advanced topics:
<details>
<summary><code>phantom</code> type parameters</summary>

In definitions of both `Coin` and `Balance`, we declare the type parameter `CoinType`
to be phantom because `CoinType` is not used in the struct definition or is only used as a phantom type
parameter.

Read more about phantom type parameters <a href="https://diem.github.io/move/generics.html#phantom-type-parameters">here</a>.
</details>

## Advanced steps

## Step 7: Writing formal specifications for my `BasicCoin` module<span id="Step7"><span>

The blockchain requires high assurance. Smart contracts deployed on the blockchain may maniputate high-value assets, which are targets of highly-motivated and well-resourced adversaries. Hundreds of millions of dollars have been lost from bugs on other blockchains. As a technique that uses strict mathematical methods to describe behavior and reason correctness of computer systems, formal verification has been used in blockchains to prevent bugs in smart contracts. [The Move prover](https://github.com/diem/diem/tree/main/language/move-prover) is an evolving formal verification tool for smart contracts written in the Move language. It supports complete specification of functional properties of smart contracts. Properties can be verified automatically efficiently (only slightly slower than a linter). Moreover, it can be integrated in the CI system for re-verification after every change to the code base. In this step, we will define the formal specification of the `BasicCoin` module.

The property specification is written in the [Move Specification Language (MSL)](https://github.com/diem/diem/blob/main/language/move-prover/doc/user/spec-lang.md). Developers can provide pre and post conditions for functions, which include conditions over (mutable) parameters and global memory. Developers can also provide invariants over data structures, as well as the (state-dependent) content of the global memory. Universal and existential quantification both over bounded domains (like the indices of a vector) as well of unbounded domains (like all memory addresses, all integers, etc.) are supported. In this tutorial, we will learn how to define functional properties for methods.

#### Method `withdraw`

The signature of the method `withdraw` is given below:
```
fun withdraw<CoinType>(addr: address, amount: u64) : Coin<CoinType> acquires Balance
```

The method withdraws tokens with value `amount` from the address `addr` and returns a created Coin of value `amount`. The specification is defined in the `spec withdraw` block:

```
   spec withdraw {
        // The property of the method withdraw is defined here.
    }
```

For a function, we usually want to define when it aborts, the expected effect on the global memory, and its return value. MSL provides `aborts_if` to define conditions under which the function aborts. The method `withdraw` aborts when 1) `addr` does not have the resource `Balance<CoinType>` or 2) the number of tokens in `addr` is smaller than `amount`. We can define conditions like this:

```
   spec withdraw {
        let balance = global<Balance<CoinType>>(addr).coin.value;
        aborts_if !exists<Balance<CoinType>>(addr);
        aborts_if balance < amount;
    }
```

As we can see here, a spec block can contain let bindings which introduce names for expressions. `global<T>(address): T` is a built-in function that returns the resource value at `addr`. `balance` is the number of tokens owned by `addr`. `exists<T>(address): bool` is a built-in function that returns true if the resource T exists at address. Two `aborts_if` clauses correspond to the two conditions mentioned above. In general, if a function has more than one `aborts_if` condition, those conditions are or-ed with each other. By default, if a user wants to specify aborts conditions, all possible conditions need to be listed. Otherwise, the prover will generate a verification error. However, if `pragma aborts_if_is_partial` is defined in the spec block, the combined aborts condition (the or-ed individual conditions) only *imply* that the function aborts. The reader can refer to the [MSL](https://github.com/diem/diem/blob/main/language/move-prover/doc/user/spec-lang.md) document for more information.

The next step is to define functional properties, which are described in the two `ensures` clauses below. First, by using the `let post` binding, `balance_post` represents the balance of `addr` after the execution, which should be equal to `balance - amount`. Then, the return value (denoted as `result`) should be a coin with value `amount`.

```
   spec withdraw {
        let balance = global<Balance<CoinType>>(addr).coin.value;
        aborts_if !exists<Balance<CoinType>>(addr);
        aborts_if balance < amount;

        let post balance_post = global<Balance<CoinType>>(addr).coin.value;
        ensures balance_post == balance - amount;
        ensures result == Coin<CoinType> { value: amount };
    }
```


#### Method `deposit`


The signature of the method `deposit` is given below:

```
fun deposit<CoinType>(addr: address, check: Coin<CoinType>) acquires Balance
```

The method deposits the `check` into `addr`. The specification is defined below:

```
    spec deposit {
        let balance = global<Balance<CoinType>>(addr).coin.value;
        let check_value = check.value;

        aborts_if !exists<Balance<CoinType>>(addr);
        aborts_if balance + check_value > MAX_U64;

        let post balance_post = global<Balance<CoinType>>(addr).coin.value;
        ensures balance_post == balance + check_value;
    }
```

`balance` represents the number of tokens in `addr` before execution and `check_value` represents the number of tokens to be deposited. The method would abort if 1) `addr` does not have the resource `Balance<CoinType>` or 2) the sum of `balance` and `check_value` is greater than the maxium value of the type `u64`. The functional property checks that the balance is correctly updated after the execution.


#### Method `transfer`

The signature of the method `transfer` is given below:

```
public fun transfer<CoinType: drop>(from: &signer, to: address, amount: u64, _witness: CoinType) acquires Balance
```

The method transfers the `amount` of coin from the account of `from` to the address `to`.

```
spec transfer {
        let addr_from = Signer::address_of(from);

        let balance_from = global<Balance<CoinType>>(addr_from).coin.value;
        let balance_to = global<Balance<CoinType>>(to).coin.value;

        let post balance_from_post = global<Balance<CoinType>>(addr_from).coin.value;
        let post balance_to_post = global<Balance<CoinType>>(to).coin.value;

        ensures addr_from != to ==> balance_from_post == balance_from - amount;
        ensures addr_from != to ==> balance_to_post == balance_to + amount;
        ensures addr_from == to ==> balance_from_post == balance_from;
    }
```

The function `Signer::address_of` is called to obtain the address of `from`. Then the balances of `addr_from` and `to` before and after the execution are obtained. In the three `ensures` clauses, `p ==> q` is used to represented the logical implication between p and q. If the source and the target addresses are the same, the balance remains the same. Otherwise, `amount` is deducted from `addr_from` and added to `to`. The aborts conditions are left as an exercise.


### Exercises
- Implement the `aborts_if` conditions for the `transfer` method.
- Implement the specification for the `mint` and `publish_balance` method.

The solution to this exercise can be found in [`step_7_sol`](./step_7_sol).

### Step 8: Formally verify the `BasicCoin` module using the Move Prover

We can use the command `move package -p <path/to/BasicCoin> prove` to prove properties for the BasicCoin module. More prover options can be found [here](https://github.com/diem/diem/blob/main/language/move-prover/doc/user/prover-guide.md).

## Footnotes
---------------------------------------------------------------------------
[1] Move code can also live a number of other places, but for more
information on that see the [documentation on Move
packages](https://diem.github.io/move/packages.html).

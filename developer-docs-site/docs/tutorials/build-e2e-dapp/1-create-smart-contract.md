---
title: "Create a smart contract"
id: "create-a-smart-contract"
---

# Create a smart contract

If you haven’t done it, you would want to Install Aptos CLI now. You can follow the instructions [here](../../cli-tools/aptos-cli-tool/automated-install-aptos-cli.md) - just make sure you use CLI version 1.0.4 as this what we use in this tutorial.

1.  `cd` into `my-first-dapp` root folder, and create a new `move` folder
2.  `cd` into the new `move` folder and run
    `aptos move init --name myTodolist`
    That would create a `sources/` folder and `Move.toml` file inside the `move` folder.
3.  Your new `move` folder should look like that

    ![move-folder](../../../static/img/docs/build-e2e-dapp-img-1.png)

### What is a `Move.toml` file?

`Move.toml` file is a manifest file that contains metadata such as name, version, and dependencies for the package.

Take a look at the new `Move.toml` file, you should see your package info (pay attention that the `name` prop is the same `--name` attribute we pass to the `aptos move init` command before) and an `AptosFramework` dependency. The AptosFramework dependency points to `aptos-core/aptos-move/framework/aptos-framework` github repo main branch.

### Why `sources` folder?

sources folder holds a collection of `.move` modules files and later when we would want to compile the package using the CLI, it will look for that `sources` file (and for the Move.toml file).

### Create a Move module

An account is needed to publish a Move module. So first we need to create an account, that we have its private key, so we can create a module under its account address and publish the module using that account.

1. On our `move` folder, run `aptos config init --network devnet`. Press enter when the prompt pops up.

   This creates for us a `.aptos` folder with a `config.yaml` file that holds our profile info. On the `config` file we know have our profiles list that holds a `default` profile.

   From now on, whenever we run a CLI command in this `move` folder, it will run with that default profile.
   We use the `devnet` network flag so eventually when we publish our package it would get published to the `devnet` network.

As mentioned, our `sources` folder holds our `.move` module files, so let’s add our first move file.

1. Open the `Move.toml` file
2. Add the following code into that file

```toml
[addresses]
myaddr='<default_profile_account_address>'
```

If the default profile account address is `cbddf398841353776903dbab2fdaefc54f181d07e114ae818b1a67af28d1b018`, your `Move.toml` file should look like that

```toml
[addresses]
myaddr='cbddf398841353776903dbab2fdaefc54f181d07e114ae818b1a67af28d1b018'
```

3. Create a new `main.move` file under the sources folder and add the following

```rust
module myaddr::main {

}
```

we are declaring a move module with the myaddr prop from our Move.toml file, so we can use that prop name as the address that holds that move module.

### Our contract logic

Before jumping into writing code, let’s first understand what we want our program (smart contract) to do. For simplicity we will keep the logic pretty simple.

1. An account creates a new list
2. An account creates a new task on their list
   - Whenever someone creates a new task, emit a task_created event
3. Ability for an account to mark their task as completed

:::tip
creating an event is not a mandatory thing to do, but is more for in case dapps/users want to monitor data, such as how many people create a new task, using [Indexer](https://aptos.dev/guides/indexing)
:::

We can start with defining a `TaskHolder` struct, that holds the

- tasks array
- new task event
- a task counter that counts the number of created tasks (we can use that to differentiate between the tasks)

And can also create a `Task` struct that holds

- the task id - derived from the task counter
- address - the account address who created that task
- content - the task content
- completed - a boolean that marks whether that task is completed or not

On the `main.move` file, update the content in the module with

```rust
...
struct TasksHolder has key {
    tasks: Table<u64, Task>,
    set_task_event: event::EventHandle<Task>,
    task_counter: u64
  }

struct Task has store, drop, copy {
    task_id: u64,
    address:address,
    content: String,
    completed: bool,
  }
...
```

**What did we just add?**

**TaskHolder**

A struct that has the `key` and `store` abilities

- `Key` ability allows struct to be used as a storage identifier. In other words, `key`
   is an ability to be stored as at top-level and be a storage. We need it here to have `TasksHolder` be a resource stored in our user account

When a struct has the `key` ability, it turns this struct into a `resource`

- `Resource` is stored under account - therefore it *exists* only when assigned to account; and can only be *accessed* through this account.

**Task**

A struct that has the `store`, `drop` and `copy`abilities.

• `Store` - Task needs Store as it’s stored inside another struct (TasksHolder)

• `Copy` - value can be *copied* (or cloned by value).

• `Drop` - value can be *dropped* by the end of scope.

Let’s try to compile what we have by now.

1. `cd` into the `move` folder
2. run `aptos move compile`

**Seeing errors?!** let’s understand.

we have some errors on `Unbound type`- that is because we use some types but never really imported them and the compiler doesnt know where to get it from.

3. On the top of the module, use those types by adding

```rust
...
use aptos_framework::event;
use std::string::String;
use aptos_std::table::Table;
...
```

that will tell the compiler where it can get those types from.

4. Run the aptos move compile command again, if all goes great, we should see a response like that (where the account address is your default profile account address)

```rust
INCLUDING DEPENDENCY AptosFramework
INCLUDING DEPENDENCY AptosStdlib
INCLUDING DEPENDENCY MoveStdlib
BUILDING myTodolist
{
"Result": [
    "cbddf398841353776903dbab2fdaefc54f181d07e114ae818b1a67af28d1b018::main"
  ]
}
```

At this point we have successfully compiled our Move module. Yay.

We also have a new `move/build` folder (created by the compiler) that holds our compiled modules, build info and sources folder.

### Create list function

The first thing an account can and should do with our contract is to create a new list.

Creating a task is basically submitting a transaction and so we need to know the `signer` who signed and submitted the transaction

1. Add a `create_list` function that accepts a `signer`

```rust
public entry fun create_list(account: &signer){

}
```

**Let’s understand some words in this function**

`entry` - an entry function is a function that can be called via transactions. In simple word, whenever you want to submit a transaction to chain, you should call an entry function.

`&signer` - This argument injected by the Move VM as the address who signed that transaction.

Our code has a `TaskHolder` resource. Resource is stored under account - therefore it *exists* only when assigned to account; and can only be *accessed* through this account.

That means, to create the `TaskHolder` resource we need to assign it to an account that only this account can have access to it.

The `create_list` function can handle that `TaskHolder` resource creation.

2. Add the following to the `create_list` function

```rust
public entry fun create_list(account: &signer){
  let tasks_holder = TasksHolder {
    tasks: table::new(),
    set_task_event: account::new_event_handle<Task>(account),
    task_counter: 0
  };
  // move the TasksHolder resource under the signer account
  move_to(account, tasks_holder);
}
```

This function takes in a signer, creates a new TaskHolder resource, and move_to to be stored in the provided signer account.

### Create task function

As mentioned before, our contract has a create task function that lets an account to create a new task. Creating a task is basically submitting a transaction and so we need to know the `signer` who signed and submitted the transaction. Another thing we want to accept in our function is the task `content`.

1. Add a `create_task` function that accepts a `signer` and task `content` and the function logic.

```rust
public entry fun create_task(account: &signer, content: String) acquires TasksHolder {
    // gets the signer address
    let signer_address = signer::address_of(account);
    // gets the TaskHolder resource
    let todo_list = borrow_global_mut<TasksHolder>(signer_address);
    // increment task counter
    let counter = todo_list.task_counter + 1;
    // creates a new Task
    let new_task = Task {
      task_id: counter,
      address: signer_address,
      content,
      completed: false
    };
    // adds the new task into the tasks table
    table::upsert(&mut todo_list.tasks, counter, new_task);
    // sets the task counter to be the incremented counter
    todo_list.task_counter = counter;
    // fires a new task created event
    event::emit_event<Task>(
      &mut borrow_global_mut<TasksHolder>(signer_address).set_task_event,
      new_task,
    );
  }
```

2. Since we now use 2 new modules - signer and table (you can see it being used in `signer::` and `table::`), we need to import these modules.
   At the top of the file, add those 2 use statements

```rust
use std::signer;
use aptos_std::table::{Self, Table}; // This one we already had, need to modify it
```

**Back to the code, what is happening here?**

First, we want to get the signer address so we can get this account’s `TaskHolder` resource.
Then, we retrieve the `TaskHolder` resource with the `signer_address` , with that we have access to the `TaskHolder` properties.
We can now increment the `task_counter` prop, and create a new `Task` with the `signer_address`, `counter` and the provided `content`.
We will push it to the `todo_list.tasks` table that holds all of our tasks along with the new `counter` (which is the table key) and the new created Task.
Then we assign the global `task_counter` to be the new incremented counter.
Finally, we emit the task created event that holds the new Task data.

### Toggle completed function

Another function we want our contract to hold, is the option to toggle a task as completed.

1. Add a `toggle_completed` function that accepts a signer and a task_id

```rust
public entry fun toggle_completed(account: &signer, task_id: u64) acquires TasksHolder {
  // gets the signer address
  let signer_address = signer::address_of(account);
  // gets the TaskHolder resource
  let todo_list = borrow_global_mut<TasksHolder>(signer_address);
  // gets the task matched the task_id
  let task_record = table::borrow_mut(&mut todo_list.tasks, task_id);
  // update task as completed
  task_record.completed = true;
}
```

**Let’s understand the code.**
As before in our create task function, we retrieve the TaskHolder struct by the signer address so we can have access to the tasks table that holds all of the account tasks. Then, we look for the task with the provided task_id on the todo_list.tasks table. Finally, we update that task completed prop to be true.

Let’s try to compile the code.

2. run `aptos move compile`
3. Another Unbound error? Let’s add a use statement to use the `account` module.

```rust
use aptos_framework::account;
```

4. run `aptos move compile` again.

### Add validations

As this code now compiles, we want to have some validations and checks before creating a new task or updating task as completed.

1. Add a check to the `create_task` function to make sure the signer account has a list

```rust
public entry fun create_task(account: &signer, content: String) acquires TasksHolder {
  // gets the signer address
  let signer_address = signer::address_of(account);

  // assert signer has created a list
  assert!(exists<TasksHolder>(signer_address), 1);

  ...
}
```

1. Add check to the `toggle_completed` function to make sure
   a. signer has created a list
   b. task exists
   c. task is not completed

```rust
public entry fun toggle_completed(account: &signer, task_id: u64) acquires TasksHolder {
  // gets the signer address
  let signer_address = signer::address_of(account);
  // assert signer has created a list
  assert!(exists<TasksHolder>(signer_address), 1);
  // gets the TaskHolder resource
  let todo_list = borrow_global_mut<TasksHolder>(signer_address);
  // assert task exists
  assert!(table::contains(&todo_list.tasks, task_id), 2);
  // gets the task matched the task_id
  let task_record = table::borrow_mut(&mut todo_list.tasks, task_id);
  // assert task is not completed
  assert!(task_record.completed == false, 3);
  // update task as completed
  task_record.completed = true;
}
```

We just added our first `assert` statements!

If you noticed, `assert` accepts 2 arguments. first is what to check for and the second is an error code. Instead of passing in an arbitrary number, a convention is to declare `errors` on the top of the module file and use these instead.

On the top of the module file (under the `use` statements), add those error declarations

```rust
// Errors
const E_NOT_INITIALIZED: u64 = 1;
const ETASK_DOESNT_EXIST: u64 = 2;
const ETASK_IS_COMPLETED: u64 = 3;
```

Now we can update our asserts with these constants

```rust
public entry fun create_task(account: &signer, content: String) acquires TasksHolder {
  // gets the signer address
  let signer_address = signer::address_of(account);

  // assert signer has created a list
  assert!(exists<TasksHolder>(signer_address), E_NOT_INITIALIZED);

  ...
}



public entry fun toggle_completed(account: &signer, task_id: u64) acquires TasksHolder {
  // gets the signer address
  let signer_address = signer::address_of(account);
  assert!(exists<TasksHolder>(signer_address), E_NOT_INITIALIZED);
  // gets the TaskHolder resource
  let todo_list = borrow_global_mut<TasksHolder>(signer_address);
  // assert task exists
  assert!(table::contains(&todo_list.tasks, task_id), ETASK_DOESNT_EXIST);
  // gets the task matched the task_id
  let task_record = table::borrow_mut(&mut todo_list.tasks, task_id);
  // assert task is not completed
  assert!(task_record.completed == false, ETASK_IS_COMPLETED);
  // update task as completed
  task_record.completed = true;
}
```

**WONDERFUL!!**

Let’s stop for one sec, and make sure our code compiles by running the `aptos move compile` command. If all goes good, we should see a something like that

```rust
INCLUDING DEPENDENCY AptosFramework
INCLUDING DEPENDENCY AptosStdlib
INCLUDING DEPENDENCY MoveStdlib
BUILDING myTodolist
{
"Result": [
    "cbddf398841353776903dbab2fdaefc54f181d07e114ae818b1a67af28d1b018::main"
  ]
}
```

If some errors, make sure you followed the steps and try to find what the issues are.

### Write Tests

Now that we have our smart contract logic ready, we need (and should) add some tests to test it.

Test functions use the `#[test]` annotation.

1. Add the following code to bottom of the file

```rust
#[test]
public entry fun test_flow() {

}
```

:::tip
we need to use `entry` here because we are testing an `entry` function
:::

2. For simplicity, and because we dont have a lot of code to test, we use one function to test the whole flow of the app.
   The test steps are

```
  // create a list
  // create a task
  // update task as completed
```

Update the test function to be

```rust
#[test(admin = @0x123)]
public entry fun test_flow(admin: signer) acquires TasksHolder {
  // creates an admin @myaddr account for test
  account::create_account_for_test(signer::address_of(&admin));
  // initialize contract with admin account
  create_list(&admin);

  // creates a task by the admin account
  create_task(&admin, string::utf8(b"New Task"));
  let task_count = event::counter(&borrow_global<TasksHolder>(signer::address_of(&admin)).set_task_event);
  assert!(task_count == 1, 4);
  let todo_list = borrow_global<TasksHolder>(signer::address_of(&admin));
  assert!(todo_list.task_counter == 1, 5);
  let task_record = table::borrow(&todo_list.tasks, todo_list.task_counter);
  assert!(task_record.task_id == 1, 6);
  assert!(task_record.completed == false, 7);
  assert!(task_record.content == string::utf8(b"New Task"), 8);
  assert!(task_record.address == signer::address_of(&admin), 9);

  // updates task as completed
  toggle_completed(&admin, 1);
  let todo_list = borrow_global<TasksHolder>(signer::address_of(&admin));
  let task_record = table::borrow(&todo_list.tasks, 1);
  assert!(task_record.task_id == 1, 10);
  assert!(task_record.completed == true, 11);
  assert!(task_record.content == string::utf8(b"New Task"), 12);
  assert!(task_record.address == signer::address_of(&admin), 13);
}
```

Our `#[test]` annotation has changed and declares an account variable.

Additionally, the function itself now accepts a signer argument.

**Let’s understand.**

Since our tests runs outside of an account scope, we need to “create” accounts to use in our tests. The `#[test]` annotation gives us the option to declare those accounts. We use an `admin` account and set it to a random account address (`@0x123`). The function accepts this signer (account) and creates it by using a built-in function to create an account for test.

Then we simply go through the flow by

- creating a list
- creating a task
- updating a task as completed

And assert the expected data/behavior at each step.

Before running the tests again, we need to use some new modules we are now using in our code

3. At the top of the file, add this use statement

```rust
use std::string::{Self, String}; // already have it, need to modify
```

4. Run the `aptos move test` command. If all goes right, we should see a success message like that

```rust
Running Move unit tests
[ PASS    ] 0xcbddf398841353776903dbab2fdaefc54f181d07e114ae818b1a67af28d1b018::main::test_flow
Test result: OK. Total tests: 1; passed: 1; failed: 0
{
  "Result": "Success"
}
```

5. Let’s add one more test to make sure our `toggle_completed` function works as expected. Add another test function like that

```rust
#[test(admin = @0x123)]
#[expected_failure(abort_code = E_NOT_INITIALIZED)]
public entry fun account_can_not_update_task(admin: signer) acquires TasksHolder {
  // creates an admin @myaddr account for test
  account::create_account_for_test(signer::address_of(&admin));
  // account can not toggle task as no list was created
  toggle_completed(&admin, 2);
}
```

This test tests that an account can’t use that function if they haven’t created a list before.

It also uses a special annotation `#[expected_failure]` that as the name suggests, this test functions expects to fail with an `E_NOT_INITIALIZED` error code.

6. Run the `aptos move test` command. If all goes right, we should see a success message like that

```rust
Running Move unit tests
[ PASS    ] 0xcbddf398841353776903dbab2fdaefc54f181d07e114ae818b1a67af28d1b018::main::account_can_not_update_task
[ PASS    ] 0xcbddf398841353776903dbab2fdaefc54f181d07e114ae818b1a67af28d1b018::main::test_flow
Test result: OK. Total tests: 2; passed: 2; failed: 0
{
  "Result": "Success"
}
```

Now that everything works, we can compile the move modules and publish the move package to chain so our react app (and everyone else) can interact with our smart contract!

### Publish todolist module to chain

For now, the easiest way to publish a move package to chain is using the CLI.

1. `cd` into our `move` folder, and run `aptos move compile`

We are getting some Unused alias errors. This is because, before we added the `string` alias since we use it in our tests. But we dont use this alias in our smart contract code.

This is why we are getting this error when we want to compile the module but not getting it when we only run tests.

To fix it, we can add `use` statement that would be used only in tests.

Add the following `use statement` where we have all of our use statements.

```rust
use std::string::String; // change to this
...
#[test_only]
use std::string;
```

2. run `aptos move test` and `aptos move compile` - all should work without errors.
3. run `aptos move publish`
4. Enter `yes` in the prompt
5. That would compile, simulate and finally publish you module into devnet. You should see a success message, something like that

```rust
{
  "Result": {
    "transaction_hash": "0x96b84689a53a28db7be6346627a99967f719946bc22766811a674e69da7783fa",
    "gas_used": 7368,
    "gas_unit_price": 100,
    "sender": "cbddf398841353776903dbab2fdaefc54f181d07e114ae818b1a67af28d1b018",
    "sequence_number": 2,
    "success": true,
    "timestamp_us": 1674246585276143,
    "version": 651327,
    "vm_status": "Executed successfully"
  }
}
```

6. You can now head to https://explorer.aptoslabs.com/ , change the dropdown on the top right to devnet and look for that `transaction_hash` - that would show you the transaction details.

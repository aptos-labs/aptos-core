---
title: "Create a Smart Contract"
id: "create-a-smart-contract"
---

# Create a Smart Contract

If you haven’t done it, [install the Aptos CLI](../../cli-tools/aptos-cli-tool/index.md). Make sure you use CLI version 1.0.4 or later as this what we use in this tutorial.

1.  `cd` into the `my-first-dapp` root directory, and create a new `move` directory.
2.  `cd` into the new `move` directory and run: `aptos move init --name my_todo_list`
    That command creates a `sources/` directory and `Move.toml` file inside the `move` directory.
3.  Your new `move` directory should now resemble:

    ![move-directory](../../../static/img/docs/build-e2e-dapp-img-1.png)

### What is a `Move.toml` file?

A `Move.toml` file is a manifest file that contains metadata such as name, version, and dependencies for the package.

Take a look at the new `Move.toml` file. You should see your package information and an `AptosFramework` dependency. Note that the `name` property is the same `--name` attribute we passed to the `aptos move init` command before. The `AptosFramework` dependency points to the `aptos-core/aptos-move/framework/aptos-framework` GitHub repo main branch.

### Why `sources` directory?

The `sources` directory holds a collection of `.move` modules files. And later when we want to compile the package using the CLI, the compiler will look for that `sources` directory and its `Move.toml` file.

### Create a Move module

An account is needed to publish a Move module. So first we need to create an account. Once we have the account's private key, we can create a module under its account address and publish the module using that account.

1. In our `move` directory, run `aptos init --network devnet`. Press enter when prompted.

   This creates for us a `.aptos` directory with a `config.yaml` file that holds our profile information. In the `config.yaml` file, we now have our profiles list that holds a `default` profile. If you open that file, you will see content resembling:

   ```yaml
   profiles:
     default:
       private_key: "0xee8f387ef0b4bb0018c4b91d1c0f71776a9b85935b4c6ec2823d6c0022fbf5cb"
       public_key: "0xc6c07218d79a806380ca67761905063ec7a78d41f79619f4562462a0f8b6be11"
       account: cbddf398841353776903dbab2fdaefc54f181d07e114ae818b1a67af28d1b018
       rest_url: "https://fullnode.devnet.aptoslabs.com"
       faucet_url: "https://faucet.devnet.aptoslabs.com"
   ```

   From now on, whenever we run a CLI command in this `move` directory, it will run with that default profile.
   We use the `devnet` network flag so eventually when we publish our package it will get published to the `devnet` network.

   :::tip
   You just created a new account on the Aptos (dev) network! Yay! You can see it by going to the [Aptos Explorer](https://explorer.aptoslabs.com/?network=devnet) Devnet network view, pasting the `account` address value from your configuration file into the search field, and clicking on the dropdown option!
   :::

As mentioned, our `sources` directory holds our `.move` module files; so let’s add our first Move file.

2. Open the `Move.toml` file.
3. Add the following code to that Move file, substituting your actual default profile account address from `.aptos/config.yaml`:

```toml
[addresses]
todolist_addr='<default-profile-account-address>'
```

If the default profile account address is `cbddf398841353776903dbab2fdaefc54f181d07e114ae818b1a67af28d1b018`, your `Move.toml` file should look like:

```toml
[addresses]
todolist_addr='cbddf398841353776903dbab2fdaefc54f181d07e114ae818b1a67af28d1b018'
```

4. Create a new `todolist.move` file within the `sources` directory and add the following to that file:

```rust
module todolist_addr::todolist {

}
```

:::tip
A Move module is stored under an address (so when it published anyone can access it using that address); the syntax for a Move module is

```rust
module <account-address>::<module-name> {

}
```

In our module, the `account-address` is `todolist_addr` (a variable we just declared on the `Move.toml` file in the previous step that holds an `address`), and the `module-name` is `todolist` (a random name we selected).
:::

### Our contract logic

Before jumping into writing code, let’s first understand what we want our smart contract program to do. For ease of understanding, we will keep the logic pretty simple:

1. An account creates a new list.
2. An account creates a new task on their list.
   - Whenever someone creates a new task, emit a `task_created` event.
3. Let an account mark their task as completed.

:::tip
Creating an event is not mandatory yet useful if dapps/users want to monitor data, such as how many people create a new task, using the [Aptos Indexer](../../guides/indexing.md).
:::

We can start with defining a `TodoList` struct, that holds the:

- tasks array
- new task event
- a task counter that counts the number of created tasks (we can use that to differentiate between the tasks)

And also create a `Task` struct that holds:

- the task ID - derived from the TodoList task counter.
- address - the account address who created that task.
- content - the task content.
- completed - a boolean that marks whether that task is completed or not.

On the `todolist.move` file, update the content in the module with:

```rust
...
struct TodoList has key {
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

**TodoList**

A struct that has the `key` and `store` abilities:

- `Key` ability allows struct to be used as a storage identifier. In other words, `key`
   is an ability to be stored at the top-level and act as a storage. We need it here to have `TodoList` be a resource stored in our user account.

When a struct has the `key` ability, it turns this struct into a `resource`:

- `Resource` is stored under the account - therefore it *exists* only when assigned to an account and can be *accessed* through this account only.

**Task**

A struct that has the `store`, `drop` and `copy`abilities.

• `Store` - Task needs `Store` as it’s stored inside another struct (TodoList)

• `Copy` - value can be *copied* (or cloned by value).

• `Drop` - value can be *dropped* by the end of scope.

Let’s try to compile what we have now:

1. `cd` into the `move` directory.
2. Run: `aptos move compile`

**Seeing errors?!** Let’s understand them.

We have some errors on `Unbound type`- this is happening because we used some types but never imported them, and the compiler doesnt know where to get them from.

3. On the top of the module, import those types by adding:

```rust
...
use aptos_framework::event;
use std::string::String;
use aptos_std::table::Table;
...
```

That will tell the compiler where it can get those types from.

4. Run the `aptos move compile` command again; If all goes well, we should see a response resembling (where the resulting account address is your default profile account address):

```rust
INCLUDING DEPENDENCY AptosFramework
INCLUDING DEPENDENCY AptosStdlib
INCLUDING DEPENDENCY MoveStdlib
BUILDING myTodolist
{
"Result": [
    "cbddf398841353776903dbab2fdaefc54f181d07e114ae818b1a67af28d1b018::todolist"
  ]
}
```

At this point, we have successfully compiled our Move module. Yay!

We also have a new `move/build` directory (created by the compiler) that holds our compiled modules, build information and `sources` directory.

### Create list function

The first thing an account can and should do with our contract is create a new list.

Creating a list is essentially submitting a transaction, and so we need to know the `signer` who signed and submitted the transaction:

1. Add a `create_list` function that accepts a `signer`

```rust
public entry fun create_list(account: &signer){

}
```

**Let’s understand the components of this function**

- `entry` - an _entry_ function is a function that can be called via transactions. Simply put, whenever you want to submit a transaction to the chain, you should call an entry function.

- `&signer` - The **signer** argument is injected by the Move VM as the address who signed that transaction.

Our code has a `TodoList` resource. Resource is stored under the account; therefore, it *exists* only when assigned to an account and can be *accessed* only through this account.

That means to create the `TodoList` resource, we need to assign it to an account that only this account can have access to.

The `create_list` function can handle that `TodoList` resource creation.

2. Add the following to the `create_list` function

```rust
public entry fun create_list(account: &signer){
  let tasks_holder = TodoList {
    tasks: table::new(),
    set_task_event: account::new_event_handle<Task>(account),
    task_counter: 0
  };
  // move the TodoList resource under the signer account
  move_to(account, tasks_holder);
}
```

This function takes in a `signer`, creates a new `TodoList` resource, and uses `move_to` to have the resource stored in the provided signer account.

### Create task function

As mentioned before, our contract has a create task function that lets an account create a new task. Creating a task is also essentially submitting a transaction, and so we need to know the `signer` who signed and submitted the transaction. Another element we want to accept in our function is the task `content`.

1. Add a `create_task` function that accepts a `signer` and task `content` and the function logic.

```rust
public entry fun create_task(account: &signer, content: String) acquires TodoList {
    // gets the signer address
    let signer_address = signer::address_of(account);
    // gets the TodoList resource
    let todo_list = borrow_global_mut<TodoList>(signer_address);
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
      &mut borrow_global_mut<TodoList>(signer_address).set_task_event,
      new_task,
    );
  }
```

2. Since we now use two new modules - signer and table (you can see it being used in `signer::` and `table::`) - we need to import these modules.
   At the top of the file, add those two use statements:

```rust
use std::signer;
use aptos_std::table::{Self, Table}; // This one we already have, need to modify it
```

**Back to the code; what is happening here?**

- First, we want to get the signer address so we can get this account’s `TodoList` resource.
- Then, we retrieve the `TodoList` resource with the `signer_address`; with that we have access to the `TodoList` properties.
- We can now increment the `task_counter` property, and create a new `Task` with the `signer_address`, `counter` and the provided `content`.
- We push it to the `todo_list.tasks` table that holds all of our tasks along with the new `counter` (which is the table key) and the newly created Task.
- Then we assign the global `task_counter` to be the new incremented counter.
- Finally, we emit the `task_created` event that holds the new Task data. `emit_event` is an `aptos-framework` function that accepts a reference to the event handle and a message. In our case, we are passing the function a reference (using the sign &) to the account’s `TodoListresource` `set_task_event` property as the first argument and a second message argument which is the new Task we just created. Remember, we have a `set_task_event` property in our `TodoList` struct.

### Complete task function

Another function we want our contract to hold is the option to mark a task as completed.

1. Add a `complete_task` function that accepts a `signer` and a `task_id`:

```rust
public entry fun complete_task(account: &signer, task_id: u64) acquires TodoList {
  // gets the signer address
  let signer_address = signer::address_of(account);
  // gets the TodoList resource
  let todo_list = borrow_global_mut<TodoList>(signer_address);
  // gets the task matches the task_id
  let task_record = table::borrow_mut(&mut todo_list.tasks, task_id);
  // update task as completed
  task_record.completed = true;
}
```

**Let’s understand the code.**

- As before in our create list function, we retrieve the `TodoList` struct by the signer address so we can have access to the tasks table that holds all of the account tasks.
- Then, we look for the task with the provided `task_id` on the `todo_list.tasks` table.
- Finally, we update that task completed property to be true.

Now try to compile the code:

2. Run: `aptos move compile`
3. Another `Unbound` error? To fix this, add a `use` statement to use the `account` module.

```rust
use aptos_framework::account;
```

4. run `aptos move compile` again.

### Add validations

As this code now compiles, we want to have some validations and checks before creating a new task or updating the task as completed so we can be sure our functions work as expected.

1. Add a check to the `create_task` function to make sure the signer account has a list:

```rust
public entry fun create_task(account: &signer, content: String) acquires TodoList {
  // gets the signer address
  let signer_address = signer::address_of(account);

  // assert signer has created a list
  assert!(exists<TodoList>(signer_address), 1);

  ...
}
```

1. Add a check to the `complete_task` function to make sure the:
   - signer has created a list.
   - task exists.
   - task is not completed.

With:

```rust
public entry fun complete_task(account: &signer, task_id: u64) acquires TodoList {
  // gets the signer address
  let signer_address = signer::address_of(account);
  // assert signer has created a list
  assert!(exists<TodoList>(signer_address), 1);
  // gets the TodoList resource
  let todo_list = borrow_global_mut<TodoList>(signer_address);
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

If you noticed, `assert` accepts two arguments: the first is what to check for, and the second is an error code. Instead of passing in an arbitrary number, a convention is to declare `errors` on the top of the module file and use these instead.

On the top of the module file (under the `use` statements), add those error declarations:

```rust
// Errors
const E_NOT_INITIALIZED: u64 = 1;
const ETASK_DOESNT_EXIST: u64 = 2;
const ETASK_IS_COMPLETED: u64 = 3;
```

Now we can update our asserts with these constants:

```rust
public entry fun create_task(account: &signer, content: String) acquires TodoList {
  // gets the signer address
  let signer_address = signer::address_of(account);

  // assert signer has created a list
  assert!(exists<TodoList>(signer_address), E_NOT_INITIALIZED);

  ...
}



public entry fun complete_task(account: &signer, task_id: u64) acquires TodoList {
  // gets the signer address
  let signer_address = signer::address_of(account);
  assert!(exists<TodoList>(signer_address), E_NOT_INITIALIZED);
  // gets the TodoList resource
  let todo_list = borrow_global_mut<TodoList>(signer_address);
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

Let’s stop for one moment and make sure our code compiles by running the `aptos move compile` command. If all goes well, we should output resembling:

```rust
INCLUDING DEPENDENCY AptosFramework
INCLUDING DEPENDENCY AptosStdlib
INCLUDING DEPENDENCY MoveStdlib
BUILDING myTodolist
{
"Result": [
    "cbddf398841353776903dbab2fdaefc54f181d07e114ae818b1a67af28d1b018::todolist"
  ]
}
```

If you encounter errors, make sure you followed the steps above correctly and try to determine the cause of the issues.

### Write tests

Now that we have our smart contract logic ready, we need to add some tests for it.

Test functions use the `#[test]` annotation.

1. Add the following code to the bottom of the file:

```rust
#[test]
public entry fun test_flow() {

}
```

:::tip
we need to use `entry` here because we are testing an `entry` function.
:::

2. For simplicity, and because we don't have much code to test, we use one function to test the whole flow of the app.
   The test steps are:

```
  // create a list
  // create a task
  // update task as completed
```

Update the test function to be:

```rust
#[test(admin = @0x123)]
public entry fun test_flow(admin: signer) acquires TodoList {
  // creates an admin @todolist_addr account for test
  account::create_account_for_test(signer::address_of(&admin));
  // initialize contract with admin account
  create_list(&admin);

  // creates a task by the admin account
  create_task(&admin, string::utf8(b"New Task"));
  let task_count = event::counter(&borrow_global<TodoList>(signer::address_of(&admin)).set_task_event);
  assert!(task_count == 1, 4);
  let todo_list = borrow_global<TodoList>(signer::address_of(&admin));
  assert!(todo_list.task_counter == 1, 5);
  let task_record = table::borrow(&todo_list.tasks, todo_list.task_counter);
  assert!(task_record.task_id == 1, 6);
  assert!(task_record.completed == false, 7);
  assert!(task_record.content == string::utf8(b"New Task"), 8);
  assert!(task_record.address == signer::address_of(&admin), 9);

  // updates task as completed
  complete_task(&admin, 1);
  let todo_list = borrow_global<TodoList>(signer::address_of(&admin));
  let task_record = table::borrow(&todo_list.tasks, 1);
  assert!(task_record.task_id == 1, 10);
  assert!(task_record.completed == true, 11);
  assert!(task_record.content == string::utf8(b"New Task"), 12);
  assert!(task_record.address == signer::address_of(&admin), 13);
}
```

Our `#[test]` annotation has changed and declares an account variable.

Additionally, the function itself now accepts a signer argument.

**Let’s understand our tests.**

Since our tests runs outside of an account scope, we need to _create_ accounts to use in our tests. The `#[test]` annotation gives us the option to declare those accounts. We use an `admin` account and set it to a random account address (`@0x123`). The function accepts this signer (account) and creates it by using a built-in function to create an account for test.

Then we simply go through the flow by:

- creating a list
- creating a task
- updating a task as completed

And assert the expected data/behavior at each step.

Before running the tests again, we need to import (`use`) some new modules we are now employing in our code:

3. At the top of the file, add this `use` statement:

```rust
use std::string::{Self, String}; // already have it, need to modify
```

4. Run the `aptos move test` command. If all goes right, we should see a success message like:

```rust
Running Move unit tests
[ PASS    ] 0xcbddf398841353776903dbab2fdaefc54f181d07e114ae818b1a67af28d1b018::todolist::test_flow
Test result: OK. Total tests: 1; passed: 1; failed: 0
{
  "Result": "Success"
}
```

5. Let’s add one more test to make sure our `complete_task` function works as expected. Add another test function with:

```rust
#[test(admin = @0x123)]
#[expected_failure(abort_code = E_NOT_INITIALIZED)]
public entry fun account_can_not_update_task(admin: signer) acquires TodoList {
  // creates an admin @todolist_addr account for test
  account::create_account_for_test(signer::address_of(&admin));
  // account can not toggle task as no list was created
  complete_task(&admin, 2);
}
```

This test confirms that an account can’t use that function if they haven’t created a list before.

The test also uses a special annotation `#[expected_failure]` that, as the name suggests, expects to fail with an `E_NOT_INITIALIZED` error code.

6. Run the `aptos move test` command. If all goes right, we should see a success message like:

```rust
Running Move unit tests
[ PASS    ] 0xcbddf398841353776903dbab2fdaefc54f181d07e114ae818b1a67af28d1b018::todolist::account_can_not_update_task
[ PASS    ] 0xcbddf398841353776903dbab2fdaefc54f181d07e114ae818b1a67af28d1b018::todolist::test_flow
Test result: OK. Total tests: 2; passed: 2; failed: 0
{
  "Result": "Success"
}
```

Now that everything works, we can compile the Move modules and publish the Move package to chain so our React app (and everyone else) can interact with our smart contract!

### Publish todolist module to chain

For now, the easiest way to publish a Move package to chain is using the CLI:

1. `cd` into our `move` directory, and run: `aptos move compile`

We are getting some _Unused alias_ errors. This is because we added the `string` alias before since we use it in our tests. But we don't use this alias in our smart contract code.

This is why we are getting this error when we want to compile the module but not are getting it when we only run tests.

To fix it, we can add a `use` statement that would be used only in tests.

Add the following `use` statement where we have all of our import statements.

```rust
use std::string::String; // change to this
...
#[test_only]
use std::string; // add this
```

2. Run: `aptos move test` and `aptos move compile` - all should work without errors.
3. Run: `aptos move publish`
4. Enter `yes` in the prompt.
5. That will compile, simulate and finally publish you module into devnet. You should see a success message:

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

6. You can now head to the [Aptos Explorer](https://explorer.aptoslabs.com/), change the dropdown on the top right to the _Devnet_ network and look for that `transaction_hash` value - this will show you the transaction details.

Now let's [set up a React app](./2-set-up-react-app.md).

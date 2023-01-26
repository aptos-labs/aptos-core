---
title: "Handle Tasks"
id: "handle-tasks"
---

# Handle tasks

By now, we went over on how to fetch data (account’s todo list) from chain and how to submit a transaction (new todo list) to chain using Wallet.

Let’s continue building our app and implement fetch tasks and add a task functions.

### Fetch Tasks

1. Create a local state `tasks` that would hold our tasks. It would be a state of a Task type (that has the same properties we set on our smart contract)

```ts
type Task = {
  address: string;
  completed: boolean;
  content: string;
  task_id: string;
};

function App() {
	const [tasks, setTasks] = useState<Task[]>([]);
	...
}
```

2. Update our `fetchList` function to fetch the tasks in the account’s `TodoList` resource.

```js
const fetchList = async () => {
  if (!account) return [];
  try {
    const TodoListResource = await client.getAccountResource(
      account?.address,
      `${moduleAddress}::main::TodoList`
    );
    setAccountHasList(true);
		// tasks table handle
    const tableHandle = (TodoListResource as any).data.tasks.handle;
		// tasks table counter
    const taskCounter = (TodoListResource as any).data.task_counter;

    let tasks = [];
    let counter = 1;
    while (counter <= taskCounter) {
      const tableItem = {
        key_type: "u64",
        value_type: `${moduleAddress}::main::Task`,
        key: `${counter}`,
      };
      const task = await client.getTableItem(tableHandle, tableItem);
      tasks.push(task);
      counter++;
    }
		// set tasks in local state
    setTasks(tasks);
  } catch (e: any) {
    setAccountHasList(false);
  }
};
```

**This part is a bit confusing so bear with me!**

Tasks are stored in a table (this is how we built our contract). To fetch a table item (i.e a task) we need that tasks table handle. We also need the `task_counter` in that resource so we can loop over and fetch the task that the task_id matches the task_counter.

```js
const tableHandle = (TodoListResource as any).data.tasks.handle;
const taskCounter = (TodoListResource as any).data.task_counter;
```

Now that we have our tasks table handle and our task_counter variable, lets loop over the `taskCounter` . We define a `counter` and set it to 1 (as the task_counter / task_id) is never less than 1.

We loop while the `counter` is less then the `taskCounter` and fetch the table item and push it to the tasks array.

```js
let tasks = [];
let counter = 1;
while (counter <= taskCounter) {
  const tableItem = {
    key_type: "u64",
    value_type: `${moduleAddress}::main::Task`,
    key: `${counter}`,
  };
  const task = await client.getTableItem(tableHandle, tableItem);
  tasks.push(task);
  counter++;
}
```

We build a `tableItem` object to fetch. If we will take a look at our table structure from the contract

```rust
tasks: Table<u64, Task>,
```

We see that it has a `key` type `u64` and a `value` of type `Task` and whenever we create a new task, we assign the `key` to be the incremented task counter

```rust
// adds the new task into the tasks table
table::upsert(&mut todo_list.tasks, counter, new_task);
```

So the object we built is

```js
{
  key_type: "u64",
  value_type:`${moduleAddress}::main::Task`,
  key: `${taskCounter}`,
}
```

where `key_type` is the table `key` type, `key` is the key value we are looking for, and the `value_type` is the table `value` which is a `Task` struct. The Task struct use the same format from our previous resource query

- The account address who holds that module = our profile account address
- The module name the resource lives in = `main`
- The struct name = `Task`

Last thing we want to do, is actually display the tasks we just fetched.

6. On our `App.tsx` , update our UI with the following code

```jsx
{
  !accountHasList ? (
    <Row gutter={[0, 32]} style={{ marginTop: "2rem" }}>
      <Col span={8} offset={8}>
        <Button
          disabled={!account}
          block
          onClick={addNewList}
          type="primary"
          style={{ height: "40px", backgroundColor: "#3f67ff" }}
        >
          Add new list
        </Button>
      </Col>
    </Row>
  ) : (
    <Row gutter={[0, 32]} style={{ marginTop: "2rem" }}>
      <Col span={8} offset={8}>
        {tasks && (
          <List
            size="small"
            bordered
            dataSource={tasks}
            renderItem={(task: any) => (
              <List.Item actions={[<Checkbox />]}>
                <List.Item.Meta
                  title={task.content}
                  description={
                    <a
                      href={`https://explorer.aptoslabs.com/account/${task.address}/`}
                      target="_blank"
                    >{`${task.address.slice(0, 6)}...${task.address.slice(-5)}`}</a>
                  }
                />
              </List.Item>
            )}
          />
        )}
      </Col>
    </Row>
  );
}
```

That would display the “Add new list” button if account doesn’t have a list or the tasks if the account has a list.

Go ahead and refresh your browser - see the magic!

We haven’t added any task yet, so we simply see a box with empty data content. Let’s add some tasks!

### Add task

1. Update our UI with a “add task” input

```jsx
{!accountHasList ? (
  ...
) : (
  <Row gutter={[0, 32]} style={{ marginTop: "2rem" }}>
		// Add this!
    <Col span={8} offset={8}>
      <Input.Group compact>
        <Input
          style={{ width: "calc(100% - 60px)" }}
          placeholder="Add a Task"
          size="large"
        />
        <Button
          type="primary"
          style={{ height: "40px", backgroundColor: "#3f67ff" }}
        >
          Add
        </Button>
      </Input.Group>
    </Col>
    ...
  </Row>
)}
```

We have added a text input to write the task and a button to add the task.

2. Create a new local state that holds the task content.

```jsx
function App() {
  ...
  const [newTask, setNewTask] = useState<string>("");
  ...
}
```

3. Add a `onWriteTask` function that would get called whenever a user types something in the input text

```jsx
function App() {
  ...
  const [newTask, setNewTask] = useState<string>("");

  const onWriteTask = (event: React.ChangeEvent<HTMLInputElement>) => {
    const value = event.target.value;
    setNewTask(value);
  };
  ...
}
```

4. Find our `<Input/>` component and add the `onChange` event to it and pass it our `onWriteTask` function and set the input value to be the `newTask` local state

```jsx
<Input
  onChange={(event) => onWriteTask(event)} // add this
  style={{ width: "calc(100% - 60px)" }}
  placeholder="Add a Task"
  size="large"
  value={newTask} // add this
/>
```

Cool! Now we have a working flow that when the user types something on the Input component, a function would get fired and set our local state with that content.

5. Let’s also add a function that submits the typed task to chain! Find our Add `<Button />` component and update it with the following

```jsx
<Button
  onClick={onTaskAdded} // add this
  type="primary"
  style={{ height: "40px", backgroundColor: "#3f67ff" }}
>
  Add
</Button>
```

That adds an `onClickevent` that triggers a `onTaskAdded` function.

When someones adds a new task we

- want to verify they are connected with a wallet
- build a transaction payload that would be submitted to chain
- submit it to chain using our wallet
- wait for the transaction
- update our UI with that new task (with out the need to refresh the page)

6. Add a `onTaskAdded` function with the following code

```jsx
const onTaskAdded = async () => {
    // check for connected account
    if (!account) return;
    setTransactionInProgress(true);
    // build a transaction payload to be submited
    const payload = {
      type: "entry_function_payload",
      function: `${moduleAddress}::main::create_task`,
      type_arguments: [],
      arguments: [newTask],
    };

    try {
      // sign and submit transaction to chain
      const response = await signAndSubmitTransaction(payload);
      // wait for transaction
      await client.waitForTransaction(response.hash);

			// hold the latest task.task_id from our local state
      const latestId = tasks.length > 0 ? parseInt(tasks[tasks.length - 1].task_id) + 1 : 1;

      // build a newTaskToPush objct into our local state
      const newTaskToPush = {
        address: account.address,
        completed: false,
        content: newTask,
        task_id: latestId + "",
      };

      // Create a new array based on current state:
      let newTasks = [...tasks];

      // Add item to it
      newTasks.unshift(newTaskToPush);

      // Set state
      setTasks(newTasks);
			// clear input text
      setNewTask("");
    } catch (error: any) {
      console.log("error", error);
    } finally {
      setTransactionInProgress(false);
    }
  };
```

**Let’s go over on what is happening.**

First thing we use the `account` prop from our wallet provider to make sure there is an account connected to our app.

Then we build our transaction payload to be submitted to chain

```js
const payload = {
  type: "entry_function_payload",
  function: `${moduleAddress}::main::create_task`,
  type_arguments: [],
  arguments: [newTask],
};
```

- `type` is the function type we want to hit - our create_task function is an `entry` type function
- `function`- is built from the module address, module name and the function name
- `type_arguments`- this is for the case a move function expects a generic type argument
- `arguments` - the argument the function expects, in our case is the task content

Then, within our try/catch block, we use a wallet provider function to submit the transaction to chain and a SDK function to wait for that transaction.
If all went good, we want to find the current latest task id so we can add it to our current tasks state array, and create a new task to push to the current tasks state array (so we can display the new task in our tasks list on the UI without the need to refresh the page).

TRY IT!

Type a new task in the text input, click Add, approve the transaction and see it being added to the tasks list.

### Mark task as completed

Next, we can implement the complete_task function. We have the checkbox in our UI so users can mark a task as completed.

1. Update the `<Checkbox/>` component with a `onCheck` prop that would call a `onCheckboxChange` function once it is checked

```jsx
<List.Item actions={[
  <Checkbox onChange={(event) => onCheckboxChange(event, task.task_id)}/>
]}>
```

2. Create the `onCheckboxChange` function (make sure to import `CheckboxChangeEven`t from `antd` - `import { CheckboxChangeEvent } from "antd/es/checkbox";`)

```js
const onCheckboxChange = async (
    event: CheckboxChangeEvent,
    taskId: string
  ) => {
    if (!account) return;
    if (!event.target.checked) return;
    setTransactionInProgress(true);
    const payload = {
      type: "entry_function_payload",
      function:
        `${moduleAddress}::main::complete_task`,
      type_arguments: [],
      arguments: [taskId],
    };

    try {
      // sign and submit transaction to chain
      const response = await signAndSubmitTransaction(payload);
      // wait for transaction
      await client.waitForTransaction(response.hash);

      setTasks((prevState) => {
        const newState = prevState.map((obj) => {
          // if task_id equals the checked taskId, update completed property
          if (obj.task_id === taskId) {
            return { ...obj, completed: true };
          }

          // otherwise return object as is
          return obj;
        });

        return newState;
      });
    } catch (error: any) {
      console.log("error", error);
    } finally {
      setTransactionInProgress(false);
    }
  };
```

Here we basically so the same thing we did when we created a new list or a new task.

we make sure there is an account connected, set the transaction in progress state, build the transaction payload and then submit the transaction, wait for it and update the task on the UI as completed.

3. Update the `Checkbox` component to be checked by default is a task has already marked as completed

```jsx
...
<List.Item
  actions={[
    <div>
      {task.completed ? (
        <Checkbox defaultChecked={true} disabled />
      ) : (
        <Checkbox
          onChange={(event) =>
            onCheckboxChange(event, task.task_id)
          }
        />
      )}
    </div>,
  ]}
>
...
```

Try it! check a task’s checkbox, approve the transaction and see the task marked as completed.

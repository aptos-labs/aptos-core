---
title: "6. Handle Tasks"
id: "handle-tasks"
---

# 6. Handle Tasks

In the sixth and final chapter of the tutorial on [building an end-to-end dapp on Aptos](./index.md), you will add functionality to the app so the user interface is able to handle cases where an account has created a list.

We have covered how to [fetch data](./4-fetch-data-from-chain.md) (an account’s todo list) from chain and how to [submit a transaction](./5-submit-data-to-chain.md) (new todo list) to chain using Wallet.

Let’s finish building our app by implementing fetch tasks and adding a task function.

## Fetch tasks

1. Create a local state `tasks` that will hold our tasks. It will be a state of a Task type (that has the same properties we set on our smart contract):

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

2. Update our `fetchList` function to fetch the tasks in the account’s `TodoList` resource:

```js
const fetchList = async () => {
  if (!account) return [];
  try {
    const TodoListResource = await provider.getAccountResource(
      account?.address,
      `${moduleAddress}::todolist::TodoList`
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
        value_type: `${moduleAddress}::todolist::Task`,
        key: `${counter}`,
      };
      const task = await provider.getTableItem(tableHandle, tableItem);
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

**This part is a bit confusing, so stick with us!**

Tasks are stored in a table (this is how we built our contract). To fetch a table item (i.e a task), we need that task's table handle. We also need the `task_counter` in that resource so we can loop over and fetch the task with the `task_id` that matches the `task_counter`.

```js
const tableHandle = (TodoListResource as any).data.tasks.handle;
const taskCounter = (TodoListResource as any).data.task_counter;
```

Now that we have our tasks table handle and our `task_counter` variable, lets loop over the `taskCounter` . We define a `counter` and set it to 1 as the task_counter / task_id is never less than 1.

We loop while the `counter` is less then the `taskCounter` and fetch the table item and push it to the tasks array:

```js
let tasks = [];
let counter = 1;
while (counter <= taskCounter) {
  const tableItem = {
    key_type: "u64",
    value_type: `${moduleAddress}::todolist::Task`,
    key: `${counter}`,
  };
  const task = await provider.getTableItem(tableHandle, tableItem);
  tasks.push(task);
  counter++;
}
```

We build a `tableItem` object to fetch. If we take a look at our table structure from the contract:

```rust
tasks: Table<u64, Task>,
```

We see that it has a `key` type `u64` and a `value` of type `Task`. And whenever we create a new task, we assign the `key` to be the incremented task counter.

```rust
// adds the new task into the tasks table
table::upsert(&mut todo_list.tasks, counter, new_task);
```

So the object we built is:

```js
{
  key_type: "u64",
  value_type:`${moduleAddress}::todolist::Task`,
  key: `${taskCounter}`,
}
```

Where `key_type` is the table `key` type, `key` is the key value we are looking for, and the `value_type` is the table `value` which is a `Task` struct. The Task struct uses the same format from our previous resource query:

- The account address who holds that module = our profile account address
- The module name the resource lives in = `todolist`
- The struct name = `Task`

The last thing we want to do is display the tasks we just fetched.

6. In our `App.tsx` file, update our UI with the following code:

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

That will display the **Add new list** button if account doesn’t have a list or instead the tasks if the account has a list.

Go ahead and refresh your browser - see the magic!

We haven’t added any tasks yet, so we simply see a box of empty data. Let’s add some tasks!

## Add task

1. Update our UI with an _add task_ input:

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

2. Create a new local state that holds the task content:

```jsx
function App() {
  ...
  const [newTask, setNewTask] = useState<string>("");
  ...
}
```

3. Add an `onWriteTask` function that will get called whenever a user types something in the input text:

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

4. Find our `<Input/>` component, add the `onChange` event to it, pass it our `onWriteTask` function and set the input value to be the `newTask` local state:

```jsx
<Input
  onChange={(event) => onWriteTask(event)} // add this
  style={{ width: "calc(100% - 60px)" }}
  placeholder="Add a Task"
  size="large"
  value={newTask} // add this
/>
```

Cool! Now we have a working flow that when the user types something on the Input component, a function will get fired and set our local state with that content.

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

That adds an `onClickevent` that triggers an `onTaskAdded` function.

When someones adds a new task we:

- want to verify they are connected with a wallet.
- build a transaction payload that would be submitted to chain.
- submit it to chain using our wallet.
- wait for the transaction.
- update our UI with that new task (without the need to refresh the page).

6. Add an `onTaskAdded` function with:

```jsx
  const onTaskAdded = async () => {
    // check for connected account
    if (!account) return;
    setTransactionInProgress(true);
    // build a transaction payload to be submited
    const payload = {
      type: "entry_function_payload",
      function: `${moduleAddress}::todolist::create_task`,
      type_arguments: [],
      arguments: [newTask],
    };

    // hold the latest task.task_id from our local state
    const latestId = tasks.length > 0 ? parseInt(tasks[tasks.length - 1].task_id) + 1 : 1;

    // build a newTaskToPush object into our local state
    const newTaskToPush = {
      address: account.address,
      completed: false,
      content: newTask,
      task_id: latestId + "",
    };

    try {
      // sign and submit transaction to chain
      const response = await signAndSubmitTransaction(payload);
      // wait for transaction
      await provider.waitForTransaction(response.hash);

      // Create a new array based on current state:
      let newTasks = [...tasks];

      // Add item to the tasks array
      newTasks.push(newTaskToPush);
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

First, note we use the `account` property from our wallet provider to make sure there is an account connected to our app.

Then we build our transaction payload to be submitted to chain:

```js
const payload = {
  type: "entry_function_payload",
  function: `${moduleAddress}::todolist::create_task`,
  type_arguments: [],
  arguments: [newTask],
};
```

- `type` is the function type we want to hit - our `create_task` function is an `entry` type function.
- `function`- is built from the module address, module name and the function name.
- `type_arguments`- this is for the case a Move function expects a generic type argument.
- `arguments` - the arguments the function expects, in our case the task content.

Then, within our try/catch block, we use a wallet provider function to submit the transaction to chain and an SDK function to wait for that transaction.
If all goes well, we want to find the current latest task ID so we can add it to our current tasks state array. We will also create a new task to push to the current tasks state array (so we can display the new task in our tasks list on the UI without the need to refresh the page).

TRY IT!

Type a new task in the text input, click **Add**, approve the transaction and see it being added to the tasks list.

## Mark task as completed

Next, we can implement the `complete_task` function. We have the checkbox in our UI so users can mark a task as completed.

1. Update the `<Checkbox/>` component with an `onCheck` property that would call an `onCheckboxChange` function once it is checked:

```jsx
<List.Item actions={[
  <Checkbox onChange={(event) => onCheckboxChange(event, task.task_id)}/>
]}>
```

2. Create the `onCheckboxChange` function (make sure to import `CheckboxChangeEvent` from `antd` - `import { CheckboxChangeEvent } from "antd/es/checkbox";`):

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
        `${moduleAddress}::todolist::complete_task`,
      type_arguments: [],
      arguments: [taskId],
    };

    try {
      // sign and submit transaction to chain
      const response = await signAndSubmitTransaction(payload);
      // wait for transaction
      await provider.waitForTransaction(response.hash);

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

Here we basically do the same thing we did when we created a new list or a new task.

We make sure there is an account connected, set the transaction in progress state, build the transaction payload, submit the transaction, wait for it and update the task on the UI as completed.

3. Update the `Checkbox` component to be checked by default if a task has already marked as completed:

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

Try it! Check a task’s checkbox, approve the transaction and see the task marked as completed.

You have now learned how to build a dapp on Aptos from end to end. Congratulations! Tell your friends. :-)

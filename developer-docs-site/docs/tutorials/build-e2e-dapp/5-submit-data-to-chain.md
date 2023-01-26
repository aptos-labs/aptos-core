---
title: "Submit data to chain"
id: "submit-data-to-chain"
---

# Submit data to chain

For now, we have a “Add new list” button that shows up if the connected account hasn’t created any list yet. We still dont have a way for an account to create a list, so let’s add it.

1. First, our wallet adapter provider has a `signAndSubmitTransaction` function, let’s extract it by updating the following

```js
const { account, signAndSubmitTransaction } = useWallet();
```

2. Add `onClick` event to the new list button

```js
<Button onClick={addNewList} block type="primary" style={{ height: "40px", backgroundColor: "#3f67ff" }}>
  Add new list
</Button>
```

3. Add the `addNewList` function

```js
const addNewList = async () => {
  if (!account) return [];
  // build a transaction payload to be submited
  const payload = {
    type: "entry_function_payload",
    function: `${moduleAddress}::main::create_list`,
    type_arguments: [],
    arguments: [],
  };
  try {
    // sign and submit transaction to chain
    const response = await signAndSubmitTransaction(payload);
    // wait for transaction
    await client.waitForTransaction(response.hash);
    setAccountHasList(true);
  } catch (error: any) {
    setAccountHasList(false);
  }
};
```

4. Since our new function also uses `moduleAddress` - let’s get it out of the `fetchList` function scope so it can be used globally. Let’s get this const out of the local function scope to the global scope so we can use it in our new function.
   In our `fetchList` function, find the line

```js
// replace with your own address
const moduleAddress = "0xcbddf398841353776903dbab2fdaefc54f181d07e114ae818b1a67af28d1b018";
```

and move it to outside of the main `App` function - right beneath our const `NODE_URL` and const `client` declarations.

**Let’s go over the `addNewList` function code.**

First, we use the `account` prop from our wallet provider to make sure there is an account connected to our app.

Then we build our transaction payload to be submitted to chain

```js
const payload = {
  type: "entry_function_payload",
  function: `${moduleAddress}::main::create_list`,
  type_arguments: [],
  arguments: [],
};
```

- `type` is the function type we want to hit - our create_list function is an `entry` type function
- `function`- is built from the module address, module name and the function name
- `type_arguments`- this is for the case a move function expects a generic type argument
- `arguments` - the argument the function expects, in our case it doesn’t expect any arguments

Next, we submit the transaction payload and wait for its response. The response returned from the `signAndSubmitTransaction` function holds the transaction hash. Since it can take a bit for the transaction to fully submitted to chain and we also want to make sure it submitted successfully, we `waitForTransaction` and only then we can set our local `accountHasList` state to true.

5. Before testing it on our App, let’s tweak our UI a bit add a Spinner component to show up while we are waiting for the transaction.
   Add a local state to keep track whether a transaction is in progress

```js
const [transactionInProgress, setTransactionInProgress] = useState < boolean > false;
```

6. Update our `addNewList` function to update the local state

```js
const addNewList = async () => {
  if (!account) return [];
  setTransactionInProgress(true);
  // build a transaction payload to be submited
  const payload = {
    type: "entry_function_payload",
    function: `${moduleAddress}::main::create_list`,
    type_arguments: [],
    arguments: [],
  };
  try {
    // sign and submit transaction to chain
    const response = await signAndSubmitTransaction(payload);
    // wait for transaction
    await client.waitForTransaction(response.hash);
    setAccountHasList(true);
  } catch (error: any) {
    setAccountHasList(false);
  } finally {
    setTransactionInProgress(false);
  }
};
```

7. Update our UI with the following

```js
return (
  <>
    ...
    <Spin spinning={transactionInProgress}>
      {!accountHasList && (
        <Row gutter={[0, 32]} style={{ marginTop: "2rem" }}>
          <Col span={8} offset={8}>
            <Button onClick={addNewList} block type="primary" style={{ height: "40px", backgroundColor: "#3f67ff" }}>
              Add new list
            </Button>
          </Col>
        </Row>
      )}
    </Spin>
  </>
);
```

Now we can head over to our app, and add a new list!

**Note:** keep in mind that we haven’t handled our UI in case an account has created a list, we will do it in the next section.

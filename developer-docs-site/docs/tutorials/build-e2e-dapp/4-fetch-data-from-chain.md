---
title: "4. Fetch Data from Chain"
id: "fetch-data-from-chain"
---

# 4. Fetch Data from Chain

In the fourth chapter of the tutorial on [building an end-to-end dapp on Aptos](./index.md), you will be learning to fetch data from chain.

Our UI logic relies on whether the connected account has created a todo list. If the account has created a todo list, our app should display that list; if not, the app should display a button offering the option to create a new list.

For that, we first need to check if the connected account has a `TodoList` resource. In our smart contract, whenever someone creates a todo list we create and assign a `TodoList` resource to their account.

To fetch data from chain, we can use the [Aptos TypeScript SDK](../../sdks/ts-sdk/index.md). The SDK provides classes and functions for us to easily interact and query the Aptos chain.

To get started:

1. Stop the local server if running.
2. In the `client` directory, run: `npm i aptos`
3. In the `App.tsx` file, import the `Provider` class and the `Network` type like so:

```js
import { Provider, Network } from "aptos";
```

The TypeScript SDK provides us with a `Provider` class where we can initialize and query the Aptos chain and Indexer. `Provider` expects `Network` type as an argument, which is the [network name](../../guides/system-integrators-guide.md#choose-a-network) we want to interact with.

:::tip
Read more about the [`Provider`](../../sdks/ts-sdk/typescript-sdk-overview.md#provider-class) class in the Aptos TypeScript SDK overview.
:::

1. In the `App.tsx` file, add:

```js
const provider = new Provider(Network.DEVNET);
```

This will initialize a `Provider` instance for us with the devnet network.

Our app displays different UIs based on a user resource (i.e if a user has a list ⇒ if a user has a `TodoList` resource). For that, we need to know the current account connected to our app.

1. Import wallet from the wallet adapter React provider:

```js
import { useWallet } from "@aptos-labs/wallet-adapter-react";
```

2. Extract the account object from the wallet adapter:

```js
function App (
	const { account } = useWallet();
	...
)
```

The `account` object is `null` if there is no account connected; when an account is connected, the `account` object holds the account information, including the account address.

3. Next, we want to fetch the account’s TodoList resource.
   Begin by importing `useEffect` by using `jsx import useEffect from "react"; `
   Let’s add a `useEffect` hook to our file that would call a function to fetch the resource whenever our account address changes:

```jsx
function App() {
  ...
  useEffect(() => {
    fetchList();
  }, [account?.address]);
  ...
}
```

4. Before creating our `fetchList` function, let’s also create a local state to store whether the account has a list:

```js
function App (
  ...
  const [accountHasList, setAccountHasList] = useState<boolean>(false);
  ...
)
```
also import `useEffect` using 
```import { useState, useEffect } from "react"; ```

5. Our `useEffect` hook is calling a `fetchList` function; let’s create it:

```jsx
const fetchList = async () => {
  if (!account) return [];
  // change this to be your module account address
  const moduleAddress = "0xcbddf398841353776903dbab2fdaefc54f181d07e114ae818b1a67af28d1b018";
  try {
    const TodoListResource = await provider.getAccountResource(
      account.address,
      `${moduleAddress}::todolist::TodoList`
    );
    setAccountHasList(true);
  } catch (e: any) {
    setAccountHasList(false);
  }
};
```

The `moduleAddress` is the address we publish the module under, i.e the account address you have in your `Move.toml` file (`myaddr`).

The `provider.getAccountResource()`expects an _account address_ that holds the resource we are looking for and a string representation of an on-chain _Move struct type_.

- account address - is the current connected account (we are getting it from the wallet account object)
- Move struct type string syntax:
  - The account address who holds the move module = our profile account address (You might want to change the `moduleAddress` const to be your own account address)
  - The module name the resource lives in = `todolist`
  - The resource name = `TodoList`

If the request succeeds and there is a resource for that account, we want to set our local state to `true`; otherwise, we would set it to `false`.

6. Let’s update ```import { Layout, Row, Col } from "antd"; ``` to import Button:
   ```import { Layout, Row, Col, Button  } from "antd"; ```

7. Let’s update our UI based on the `accountHasList` state:

```jsx
return (
  <>
    <Layout>
      <Row align="middle">
        <Col span={10} offset={2}>
          <h1>Our todolist</h1>
        </Col>
        <Col span={12} style={{ textAlign: "right", paddingRight: "200px" }}>
          <WalletSelector />
        </Col>
      </Row>
    </Layout>
    {!accountHasList && (
      <Row gutter={[0, 32]} style={{ marginTop: "2rem" }}>
        <Col span={8} offset={8}>
          <Button block type="primary" style={{ height: "40px", backgroundColor: "#3f67ff" }}>
            Add new list
          </Button>
        </Col>
      </Row>
    )}
  </>
);
```

We now have an **Add new list** button that appears only if the account doesn’t have a list.

Start the local server with `npm start`. You should see the **Add new list** button.

Next, let’s understand how to create a new list by [submitting data to chain](./5-submit-data-to-chain.md) in chapter 5.

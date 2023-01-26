---
title: "Fetch data from chain"
id: "fetch-data-from-chain"
---

# Fetch data from chain

Our UI logic relies on whether the connected account has created a list or not. If they have, we should display their todo list, if they haven’t we should display a button letting them the option to create a new list.

For that, we first need to fetch the connected account’s `TaskHolder` resource. this is because, in our smart contract, whenever someone creates a todo list we create and assign a `TaskHolder` resource to their account.

To fetch data from chain, we can use Aptos TS SDK. The SDK provides classes and functions for us to easily interact and query the Aptos chain

1. Stop the local server if running
2. on the `client` folder, run `npm i aptos@1.6.0`
3. On the `App.tsx` file import the `AptosClient` class like that

```js
import { AptosClient } from "aptos";
```

The TS SDK provides us with an `AptosClient` class where we can initialize and query the Aptos chain. `AptosClient` expects the get a `node_url` as an argument which is the network URL we want to interact with.

1. On the `App.tsx` file add the following

```js
const NODE_URL = "https://fullnode.devnet.aptoslabs.com";
const client = new AptosClient(NODE_URL);
```

That would initialize an AptosClient instance for us with the devnet node url.

Our app displays different UIs based on a user resource (i.e if a user has a list ⇒ if a user has a `TaskHolder` resource). For that, we need to know the current account connected to our app.

1. Import Wallet from the wallet adapter react provider

```js
import { useWallet } from "@aptos-labs/wallet-adapter-react";
```

2. Extract the account object from the wallet adapter

```js
function App (
	const { account } = useWallet();
	...
)
```

The `account` object is `null` if there is no account connected and holds the account info, like the account address, when account is connected

3. Next thing we want to do is to fetch the account’s TaskHolder resource.
   Let’s add a useEffect hook to our file that would call a function to fetch the resource whenever our account address changes.

```js
function App() {
  ...
  useEffect(() => {
    fetchList();
  }, [account?.address]);
  ...
}
```

4. Before creating our `fetchList` function, let’s also create a local state to store whether the account has a list

```js
function App (
  ...
  const [accountHasList, setAccountHasList] = useState<boolean>(false);
  ...
)
```

5. Our `useEffect` hook is calling a `fetchList` function, let’s create it.

```js
const fetchList = async () => {
  if (!account) return [];
  // change this to be your module account address
  const moduleAddress = "0xcbddf398841353776903dbab2fdaefc54f181d07e114ae818b1a67af28d1b018";
  try {
    const taskHolderResource = await client.getAccountResource(
      account.address,
      `${moduleAddress}::main::TasksHolder`
    );
    setAccountHasList(true);
  } catch (e: any) {
    setAccountHasList(false);
  }
};
```

`moduleAddress` is the address we publish the module under, i.e the account address you have on your `Move.toml` file (`myaddr`)

The `client.getAccountResource()`expects an `account address` that holds the resource we are looking for and a string representation of an on-chain `Move struct type` .

- account address - is the current connected account (we are getting it from the wallet account object)
- Move struct type string syntax
  - The account address who holds the move module = our profile account address (You might want to change the `moduleAddress` const to be your own account address)
  - The module name the resource lives in = `main`
  - The resource name = `TaskHolder`

If the request succeed and there is a resource for that account, we want to set our local state to true, otherwise we would set it to false.

6. Let’s update our UI based on the `accountHasList` state

```js
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

We now have a “add new list” button that only shows up if the account doesn’t have a list.

Start the local server with `npm start` , you should see the “Add new list” button. Let’s understand how we create a new list which is submit a transaction to chain.

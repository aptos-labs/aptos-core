---
title: "Your First Dapp"
slug: "your-first-dapp"
---

# Your First Dapp

In this tutorial, you will learn how to build a [dapp](https://en.wikipedia.org/wiki/Decentralized_application) on the Aptos blockchain. A dapp usually consists of a user interface written in JavaScript, which  interacts with one or more Move modules.

For this tutorial, we will use the Move module `HelloBlockchain` described in [Your First Move Module](first-move-module.md) and focus on building the user interface.

We will use:

- The [Aptos Typescript SDK][ts_sdk].
- The [Aptos Wallet][building_wallet], and
- The [Aptos CLI][install_cli] to interact with the Aptos blockchain.

The end result is a dapp that lets users publish and share snippets of text on the Aptos blockchain.

:::tip Full source code

The full source code for this tutorial is being updated. Meanwhile, the older one is available [here](https://github.com/aptos-labs/aptos-core/tree/53e240003e95c9b865441ea792ab4e1e8134a267/developer-docs-site/static/examples/typescript/dapp-example).
:::

## Prerequisites

### Aptos Wallet

Before starting this tutorial, install the [Aptos Wallet extension](../guides/building-wallet-extension.md).

After you install it:

1. Open the Wallet and click **Create a new wallet**. Then click **Create account** to create an Aptos Account.
2. Copy the private key. You will need it to set up the Aptos CLI in the next section.

:::tip
Ensure that your account has sufficient funds to perform transactions by clicking the **Faucet** button.
:::

### Aptos CLI

1. Install the [Aptos CLI][install_cli].

2. Run `aptos init`, and when it asks for your private key, paste the private key from the Aptos Wallet that you copied earlier. This will initialize the Aptos CLI to use the same account as used by the Aptos Wallet.

3. Run `aptos account list` to verify that it is working.

## Step 1: Set up a single page app

We will now set up the frontend user interface for our dapp. We will use [`create-react-app`](https://create-react-app.dev/) to set up the app in this tutorial, but neither React nor `create-react-app` are required. You can use your preferred JavaScript framework.

```bash
npx create-react-app first-dapp --template typescript
cd first-dapp
npm start
```

You will now have a basic React app running in your browser.

## Step 2: Integrate the Aptos Wallet Web3 API

The Aptos Wallet provides a Web3 API for dapps at `window.aptos`. You can see how it works by opening up the browser console and running `await window.aptos.account()`. It will print out the address corresponding to the account you set up in the Aptos Wallet.

Next we will update our app to use this API to display the Wallet account's address.

### Wait until `window.aptos` is defined

The first step when integrating with the `window.aptos` API is to delay rendering the application until the `window.onload` event has fired.

Open up `src/index.tsx` and change the following code snippet:

```typescript
root.render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
```

to this:

```typescript
window.addEventListener('load', () => {
  root.render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  );
});
```

This change will ensure that the `window.aptos` API has been initialized by the time we render the app (if we render too early, the Wallet extension may not have had a chance to initialize the API yet and thus `window.aptos` will be `undefined`).

### (Optional) TypeScript setup for `window.aptos`

If you are using TypeScript, you may also want to inform the compiler of the existence of the `window.aptos` API. Add the following to `src/index.tsx`:

```typescript
declare global {
  interface Window { aptos: any; }
}
```

This lets us use the `window.aptos` API without having to do `(window as any).aptos`.

### Display `window.aptos.account()` in the app

Our app is now ready to use the `window.aptos` API. We will change `src/App.tsx` to retrieve the value of `window.aptos.account()` (the wallet account) on initial render, store it in state, and then display it:

```typescript
import React from 'react';
import './App.css';

function App() {
  // Retrieve aptos.account on initial render and store it.
  const [address, setAddress] = React.useState<string | null>(null);
  React.useEffect(() => {
    window.aptos.account().then((data : {address: string}) => setAddress(data.address));
  }, []);

  return (
    <div className="App">
      <p><code>{ address }</code></p>
    </div>
  );
}

export default App;
```

Refresh the page and you will see your account address.

### Add some CSS

Next, replace the contents of `src/App.css`:

```css
a, input, textarea {
  display: block;
}

textarea {
  border: 0;
  min-height: 50vh;
  outline: 0;
  padding: 0;
  width: 100%;
}
```

## Step 3: Use the SDK to get data from the blockchain

The Wallet is now integrated with our dapp. Next, we will integrate the Aptos SDK to get data from the blockchain. We will use the Aptos SDK to retrieve information about our account and display that information on the page.

### Add the `aptos` dependency to `package.json`

First, add the SDK to the project's dependencies:

```bash
npm install --save aptos
```

You will now see `"aptos": "^0.0.20"` (or similar) in your `package.json`.

### Create an `AptosClient`

Now we can import the SDK and create an `AptosClient` to interact with the blockchain (technically it interacts with [the REST API](https://github.com/aptos-labs/aptos-core/tree/main/api), which interacts with the blockchain).

As our wallet account is on devnet, we will set up the `AptosClient` to interact with devnet as well. Add the following to `src/App.tsx`:

```typescript
import { Types, AptosClient } from 'aptos';

// Create an AptosClient to interact with devnet.
const client = new AptosClient('https://fullnode.devnet.aptoslabs.com/v1');

function App() {
  // ...

  // Use the AptosClient to retrieve details about the account.
  const [account, setAccount] = React.useState<Types.AccountData | null>(null);
  React.useEffect(() => {
    if (!address) return;
    client.getAccount(address).then(setAccount);
  }, [address]);

  return (
    <div className="App">
      <p><code>{ address }</code></p>
      <p><code>{ account?.sequence_number }</code></p>
    </div>
  );
}
```

Now, in addition to displaying the account address, the app will also display the account's `sequence_number`. This `sequence_number` represents the next transaction sequence number to prevent replay attacks of transactions. You will see this number increasing as you make transactions with the account.

## Step 4: Publish a Move module

Our dapp is now set up to read from the blockchain. The next step is to write to the blockchain. To do so, we will publish a Move module to our account.

The Move module provides a location for this data to be stored. Specifically, we will use the `HelloBlockchain` module from [Your First Move Module](first-move-module.md), which provides a resource called `MessageHolder` that holds a string (called `message`).

### Publish the `HelloBlockchain` module with the Aptos CLI

We will use the Aptos CLI to compile and publish the `HelloBlockchain` module.

1. Download [the `hello_blockchain` package](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/hello_blockchain).

2. Next, use the `aptos move publish` command (replacing `/path/to/hello_blockchain/` and `<address>`):

```bash
aptos move publish --package-dir /path/to/hello_blockchain/ --named-addresses HelloBlockchain=<address>
```

For example:

```bash
aptos move publish --package-dir ~/code/aptos-core/aptos-move/move-examples/hello_blockchain/ --named-addresses HelloBlockchain=0x5af503b5c379bd69f46184304975e1ef1fa57f422dd193cdad67dc139d532481
```

The `--named-addresses` replaces the named address `HelloBlockchain` in `HelloBlockchain.move` with the specified address. For example, if we specify `--named-addresses HelloBlockchain=0x5af503b5c379bd69f46184304975e1ef1fa57f422dd193cdad67dc139d532481`, then the following:

```rust
module HelloBlockchain::message {
```

becomes:

```rust
module 0x5af503b5c379bd69f46184304975e1ef1fa57f422dd193cdad67dc139d532481::message {
```

This makes it possible to publish the module for the given account (in this case our wallet account, `0x5af503b5c379bd69f46184304975e1ef1fa57f422dd193cdad67dc139d532481`).

Assuming that your account has enough funds to execute the transaction, you can now publish the `HelloBlockchain` module in your account. If you refresh the app, you will see that the account sequence number has increased from 0 to 1.

You can also verify that the module was published by going to the [Aptos Explorer](https://explorer.aptoslabs.com/) and looking up your account. If you scroll down to the Account Modules section, you should see something like the following:

```json
{
  "address": "0x5af503b5c379bd69f46184304975e1ef1fa57f422dd193cdad67dc139d532481",
  "name": "Message",
  "friends": [],
  "exposedFunctions": [
    {
      "name": "get_message",
      "visibility": "public",
      "genericTypeParams": [],
      "params": [
        "address"
      ],
      "_return": [
        "0x1::string::String"
      ]
    },
    {
      "name": "set_message",
      "visibility": "script",
      "genericTypeParams": [],
      "params": [
        "signer",
        "vector"
      ],
      "_return": []
    }
  ],
  "structs": [
    {
      "name": "MessageChangeEvent",
      "isNative": false,
      "abilities": [
        "drop",
        "store"
      ],
      "genericTypeParams": [],
      "fields": [
        {
          "name": "from_message",
          "type": "0x1::string::String"
        },
        {
          "name": "to_message",
          "type": "0x1::string::String"
        }
      ]
    },
    {
      "name": "MessageHolder",
      "isNative": false,
      "abilities": [
        "key"
      ],
      "genericTypeParams": [],
      "fields": [
        {
          "name": "message",
          "type": "0x1::string::String"
        },
        {
          "name": "message_change_events",
          "type": "0x1::event::EventHandle<0x5af503b5c379bd69f46184304975e1ef1fa57f422dd193cdad67dc139d532481::message::MessageChangeEvent>"
        }
      ]
    }
  ]
}
```

Make a note of `"name": "Message"`, we will use it in the next section.

### Add module publishing instructions to the dapp

As a convenience to the users, we can have the app display the `aptos move publish` command if the module does not exist. To do so, we will use the Aptos SDK to retrieve the account modules and look for one where `module.abi.name` equals `"Message"` (i.e., the `"name": "Message"` we saw in the Aptos Explorer).

Update `src/App.tsx`:

```typescript
function App() {
  // ...

  // Check for the module; show publish instructions if not present.
  const [modules, setModules] = React.useState<Types.MoveModule[]>([]);
  React.useEffect(() => {
    if (!address) return;
    client.getAccountModules(address).then(setModules);
  }, [address]);

  const hasModule = modules.some((m) => m.abi?.name === 'Message');
  const publishInstructions = (
    <pre>
      Run this command to publish the module:
      <br />
      aptos move publish --package-dir /path/to/hello_blockchain/
      --named-addresses HelloBlockchain={address}
    </pre>
  );

  return (
    <div className="App">
      {!hasModule && publishInstructions}
    </div>
  );
}
```

New users will be able to use this command to create a page for their account.

## Step 5: Write a message on the blockchain

Now that the module has been published, we are ready to use it to write a message on the blockchain. For this step we will use the `set_message` function exposed by the module.

### A transaction that calls the `set_message` function

The signature for `set_message` looks like this:

```move
public(script) fun set_message(account: signer, message_bytes: vector<u8>)
```

To call this function, we need to use the `window.aptos` API provided by the wallet to submit a transaction. Specifically, we will create a `entry_function_payload` transaction that looks like this:

```javascript
{
  type: "entry_function_payload",
  function: "<address>::message::set_message",
  arguments: ["<hex encoded utf-8 message>"],
  type_arguments: []
}
```

There is no need to provide the `account: signer` argument. Aptos provides it automatically.

However, we do need to specify the `message_bytes` argument: this is the `"<hex encoded utf-8 message>"` in the transaction. We need a way to convert a JS string to this format. We can do so by using `TextEncoder` to convert to utf-8 bytes and then a one-liner to hex encode the bytes.

Add this function to `src/App.tsx`:

```typescript
/** Convert string to hex-encoded utf-8 bytes. */
function stringToHex(text: string) {
  const encoder = new TextEncoder();
  const encoded = encoder.encode(text);
  return Array.from(encoded, (i) => i.toString(16).padStart(2, "0")).join("");
}
```

Using this function, our transaction payload becomes:

```javascript
{
  type: "entry_function_payload",
  function: "<address>::message::set_message",
  arguments: [stringToHex(message)],
  type_arguments: []
}
```

### Use the `window.aptos` API to submit the `set_message` transaction

Now that we understand how to use a transaction to call the `set_message` function, next we call this function from our app using `window.aptos.signAndSubmitTransaction()`.

We will add:

- A `<textarea>` where the user can input a message, and
- A `<button>` that calls the `set_message` function with the contents of the `<textarea>`.

Update `src/App.tsx`:

```typescript
function App() {
  // ...

  // Call set_message with the textarea value on submit.
  const ref = React.createRef<HTMLTextAreaElement>();
  const [isSaving, setIsSaving] = React.useState(false);
  const handleSubmit = async (e: any) => {
    e.preventDefault();
    if (!ref.current) return;

    const message = ref.current.value;
    const transaction = {
      type: "entry_function_payload",
      function: `${address}::message::set_message`,
      arguments: [stringToHex(message)],
      type_arguments: [],
    };

    try {
      setIsSaving(true);
      await window.aptos.signAndSubmitTransaction(transaction);
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="App">
      {hasModule ? (
        <form onSubmit={handleSubmit}>
          <textarea ref={ref} />
          <input disabled={isSaving} type="submit" />
        </form>
      ) : publishInstructions}
    </div>
  );
}

```

To test it:

- Type something in the `<textarea>` and submit the form.
- Find your account in the [Aptos Explorer](https://explorer.aptoslabs.com/) and you will now see a `MessageHolder` resource under Account Resources with the `message` you wrote.

If you don't see it, try a shorter message. Long messages may cause the transaction to fail because longer messages take more gas.

## Step 6: Display the message in the dapp

Now that the `MessageHolder` resource has been created, we can use the Aptos SDK to retrieve it and display the message.

### Get the wallet account's message

To retrieve the message, we will:

- First use `AptosClient.getAccountResources()` function to fetch the account's resources and store them in state.

- Then we will look for one whose `type` is `MessageHolder`. The full type is `$address::message::MessageHolder` as it is part of the `$address::message` module.

  In our example it is:

  ```typescript
   0x5af503b5c379bd69f46184304975e1ef1fa57f422dd193cdad67dc139d532481::message::MessageHolder
  ```

- We will use this for the initial value of the `<textarea>`.

Update `src/App.tsx`:

```typescript
function App() {
  // ...

  // Get the message from account resources.
  const [resources, setResources] = React.useState<Types.AccountResource[]>([]);
  React.useEffect(() => {
    if (!address) return;
    client.getAccountResources(address).then(setResources);
  }, [address]);
  const resourceType = `${address}::message::MessageHolder`;
  const resource = resources.find((r) => r.type === resourceType);
  const data = resource?.data as {message: string} | undefined;
  const message = data?.message;

  return (
    // ...
          <textarea ref={ref} defaultValue={message} />
    // ...
  );
}
  ```

To test it:

- Refresh the page and you will see the message you wrote earlier.
- Change the text, submit the form, and refresh the page again. You will see that the contents have been updated with your new message.

This confirms that you are reading and writing messages on the Aptos blockchain.

### Display messages from other accounts

So far, we have built a "single-player" dapp where you can read and write a message on your own account. Next, we will make it possible for other people to read messages, including people who do not have the Aptos Wallet installed.

We will set it up so that going to the URL `/<account address>` displays the message stored at `<account address>` (if it exists).

- If the app is loaded at `/<account address>`, we will also disable editing.

- If editing is enabled, we will show a "Get public URL" link so you can share your message.

Update `src/App.tsx`:

```typescript
function App() {
  // Retrieve aptos.account on initial render and store it.
  const urlAddress = window.location.pathname.slice(1);
  const isEditable = !urlAddress;
  const [address, setAddress] = React.useState<string | null>(null);
  React.useEffect(() => {
    if (urlAddress) {
      setAddress(urlAddress);
    } else {
      window.aptos.account().then((data : {address: string}) => setAddress(data.address));
    }
  }, [urlAddress]);

  // ...

  return (
    <div className="App">
      {hasModule ? (
        <form onSubmit={handleSubmit}>
          <textarea ref={ref} defaultValue={message} readOnly={!isEditable} />
          {isEditable && (<input disabled={isSaving} type="submit" />)}
          {isEditable && (<a href={address!}>Get public URL</a>)}
        </form>
      ) : publishInstructions}
    </div>
  );
}
```

That concludes this tutorial.

[building_wallet]: /guides/building-wallet-extension
[install_cli]: /cli-tools/aptos-cli-tool/install-aptos-cli
[ts_sdk]: /sdks/ts-sdk/index

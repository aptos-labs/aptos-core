---
title: "Your first dapp"
---

# Your first dapp

In this tutorial, we will see how to build a [dapp](https://en.wikipedia.org/wiki/Decentralized_application) on the Aptos Blockchain. A dapp typically consists of a user interface written in JavaScript that interacts with one or more Move Modules. For this tutorial, we will use the `HelloBlockchain` Move Module described in [Your first Move Module](your-first-move-module) and focus on building the user interface. We will see how to use the [Aptos SDK](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/typescript/sdk), the [Aptos Wallet](https://github.com/aptos-labs/aptos-core/tree/main/ecosystem/web-wallet), and the [Aptos CLI](https://github.com/aptos-labs/aptos-core/tree/main/crates/aptos) to interact with blockchain. The end result is a dapp that lets users publish and share snippets of text on the Aptos Blockchain.

The full source code for this tutorial is available [here](https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/static/examples/typescript/dapp-example).

## Prerequisites

### Aptos Wallet

Before starting this tutorial, you will need to [install the Aptos Wallet extension](building-wallet-extension). Once installed, open the Wallet and click "Create a new wallet" and then "Create account" to create an Aptos Account. Copy the private key; you will need it to set up the Aptos CLI in the next section.

Ensure that your account has sufficient funds to perform transactions by clicking the Faucet button.

### Aptos CLI

First, [install the Aptos CLI](https://github.com/aptos-labs/aptos-core/tree/main/crates/aptos). Once installed, we will initialize the CLI to use the same account as the Aptos Wallet. Run `aptos init`, and when it asks for your private key, paste the private key from the Aptos Wallet that you copied earlier. You can run `aptos account list` to verify that it's working.

## Step 1: Set up a single page app

We'll now set up the frontend user interface for our dapp. We will use [`create-react-app`](https://create-react-app.dev/) to set up the app in this tutorial, but neither React nor `create-react-app` are required - feel free to use whatever JavaScript framework you prefer.

```console
$ npx create-react-app first-dapp --template typescript
$ cd first-dapp
$ npm start
```

You should now have a basic React app up and running in your browser.

## Step 2: Integrate the Aptos Wallet Web3 API

The Aptos Wallet provides a Web3 API for dapps at `window.aptos`. You can see how it works by opening up the browser console and running `await window.aptos.account()`. It should print out the address corresponding to the account you set up in the Aptos Wallet.

Let's update our app to use this API to display the Wallet account's address.

### Wait until `window.aptos` is defined

The first thing we want to do when integrating with the `window.aptos` API is to delay rendering the application until the `window.onload` event has fired. Open up `src/index.tsx` and change this...

```typescript
root.render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
```

...to this:

```typescript
window.addEventListener('load', () => {
  root.render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  );
});
```

This will ensure that the `window.aptos` API has been initialized by the time we render the app (if we render too early, the Wallet extension may not have had a chance to initialize the API yet and thus `window.aptos` will be `undefined`). 

### Optional: TypeScript setup for `window.aptos`

If you're using TypeScript, you may also want to inform the compiler of the existence of the `window.aptos` API. Add the following to `src/index.tsx`:

```typescript
declare global {
  interface Window { aptos: any; }
}
```

This way we can use the `window.aptos` API without having to do `(window as any).aptos`.

### Display `window.aptos.account()` in the app

With that out of the way, our app is now ready to use the `window.aptos` API. We will change `src/App.tsx` to retrieve the value of `window.aptos.account()` (the wallet account) on initial render, store it in state, and then display it:

```typescript
import React from 'react';
import './App.css';

function App() {
  // Retrieve aptos.account on initial render and store it.
  const [address, setAddress] = React.useState<string | null>(null);
  React.useEffect(() => {
    window.aptos.account().then(setAddress);
  }, []);

  return (
    <div className="App">
      <p><code>{ address }</code></p>
    </div>
  );
}

export default App;
```

Refresh the page and you should see your account address.

### Add some CSS

To follow along with the rest of the tutorial, replace the contents of `src/App.css`:

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

Now that we have the Wallet integrated with our dapp, let's integrate the Aptos SDK to get data from the blockchain. We'll use it to retrieve information about our account and then we'll display that information on the page.

### Add the `aptos` dependency to `package.json`

First, add the SDK to the project's dependencies:

```console
$ npm add --save aptos
```

You should now see `"aptos": "^0.0.20"` (or similar) in your `package.json`.

### Create an `AptosClient`

Now we can import the SDK and create an `AptosClient` to interact with the blockchain (technically it interacts with [the REST API](https://github.com/aptos-labs/aptos-core/tree/main/api), which interacts with the blockchain). Since our wallet account is on devnet, we'll set up the `AptosClient` to interact with devnet as well. Add the following to `src/App.tsx`:

```typescript
import { Types, AptosClient } from 'aptos';

// Create an AptosClient to interact with devnet.
const client = new AptosClient('https://fullnode.devnet.aptoslabs.com');

function App() {
  // ...

  // Use the AptosClient to retrieve details about the account.
  const [account, setAccount] = React.useState<Types.Account | null>(null);
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

Now in addition to the account address, the app will also display the account's `sequence_number` (which represents the next transaction sequence number to prevent replay attacks of transactions). You'll see this number increasing as you make transactions with the account.

## Step 4: Publish a Move Module

Our dapp is now set up to read from the blockchain. The next step is to write to the blockchain. To do so, we'll need to publish a Move Module to our account. The Move Module provides a location for this data to be stored. Specifically, we'll use the `HelloBlockchain` module from [Your first Move Module](your-first-move-module), which provides a resource called `MessageHolder` that holds a string (called `message`).

### Publish the `HelloBlockchain` module with the Aptos CLI

We'll use the Aptos CLI to compile and publish the `HelloBlockchain` module. First, download [the `hello_blockchain` package](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/hello_blockchain). Next, use the `aptos move publish` command (replacing `/path/to/hello_blockchain/` and `<address>`):

```console
$ aptos move publish --package-dir /path/to/hello_blockchain/ --named-addresses HelloBlockchain=<address>
```

For example:

```console
$ aptos move publish --package-dir ~/code/aptos-core/aptos-move/move-examples/hello_blockchain/ --named-addresses HelloBlockchain=0x5af503b5c379bd69f46184304975e1ef1fa57f422dd193cdad67dc139d532481
```

The `--named-addresses` part may need some explanation. It replaces the named address `HelloBlockchain` in `HelloBlockchain.move` with the specified address. For example, if we specify `--named-addresses HelloBlockchain=0x5af503b5c379bd69f46184304975e1ef1fa57f422dd193cdad67dc139d532481`, then this...

```move
module HelloBlockchain::Message {
```

...becomes:

```move
module 0x5af503b5c379bd69f46184304975e1ef1fa57f422dd193cdad67dc139d532481::Message {
```

This makes it possible to publish the module for the given account (in this case our wallet account, `0x5af503b5c379bd69f46184304975e1ef1fa57f422dd193cdad67dc139d532481`).

Assuming that your account has enough funds to execute the transaction, the `HelloBlockchain` module will now be published on your account. If you refresh the app, you should see that the account sequence number has increased from 0 to 1.

You can also verify that the module was published by going to the [Aptos Explorer](https://explorer.devnet.aptos.dev/) and looking up your account. If you scroll down to the Account Modules section, you should see something like the following:

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
        "0x1::ASCII::String"
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
          "type": "0x1::ASCII::String"
        },
        {
          "name": "to_message",
          "type": "0x1::ASCII::String"
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
          "type": "0x1::ASCII::String"
        },
        {
          "name": "message_change_events",
          "type": "0x1::Event::EventHandle<0x5af503b5c379bd69f46184304975e1ef1fa57f422dd193cdad67dc139d532481::Message::MessageChangeEvent>"
        }
      ]
    }
  ]
}
```

Take note of `"name": "Message"` - we'll use it in the next section.

### Add module publishing instructions to the dapp

As a convenience to our users, we can have the app display the `aptos move publish` command if the module doesn't exist. To do so, we'll use the SDK to retrieve the account modules and look for one where `module.abi.name` equals `"Message"` (i.e. the `"name": "Message"` we saw in the Explorer). Update `src/App.tsx`:

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

Now that the module has been published, we're ready to go ahead and use it to write a message on the blockchain. We'll use the `set_message` function exposed by the module.

### A transaction that calls the `set_message` function

The signature for `set_message` looks like this:

```move
public(script) fun set_message(account: signer, message_bytes: vector<u8>)
```

To call this function, we need to use the `window.aptos` API provided by the wallet to submit a transaction. Specifically, we'll create a `script_function_payload` transaction that looks like this:

```javascript
{
  type: "script_function_payload",
  function: "<address>::Message::set_message",
  arguments: ["<hex encoded utf-8 message>"],
  type_arguments: []
}
```

There's no need to provide the `account: signer` argument; Aptos provides it automatically. However, we do need to specify the `message_bytes` argument. That's the `"<hex encoded utf-8 message>"` in the transaction. We need a way to convert a JS string to this format. We can do so by using `TextEncoder` to convert to utf-8 bytes and then a one-liner to hex encode the bytes. Add this function to `src/App.tsx`:

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
  type: "script_function_payload",
  function: "<address>::Message::set_message",
  arguments: [stringToHex(message)],
  type_arguments: []
}
```

### Use the `window.aptos` API to submit the `set_message` transaction

Now that we understand how to use a transaction to call the `set_message` function, let's call it from our app using `window.aptos.signAndSubmitTransaction()`. We'll add a `<textarea>` where the user can input a message and a `<button>` that calls the `set_message` function with the contents of the `<textarea>`. Update `src/App.tsx`:

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
      type: "script_function_payload",
      function: `${address}::Message::set_message`,
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

Type something in the `<textarea>` and submit the form. Find your account in the [Aptos Explorer](https://explorer.devnet.aptos.dev/) and you should now see a `MessageHolder` resource under Account Resources with the `message` you wrote. If you don't see it, try a shorter message - long messages may cause the transaction to fail because longer messages take more gas.

## Step 6: Display the message in the dapp

Now that the `MessageHolder` resource has been created, we can use the SDK to retrieve it and display the message.

### Get the wallet account's message

To retrieve the message, we'll first use `AptosClient.getAccountResources()` function to fetch the account's resources and store them in state. Then we'll look for one whose `type` is `MessageHolder` (the full type is `$address::Message::MessageHolder` since it's part of the `$address::Message` module - `0x5af503b5c379bd69f46184304975e1ef1fa57f422dd193cdad67dc139d532481::Message::MessageHolder` in our example). We'll use this for the initial value of the `<textarea>`. Update `src/App.tsx`:

```typescript
function App() {
  // ...

  // Get the message from account resources.
  const [resources, setResources] = React.useState<Types.AccountResource[]>([]);
  React.useEffect(() => {
    if (!address) return;
    client.getAccountResources(address).then(setResourdces);
  }, [address]);
  const resourceType = `${address}::Message::MessageHolder`;
  const resource = resources.find((r) => r.type === resourceType);
  const data = resource?.data as {message: string} | undefined;
  const message = data?.message;

  return (
    // ...
          <textarea ref={ref} defaultValue={message} />
    // ...
  );
```

Refresh the page, and you should see the message you wrote earlier. Change the text, submit the form, and refresh the page again. You'll see that the contents have been updated with your new message. You're reading and writing messages on the Aptos blockchain!

### Display messages from other accounts

So far, we've built a "single-player" dapp where you can read and write a message on your own account. Let's take it a step further and make it possible for other people to read messages, including people who don't have the Aptos Wallet installed. We'll set it up so that going to the URL `/<account address>` displays the message stored at `<account address>` (if it exists). If the app is loaded at `/<account address>`, we'll also disable editing. If editing is enabled, we'll show a "Get public URL" link so you can share your message. Update `src/App.tsx`:

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
      window.aptos.account().then(setAddress);
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

That concludes this tutorial. Thanks for following along!

---
title: "Your First Dapp"
slug: "your-first-dapp"
---

# Your First Dapp

In this tutorial, you will learn how to build a [dapp](https://en.wikipedia.org/wiki/Decentralized_application)
on the Aptos blockchain. A dapp usually consists of a graphical user interface, which interacts with one or more Move
modules.  This dapp will let users publish and share snippets of text on the Aptos blockchain.

For this tutorial, we will use the Move module [`hello_blockchain`](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/hello_blockchain)
described in [Your First Move Module](first-move-module.md) and focus on building the user interface around the module.

For a more comprehensive view of this process, see [Build an End-to-End Dapp on Aptos](build-e2e-dapp/index.md).

We will use the:

* [TypeScript SDK](../sdks/ts-sdk/index.md)
* [Petra Wallet](https://petra.app)
* [Aptos CLI](../tools/aptos-cli/use-cli/use-aptos-cli.md)

:::tip Full source code

We recommend becoming familiar with the newer full source code documented in the [Build an End-to-End Dapp on Aptos](build-e2e-dapp/index.md) tutorial. The full source code for this tutorial is still available in the [`dapp-example`](https://github.com/aptos-labs/aptos-core/tree/53e240003e95c9b865441ea792ab4e1e8134a267/developer-docs-site/static/examples/typescript/dapp-example) directory.
:::

## Prerequisites

### Aptos Wallet

Before starting this tutorial, you'll need a chrome extension wallet to interact with the dapp, such as, the the [Petra wallet extension](https://petra.app).

If you haven't installed the Petra wallet extension before:
1. Open the Wallet and click **Create a new wallet**. Then click **Create account** to create an Aptos Account.
2. Copy the private key. You will need it to set up the Aptos CLI in the next section.
3. See the [user instructions](https://petra.app/docs/use) on petra.app for help.
4. Switch to the Devnet network by clicking, settings, network, and selecting **devnet**.
5. Click the faucet button to ensure you can receive test tokens.

If you already have the Petra wallet installed, we suggest you create a new wallet for purposes of this tutorial.
1. In the extension, go to settings, switch account, add account, create new account to create a new account.
2. Switch to the Devnet network by clicking, settings, network, and selecting **devnet**.
3. Click the faucet button to ensure you can receive test tokens.

:::tip
Ensure your account has sufficient funds to perform transactions by clicking the **Faucet** button.
:::

### Aptos CLI

We will also be installing the Aptos CLI so that we can publish 

1. Install the [Aptos CLI](../tools/aptos-cli/install-cli/index.md).

2. Run `aptos init --profile my-first-nft`.

3. Select the network `devnet`

4. When prompted for your private key, paste the private key from the Petra Wallet and press **Return**. 
   1. You can find the private key by going to settings, manage account, show the private key, and copy that field.

You will see output resembling:

```text
Account <account-number> has been already found onchain

---
Aptos CLI is now set up for account <account-number> as profile my-first-nft!  Run `aptos --help` for more information about commands
{
  "Result": "Success"
}
```
This initializes the Aptos CLI to use the same account as used by the Aptos Wallet.

5. Run `aptos account list --profile my-first-nft` to verify that it is working. You should see your account address listed in the `addr` field for all events.

## Step 1: Set up a single page app

We will now set up the frontend user interface for our dapp. We will use [`create-react-app`](https://create-react-app.dev/) to set up the app in this tutorial, but neither React nor `create-react-app` are required. You can use your preferred JavaScript framework.

First run:

```bash
npx create-react-app first-dapp --template typescript
```

Accept installation of the `create-react-app` package if prompted. Then navigate to the newly created `first-dapp` directory:

```bash
cd first-dapp
```

And start the app with:

```bash
npm start
```

You will now have a basic React app running in your browser at: http://localhost:3000/

## Step 2: Integrate the Aptos Wallet Web3 API

The Aptos Wallet provides a Web3 API for dapps at `window.aptos`. You can see how it works by opening up the browser console and running `await window.aptos.account()`. It will print out the address corresponding to the account you set up in the Aptos Wallet.

Next we will update our app to use this API to display the Wallet account's address.

### Wait until `window.aptos` is defined

The first step when integrating with the `window.aptos` API is to delay rendering the application until the `window.onload` event has fired.

Quit the app by hitting Ctrl-C in the terminal running the `npm start` process.

Still in the `first-dapp` directory, open the `src/index.tsx` file and change the following code snippet:

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

This change will ensure that the `window.aptos` API has been initialized by the time we render the app. If we render too early, the Wallet extension may not have had a chance to initialize the API yet and thus `window.aptos` will be `undefined`.

To see the change, once again run: `npm start`

### (Optional) TypeScript setup for `window.aptos`

If you are using TypeScript, you may also want to inform the compiler of the existence of the `window.aptos` API. Add the following to `src/index.tsx`:

```typescript
declare global {
  interface Window { aptos: any; }
}
```

This lets us use the `window.aptos` API without having to do `(window as any).aptos`.

### Display `window.aptos.account()` in the app

Our app is now ready to use the `window.aptos` API. We will change `src/App.tsx` to retrieve the value of `window.aptos.account()` (the wallet account) on initial render, store it in state, and then display it by replacing the contents in the file with:

```typescript
import React from 'react';
import './App.css';

function App() {
  // Retrieve aptos.account on initial render and store it.
  const [address, setAddress] = React.useState<string | null>(null);
  
  /**
   * init function
   */
  const init = async() => {
    // connect
    const { address, publicKey } = await window.aptos.connect();
    setAddress(address);
  }
  
  React.useEffect(() => {
     init();
  }, []);

  return (
    <div className="App">
      <p>Account Address: <code>{ address }</code></p>
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

You will now see `"aptos": "^1.3.15"` (or similar) in your `package.json`.

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
      <p>Account Address: <code>{ address }</code></p>
      <p>Sequence Number: <code>{ account?.sequence_number }</code></p>
    </div>
  );
}
```

Now, in addition to displaying the account address, the app will also display the account's `sequence_number`. This `sequence_number` represents the next transaction sequence number to prevent replay attacks of transactions. You will see this number increasing as you make transactions with the account.

:::tip
If the account you're using for this application doesn't exist on-chain, you will not see a sequence number.  You'll need
to create the account first via a faucet.
:::

## Step 4: Publish a Move module

Our dapp is now set up to read from the blockchain. The next step is to write to the blockchain. To do so, we will publish a Move module to our account.

The Move module provides a location for this data to be stored. Specifically, we will use the `hello_blockchain` module from [Your First Move Module](first-move-module.md), which provides a resource called `MessageHolder` that holds a string (called `message`).

<details>
<summary>Publish the `hello_blockchain` module with the Aptos CLI</summary>
We will use the Aptos CLI to compile and publish the `hello_blockchain` module.

1. Download [the `hello_blockchain` package](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/hello_blockchain).

2. Use the `aptos move publish` command (replacing `/path/to/hello_blockchain/` and `<address>`):

```bash
aptos move publish --profile my-first-nft --package-dir /path/to/hello_blockchain/ --named-addresses hello_blockchain=<address>
```

For example:

```bash
aptos move publish --profile my-first-nft --package-dir ~/code/aptos-core/aptos-move/move-examples/hello_blockchain/ --named-addresses hello_blockchain=0x5af503b5c379bd69f46184304975e1ef1fa57f422dd193cdad67dc139d532481
```

The `--named-addresses` replaces the named address `hello_blockchain` in `hello_blockchain.move` with the specified address. For example, if we specify `--named-addresses hello_blockchain=0x5af503b5c379bd69f46184304975e1ef1fa57f422dd193cdad67dc139d532481`, then the following:

```rust
module hello_blockchain::message {
```

becomes:

```rust
module 0x5af503b5c379bd69f46184304975e1ef1fa57f422dd193cdad67dc139d532481::message {
```

This makes it possible to publish the module for the given account, in this case our wallet account:
`0x5af503b5c379bd69f46184304975e1ef1fa57f422dd193cdad67dc139d532481`

Assuming that your account has enough funds to execute the transaction, you can now publish the `hello_blockchain` module in your account. If you refresh the app, you will see that the account sequence number has increased from 0 to 1.

You can also verify the module was published by going to the [Aptos Explorer](https://explorer.aptoslabs.com/) and looking up your account. If you scroll down to the *Account Modules* section, you should see something resembling:

```json
{
  "address": "0x5af503b5c379bd69f46184304975e1ef1fa57f422dd193cdad67dc139d532481",
  "name": "message",
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

Make a note of `"name": "message"; we will use it in the next section.
</details>

<details>
<summary>Publish the `hello_blockchain` module with the TS SDK</summary>
We will use the Aptos CLI to compile the `hello_blockchain` module and use the [TypeScript SDK](../sdks/ts-sdk/index.md) to publish the module.

1. Download the [`hello_blockchain`](https://github.com/aptos-labs/aptos-core/tree/main/aptos-move/move-examples/hello_blockchain) package.

2. Next, use the `aptos move compile --save-metadata` command (replacing `/path/to/hello_blockchain/` and `<address>`):

```bash
aptos move compile --save-metadata --package-dir /path/to/hello_blockchain/ --named-addresses hello_blockchain=<address>
```

For example:

```bash
aptos move compile --save-metadata --package-dir ~/code/aptos-core/aptos-move/move-examples/hello_blockchain/ --named-addresses hello_blockchain=0x5af503b5c379bd69f46184304975e1ef1fa57f422dd193cdad67dc139d532481
```

The `--named-addresses` replaces the named address `hello_blockchain` in `hello_blockchain.move` with the specified address. For example, if we specify `--named-addresses hello_blockchain=0x5af503b5c379bd69f46184304975e1ef1fa57f422dd193cdad67dc139d532481`, then the following:

```rust
module hello_blockchain::message {
```

becomes:

```rust
module 0x5af503b5c379bd69f46184304975e1ef1fa57f422dd193cdad67dc139d532481::message {
```

This makes it possible to publish the module for the given account, in this case our wallet account: `0x5af503b5c379bd69f46184304975e1ef1fa57f422dd193cdad67dc139d532481`

The `--save-metadata` argument, if set, generates and saves the package metadata in the package's `build` directory. This metadata can be used to construct a transaction to publish a package.

At this point, we should have a `build` folder in the same directory of our `hello_blockchain` folder. The next step would be to publish the module to the chain. 
The TypeScript SDK provides us a `publishPackage()` function where it expects to get both package metadata and the move module as `Uint8Array`. We can supply this by converting both the `package-metadata.bcs` file and the `bytecode_modules/message.mv` module into hex strings (using a command, below), and then to `Uint8Array` (using the SDK).

Convert `package-metadata.bcs` file and the `bytecode_modules/message.mv` module into hex strings:

Navigate to the `hello_blockchain/build/Example` directory:
```bash
cd hello_blockchain/build/Example
```

Convert `package-metadata.bcs` to a hex string. On macOS and Linux, we can use the command:
```bash
cat package-metadata.bcs | od -v -t x1 -A n | tr -d ' \n'
```
That will output a hex string we can later use.

Convert `message.mv` to a hex string. On Mac and Linux we can use the command:
```bash
cat bytecode_modules/message.mv | od -v -t x1 -A n | tr -d ' \n'
```
That will also output a hex string we can later use. Keep both of the hex strings ready!

Back to our react app, let's add a button to click on to publish the module, use the `publishPackage` function TypeScript SDK provides us and display a link to get the account's resources where we can see the published module.

We would need our account's private key to initialize an `AptosAccount` to publish the module with. You can get the private key from the Petra Wallet by going to: **Settings** > **Manage account**, show the private key, and copy that field. Since a private key is *very* sensitive data, we dont want to expose it in the code but rather hold it in an `.env` file and use it from there.

1. Create a new `.env` file on the `root` of the project and add to the file:
```bash
REACT_APP_ACCOUNT_PK=<account-private-key>
```
Make sure to restart the local server so the app will load the new `.env` file.

2. Add the following to `src/App.tsx`, where:
- `process.env.REACT_APP_ACCOUNT_PK` holds the account private key. 
- `<package-metadata.bcs hex string>` is the `package-metadata.bcs` hex string output we get from the previous step.
- `<message.mv hex string>` is the `message.mv` hex string output we get from the previous step.

```typescript
import { Types, AptosClient, AptosAccount, HexString, TxnBuilderTypes} from "aptos";
  // ...

function App() {
  // ...

  // Publish the module using the TS SDK
  const [publishPackageTxnHash, setPublishPackageTxnHash] = useState<string | null>(null);
  const [isPublishing, setIsPublishing] = useState<boolean>(false);
  const onPublishModule = async () => {
    if (!process.env.REACT_APP_ACCOUNT_PK) return;
    setIsPublishing(true);
    const aptosAccount = new AptosAccount(
      new HexString(process.env.REACT_APP_ACCOUNT_PK).toUint8Array()
    );
    try{
      const txnHash = await client.publishPackage(
      aptosAccount,
      new HexString(
        // package-metadata
        "<package-metadata.bcs hex string>"
      ).toUint8Array(),
      [
        new TxnBuilderTypes.Module(
          new HexString(
            // modules
            "<message.mv hex string>"
          ).toUint8Array()
        ),
      ]
    );
      await client.waitForTransaction(txnHash);
      setPublishPackageTxnHash(txnHash);
    }catch(error: any){
      console.log("publish error", error)
    }finally{
      setIsPublishing(false);
    }
  };

  return (
    <div className="App">
      // ...
      <div>
        <button onClick={onPublishModule} disabled={isPublishing}>
          Publish Package
        </button>
        {publishPackageTxnHash && (
          <div>
            <p>
              <a
                href={`https://fullnode.devnet.aptoslabs.com/v1/accounts/${address}/modules`}
                target="_blank"
              >
                Account modules
              </a>
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
```
We wrap our publishing attempt in a `try / catch` block to catch any potential errors coming from `await client.waitForTransaction(txnHash);`.

`waitForTransaction(txnHash)` waits for a transaction (given a transaction hash) to move past pending state and can end up in one of the 4 states:

- processed and successfully committed to the blockchain
- rejected and is not committed to the blockchain
- committed but execution failed
- not processed within the specified timeout

`setIsPublishing()` is an internal state to know if our app is currently publishing, if it is we want to disable the "Publish Package" button. When it is done publishing, we want to enable the "Publish Package" button. We set it to`true` when we start publishing the package and to `false` inside the `finally` block whether it succeed or not.

`setPublishPackageTxnHash()` is an internal state for us to keep the transaction hash we just published to know if we should display the `Account modules` link

#### Publish the package

Click the **Publish Package** button. Once the module has been published, we should see an **Account modules** link. By clicking it, we should see something resembling:

```json
{
  "address": "0x5af503b5c379bd69f46184304975e1ef1fa57f422dd193cdad67dc139d532481",
  "name": "message",
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

Make a note of `"name": "message"`; we will use it in the next section.
</details>

### Add module publishing instructions to the dapp

As a convenience to the users, we can have the app display the `aptos move publish` command if the module does not exist. To do so, we will use the Aptos SDK to retrieve the account modules and look for one where `module.abi.name` equals `"message"` (i.e., the `"name": "message"` we saw in the Aptos Explorer).

Update `src/App.tsx`:

```typescript
function App() {
  // ...

  // Check for the module; show publish instructions if not present.
  const [modules, setModules] = React.useState<Types.MoveModuleBytecode[]>([]);
  React.useEffect(() => {
    if (!address) return;
    client.getAccountModules(address).then(setModules);
  }, [address]);

  const hasModule = modules.some((m) => m.abi?.name === 'message');
  const publishInstructions = (
    <pre>
      Run this command to publish the module:
      <br />
      aptos move publish --package-dir /path/to/hello_blockchain/
      --named-addresses hello_blockchain={address}
    </pre>
  );

  return (
    <div className="App">
      // ...
      {!hasModule && publishInstructions}
    </div>
  );
}
```

New users will be able to use this command to create a page for their account.

In this step, we can also hide the **Publish Package** button when the module does exist.
Update the `button` on the `src/App.tsx` with:

```typescript
function App() {
  // ...

  return (
    <div className="App">
      // ...
      {!hasModule && <button onClick={onPublishModule} disabled={isPublishing}>
          Publish Package
        </button>}
    </div>
  );
}
```

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
  arguments: ["Message to store"],
  type_arguments: []
}
```

There is no need to provide the `account: signer` argument. Aptos provides it automatically.

However, we do need to specify the `message` argument: this is the `"Message to store"` in the transaction.

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
      arguments: [message],
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
          <p>On-chain message</p>
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
  const [resources, setResources] = React.useState<Types.MoveResource[]>([]);
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
          <textarea defaultValue={message} />
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
          <p>On-chain message</p>
          <textarea ref={ref} defaultValue={message} readOnly={!isEditable} />
          {isEditable && (<input disabled={isSaving} type="submit" />)}
          {isEditable && (<a href={address!}>Get public URL</a>)}
        </form>
      ) : publishInstructions}
    </div>
  );
}
```

This concludes the tutorial.

## Supporting documentation

* [Aptos CLI](../tools/aptos-cli/use-cli/use-aptos-cli.md)
* [TypeScript SDK](../sdks/ts-sdk/index.md)
* [Wallet Standard](../standards/wallets.md)

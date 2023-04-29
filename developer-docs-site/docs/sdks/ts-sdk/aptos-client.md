---
title: "AptosClient Class"
slug: "typescript-sdk-aptos-client-class"
---

The [AptosClient](https://aptos-labs.github.io/ts-sdk-doc/classes/AptosClient.html) class is a component of the aptos SDK that enables developers to interact with the network through the use of REST APIs generated from an [OpenAPI](https://aptos-labs.github.io/ts-sdk-doc/) specification.

The `AptosClient` is used to communicate with the Aptos REST API and handling of errors and exceptions.
In addition, the `AptosClient` component supports submitting transactions in BCS format, which prepares and signs the raw transactions on the client-side. This method leverages the BCS Library and Transaction Builder for constructing the transaction payloads.

### OpenAPI

The [AptosClient](https://aptos-labs.github.io/ts-sdk-doc/classes/AptosClient.html) uses the [OpenAPI](https://aptos-labs.github.io/ts-sdk-doc/) specification to generate a set of classes that represent the various endpoints and operations of the Aptos REST API.

`OpenAPI` is a specification for building and documenting RESTful APIs. It provides a standard format for describing the structure of an API, including the available endpoints, methods, input and output parameters. By using the OpenAPI specification, developers integrated with the aptos SDK can ensure that their APIs are consistent, well-documented, and easily integrated with other applications.

### Usage

To use the AptosClient class, you will need to create an instance of the AptosClient class and call the desired API method. The AptosClient object will handle the HTTP requests and responses and return the result to your application.

### Configuration

Before using the AptosClient class, you will need to configure it with the necessary parameters. These parameters may include the network endpoint URL, custom configuration, and any other required settings. You can configure the AptosClient class by passing in the necessary parameters when you initialize the client object.

### Initialization

To initialize the AptosClient class, you will need to pass in the necessary configuration parameters. Here is an example:

```js
import { AptosClient } from "aptos";

const client = new AptosClient("https://fullnode.testnet.aptoslabs.com");
```

### Making API fetch calls

To make an API call, you will need to call the appropriate method on the AptosClient object. The method name and parameters will depend on the specific API you are using. Here is an example:

```js
const accountResources = await provider.getAccountResources("0x123");
```

In this example, we are using the `getAccountResources()` method to retrieve the resources of an account with the address `0x123`.

### Submit transaction to chain

To submit a transaction to chain, you will need to build a transaction payload to be submitted to chain. Here is an example:

```js
const alice = new AptosAccount();

const payload = {
  function: "0x123::todolist::create_task",
  type_arguments: [],
  arguments: ["read aptos.dev"],
};

const rawTxn = await client.generateTransaction(alice.address(), payload);
const bcsTxn = AptosClient.generateBCSTransaction(alice, rawTxn);
const transactionRes = await client.submitSignedBCSTransaction(bcsTxn);
```

`function` - is built from the module address, module name and the function name.
`type_arguments` - this is for the case a Move function expects a generic type argument.
`arguments` - the arguments the function expects.

:::tip
You can use the `AptosClient` class directly or the [Provider](./sdk-client-layer.md) class (preferred).
:::

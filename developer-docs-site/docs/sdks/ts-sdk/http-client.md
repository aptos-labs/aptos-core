---
title: "HTTP Client"
---

### Default HTTP Client

The SDK uses [@aptos-labs/aptos-client](https://www.npmjs.com/package/@aptos-labs/aptos-client) library with the ability to modify some request configurations like AUTH_TOKEN, HEADERS, etc.

The `@aptos-labs/aptos-client` package supports `http2` protocol and implements 2 clients environment based:

- **axios** - To use in a browser environment (in a browser env it is up to the browser and the server to negotiate http2 connection)
- **got** - To use in a node environment (to support http2 in node environment, still the server must support http2 also)

### Custom HTTP Client

Sometimes developers want to set custom configurations or use a specific http client for queries.

The SDK supports a custom client configuration as a function with this signature:

```ts
<Req, Res>(requestOptions: ClientRequest<Req>): Promise<ClientResponse<Res>>
```

:::note
Both `ClientRequest` and `ClientResponse` are types defined in the SDK.
:::

```ts
async function customClient<Req, Res>(requestOptions: ClientRequest<Req>): Promise<ClientResponse<Res>> {
  ....
}

const config = new AptosConfig({ client: { provider: customClient } });
const aptos = new Aptos(config);
```

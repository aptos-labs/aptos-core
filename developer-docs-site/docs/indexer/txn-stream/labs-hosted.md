---
title: "Labs-Hosted Transaction Stream Service"
---

# Labs-Hosted Transaction Stream Service

If you are running your own instance of the [Indexer API](/indexer/api), or a [custom processor](/indexer/custom-processors), you must have access to an instance of the Transaction Stream Service. This page contains information about how to use the Labs-Hosted Transaction Stream Service.

## Endpoints
All endpoints are in GCP us-central1 unless otherwise specified.

- **Mainnet:** grpc.mainnet.aptoslabs.com:443
- **Testnet:** grpc.testnet.aptoslabs.com:443
- **Devnet:** grpc.devnet.aptoslabs.com:443

<!--
## Rate limits
The following rate limit applies for the Aptos Labs hosted Transaction Stream Service:

- todo todo

If you need a higher rate limit, consider running the Transaction Stream Service yourself. See the guide to self hosting [here](./self-hosted).
-->

## Auth tokens

In order to use the Labs-Hosted Transaction Stream Service you must have an auth token. To get an auth token, do the following:
1. Go to https://aptos-api-gateway-prod.firebaseapp.com.
1. Sign in and select "API Tokens" in the left sidebar.
1. Create a new token. You will see the token value in the first table.

You can provide the auth key by setting the `Authorization` HTTP header ([MDN](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Authorization)). For example, with curl:
```
curl -H 'Authorization: Bearer aptoslabs_yj4donpaKy_Q6RBP4cdBmjA8T51hto1GcVX5ZS9S65dx'
```

For more comprehensive information about how to use the Transaction Stream Service, see the docs for the downstream systems:
- [Indexer API](/indexer/api/self-hosted)
- [Custom Processors](/indexer/custom-processors)

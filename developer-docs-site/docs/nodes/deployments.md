---
title: "Deployments"
slug: "deployments"
hide_table_of_contents: true
---

# Aptos Deployments

|Description                                 |Mainnet | Testnet | Devnet |
|--------------------------------------------|---|---|---|
|**REST API**             | https://fullnode.mainnet.aptoslabs.com/v1 | https://fullnode.testnet.aptoslabs.com/v1 | https://fullnode.devnet.aptoslabs.com/v1 |
|**REST API Spec**        | <a href="https://fullnode.mainnet.aptoslabs.com/v1/spec#/">Link</a> | <a href="https://fullnode.testnet.aptoslabs.com/v1/spec#/">Link</a> | <a href="https://fullnode.devnet.aptoslabs.com/v1/spec#/">Link</a> |
|**Indexer API**          | https://indexer.mainnet.aptoslabs.com/v1/graphql | https://indexer-testnet.staging.gcp.aptosdev.com/v1/graphql | https://indexer-devnet.staging.gcp.aptosdev.com/v1/graphql |
|**Indexer API Spec**     | <a href="https://cloud.hasura.io/public/graphiql?endpoint=https://indexer.mainnet.aptoslabs.com/v1/graphql">Link</a> | <a href="https://cloud.hasura.io/public/graphiql?endpoint=https://indexer-testnet.staging.gcp.aptosdev.com/v1/graphql">Link</a> | <a href="https://cloud.hasura.io/public/graphiql?endpoint=https://indexer-devnet.staging.gcp.aptosdev.com/v1/graphql">Link</a> |
|**Faucet**               | No Faucet | https://faucet.testnet.aptoslabs.com/ | https://faucet.devnet.aptoslabs.com/ |
|**Genesis and Waypoint** | https://github.com/aptos-labs/aptos-networks/tree/main/mainnet | https://github.com/aptos-labs/aptos-networks/tree/main/testnet| https://github.com/aptos-labs/aptos-networks/tree/main/devnet |
|**Chain ID**             | 1 | 2 | [On Aptos Explorer **select Devnet from top right**](https://explorer.aptoslabs.com/?network=Devnet).|
|**Epoch duration**       | 7200 seconds |7200 seconds | 7200 seconds |
|**Network providers**    | Fully decentralized. | Managed by Aptos Labs on behalf of Aptos Foundation. | Managed by Aptos Labs on behalf of Aptos Foundation. |
|**Release cadence**      | Monthly | Monthly | Weekly |
|**Wipe cadence**         | Never. | Never. | On update. |
|**Purpose**              | The main Aptos network. | Long-lived test network. | Bleeding edge and exploratory. |
|**Network status**       | Always live. | Always live. | Almost always live, with brief interruptions during updates. |

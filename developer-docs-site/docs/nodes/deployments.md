---
title: "Deployments"
slug: "deployments"
hide_table_of_contents: true
---

# Aptos Deployments

|Description                                 |Mainnet | Devnet | Testnet |
|--------------------------------------------|---|---|---|
|**REST API**             | https://fullnode.mainnet.aptoslabs.com/v1 | https://fullnode.devnet.aptoslabs.com/v1 | https://fullnode.testnet.aptoslabs.com/v1 |
|**REST API Spec**        | <a href="https://fullnode.mainnet.aptoslabs.com/v1/spec#/">Link</a> | <a href="https://fullnode.devnet.aptoslabs.com/v1/spec#/">Link</a> | <a href="https://fullnode.testnet.aptoslabs.com/v1/spec#/">Link</a> | 
|**Indexer API**          | https://indexer.mainnet.aptoslabs.com/v1/graphql | https://indexer-devnet.staging.gcp.aptosdev.com/v1/graphql | https://indexer-testnet.staging.gcp.aptosdev.com/v1/graphql |
|**Indexer API Spec**     | <a href="https://cloud.hasura.io/public/graphiql?endpoint=https://indexer.mainnet.aptoslabs.com/v1/graphql">Link</a> | <a href="https://cloud.hasura.io/public/graphiql?endpoint=https://indexer-devnet.staging.gcp.aptosdev.com/v1/graphql">Link</a> | <a href="https://cloud.hasura.io/public/graphiql?endpoint=https://indexer-testnet.staging.gcp.aptosdev.com/v1/graphql">Link</a> | 
|**Faucet**               | No Faucet | https://faucet.devnet.aptoslabs.com/ | (API): https://faucet.testnet.aptoslabs.com <br/> (dApp): https://aptoslabs.com/testnet-faucet |
|**Genesis and Waypoint** | https://github.com/aptos-labs/aptos-networks/tree/main/mainnet | https://github.com/aptos-labs/aptos-networks/tree/main/devnet | https://github.com/aptos-labs/aptos-networks/tree/main/testnet |
|**Chain ID**             | 1 | [On Aptos Explorer **select Devnet from top right**](https://explorer.aptoslabs.com/?network=Devnet). | 2 |
|**Epoch duration**       | 7200 seconds |7200 seconds | 7200 seconds |
|**Network providers**    | Fully decentralized. | Managed by Aptos Labs on behalf of Aptos Foundation. | Managed by Aptos Labs on behalf of Aptos Foundation. |
|**Release cadence**      | Monthly | Weekly | Monthly |
|**Wipe cadence**         | Never. | On update.| Never. |
|**Purpose**              | The main Aptos network. | Bleeding edge and exploratory. | Long-lived test network. |
|**Network status**       | Always live. | Almost always live, with brief interruptions during updates. | Always live. |

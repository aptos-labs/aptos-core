---
title: "Aptos Blockchain Deployments"
slug: "aptos-deployments"
hide_table_of_contents: true
---

# Aptos Blockchain Deployments

You can connect to the Aptos blockchain in a few ways. See the below table:

:::tip What not to do

Make sure to see the row describing [**What not to do**](#what-not-to-do).

:::

|Description | Mainnet | Devnet | Long-lived Testnet | Local Testnet | Aptos Incentivized Testnet (AIT)|
|---|---|---|---|---|---|
|**Network runs where**| Validators run on Aptos Labs servers. Fullnodes are run by both Aptos Labs and you (i.e., the Aptos community). |Validators run on Aptos Labs servers. Fullnodes are run by both Aptos Labs and you (i.e., the Aptos community).|Validators run on Aptos Labs servers. Fullnodes are run by both Aptos Labs and you (i.e., the Aptos community). | Validators and fullnodes run locally on your computer, entirely independent from devnet. | Some Validators run on Aptos servers, others are run by the Aptos community. Fullnodes are run by Aptos Labs and the community.|
|**Who is responsible for the network**| Managed by Aptos Team. |Managed by Aptos Team. | Managed by Aptos Team. |Deployed, managed and upgraded by you.| Managed by Aptos Labs and the community.|
|**Update release cadence EDIT**| Monthly |Weekly |Every 2 weeks |Controlled by you.| Managed by Aptos Labs and the community.|
|**How often wiped EDIT**| Not wiped |Weekly | Not wiped |Controlled by you.| Wiped permanently after AIT progra concludes.|
|**REST API URL EDIT**| m |Managed by Aptos Team. | lltn |Deployed, managed and upgraded by you.| Managed by Aptos Labs and the community.|
|**Genesis blob EDIT**| m |Managed by Aptos Team. | lltn |Deployed, managed and upgraded by you.| Managed by Aptos Labs and the community.|
|**Waypoint EDIT**| m |Managed by Aptos Team. | lltn |Deployed, managed and upgraded by you.| Managed by Aptos Labs and the community.|
|**Faucet EDIT**| m |Managed by Aptos Team. | lltn |Deployed, managed and upgraded by you.| Managed by Aptos Labs and the community.|
|**Purpose of the network**| m |The devnet is built to experiment with new ideas, improve performance and enhance the user experience.|lltn | The local testnet is for your development purposes and runs on your local computer.| For executing the Aptos Incentivized Testnet programs for the community.|
|**Network status**| m |Mostly live, with brief interruptions during regular updates. |lltn | On your local computer. | **Live only during Incentivized Testnet drives**. |
|**Type of nodes** |m |Validators and fullnodes. |lltn | Validators and fullnodes. | Validators and fullnodes.|
|**How to run a node**| m |N/A, run by Aptos Labs team. |lltn | See the [tutorial](local-testnet/using-cli-to-run-a-local-testnet.md). | See the node deployment guides (ADD-LINK).|
|**How to connect to the network**|m |<ul><li> You can transact directly with devnet without a local testnet. See, for example, [Your first transaction](../tutorials/first-transaction.md).</li><li> You can run fullnodes locally on your computer and synchronize with devnet. See [Run a Fullnode](/nodes/full-node/public-fullnode).</li></ul>|lltn | You can start a Validator and fullnode locally and connect with your local testnet for development purposes. | You must start both a local AIT validator node locally to connect to the AIT. Optionally, fullnodes can also be run locally and connected to AIT.|
|**TypeScript / JavaScript SDK to use EDIT**|m |The latest version of the [aptos](https://www.npmjs.com/package/aptos) package. The package on npmjs is tested and released with devnet. | lltn |Use the TS / JS SDK built from the same commit as the local testnet. See [this document](../guides/local-testnet-dev-flow) for an end-to-end guide explaining this flow. | N/A, do not develop against AIT. |
|<span id="what-not-to-do">**What not to do**</span>| m |Do not attempt to sync your local AIT fullnode or AIT validator node with devnet. |lltn | Do not attempt to sync your local testnet validator node with AIT. | Make sure you deploy your local AIT fullnode, AIT validator node and AIT validator fullnode in the test mode, and follow the instructions in the node deployment guides (ADD LINK).|


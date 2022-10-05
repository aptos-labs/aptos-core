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

|Description | Mainnet | Devnet | Long-lived Testnet | Aptos Incentivized Testnet (AIT)|
|---|---|---|---|---|
|**REST API URL EDIT**| m |https://fullnode.devnet.aptoslabs.com/v1 | https://fullnode.testnet.aptoslabs.com/v1 | Available during AIT program.|
|**Genesis blob EDIT**| m |https://devnet.aptoslabs.com/genesis.blob | https://testnet.aptoslabs.com/genesis.blob| Available during AIT program. |
|**Waypoint EDIT**| m |https://devnet.aptoslabs.com/waypoint.txt |https://github.com/aptos-labs/aptos-genesis-waypoint/tree/main/testnet| Available during AIT program.|
|**Faucet EDIT**| m |https://faucet.devnet.aptoslabs.com/ | https://faucet.testnet.aptoslabs.com/ |Available during AIT program.|
|**How to connect to the network**|You must run both a validator node and a validator fullnode to connect to the mainnet. See [Validators](/nodes/validator-node/validators) and [Public Fullnode](/nodes/full-node/public-fullnode) sections.  |You can transact directly with devnet without a local testnet. See, for example, [Your first transaction](../tutorials/first-transaction.md). You can run fullnodes locally on your computer and synchronize with devnet. See [Run a Fullnode](/nodes/full-node/public-fullnode).|See [Validators](/nodes/validator-node/validators) and [Public Fullnode](/nodes/full-node/public-fullnode) sections. | You must start both a local AIT validator node locally to connect to the AIT. Optionally, fullnodes can also be run locally and connected to AIT.|
|***Description*** | ***Mainnet*** | ***Devnet*** | ***Long-lived Testnet*** | ***AIT***|
|**Network runs where**| Validators, validator fullnodes and public fullnodes are run by you (i.e., the Aptos community) and Aptos Labs. |Validators run on Aptos Labs servers. Fullnodes are run by both Aptos Labs and you (i.e., the Aptos community).|Validators run on Aptos Labs servers. Fullnodes are run by both Aptos Labs and you (i.e., the Aptos community). | Some Validators run on Aptos servers, others are run by the Aptos community. Fullnodes are run by Aptos Labs and the community.|
|**Who is responsible for the network**| Managed by Aptos Team. |Managed by Aptos Team. | Managed by Aptos Team. | Managed by Aptos Labs and the community.|
|**Update release cadence**| Every month. |Every week. |Every 2 weeks. | Managed by Aptos Labs and the community.|
|**How often wiped**| Not wiped. |Every week.| Not wiped. | Wiped permanently after AIT program concludes.|
|***Description*** | ***Mainnet*** | ***Devnet*** | ***Long-lived Testnet*** |  ***AIT***|
|**Purpose of the network**| The main Aptos network. |The devnet is built to experiment with new ideas, improve performance and enhance the user experience.| | For executing the Aptos Incentivized Testnet programs for the community.|
|**Network status**| Always live. |Mostly live, with brief interruptions during regular updates. |Mostly live, with brief interruptions during regular updates. | Live only during Incentivized Testnet drives. |
|**Type of nodes** |Validators and validator fullnodes. |Validators and public fullnodes. | Validators and public fullnodes. | Validators and validator fullnodes.|
|**How to run a node**| See [Validators](/nodes/validator-node/validators) and [Public Fullnode](/nodes/full-node/public-fullnode) sections.  |N/A, run by Aptos Labs team. |See [Validators](/nodes/validator-node/validators) and [Public Fullnode](/nodes/full-node/public-fullnode) sections. | See the node deployment guides published during AIT program.|
|<span id="what-not-to-do">**What not to do**</span>||Do not attempt to sync your local AIT fullnode or AIT validator node with devnet. | Make sure you deploy your local AIT fullnode, AIT validator node and AIT validator fullnode in the test mode, and follow the instructions in the node deployment guides published during AIT program.|


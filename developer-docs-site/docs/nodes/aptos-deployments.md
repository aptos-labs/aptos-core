---
title: "Aptos Blockchain Deployments"
slug: "aptos-deployments"
hide_table_of_contents: true
---

# Aptos Blockchain Deployments

Ensure your nodes are running the [latest releases](../releases/index.md) of Aptos.

You can connect to the Aptos blockchain by [choosing a network](../guides/system-integrators-guide.md#choose-a-network). See [Check out release branch](../guides/getting-started.md#check-out-release-branch) for the commands to download the versions of Aptos tied to those networks.

See the table below for details on each Aptos network:

|Description | Mainnet | Devnet | Long-lived Testnet | Aptos Incentivized Testnet (AIT)|
|---|---|---|---|---|
|<div style={{width: 120}}>**Chain ID**</div>| 1 |[On Aptos Explorer **select Devnet from top right**](https://explorer.aptoslabs.com/?network=Devnet).| 2|  Available during AIT program.|
|**REST API URL**| <div style={{width: 220}}>https://fullnode.mainnet.aptoslabs.com/v1</div> |<div style={{width: 220}}>https://fullnode.devnet.aptoslabs.com/v1</div> | <div style={{width: 220}}>https://fullnode.testnet.aptoslabs.com/v1</div> | <div style={{width: 110}}>Available during AIT program. </div>|
|**Genesis blob and Waypoint**| In the `mainnet` directory on https://github.com/aptos-labs/aptos-networks |In the `devnet` directory on https://github.com/aptos-labs/aptos-networks  | <div style={{width: 200}}>In the `testnet` directory on https://github.com/aptos-labs/aptos-networks </div>| Available during AIT program.  |
|**Faucet**| No Faucet |<div style={{width: 200}}>https://faucet.devnet.aptoslabs.com/</div> | <div style={{width: 200}}>(dApp): https://aptoslabs.com/testnet-faucet </div>|Available during AIT program.|
|**Epoch**| 7200 seconds (two hours, set by governance) |--- | 7200 seconds (two hours) |Available during AIT program.|
|**Network runs where**| Validators, validator fullnodes and public fullnodes are run by you (i.e., the Aptos community) and Aptos Labs. |<div style={{width: 200}}>Validators run on Aptos Labs servers. Fullnodes are run by both Aptos Labs and you (i.e., the Aptos community).</div>|<div style={{width: 200}}>Validators run on Aptos Labs servers. Fullnodes are run by both Aptos Labs and you (i.e., the Aptos community).</div> | Some Validators run on Aptos servers, others are run by the Aptos community. Fullnodes are run by Aptos Labs and the community.|
|**Who is responsible for the network**| Fully decentralized. |Managed by Aptos Team. | Managed by Aptos Team. | Managed by Aptos Labs and the community.|
|**Update release cadence**| Every month. |Every week. |Every 2 weeks. | Managed by Aptos Labs and the community.|
|**How often wiped**| Not wiped. |Every week.| Not wiped. | Wiped permanently after AIT program concludes.|
|***Description*** | ***Mainnet*** | ***Devnet*** | ***Long-lived Testnet*** |  ***AIT***|
|**Purpose of the network**| The main Aptos network. |The devnet is built to experiment with new ideas, improve performance and enhance the user experience.| | For executing the Aptos Incentivized Testnet programs for the community.|
|**Network status**| Always live. |Mostly live, with brief interruptions during regular updates. |Mostly live, with brief interruptions during regular updates. | Live only during Incentivized Testnet drives. |
|**Type of nodes** |Validators and validator fullnodes. |Validators and public fullnodes. | Validators and public fullnodes. | Validators and validator fullnodes.|
|**How to run a node**| See [Validators](./validator-node/index.md) and [Public Fullnode](./full-node/index.md) sections.  |N/A, run by Aptos Labs team. |See [Validators](./validator-node/index.md) and [Public Fullnode](./full-node/index.md) sections. | See the node deployment guides published during the AIT program.|

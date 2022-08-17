---
title: "Whats New in AIT-3"
slug: "whats-new-in-ait3"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Whats New in AIT-3

:::caution DRAFT-only
These AIT-3 docs are draft-only for now.
:::

Several new features are up for testing by the AIT-3 participants. See below:

### Aptos Wallet

- The new Aptos Wallet, available as a Chrome webapp extension. You will use this Wallet to participate in staking and governance in AIT-3. 

### Owner, operator and voter personas

- Participation in staking and governance is now enabled with three personas: an owner, an operator and a voter.  
- The owner is the owner of the funds, typically an investor. For example, the owner Bob can assign his operator address to the account of Alice, a trusted validator operator. See [How a custodian can stake on Aptos](/nodes/staking#how-a-custodian-can-stake-on-aptos) for more details on owner and operator definitions.  
- Voter is a new persona being introduced in AIT-3. An owner can designate a voter. This enables the voter to participate in governance.

### Staking

- A new staking UI, making it easier to manage staking.
<!--- TODO: Is rotating the keys supported in AIT-3? --->
- Rotating the keys. 
- Effects of changing the stake to weigh more on the proposer. **Hypothesis**: This better reflects the higher compute cost of the proposer.


### On-chain governance

Community to vote on proposals. Proposals will be in the following areas:

- Staking.
<!--- TODO: What exactly is gas schedule? --->
- Gas schedule.
- Proposals for onchain upgrades of the AptosFramework modules, including:
  - Deploy AptosFramework modules.
  - Upgrade AptosFramework modules.
  - Proposals on breaking changes.
- Off-chain upgrades, such as:
  - Changes to consensus.
  - Upgrade the Move VM version.
  - See the version of the software.
- Nodes
  - Nodes dynamically joining and leaving when the network is under load. Require the node to leave the network for at least X minutes duration.
  - Send all types of transactions to the Aptos blockchain to test for a consistent load on the network and monitor the cost.
  - Operator to rollback the software from version B to version A, for testing.
  - Operator to update node configuration, for testing.
  - Operator to restore the node from the backup data.

## Personas, accounts and keys

To participate in testing the staking and the governance features in the AIT-3, you will create three personas. See below an explanation of these personas: 

- **Owner**: The owner account contains the validator settings and the coins. The coins are airdropped into the owner account.
- **Operator**: If you are the owner, then, using your owner key, you will select the specific operator and you will:
  - Manage the settings for the specific validator, and
  - Delegate the stake pool to the validator.
  - The operator public key is same key you (owner) used while registering your validator node with the Aptos Community Platform. This is the  `account_public_key` from the "aptosbot.yaml" file for the validator.
- **Voter**: You will use the voter key to sign the governance votes in the transactions.


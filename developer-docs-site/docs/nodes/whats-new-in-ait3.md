---
title: "Whats New in AIT-3"
slug: "whats-new-in-ait3"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Whats New in AIT-3

Several new features are up for testing by the AIT-3 participants. See below:

### Petra (Aptos Wallet)

- Petra, the new Aptos Wallet, is now available as a Chrome webapp extension. You will use this Wallet to participate in staking and governance in AIT-3.  See [installation instructions for Petra here](/guides/install-petra-wallet-extension).

### Owner, operator and voter personas

Participation in staking and governance is now enabled with three personas: an owner, an operator and a voter.  

- **Owner**: The owner is the owner of the funds. For example, the owner Bob can assign his operator address to the account of Alice, a trusted validator operator. See also [How a custodian can stake on Aptos](/concepts/staking#how-a-custodian-can-stake-on-aptos). The owner account contains the validator settings and the coins. The coins are airdropped into the owner account.
- **Operator**: If you are the owner, then, using your owner key, you will select the specific operator and you will:
  - Manage the settings for the specific validator, and
  - Delegate the stake pool to the validator.
  - The operator public key is same key you (owner) used while registering your validator node with the Aptos Community Platform. This is the  `account_public_key` from the "aptosbot.yaml" file for the validator.
- **Voter**: Voter is a new persona being introduced in AIT-3. An owner can designate a voter. This enables the voter to participate in governance. You will use the voter key to sign the governance votes in the transactions.


### Staking

- A new staking UI, making it easier to manage staking.

### On-chain governance

Community to vote on proposals. Proposals will be in the following areas:

- Blockchain on-chain configuration, for example, Epoch length.
- Staking configuration, for example, reward rate.
- Governance configuration, for example, voting power requirement.
- Move module upgrades.




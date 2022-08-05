---
title: "AIT-3"
slug: "ait-3"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# AIT-3

:::caution DRAFT-only
These AIT-3 docs are draft-only for now.
:::

<p class="card-section-h2">Welcome to AIT-3</p>

The Aptos Incentivized Testnet-3 (AIT-3) is a rewards program for any Aptos community member. All you need is an interest in becoming an Aptos validator node operator, and be willing and capable to test the new features.

## Key AIT-3 dates

:::tip **Key AIT-3 dates:**
_All dates and times shown are for Pacific Time, year 2022._

- **August 19:** Registration starts. Node and identity verification begins.
- ~~**July 7:** Registration ends. Only 48 hours left to complete identity verification.~~
- ~~**July 11:** Selection process concludes. Email notifications are sent. If your node is selected, you will have 24 hours to join the AIT-3 Validator set.~~
- **August 19:** AIT-3 goes live at noon. Validator score tracking begins.
- **September 9:** AIT-3 concludes.

:::

<div class="docs-card-container">
<div class="row row-cols-1 row-cols-md-3 g-4">
  <div class="col">
    <div class="card h-100">
    <h3 class="card-header">1. See</h3>
      <div class="card-body d-flex flex-column">
        <a href="#whats-new-in-ait-3" class="card-title card-link"> <h2>What's New in AIT-3</h2></a>
        <p class="card-text">See the new features that are up for testing by the AIT-3 participants. </p>
      </div>
    </div>
  </div>
  <div class="col" >
    <div class="card h-100">
     <h3 class="card-header">2. Read</h3>
      <div class="card-body d-flex flex-column">
      <a href="#ait-3-program" class="card-title card-link stretched-link"> <h2>Steps in AIT-3</h2></a>
        <p class="card-text">Read the summary flowchart and the detailed steps of the AIT-3.</p>     
      </div>
    </div>
  </div>
  <div class="col" >
    <div class="card h-100">
     <h3 class="card-header">3. Participate</h3>
      <div class="card-body d-flex flex-column">
      <a href="#install-the-nodes-for-ait-3" class="card-title card-link stretched-link"> <h2>Install the nodes</h2></a>
        <p class="card-text">Ready to dive in? Follow these guides to install your validator node.</p>     
      </div>
    </div>
  </div>
  
</div>
</div>

## What's new in AIT-3

Several new features are up for testing by the AIT-3 participants. See below:

### Aptos Wallet

- The new Aptos Wallet, available as a Chrome webapp extension.

### Staking

- Separate accounts for the fund owner and the node operator.
- Rotating the keys.
- Effects of changing the stake to weigh more on the proposer. **Hypothesis**: This better reflects the higher compute cost of the proposer.

### On-chain governance

Community to vote on proposals. The following proposals are being considered:

#### Proposal to change the staking parameters

The following staking parameters are being considered:

- Minimum and maximum stake.
- Minimum and maximum lockup.
- Rewards rate.
- Limits on adding and withdrawing the stake.
- Epoch duration.

#### Gas schedule

- Proposal on the gas schedule.

#### AptosFramework modules

Proposals on AptosFramework modules, such as:

- Deploy AptosFramework modules.
- Upgrade AptosFramework modules.
- Proposals on breaking changes.

### Off-chain upgrades

- Changes to consensus.
- Upgrade the Move VM version.
- See the version of the software.

### Nodes

- Nodes dynamically joining and leaving when the network is under load. Require the node to leave the network for at least X duration of time (in minutes).
- Send all types of transactions to the Aptos blockchain to test for a consistent load on the network and monitor the cost.

### Disaster recovery

- Conduct disaster recovery exercise in the simulation for:
  - DDOS mitigation.
  - Data corruption and data loss.
  - Operators to restore the node from the backup data.
- Other operational exercises
  - Operator to rollback from version B to version A.
  - Operator to update node configuration.
- Manual writeset transaction.

## AIT-3 program

Participants in the AIT-3 program must demonstrate the ability to configure and deploy a node, and pass the minimum performance requirements as reported by the [Node Health Checker](/nodes/node-health-checker). See below a flowchart showing the high-level steps you will execute while participating.

### Accounts and keys

Note the following definitions:

- **Owner account**: The Owner account contains the validator settings and the coins. The coins are airdropped into the Owner account.
- **Operator key**: If you are the Owner, then, using your Owner key, you will select the specific Operator key to:
  - Manage the settings for the specific validator, and
  - Delegate the stake pool to the validator.
  - The Operator key is same key you (Owner) used while registering your validator node with the Aptos Community Platform. This is the private key from the `validator-identity.yaml` file.
- You (participant) will use the Voter key to sign the governance votes in the transactions.

:::tip Consensus key
The Consensus key is generated by the Operator and is not associated with an on-chain account, i.e., there is no Aptos blockchain account address associated with Consensus key.
:::

### Summary steps

#### Install the validator node and register

<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/install-node-and-register.svg'),
    dark: useBaseUrl('/img/docs/install-node-and-register.svg'),
  }}
/>

### Detailed steps

To participate in the AIT-3 program, follow the below steps. Use these steps as a checklist to keep track of your progress. Click on the links in each step for a detailed documentation.

<div class="docs-card-container">
<div class="step">
    <div>
        <div class="circle">1</div>
    </div>
    <div>
        <div class="step-title">Read the Node Requirements</div>
        <div class="step-caption">Before you proceed, make sure that your hardware, storage and network resources satisfy the <a href="./node-requirements"><strong> Node Requirements</strong></a>.  
        </div>
    </div>
</div>
<div class="step">
    <div>
        <div class="circle">2</div>
    </div>
    <div>
        <div class="step-title">Follow the instructions and deploy a validator node in the test mode.</div>
        <div class="step-caption">Follow the detailed node installation steps provided in: <a href="#install-the-nodes-for-ait-3">Install the nodes for AIT-3</a>. <strong>Make sure to set your node in the Test mode.</strong> Instructions are provided in the node installation sections. Test mode is required for Aptos Labs to do a health check on your node.  </div>
    </div>
</div>
<div class="step">
    <div>
        <div class="circle">3</div>
    </div>
    <div>
        <div class="step-title">Register your node in the Aptos Community Platform</div>
        <div class="step-caption">Navigate to the <a href="https://aptoslabs.com/community">Aptos Community page</a> and register your node. Provide your account address, your owner public key, and your validator's network addresses. </div>
    </div>
</div>
<div class="step">
    <div>
        <div class="circle">4</div>
    </div>
    <div>
        <div class="step-title">If your node passes healthcheck, you will be prompted to complete the KYC process</div>
        <div class="step-caption">The Aptos team will perform a node health check on your validator, using the <a href="https://aptos.dev/nodes/node-health-checker">Node Health Checker</a>. When Aptos confirms that your node is healthy, you will be asked to complete the KYC process. </div>
        <div class="step-caption">You will also be automatically enrolled in the Aptos notifications. This will enable you to receive all communication from Aptos Labs throughout the AIT-3 program. </div>
    </div>
</div>
<div class="step">
    <div>
    <div class="step-active circle">5</div>
    </div>
    <div>
    <div class="step-title">If you are selected, then proceed to Install the Wallet step</div>
    </div>
    </div>
    </div>
<div>

</div>
<br />

#### Install the Wallet

<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/wallet-actions.svg'),
    dark: useBaseUrl('/img/docs/wallet-actions.svg'),
  }}
/>

#### Participate in staking and governance

<ThemedImage
alt="Signed Transaction Flow"
sources={{
    light: useBaseUrl('/img/docs/owner-staking-op-voter-actions.svg'),
    dark: useBaseUrl('/img/docs/owner-staking-op-voter-actions.svg'),
  }}
/>


## Install the nodes for AIT-3

<div class="docs-card-container">
<div class="row row-cols-1 row-cols-md-1 g-4">
  <div class="col">
    <div class="card h-100">
    <h3 class="card-header">Install node for AIT-3</h3>
      <div class="card-body d-flex flex-column">
        <p class="card-text"><a href="node-requirements" class="card-link"><strong>Make sure to first check the Node Requirements.</strong></a></p>
        <p class="card-text">Pick your preferred installation method from below:</p>
        <ul class="list-group list-group-flush">
          <li class="list-group-item"><a href="/nodes/validator-node/run-validator-node-using-source/" class="card-link">Using Aptos source</a></li>
          <li class="list-group-item"><a href="/nodes/validator-node/run-validator-node-using-aws" class="card-link">Using AWS</a></li>
          <li class="list-group-item"><a href="/nodes/validator-node/run-validator-node-using-gcp" class="card-link">Using GCP</a></li>
          <li class="list-group-item"><a href="/nodes/validator-node/run-validator-node-using-docker" class="card-link">Using Docker</a></li>
          <li class="list-group-item"><a href="/nodes/validator-node/run-validator-node-using-azure" class="card-link">Using Azure</a></li>
        </ul>
      </div>
    </div>
  </div>
</div>
</div>

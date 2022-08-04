---
title: "AIT-3"
slug: "ait-3"
---

# AIT-3

<p class="card-section-h2">Welcome to AIT-3</p>

The Aptos Incentivized Testnet-3 (AIT-3) is a rewards program for any Aptos community member. All you need is an interest in becoming an Aptos validator node operator, and be willing and capable to test the new features.

## Key AIT-3 dates

:::tip **Key AIT-3 dates:** 
*All dates and times shown are for Pacific Time, year 2022.*
- **August 19:** Registration starts. Node and identity verification begins.
- ~~**July 7:** Registration ends. Only 48 hours left to complete identity verification.~~
- ~~**July 11:** Selection process concludes. Email notifications are sent. If your node is selected, you will have 24 hours to join the AIT-3 Validator set.~~
- **August 19:** AIT-3 goes live at noon. Validator score tracking begins.
- **September 9:** AIT-3 concludes.
:::

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
  - Joining and withdrawal limit.
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

- Conduct disaster recovery exercise in simulation
    - DDOS mitigation.
    - Data corruption, data loss. 
    - Operators to restore the node from the backup data.
- Other operational exercises
    - Operator to rollback from version B to version A.
    - Operator to update node configuration.
- Manual writeset transaction.




To participate in the Aptos Incentivized Testnet-3 (AIT-3) program, follow the below steps. Use these steps as a checklist to keep track of your progress. A detailed documentation for each step is provided.

<div class="docs-card-container">
<div class="row row-cols-1 row-cols-md-2 g-4">
  <div class="col">
    <div class="card h-100">
    <h3 class="card-header">Step 1</h3>
      <div class="card-body d-flex flex-column">
        <a href="#ait-3-program" class="card-title card-link"> <h2>See the AIT-3 program</h2></a>
        <p class="card-text">Read the below AIT-3 steps thoroughly.</p>
      </div>
    </div>
  </div>
  <div class="col" >
    <div class="card h-100">
     <h3 class="card-header">Step 2</h3>
      <div class="card-body d-flex flex-column">
      <a href="#install-the-nodes-for-ait-3" class="card-title card-link stretched-link"> <h2>Install the nodes</h2></a>
        <p class="card-text">Ready to dive in? Follow these guides to install the nodes.</p>     
      </div>
    </div>
  </div>
  
</div>
</div>

## AIT-3 program

Participants in the AIT-3 program must demonstrate the ability to configure and deploy a node, and pass the sanctions screening requirements.

<div class="docs-card-container">

<div class="step">
    <div>
        <div class="circle">1</div>
    </div>
    <div>
        <div class="step-title">Read the Node Requirements</div>
        <div class="step-caption">Before you proceed, make sure that you satisfy the <a href="./node-requirements"><strong> Node Requirements</strong></a>.  </div>
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
        <div class="circle">4</div>
    </div>
    <div>
        <div class="step-title">Provide your node details to the Aptos Discord community</div>
        <div class="step-caption">Navigate to the <a href="https://community.aptoslabs.com/">Aptos Community page</a> and enter your Validator's information. Provide your account address, your public keys, and your Validator's network addresses. Optionally, you can also provide your FullNode details. </div>
    </div>
</div>
<div class="step">
    <div>
        <div class="circle">5</div>
    </div>
    <div>
        <div class="step-title">If your node passes healthcheck, you will be prompted to complete the KYC process</div>
        <div class="step-caption">When Aptos confirms that your node is healthy, you will be asked to complete the KYC process. </div>
    </div>
</div>
<div class="step">
    <div>
        <div class="circle">6</div>
    </div>
    <div>
        <div class="step-title">On July 11th, Aptos will inform you whether your node is selected</div>
        <div class="step-caption">On July 11th, you will receive an email notification. If your node is selected, you will have 24 hours to join the AIT-3 Validator set. Follow the  instructions in <a href="https://aptos.dev/tutorials/validator-node/connect-to-testnet/">Connecting to Aptos Incentivized Testnet</a> to join the AIT-3 and the AIT-3 Validator set. </div>
    </div>
</div>
<div class="step">
    <div>
        <div class="circle">7</div>
    </div>
    <div>
        <div class="step-title">If selected, you must meet the minimum requirements for the continuous node performance</div>
        <div class="step-caption">Minimum requirements for the node performance are detailed in <a href="https://aptos.dev/reference/node-liveness-criteria">Node Liveness Criteria</a>. </div>
    </div>
</div>
<div class="step">
    <div>
        <div class="circle">8</div>
    </div>
    <div>
        <div class="step-title">At the conclusion of AIT-3, follow the procedure to leave the Validator set and shutdown your node</div>
        <div class="step-caption">Steps to properly leave the validator set and shutdown your node are detailed HERE. </div>
    </div>
</div>
<div class="step">
    <div>
    <div class="step-active circle">9</div>
    </div>
    <div>
    <div class="step-title">Done</div>
    </div>
    </div>
    </div>
<div>
<br />

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
</div>

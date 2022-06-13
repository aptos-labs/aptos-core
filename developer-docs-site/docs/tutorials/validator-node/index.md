---
title: "Testing"
slug: "index"
disable_pagination: true
hide_right_sidebar: true
hide_table_of_contents: false
hide_title: false
thinner_content: true
no_pad_top: false
---


This tutorial describes how to run Aptos nodes for the Aptos Incentivized Testnet 1 (AIT1) program. It explains the following:

- How to configure a validator node to run in test mode. This will be used during the AIT1 registration stage to validate your eligibility, and 
- How to connect to the incentivized testnet if you are selected to run a validator node.

:::info

For the AIT1, we  recommend that every node operator run both a validator node and a FullNode. Hence, the reference implementation described in these sections will install both the nodes by default. 

:::

<p class="card-section-h2">Testing CardsWrapper</p>

import Card from 'react-bootstrap/Card';
import Button from 'react-bootstrap/Button';
import Row from 'react-bootstrap/Row';
import Col from 'react-bootstrap/Col';

<div class="card border-dark">
  <div class="card-body">
    <h4 class="card-title">Bologna</h4>
    <h6 class="card-subtitle mb-2 text-muted">Emilia-Romagna Region, Italy</h6>
    <p class="card-text">It is the seventh most populous city in Italy, at the heart of a metropolitan area of about one million people. </p>
    <a href="#" class="card-link">Read More</a>
    <a href="#" class="card-link">Book a Trip</a>
  </div>
</div>
<div class="card card-body" style={{ width: '14rem' }}>
    <h4 class="card-title">Card Example</h4>
    <p class="card-text">Lorem ipsum dolor sit amet, consectetur adipiscing elit. Integer posuere erat a ante..</p>
    <a href="#" class="btn btn-primary">More</a>
</div>
<div class="card" style={{width: '18rem' }}>
<h3 class="card-header">Featured</h3>
  <div class="card-body">
    <h3 class="card-title">Card title</h3>
    <h4 class="card-subtitle mb-2 text-muted">Card subtitle</h4>
    <p class="card-text">Some quick example text to build on the card title and make up the bulk of the card's content.</p>
    <a href="#" class="card-link">Card link</a>
    <a href="#" class="card-link">Another link</a>
  </div>
</div>
<div class="bd-example">
<div class="row">
  <div class="col-sm-6">
    <div class="card">
      <div class="card-body">
        <h5 class="card-title">Special title treatment</h5>
        <p class="card-text">With supporting text below as a natural lead-in to additional content.</p>
        <a href="#" class="btn btn-primary">Go somewhere</a>
      </div>
    </div>
  </div>
  <div class="col-sm-6">
    <div class="card">
      <div class="card-body">
        <h5 class="card-title">Special title treatment</h5>
        <p class="card-text">With supporting text below as a natural lead-in to additional content.</p>
        <a href="#" class="btn btn-primary">Go somewhere</a>
      </div>
    </div>
  </div>
  <div class="col-sm-6">
    <div class="card">
      <div class="card-body">
        <h5 class="card-title">Special title treatment</h5>
        <p class="card-text">With supporting text below as a natural lead-in to additional content.</p>
        <a href="#" class="btn btn-primary">Go somewhere</a>
      </div>
    </div>
  </div>
</div>
</div>


<p class="card-section-h2">Deploying for Aptos Incentivized Testnet</p>


<div class="row row-cols-1 row-cols-md-3 g-4">
  <div class="col">
    <div class="card h-100">
      <div class="card-body d-flex flex-column">
        <h3 class="card-title">On GCP</h3>
        <p class="card-text">Set up your GCP account first and then follow these instructions.</p>
        <a href="https://aptos.dev/tutorials/validator-node/run-validator-node-using-gcp" class="btn btn-primary mt-auto">Run Aptos node on GCP</a>
      </div>
    </div>
  </div>
  <div class="col">
    <div class="card h-100">     
      <div class="card-body d-flex flex-column">
        <h3 class="card-title">On AWS</h3>
        <p class="card-text">Set up your AWS account first and then follow these instructions.</p>
        <a href="https://aptos.dev/tutorials/validator-node/run-validator-node-using-aws" class="btn btn-primary mt-auto">Run Aptos node on AWS</a>
      </div>
    </div>
  </div>
  <div class="col">
    <div class="card h-100">   
      <div class="card-body d-flex flex-column">
        <h3 class="card-title"> With Aptos source</h3>
        <p class="card-text">Build a Rust binary on your local machine.</p>
        <a href="https://aptos.dev/tutorials/validator-node/run-validator-node-using-source/" class="btn btn-primary mt-auto">Run Aptos node with source</a>
      </div>
    </div>
  </div>
</div>
<br />
<br />

In order to participate in the incentivized testnet, participants must demonstrate the ability to configure and deploy a node, as well as pass the sanctions screening requirements.

Follow the below steps to participate in the Aptos Incentivized Testnet:
- Follow the instructions to deploy both a validator node and a FullNode in the test mode.
- Navigate to the [Incentivized Testnet registration page](https://community.aptoslabs.com/) and enter information about your node (pub-keys, IP/DNS address).
- If you are selected to run a node, follow instructions in [Connecting to Aptos Incentivized Testnet](connect-to-testnet) to join incentivized testnet.
- Keep the node in healthy state for the entire testing period and follow operational requests as needed. See [Node Liveness Criteria](../../reference/node-liveness-criteria.md) document.

**Before you proceed**

If you are new to Aptos Blockchain, read the following sections before proceeding:

* [Validator node concepts](/basics/basics-validator-nodes).
* [FullNode concepts](/basics/basics-fullnodes).
* [Node networks and synchronization](/basics/basics-node-networks-sync).

:::note IMPORTANT

We strongly recommend that you run the validator node and the FullNode on two separate and independent machines. Make sure that these machines are well-provisioned and isolated from each other. Guaranteeing the resource isolation between the validator node and the FullNode will help ensure smooth deployment of these nodes.

:::

**Hardware requirements**

We recommend the following hardware resources:

- For running an aptos node on incentivized testnet we recommend the following:

  - **CPU**: 4 cores (Intel Xeon Skylake or newer).
  - **Memory**: 8GiB RAM.

**Storage requirements**

The amount of data stored by Aptos depends on the ledger history (length) of the blockchain and the number
of on-chain states (e.g., accounts). These values depend on several factors, including: the age of the blockchain,
the average transaction rate and the configuration of the ledger pruner.

We recommend nodes have at least 300GB of disk space to ensure adequate storage space for load testing. You have the option to start with a smaller size and adjust based upon demands. You will be responsible for monitoring your node's disk usage and adjusting appropriately to ensure node uptime.

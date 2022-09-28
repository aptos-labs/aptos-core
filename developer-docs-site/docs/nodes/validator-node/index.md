---
title: "Validators"
slug: "validators"
---

import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';


# Validators

To participate in the consensus process in the Aptos mainnet, you must deploy and run a validator node and a validator fullnode. Optionally you can also run a public fullnode. This document presents a high-level conceptual overview of the important steps involved in deploying the nodes for validation. 

<div class="docs-card-container">
<div class="row row-cols-1 row-cols-md-5 g-4">
<div class="col">
    <div class="card h-100" >
    <div class="card-body d-flex flex-column" >
    <p class="card-title card-link stretched-link"> <h2>1</h2></p>
    <p class="card-text"><h4>Read the node requirements.</h4></p>
    <p class="card-text">Select a deployment method. Use on-premises or cloud services.</p>
</div>
</div>
</div>
  <div class="col">
    <div class="card h-100" >
    <div class="card-body d-flex flex-column" >
    <p class="card-title"> <h2>2</h2></p>
    <p class="card-text"><h4>Generate identity for nodes.</h4></p>
    <p class="card-text">Account address and private and public keys come to exist.</p>
</div>
</div>
</div>
  <div class="col">
  <div class="card h-100" >
    <div class="card-body d-flex flex-column"  >
    <p class="card-title"> <h2>3</h2></p>
    <p class="card-text"><h4>Configure validator and validator fullnode.</h4></p>
    <p class="card-text">Establishes network identity for the nodes. Ready to handshake with other nodes.</p>
</div>
</div>
</div>
<div class="col">
  <div class="card h-100" >
    <div class="card-body d-flex flex-column"  >
    <p class="card-title"> <h2>4</h2></p>
    <p class="card-text"><h4>Insert genesis and waypoint to start the nodes.</h4></p>
    <p class="card-text">Bootstrapped the nodes. Aptos network becomes aware of the nodes.</p>
</div>
</div>
</div>
<div class="col">
  <div class="card h-100" >
    <div class="card-body d-flex flex-column"  >
    <p class="card-title"> <h2>5</h2></p>
    <p class="card-text"><h4>Join the validator set.</h4></p>
    <p class="card-text">Initialize staking pool, bootstrap in production mode, start syncing. Begin validating and earn rewards.</p>
</div>
</div>
</div>
</div>
</div>

- Start by reading the node requirements to get to know the compute, memory and storage resources you need. Note also the internet bandwidth requirements. 
- Select a method to deploy your nodes, i.e., use a cloud service or Docker or source code. 
- Generate identity for the nodes. This is the first step in progressively making your nodes secure and ready to be integrated into the Aptos network. 
- Using YAML files, configure your nodes with user and network identity. This step enables the nodes to be recognized by other nodes in the Aptos network. Handshaking is possible after this step.  
- With the node identity established for the Aptos network, next you install the necessary binaries and locally generate the genesis blob and waypoint files. These will allow the node to be connected to the Aptos network. 
- Bootstrap the nodes. The nodes now have the Aptos node binary running on them with the identity set. This fulfills the requirement for the Aptos network to become aware of your nodes. However, your nodes cannot connect to the Aptos network yet because these nodes are not yet in the validator set. On the Aptos network a validator can only accept another validator for connections. Until your nodes are in the validator set, they will be rejected by other validator nodes on the network. 
- Perform the required actions before joining the validator set. For this, you must perform a few tasks such as initializing a staking pool, delegating to operators and voters, downloading the latest versions of the genesis blob and waypoint text files and restarting your nodes. 
- Join the validator set. Other nodes will see your nodes and will establish connection to your nodes. Now you can stay in sync with the Aptos blockchain by building up your database of the history of the ledger. It takes some time for your nodes to build the database. Whenever your nodes reach the latest version of the blockchain, your validator node will be able to start participating in the consensus process.


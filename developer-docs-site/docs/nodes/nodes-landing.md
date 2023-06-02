---
title: "Learn about Nodes"
slug: "nodes-landing"
hide_table_of_contents: true
---

# Learn about Nodes

The Aptos network is comprised of nodes of three types: validator node, validator fullnode and public fullnode. To participate in consensus, you are required to run both a validator node and a validator fullnode, and stake.

Also learn how to run a public fullnode on a local network and connect to either a testnet or a devnet. This section describes everything you need to stake and participate in consensus and governance. See also the [external resources](../community/external-resources.md) offered by your fellow node operators.



## Validator operations

<div class="docs-card-container">
  <div class="row row-cols-1 row-cols-md-2a g-4">
    <div class="col">
      <div class="card-no-border card-body h-100 d-flex flex-column align-items-start">
        <div class="card-body">
          <h2 class="card-title">Validation on Aptos</h2>
          <p class="card-text">
            Everything you need to know about how validation, staking and governance works on Aptos.
          </p>
        </div>
        <div class="list-group list-group-flush">
          <a href="../concepts/staking#validation-on-the-aptos-blockchain" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">How validation works</h4>
            </div>
            <small>Validator-leader proposes and earns rewards on success.</small>
          </a>
          <a href="../concepts/staking#validator-state-and-stake-state" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">Validator states</h4>
            </div>
            <small>Learn how a validator gets into a validator set.</small>
          </a>
          <div class="card-body">
          <h2 class="card-title">Staking</h2>
          </div>
          <a href="../concepts/staking" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">Staking on Aptos</h4>
            </div>
            <small>A comprehensive guide to how staking works on Aptos.</small>
          </a>
          <a href="../concepts/governance" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">Governance</h4>
            </div>
            <small>Who can propose, who can vote, and how an AIP is resolved.</small>
          </a>
          <a href="./validator-node/operator/staking-pool-operations#perform-pool-owner-operations" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">Owner</h4>
            </div>
            <small>Describes the owner operations performed for staking.</small>
          </a>
          <a href="./validator-node/voter/index" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">Voter</h4>
            </div>
            <small>Describes the voter operations performed for staking.</small>
          </a>
        </div>
      </div>
    </div>
    <div class="col">
      <div class="card-no-border card-body h-100 d-flex flex-column">
        <div class="card-body">
          <h2 class="card-title">Operator</h2>
          <p class="card-text">
            A comprehensive guide to deploying nodes, staking operations and participate in consensus.
          </p>
        </div>
        <div class="list-group list-group-flush">
          <a href="./validator-node/operator/node-requirements" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">Node requirements</h4>
            </div>
            <small>Details the compute and storage resources you need. Read this first before anything.</small>
          </a>
          <a href="./validator-node/operator/running-validator-node/running-validator-node" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">Running validator node</h4>
            </div>
            <small>In the cloud or on-premises, Docker or source, you will step-by-step instructions here.</small>
          </a>
          <a href="./validator-node/operator/node-liveness-criteria" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">Node liveness criteria</h4>
            </div>
            <small>Your nodes must pass these liveness criteria to be in an Aptos network.</small>
          </a>
          <a href="./validator-node/operator/connect-to-aptos-network" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">Connecting to Aptos network</h4>
            </div>
            <small>Steps to connect your nodes to an Aptos network. </small>
          </a>
          <a href="./validator-node/operator/staking-pool-operations" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">Staking pool operations</h4>
            </div>
            <small>Step-by-step guide for how to perform staking pool operations. </small>
          </a>
          <a href="./validator-node/operator/shutting-down-nodes" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">Shutting down nodes</h4>
            </div>
            <small>Leave the validator set first, and then shut down your node. </small>
          </a>
        </div>
      </div>
    </div>
    <div class="col">
      <div class="card-no-border card-body h-100 d-flex flex-column">
        <div class="card-body">
          <h2 class="card-title">Fullnode</h2>
          <p class="card-text">
            A section with detailed, step-by-step instructions on everything related to Aptos fullnode. 
          </p>
        </div>
        <div class="list-group list-group-flush">
          <a href="./full-node/public-fullnode" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">Public fullnode</h4>
            </div>
            <small>Follow this section to install a public fullnode.</small>
          </a>
          <a href="./indexer-fullnode" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">Indexer fullnode</h4>
            </div>
            <small
              >Describes how to run an indexer fullnode on the Aptos network. </small>
          </a>
          <a href="./local-testnet/local-testnet-index" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">Local testnet</h4>
            </div>
            <small>Run a local testnet with a validator node.</small>
          </a>
          <a href="./full-node/fullnode-network-connections" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">Fullnode network connections</h4>
            </div>
            <small>Describes in detail how to configure your node's network connections.</small>
          </a>
          <a href="./full-node/network-identity-fullnode" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">Network identity for fullnode</h4>
            </div>
            <small
              >Create a static network identity for your fullnode.</small
            >
          </a>
          <a href="./full-node/update-fullnode-with-new-devnet-releases" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">Update fullnode</h4>
            </div>
            <small>When devnet is wiped and updated with newer versions, follow this document to update your fullnode.</small>
          </a>
          <a href="./full-node/bootstrap-fullnode" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">Bootstrap a new fullnode</h4>
            </div>
            <small>Use data restore to bootstrap a new fullnode.</small>
          </a>
        </div>
      </div>
    </div>
  </div>
</div>

## General

<div class="docs-card-container">
  <div class="row row-cols-1 row-cols-md-3a g-4">
    <div class="col">
      <div class="card-no-border card-body h-100 d-flex flex-column">
        <div class="card-body">
        </div>
        <div class="list-group list-group-flush">
          <a href="./deployments" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">Aptos blockchain deployments</h4>
            </div>
            <small>See a snapshot of all Aptos deployments.</small>
          </a>
          <a href="./identity-and-configuration" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">Identity and configuration</h4>
            </div>
            <small>A mental-model of identity and configuration plus a description of the identity YAMLs.</small>
          </a>
          <a href="./node-files-all-networks/node-files" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">Node files</h4>
            </div>
            <small>All the files you need while deploying nodes, whether on mainnet, testnet or devnet.</small>
          </a>
          <a href="../integration/indexing" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">Indexing</h4>
            </div>
            <small>Access Aptos-provided indexer service or build your own custom indexer for the Aptos blockchain.</small>
          </a>
          <a href="../guides/state-sync" class="list-group-item">
            <div class="d-flex w-100 justify-content-between">
              <h4 class="mb-1">State synchronization</h4>
            </div>
            <small>Synchronize your nodes to the latest Aptos blockchain state.</small>
          </a>
        </div>
      </div>
    </div>
    <div class="col">
      <div class="card-no-border card-body h-100 d-flex flex-column">
        <div class="card-body">
        </div>
        <div class="list-group list-group-flush">
        <a href="../guides/data-pruning" class="list-group-item">
            <div class="d-flex w-100 justify-content-between align-items-start">
              <h4 class="mb-1">Data pruning</h4>
            </div>
            <small>Manage your validator node's disk space by controlling the pruning settings. Proceed with caution.</small>
          </a>
          <a href="./measure/node-health-checker" class="list-group-item">
            <div class="d-flex w-100 justify-content-between align-items-start">
              <h4 class="mb-1">Node health checker</h4>
            </div>
            <small>If you are a node operator, use the NHC service to check if your node is running correctly.</small>
          </a>
          <a href="/reference/telemetry/" class="list-group-item">
            <div class="d-flex w-100 justify-content-between align-items-start">
              <h4 class="mb-1">Telemetry</h4>
            </div>
            <small>Know what telemetry metrics are sent by your node, and control the telemetry settings.</small>
          </a>
          <a href="./leaderboard-metrics" class="list-group-item">
            <div class="d-flex w-100 justify-content-between align-items-start">
              <h4 class="mb-1">Leaderboard metrics</h4>
            </div>
            <small>A guide to interpret the validator rewards performance, as shown on the leaderboard metrics site.</small>
          </a>
        </div>
      </div>
    </div>
  </div>
</div>

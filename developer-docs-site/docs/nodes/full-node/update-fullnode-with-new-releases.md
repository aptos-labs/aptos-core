---
title: "Update Fullnode With New Releases"
slug: "update-fullnode-with-new-releases"
sidebar_position: 11
---

# Update Fullnode With New Releases

This document outlines the process for updating your fullnode. For fullnodes running in `devnet`, an additional data wipe step is required as `devnet` is wiped on every release.

## If you built the fullnode from aptos-core source code

1. Stop your fullnode by running the below command:
  ```bash
  cargo stop aptos-node
  ```

2. For users of the Rust binary, pull the latest release appropriate for your network (`devnet`, `testnet`, or `mainnet`):
  ```bash
  git checkout [network_branch] && git pull
  ```
Replace `[network_branch]` with `devnet`, `testnet`, or `mainnet` as applicable, and rebuild the binary.

3. If your fullnode is running in `devnet`, follow the additional steps in the [Additional data wipe steps for `devnet`](#additional-data-wipe-steps-for-devnet) section below.

4. Restart your fullnode by running the same start (`run`) command as before:
  ```bash
  cargo run -p aptos-node --release -- -f ./fullnode.yaml
  ```

5. See the [Verify initial synchronization](./fullnode-source-code-or-docker.md#verify-initial-synchronization) section for checking if the fullnode is syncing again.

### Additional data wipe steps for `devnet`
For devnet, follow these additional steps after stopping your fullnode:

1. Delete the data folder (the directory path is what you specified in the configuration file, e.g., `fullnode.yaml`).

    - The default data folder is `/opt/aptos/data`.

2. Delete the `genesis.blob` file and `waypoint.txt` file (depending on how you configured it, you might not have this file and may instead have a `waypoint` directly in your configuration file).

3. Download the new [genesis.blob](../node-files-all-networks/node-files.md#genesisblob) file and the new [waypoint](../node-files-all-networks/node-files.md#waypointtxt).

4. Update the configuration file (e.g., `fullnode.yaml`) with the new waypoint (if you configure the waypoint directly there).


## If you run a fullnode via Docker

1. Stop your fullnode by running the below command:
    ```bash
    docker compose down --volumes
    ```
2. If your fullnode is running in `devnet`, delete the entire directory which holds your fullnode config and data directory.
3. Re-install and configure those files as during setup.
4. Restart your fullnode:
  ```bash
  docker compose up -d
  ```

## If you run a fullnode on GCP

### Upgrade with data wipe (devnet only)
Upgrading your node in devnet requires a data wipe, as the network is reset on each deployment. Other networks (e.g., testnet and mainnet) don't require this step and we recommend not wiping your data in these networks.

1. You can increase the `era` number in `main.tf` to trigger a new data volume creation, which will start the node on a new DB.

2. Update `image_tag` in `main.tf`.

3. Update Terraform module for fullnode, run this in the same directory of your `main.tf` file:

  ```bash
  terraform get -update
  ```

4. Apply Terraform changes:

  ```bash
  terraform apply
  ```

### Upgrade without data wipe

1. Update `image_tag` in `main.tf`.

2. Update Terraform module for fullnode, run this in the same directory of your `main.tf` file:

  ```bash
  terraform get -update
  ```

3. Apply Terraform changes:

  ```bash
  terraform apply
  # if you didn't update the image tag, terraform will show nothing to change, in this case, force helm update
  terraform apply -var force_helm_update=true
  ```

[rest_spec]: https://github.com/aptos-labs/aptos-core/tree/main/api
[devnet_genesis]: https://devnet.aptoslabs.com/genesis.blob
[devnet_waypoint]: https://devnet.aptoslabs.com/waypoint.txt
[aptos-labs/aptos-core]: https://github.com/aptos-labs/aptos-core.git
[status dashboard]: https://status.devnet.aptos.dev

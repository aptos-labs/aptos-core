---
title: "Update Fullnode With New Devnet Releases"
slug: "update-fullnode-with-new-devnet-releases"
sidebar_position: 11
---

# Update Fullnode With New Releases

When `devnet` is wiped and updated with newer versions, you will need to update your fullnode as well. If you do not, your fullnode will not continue to synchronize with the network. To update your fullnode, follow these steps:

## If you built the fullnode from aptos-core source code

1. Shutdown your fullnode.

2. Delete the data folder (the directory path is what you specified in the configuration file, e.g., `fullnode.yaml`).

    - The default data folder is `/opt/aptos/data`.

3. Delete the `genesis.blob` file and `waypoint.txt` file (depending on how you configured it, you might not have this file and may instead have a `waypoint` directly in your configuration file).

4. If you use the Rust binary, pull the latest of `devnet` via `git checkout devnet && git pull`, and build the binary again.

5. Download the new [genesis.blob](../node-files-all-networks/node-files.md#genesisblob) file and the new [waypoint](../node-files-all-networks/node-files.md#waypointtxt).

6. Update the configuration file (e.g., `fullnode.yaml`) with the new waypoint (if you configure the waypoint directly there).

7. Restart the fullnode.

8. See the [Verify initial synchronization](./fullnode-source-code-or-docker.md#verify-initial-synchronization) section for checking if the fullnode is syncing again.

## If you run a fullnode via Docker

1. Shutdown your fullnode
2. Delete the entire directory which holds your fullnode config and data directory.
3. Rerun the instructions on [Approach #2: Using Docker](./fullnode-source-code-or-docker.md#Approach-#2:-Using-Docker)

## If you run a fullnode on GCP

Aptos devnet releases can be of two types: 
- One with a data wipe to start over the Aptos blockchain
- Second type is only a software update without a data wipe

### Upgrade with data wipe

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

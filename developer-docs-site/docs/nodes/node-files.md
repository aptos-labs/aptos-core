---
title: "Node Files"
slug: "node-files"
---

# Node Files

When you are deploying an Aptos node, you will need the following files. These can be downloaded from separate `aptos-labs` repos on GitHub. The `wget` commands provided below will work on macOS and Linux. Open a terminal and paste the `wget` command to download the file. 

:::tip Unless specified, all these files are required for validator node.
:::

## docker-compose.yaml

- **Git repo:** `aptos-core`
- **Git branch:** `mainnet` on https://github.com/aptos-labs/aptos-core
- **Command to download:**
    ```bash
    wget -O docker-compose.yaml https://raw.githubusercontent.com/aptos-labs/aptos-core/mainnet/docker/compose/aptos-node/docker-compose.yaml
    ```

## validator.yaml

- **Git repo:** `aptos-core`
- **Git branch:** `mainnet` on https://github.com/aptos-labs/aptos-core
- **Command to download:**
  ```bash
  wget -O validator.yaml https://raw.githubusercontent.com/aptos-labs/aptos-core/mainnet/docker/compose/aptos-node/validator.yaml
  ```

## genesis.blob 

- **Git repo:** `aptos-networks`
- **Git branch:** `main` on https://github.com/aptos-labs/aptos-networks
- **Command to download:**
  ```bash
  wget -O genesis.blob https://raw.githubusercontent.com/aptos-labs/aptos-networks/main/premainnet/genesis.blob
  ```

## waypoint.txt

- **Git repo:** `aptos-networks`
- **Git branch:** `main` on https://github.com/aptos-labs/aptos-networks
- **Command to download:**
  ```bash
  wget -O waypoint.txt https://raw.githubusercontent.com/aptos-labs/aptos-networks/main/premainnet/waypoint.txt
  ```

## blocked.ips 

- **Git repo:** `aptos-core`
- **Git branch:** `mainnet` on https://github.com/aptos-labs/aptos-core
- **Command to download:**
  ```bash
  wget -O blocked.ips https://raw.githubusercontent.com/aptos-labs/aptos-core/mainnet/docker/compose/aptos-node/blocked.ips
  ```

## docker-compose-fullnode.yaml (fullnode only)

:::tip Fullnode 
Fullnode means either a validator fullnode or a public fullnode.
:::

- **Git repo:** `aptos-core`
- **Git branch:** `mainnet` on https://github.com/aptos-labs/aptos-core
- **Command to download:**
  ```bash
  wget -O docker-compose.yaml https://raw.githubusercontent.com/aptos-labs/aptos-core/mainnet/docker/compose/aptos-node/docker-compose-fullnode.yaml
  ```

## fullnode.yaml (fullnode only)

:::tip Fullnode 
Fullnode means either a validator fullnode or a public fullnode.
:::

- **Git repo:** `aptos-core`
- **Git branch:** `mainnet` on https://github.com/aptos-labs/aptos-core
- **Command to download:**
  ```bash
  wget -O fullnode.yaml https://raw.githubusercontent.com/aptos-labs/aptos-core/mainnet/docker/compose/aptos-node/fullnode.yaml
  ```

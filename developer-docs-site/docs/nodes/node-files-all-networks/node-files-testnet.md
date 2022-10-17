---
title: "Node Files For Testnet"
slug: "node-files-testnet"
---

# Node Files For Testnet

When you are deploying an Aptos node in the **testnet**, you will need to download the files listed on this page. 

- **Mainnet:** If you are deploying in the mainnet, download the files from the [Node Files For Mainnet](node-files) page.
- **Devnet:** If you are deploying in the testnet, download the files from the [Node Files For Devnet](node-files-devnet) page.

---

These files can be downloaded from separate `aptos-labs` repos on GitHub. The `wget` commands provided below will work on macOS and Linux. Open a terminal and paste the `wget` command to download the file. 

:::tip Files for the validator node
Unless specified, all these files are required for validator node. A file with `fullnode` in its filename is required for either a validator fullnode or a public fullnode.
:::

## docker-compose.yaml

- **Git repo:** `aptos-core`
- **Git branch:** `testnet` on https://github.com/aptos-labs/aptos-core
- **Command to download:**
    ```bash
    wget -O docker-compose.yaml https://raw.githubusercontent.com/aptos-labs/aptos-core/testnet/docker/compose/aptos-node/docker-compose.yaml
    ```

## validator.yaml

- **Git repo:** `aptos-core`
- **Git branch:** `testnet` on https://github.com/aptos-labs/aptos-core
- **Command to download:**
  ```bash
  wget -O validator.yaml https://raw.githubusercontent.com/aptos-labs/aptos-core/testnet/docker/compose/aptos-node/validator.yaml
  ```

## genesis.blob 

- **Git repo:** `aptos-networks`
- **Git branch:** `main` on https://github.com/aptos-labs/aptos-networks
- **Command to download:**
  ```bash
  wget -O genesis.blob https://raw.githubusercontent.com/aptos-labs/aptos-networks/main/testnet/genesis.blob
  ```

## waypoint.txt

- **Git repo:** `aptos-networks`
- **Git branch:** `main` on https://github.com/aptos-labs/aptos-networks
- **Command to download:**
  ```bash
  wget -O waypoint.txt https://raw.githubusercontent.com/aptos-labs/aptos-networks/main/testnet/waypoint.txt
  ```

## docker-compose-src.yaml

- **Git repo:** `aptos-core`
- **Git branch:** `testnet` on https://github.com/aptos-labs/aptos-core
- **Command to download:**
  ```bash
  wget -O docker-compose-src.yaml https://raw.githubusercontent.com/aptos-labs/aptos-core/testnet/docker/compose/aptos-node/docker-compose-src.yaml
  ```

## haproxy.cfg

- **Git repo:** `aptos-core`
- **Git branch:** `testnet` on https://github.com/aptos-labs/aptos-core
- **Command to download:**
  ```bash
  wget -O haproxy.cfg https://raw.githubusercontent.com/aptos-labs/aptos-core/testnet/docker/compose/aptos-node/haproxy.cfg
  ```

## blocked.ips 

- **Git repo:** `aptos-core`
- **Git branch:** `testnet` on https://github.com/aptos-labs/aptos-core
- **Command to download:**
  ```bash
  wget -O blocked.ips https://raw.githubusercontent.com/aptos-labs/aptos-core/testnet/docker/compose/aptos-node/blocked.ips
  ```

## docker-compose-fullnode.yaml (fullnode only)

:::tip Fullnode 
Fullnode means either a validator fullnode or a public fullnode.
:::

- **Git repo:** `aptos-core`
- **Git branch:** `testnet` on https://github.com/aptos-labs/aptos-core
- **Command to download:**
  ```bash
  wget -O docker-compose.yaml https://raw.githubusercontent.com/aptos-labs/aptos-core/testnet/docker/compose/aptos-node/docker-compose-fullnode.yaml
  ```

## fullnode.yaml (fullnode only)

:::tip Fullnode 
Fullnode means either a validator fullnode or a public fullnode.
:::

- **Git repo:** `aptos-core`
- **Git branch:** `testnet` on https://github.com/aptos-labs/aptos-core
- **Command to download:**
  ```bash
  wget -O fullnode.yaml https://raw.githubusercontent.com/aptos-labs/aptos-core/testnet/docker/compose/aptos-node/fullnode.yaml
  ```

## haproxy-fullnode.cfg (fullnode only)

- **Git repo:** `aptos-core`
- **Git branch:** `testnet` on https://github.com/aptos-labs/aptos-core
- **Command to download:**
  ```bash
  wget -O haproxy-fullnode.cfg https://raw.githubusercontent.com/aptos-labs/aptos-core/testnet/docker/compose/aptos-node/haproxy-fullnode.cfg
  ```

---
title: "Additional documentation for Incentivized Testnet"
slug: "additional-doc"
sidebar_position: 15
---

## Shutdown nodes for Incentivized Testnet

Follow these instructions when you need to take down the validator node and cleanup the resources used by the node.

Before you shutdown the node, you should make sure to leave validator set first (will take effect in next epoch).

```bash
aptos node leave-validator-set --profile testnet-operator --pool-address <owner-address>
```

### Using source code

- Stop your node.
- Remove the data directory: `rm -r <your-data-directory>`.
- Remove the genesis blob file and waypoint.
- Depends on if you want to reuse your node identity, you can choose to keep or delete the `private-keys.yaml`, `validator-identity.yaml`, `validator-full-node-identity.yaml` files.

### Using Docker

- Stop your node and remove the data volumes, `docker compose down --volumes`.
- Remove the genesis blob file and waypoint.
- Depends on if you want to reuse your node identity, you can choose to keep or delete the `private-keys.yaml`, `validator-identity.yaml`, `validator-full-node-identity.yaml` files.

### Using Terraform

- Stop your node and delete all the resources: `terraform destroy`.

## Add monitoring components

:::tip Supported only using Terraform
This is currently only supported using Terraform.
:::

1. Set the `enable_monitoring` variable in your terraform module. For example:

    ```rust
    module "aptos-node" {
      ...
      enable_monitoring           = true
      utility_instance_num        = 3  # this will add one more utility instance to run monitoring component
    }
    ```

2. Apply the changes: `terraform apply`.

3. You will see a new pod getting created. Run `kubectl get pods` to check.

4. Access the dashboard.

    First, find the IP/DNS for the monitoring load balancer.

    ```bash
    kubectl get svc ${WORKSPACE}-mon-aptos-monitoring --output jsonpath='{.status.loadBalancer.ingress[0]}'
    ```

    You can access the dashboard on `http://<ip/DNS>`.

## Staking with CLI

:::tip Stake with UI
You can also use UI to perform a few staking operations. See the [**Initialize staking pool** section](/nodes/ait/steps-in-ait3#initialize-staking-pool). Proceed below to use the CLI to perform staking operations. 
:::

- Initialize CLI with your wallet private key or create new wallet

  ```bash
  aptos init --profile testnet-owner \
    --rest-url http://testnet.aptoslabs.com
  ```

  You can either enter the private key from an existing wallet, or create new wallet address depends on your need.

- Initialize staking pool using CLI

  ```bash
  aptos stake initialize-stake-owner \
    --initial-stake-amount 100000000000000 \
    --operator-address <operator-address> \
    --voter-address <voter-address> \
    --profile testnet-owner
  ```

- Transfer coin between accounts

  ```bash
  aptos account transfer \
    --account <operator-address> \
    --amount <amount> \
    --profile testnet-owner
  ```

- Switch operator

  ```bash
  aptos stake set-operator \
    --operator-address <new-operator-address> \ 
    --profile testnet-owner
  ```

- Switch voter

  ```bash
  aptos stake set-delegated-voter \
    --voter-address <new-voter-address> \ 
    --profile testnet-owner
  ```

- Add stake

  ```bash
  aptos stake add-stake \
    --amount <amount> \
    --profile testnet-owner \
    --max-gas 10000
  ```

  :::tip Max gas
    You can adjust the above `max-gas` number. Ensure that you sent your operator enough tokens to pay for the gas fee.
    :::

- Increase stake lockup

  ```bash
  aptos stake increase-lockup --profile testnet-owner
  ```

- Unlock stake

  ```bash
  aptos stake unlock-stake \
    --amount <amount> \
    --profile testnet-owner
  ```

- Withdraw stake

  ```bash
  aptos stake withdraw-stake \
    --amount <amount> \
    --profile testnet-owner
  ```

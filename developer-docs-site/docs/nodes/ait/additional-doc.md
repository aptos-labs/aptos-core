---
title: "Additional documentation for Incentivized Testnet"
slug: "additional-doc"
sidebar_position: 15
---

## Shutdown Nodes for Incentivized Testnet

Follow this instruction when you need to take down the validator node and cleanup the resources used by the node.


Before you shutdown the node, you should make sure to leave validator set first (will take effect in next epoch)

    ```
    aptos node leave-validator-set --profile ait3-operator
    ```

### Using source code

- Stop your node.
- Remove the data directory: `rm -rf <your-data-directory>`
- Remove the genesis blob file and waypoint
- Depends on if you want to reuse your node identity, you can choose to keep or delete the `private-keys.yaml`, `validator-identity.yaml`, `validator-full-node-identity.yaml` files.

### Using Docker

- Stop your node and remove the data volumes, `docker compose down --volumes`
- Remove the genesis blob file and waypoint
- Depends on if you want to reuse your node identity, you can choose to keep or delete the `private-keys.yaml`, `validator-identity.yaml`, `validator-full-node-identity.yaml` files.

### Using Terraform

- Stop your node and delete all the resources: `terraform destroy`


## Add Monitoring Components

Note: This is currently only supported using Terraform.

1. Set the `enable_monitoring` variable in your terraform module. For example:

    ```
    module "aptos-node" {
      ...
      enable_monitoring           = true
      utility_instance_num        = 3  # this will add one more utility instance to run monitoring component
    }
    ```

2. Apply the changes: `terraform apply`

3. You should see a new pod getting created. Run `kubectl get pods` to check.

4. Access the dashboard

    First, find the IP/DNS for the monitoring load balancer.

    ```
    kubectl get svc ${WORKSPACE}-mon-aptos-monitoring --output jsonpath='{.status.loadBalancer.ingress[0]}'
    ```

    You can access the dashboard on `http://<ip/DNS>`

## Staking with CLI

We now have a UI to support some staking operation, but in any case if you need to do operations not supported in UI, you can use CLI for it.

- Initialize CLI with your wallet private key or create new wallet

  ```
  aptos init --profile ait3-owner \
    --rest-url http://ait3.aptosdev.com
  ```

  You can either enter the private key from an existing wallet, or create new wallet address depends on your need.

- Initialize staking pool using CLI

  ```
  aptos stake initialize-stake-owner \
    --initial-stake-amount 100000000000000 \
    --operator-address <operator-address> \
    --voter-address <voter-address> \
    --profile ait3-owner
  ```

- Transfer coin between accounts

  ```
  aptos account transfer \
    --account <operator-address> \
    --amount <amount> \
    --profile ait3-owner
  ```

- Switch operator

  ```
  aptos stake set-operator \
    --operator-address <new-operator-address> \ 
    --profile ait3-owner
  ```

- Switch voter

  ```
  aptos stake set-delegated-voter \
    --voter-address <new-voter-address> \ 
    --profile ait3-owner
  ```

- Add stake

  ```
  aptos stake add-stake \
    --amount <amount> \
    --profile ait3-owner
  ```

- Increase stake lockup

  ```
  aptos stake increase-lockup --profile ait3-owner
  ```

- Unlock stake

  ```
  aptos stake unlock-stake \
    --amount <amount> \
    --profile ait3-owner
  ```

- Withdraw stake

  ```
  aptos stake withdraw-stake \
    --amount <amount> \
    --profile ait3-owner
  ```

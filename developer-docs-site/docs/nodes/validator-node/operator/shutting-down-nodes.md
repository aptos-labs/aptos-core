---
title: "Shutting Down Nodes"
slug: "shutting-down-nodes"
---

# Shutting Down Nodes

Follow these instructions to shut down the validator node and cleanup the resources used by the node.

:::tip Leave validator set first
Before you shutdown the node, make sure to first leave validator set first. This will be become effective in the next epoch.

```bash
aptos node leave-validator-set --profile testnet-operator --pool-address <owner-address>
```
:::

## Using source code

1. Stop your node.
2. Remove the data directory: `rm -r <your-data-directory>`.
3. Remove the genesis blob file and waypoint file.
4. If you want to reuse your node identity, you can choose to keep these configuration files: 
   - `private-keys.yaml`
   - `validator-identity.yaml`
   - `validator-full-node-identity.yaml` 
  
  or else you can delete these files.

## Using Docker

1. Stop your node and remove the data volumes: `docker compose down --volumes`.
2. Remove the genesis blob file and waypoint file.
3. If you want to reuse your node identity, you can choose to keep these configuration files: 
   - `private-keys.yaml`
   - `validator-identity.yaml`
   - `validator-full-node-identity.yaml` 
  
  or else you can delete these files.

## Using Terraform

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


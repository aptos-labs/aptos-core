---
title: "Additional documentation for Incentivized Testnet"
slug: "additional-doc"
sidebar_position: 15
---

## Shutdown Nodes for Incentivized Testnet

Follow this instruction when you need to take down the validator node and cleanup the resources used by the node.

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

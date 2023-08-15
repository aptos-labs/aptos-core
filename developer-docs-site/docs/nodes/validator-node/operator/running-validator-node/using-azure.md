---
title: "On Azure"
slug: "run-validator-node-using-azure"
---

# On Azure

This is a step-by-step guide to install an Aptos node on Microsoft Azure. Follow these steps to configure a validator node and a validator fullnode on separate machines. 

:::caution Did you set up your Azure account?
This guide assumes that you already have Azure account setup.
:::

:::danger Do you have stale volumes after bumping your deployment's era?
`era` is a concept relevant only to Kubernetes deployments of an Aptos node. Changing the `era` provides an easy way to wipe your deployment's state. However, this may lead to dangling persistent volumes on validator fullnodes. Confirm the existence of these volumes with `kubectl get pvc` and delete them manually to minimize costs.
:::

## Before you proceed

Make sure you complete these prerequisite steps before you proceed:

- **Azure account**: https://azure.microsoft.com/
- **Aptos CLI**: https://aptos.dev/tools/aptos-cli/install-cli/index
- **Terraform 1.3.6**: https://www.terraform.io/downloads.html
- **Kubernetes CLI**: https://kubernetes.io/docs/tasks/tools/
- **Azure CLI**: https://docs.microsoft.com/en-us/cli/azure/install-azure-cli

## Install

:::tip One validator node + one validator fullnode
Follow the below instructions **twice**, i.e., first on one machine to run a validator node and the second time on another machine to run a validator fullnode. 
:::

1. Create a working directory for your configuration.

    * Choose a workspace name, for example, `mainnet` for mainnet, or `testnet` for testnet, and so on. **Note**: This defines the Terraform workspace name, which, in turn, is used to form the resource names.

      ```bash
      export WORKSPACE=mainnet
      ```

    * Create a directory for the workspace.
      
      ```bash
      mkdir -p ~/$WORKSPACE
      ```
    
    * Choose a username for your node, for example `alice`.

      ```bash
      export USERNAME=alice
      ```

2. Create a blob storage container for storing the Terraform state on Azure, you can do this on Azure UI or by the command: 

    ```bash
    az group create -l <azure region> -n aptos-$WORKSPACE
    az storage account create -n <storage account name> -g aptos-$WORKSPACE -l <azure region> --sku Standard_LRS
    az storage container create -n <container name> --account-name <storage account name> --resource-group aptos-$WORKSPACE
    ```

3. Create Terraform file called `main.tf` in your working directory:
  ```bash
  cd ~/$WORKSPACE
  vi main.tf
  ```

4. Modify `main.tf` file to configure Terraform, and create fullnode from Terraform module. Example content for `main.tf`:

  ```
  terraform {
    required_version = "~> 1.3.6"
    backend "azurerm" {
      resource_group_name  = <resource group name>
      storage_account_name = <storage account name>
      container_name       = <container name>
      key                  = "state/validator"
    }
  }
  module "aptos-node" {
    # download Terraform module from aptos-labs/aptos-core repo
    source        = "github.com/aptos-labs/aptos-core.git//terraform/aptos-node/azure?ref=mainnet"
    region        = <azure region>  # Specify the region
    era           = 1  # bump era number to wipe the chain
    chain_id      = 1  # for mainnet. Use different value for testnet or devnet.
    image_tag     = "mainnet" # Specify the docker image tag to use
    validator_name = "<Name of your validator>"
  }
  ```

    For the full customization options, see the variables file [`variables.tf`](https://github.com/aptos-labs/aptos-core/blob/main/terraform/aptos-node/azure/variables.tf), and the [Helm values](https://github.com/aptos-labs/aptos-core/blob/main/terraform/helm/aptos-node/values.yaml).

5. Initialize Terraform in the same directory of your `main.tf` file.
  ```bash
  terraform init
  ```
This will download all the Terraform dependencies for you, in the `.terraform` folder in your current working directory.

6. Create a new Terraform workspace to isolate your environments:
  ```bash
  terraform workspace new $WORKSPACE
  # This command will list all workspaces
  terraform workspace list
  ```

7. Apply the configuration.
  ```bash
  terraform apply
  ```
  This might take a while to finish (~20 minutes), Terraform will create all the resources on your cloud account.

8. Once terraform apply finishes, you can check if those resources are created:

    - `az aks get-credentials --resource-group aptos-$WORKSPACE --name aptos-$WORKSPACE` to configure access for your k8s cluster.
    - `kubectl get pods` this should have haproxy, validator and fullnode. with validator and fullnode pod `pending` (require further action in later steps)
    - `kubectl get svc` this should have `validator-lb` and `fullnode-lb`, with an external-IP you can share later for connectivity.

9. Get your node IP info:

    ```bash
    export VALIDATOR_ADDRESS="$(kubectl get svc ${WORKSPACE}-aptos-node-0-validator-lb --output jsonpath='{.status.loadBalancer.ingress[0].hostname}')"

    export FULLNODE_ADDRESS="$(kubectl get svc ${WORKSPACE}-aptos-node-0-fullnode-lb --output jsonpath='{.status.loadBalancer.ingress[0].hostname}')"
    ```

10. Generate the key pairs (node owner, voter, operator key, consensus key and networking key) in your working directory.

    ```bash
    aptos genesis generate-keys --output-dir ~/$WORKSPACE/keys
    ```

    This will create 4 key files under `~/$WORKSPACE/keys` directory: 
      - `public-keys.yaml`
      - `private-keys.yaml`
      - `validator-identity.yaml`, and
      - `validator-full-node-identity.yaml`.
      
      :::danger IMPORTANT

       Backup your `private-keys.yaml` somewhere safe. These keys are important for you to establish ownership of your node. **Never share private keys with anyone.**
      :::

11. Configure the validator information.

    ```bash
    aptos genesis set-validator-configuration \
      --local-repository-dir ~/$WORKSPACE \
      --username $USERNAME \
      --owner-public-identity-file ~/$WORKSPACE/keys/public-keys.yaml \
      --validator-host $VALIDATOR_ADDRESS:6180 \
      --full-node-host $FULLNODE_ADDRESS:6182 \
      --stake-amount 100000000000000

    ```

    This will create two YAML files in the `~/$WORKSPACE/$USERNAME` directory: `owner.yaml` and `operator.yaml`. 

12. Download the following files by following the download commands on the [Node Files](../../../node-files-all-networks/node-files.md) page:
    - `genesis.blob`
    - `waypoint.txt`

13. **Summary:** To summarize, in your working directory you should have a list of files:
    - `main.tf`: The Terraform files to install the `aptos-node` module (from steps 3 and 4).
    - `keys` folder containing:
      - `public-keys.yaml`: Public keys for the owner account, consensus, networking (from step 10).
      - `private-keys.yaml`: Private keys for the owner account, consensus, networking (from step 10).
      - `validator-identity.yaml`: Private keys for setting the Validator identity (from step 10).
      - `validator-full-node-identity.yaml`: Private keys for setting validator full node identity (from step 10).
    - `username` folder containing: 
      - `owner.yaml`: Defines owner, operator, and voter mapping. They are all the same account in test mode (from step 11).
      - `operator.yaml`: Node information that will be used for both the Validator and the fullnode (from step 11). 
    - `waypoint.txt`: The waypoint for the genesis transaction (from step 12).
    - `genesis.blob` The genesis binary that contains all the information about the framework, validatorSet and more (from step 12).

14. Insert `genesis.blob`, `waypoint.txt` and the identity files as secret into k8s cluster.

    ```bash
    kubectl create secret generic ${WORKSPACE}-aptos-node-0-genesis-e1 \
        --from-file=genesis.blob=genesis.blob \
        --from-file=waypoint.txt=waypoint.txt \
        --from-file=validator-identity.yaml=keys/validator-identity.yaml \
        --from-file=validator-full-node-identity.yaml=keys/validator-full-node-identity.yaml
    ```
  
    :::tip
    
    The `-e1` suffix refers to the era number. If you changed the era number, make sure it matches when creating the secret.

    :::

15. Check that all pods are running.

    ```bash
    kubectl get pods

    NAME                                        READY   STATUS    RESTARTS   AGE
    node1-aptos-node-0-fullnode-e9-0              1/1     Running   0          4h31m
    node1-aptos-node-0-haproxy-7cc4c5f74c-l4l6n   1/1     Running   0          4h40m
    node1-aptos-node-0-validator-0                1/1     Running   0          4h30m
    ```

You have successfully completed setting up your node. Make sure that you have set up one machine to run a validator node and a second machine to run a validator fullnode.

Now proceed to [connecting to the Aptos network](../connect-to-aptos-network.md) and [establishing staking pool operations](../staking-pool-operations.md).

---
title: "On AWS"
slug: "run-validator-node-using-aws"
---

# On AWS

This is a step-by-step guide to install an Aptos node on Amazon Web Services (AWS). Follow these steps to configure a validator node and a validator fullnode on separate machines. 

:::caution Did you set up your AWS account?
This guide assumes that you already have AWS account setup.
:::

:::danger Do you have stale volumes after bumping your deployment's era?
`era` is a concept relevant only to Kubernetes deployments of an Aptos node. Changing the `era` provides an easy way to wipe your deployment's state. However, this may lead to dangling persistent volumes on validator fullnodes. Confirm the existence of these volumes with `kubectl get pvc` and delete them manually to minimize costs.
:::

## Before you proceed

Make sure you complete these prerequisite steps before you proceed:

1. Set up your AWS account. 
2. Make sure the following are installed on your local computer:
   - **Aptos CLI**: https://aptos.dev/tools/aptos-cli/install-cli/index
   - **Terraform 1.3.6**: https://www.terraform.io/downloads.html
   - **Kubernetes CLI**: https://kubernetes.io/docs/tasks/tools/
   - **AWS CLI**: https://aws.amazon.com/cli/

## Install

:::tip One validator node + one validator fullnode
Follow the below instructions **twice**, i.e., first on one machine to run a validator node and the second time on another machine to run a validator fullnode. 
:::

1. Create a working directory for your node configuration.

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

2. Create an S3 storage bucket for storing the Terraform state on AWS. You can do this on the AWS UI or by the below command: 

      ```bash
      aws s3 mb s3://<bucket name> --region <region name>
      ```

3. Create a Terraform file called `main.tf` in your working directory:

    ```bash
    cd ~/$WORKSPACE
    vi main.tf
    ```

4. Modify the `main.tf` file to configure Terraform and to create Aptos fullnode from the Terraform module. See below example content for `main.tf`:

    ```
    terraform {
      required_version = "~> 1.3.6"
      backend "s3" {
        bucket = "terraform.aptos-node"
        key    = "state/aptos-node"
        region = <aws region>
      }
    }

    provider "aws" {
      region = <aws region>
    }

    module "aptos-node" {
      # Download Terraform module from aptos-labs/aptos-core repo
      source        = "github.com/aptos-labs/aptos-core.git//terraform/aptos-node/aws?ref=mainnet"
      region        = <aws region>  # Specify the region
      # zone_id     = "<Route53 zone id>"  # zone id for Route53 if you want to use DNS
      era           = 1  # bump era number to wipe the chain
      chain_id      = 1  # for mainnet. Use different value for testnet or devnet.
      image_tag     = "mainnet" # Specify the image tag to use
      validator_name = "<Name of your validator>"
    }
    ```

    For full customization options, see:
      - The Terraform variables file [https://github.com/aptos-labs/aptos-core/blob/main/terraform/aptos-node/aws/variables.tf](https://github.com/aptos-labs/aptos-core/blob/main/terraform/aptos-node/aws/variables.tf), and 
      - The values YAML file [https://github.com/aptos-labs/aptos-core/blob/main/terraform/helm/aptos-node/values.yaml](https://github.com/aptos-labs/aptos-core/blob/main/terraform/helm/aptos-node/values.yaml).

5. Initialize Terraform in the `$WORKSPACE` directory where you created the `main.tf` file.

  ```bash
  terraform init
  ```
This will download all the Terraform dependencies into the `.terraform` folder in your current working directory.

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

  This may take a while to finish (~20 minutes). Terraform will create all the resources on your AWS cloud account.

8. After `terraform apply` finishes, you can check if those resources are created:

    - `aws eks update-kubeconfig --name aptos-$WORKSPACE`: To configure access for your k8s cluster.
    - `kubectl get pods`: This should have haproxy, validator and fullnode, with validator and fullnode pod `pending` (require further action in later steps).
    - `kubectl get svc`: This should have `validator-lb` and `fullnode-lb`, with an external IP you can share later for connectivity.

9. Get your node IP information into your environment:

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
      - `owner.yaml`: Defines owner, operator, and voter mapping. 
      - `operator.yaml`: Node information that will be used for both the validator and the validator fullnode (from step 11). 
    - `waypoint.txt`: The waypoint for the genesis transaction (from step 12).
    - `genesis.blob` The genesis binary that contains all the information about the framework, validator set, and more (from step 12).

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


15. Check that all the pods are running.

    ```bash
    kubectl get pods

    NAME                                        READY   STATUS    RESTARTS   AGE
    node1-aptos-node-0-fullnode-e9-0              1/1     Running   0          4h31m
    node1-aptos-node-0-haproxy-7cc4c5f74c-l4l6n   1/1     Running   0          4h40m
    node1-aptos-node-0-validator-0                1/1     Running   0          4h30m
    ```

You have successfully completed setting up your node. Make sure that you have set up one machine to run a validator node and a second machine to run a validator fullnode.

Now proceed to [connecting to the Aptos network](../connect-to-aptos-network.md) and [establishing staking pool operations](../staking-pool-operations.md).

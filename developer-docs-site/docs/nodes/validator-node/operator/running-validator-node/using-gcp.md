---
title: "On GCP"
slug: "run-validator-node-using-gcp"
---

# On GCP

This is a step-by-step guide to install an Aptos node on Google GCP. Follow these steps to configure a validator node and a validator fullnode on separate machines. 

:::caution Did you set up your GCP account and create a project?
This guide assumes you already have a Google Cloud Platform (GCP) account setup, and have created a new project for deploying Aptos node. If you are not familiar with GCP (Google Cloud Platform), review the [Prerequisites](../../../full-node/run-a-fullnode-on-gcp#prerequisites) section for GCP account setup.
:::

:::danger Do you have stale volumes after bumping your deployment's era?
`era` is a concept relevant only to Kubernetes deployments of an Aptos node. Changing the `era` provides an easy way to wipe your deployment's state. However, this may lead to dangling persistent volumes on validator fullnodes. Confirm the existence of these volumes with `kubectl get pvc` and delete them manually to minimize costs.
:::

## Before you proceed

Make sure the following are setup for your environment:
  - **GCP account**: hhttps://cloud.google.com/
  - **Aptos CLI**: https://aptos.dev/tools/aptos-cli/install-cli/
  - **Terraform 1.3.6**: https://www.terraform.io/downloads.html
  - **Kubernetes CLI**: https://kubernetes.io/docs/tasks/tools/
  - **Google Cloud CLI**: https://cloud.google.com/sdk/docs/install-sdk

## Install

:::tip One validator node + one validator fullnode
Follow the below instructions **twice**, i.e., first on one machine to run a validator node and the second time on another machine to run a validator fullnode. 
:::

1. Create a working directory for your configuration.

    * Choose a workspace name, for example, `mainnet` for mainnet, or `testnet` for testnet, and so on. **Note**: This defines the Terraform workspace name, which, in turn, is used to form the resource names.
      ```bash
      export WORKSPACE=mainnet
      ```

    * Create a directory for the workspace
      ```bash
      mkdir -p ~/$WORKSPACE
      ```

    * Choose a username for your node, for example `alice`.

      ```bash
      export USERNAME=alice
      ```

2. Create a storage bucket for storing the Terraform state on Google Cloud Storage.  Use the GCP UI or Google Cloud Storage command to create the bucket.  The name of the bucket must be unique. See the Google Cloud Storage documentation here: https://cloud.google.com/storage/docs/creating-buckets#prereq-cli.

  ```bash
  gsutil mb gs://BUCKET_NAME
  # for example
  gsutil mb gs://<project-name>-aptos-terraform-dev
  ```

3. Create Terraform file called `main.tf` in your working directory:
  ```bash
  cd ~/$WORKSPACE
  touch main.tf
  ```

4. Modify `main.tf` file to configure Terraform, and create fullnode from Terraform module. Example content for `main.tf`:
  ```
  terraform {
    required_version = "~> 1.3.6"
    backend "gcs" {
      bucket = "BUCKET_NAME" # bucket name created in step 2
      prefix = "state/aptos-node"
    }
  }

  module "aptos-node" {
    # download Terraform module from aptos-labs/aptos-core repo
    source        = "github.com/aptos-labs/aptos-core.git//terraform/aptos-node/gcp?ref=mainnet"
    region        = "us-central1"  # Specify the region
    zone          = "c"            # Specify the zone suffix
    project       = "<GCP Project ID>" # Specify your GCP project ID
    era           = 1  # bump era number to wipe the chain
    chain_id      = 1  # for mainnet. Use different value for testnet or devnet.
    image_tag     = "mainnet" # Specify the docker image tag to use
    validator_name = "<Name of your validator, no space, e.g. aptosbot>"
  }
  ```

  For the full customization options, see the variables file [`variables.tf`](https://github.com/aptos-labs/aptos-core/blob/main/terraform/aptos-node/gcp/variables.tf), and the [helm values](https://github.com/aptos-labs/aptos-core/blob/main/terraform/helm/aptos-node/values.yaml).

5. Initialize Terraform in the same directory of your `main.tf` file
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

  This might take a while to finish (10 - 20 minutes), Terraform will create all the resources on your cloud account. 

8. Once Terraform apply finishes, you can check if those resources are created:

    - `gcloud container clusters get-credentials aptos-$WORKSPACE --zone <region/zone> --project <project>` to configure the access for k8s cluster.
    - `kubectl get pods` this should have haproxy, validator and fullnode. with validator and fullnode pod `pending` (require further action in later steps)
    - `kubectl get svc` this should have `validator-lb` and `fullnode-lb`, with an external-IP you can share later for connectivity.

9. Get your node IP info:

    ```bash
    export VALIDATOR_ADDRESS="$(kubectl get svc ${WORKSPACE}-aptos-node-0-validator-lb --output jsonpath='{.status.loadBalancer.ingress[0].ip}')"

    export FULLNODE_ADDRESS="$(kubectl get svc ${WORKSPACE}-aptos-node-0-fullnode-lb --output jsonpath='{.status.loadBalancer.ingress[0].ip}')"
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

13. To summarize, in your working directory you should have a list of files:
    - `main.tf`: The Terraform files to install the `aptos-node` module (from steps 3 and 4).
    - `keys` folder, which includes:
      - `public-keys.yaml`: Public keys for the owner account, consensus, networking (from step 10).
      - `private-keys.yaml`: Private keys for the owner account, consensus, networking (from step 10).
      - `validator-identity.yaml`: Private keys for setting the Validator identity (from step 10).
      - `validator-full-node-identity.yaml`: Private keys for setting validator full node identity (from step 10).
    - `username` folder, which includes: 
      - `owner.yaml`: define owner, operator, and voter mapping. They are all the same account in test mode (from step 11).
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

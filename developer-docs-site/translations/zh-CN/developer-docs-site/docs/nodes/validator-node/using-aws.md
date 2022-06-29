---
title: "On AWS"
slug: "run-validator-node-using-aws"
---

# On AWS

This is a step-by-step guide to install an Aptos node on AWS. These steps will configure a Validator node and a FullNode on separate machines.

## Before you proceed

Make sure you complete these pre-requisite steps before you proceed:

1. Set up your AWS account.
2. Make sure the following are installed on your local computer:

   * **Aptos CLI**: https://github.com/aptos-labs/aptos-core/blob/main/crates/aptos/README.md
   * **Terraform 1.1.7**: https://www.terraform.io/downloads.html
   * **Kubernetes CLI**: https://kubernetes.io/docs/tasks/tools/
   * **AWS CLI**: https://aws.amazon.com/cli/

## Install

1. Create a working directory for your node configuration.

    * Choose a workspace name, for example, `testnet`. **Note**: This defines the Terraform workspace name, which, in turn, is used to form the resource names.

      ```
      export WORKSPACE=testnet
      ```

    * Create a directory for the workspace.

      ```
      mkdir -p ~/$WORKSPACE
      ```

2. Create an S3 storage bucket for storing the Terraform state on AWS. You can do this on the AWS UI or by the below command:

      ```
      aws s3api create-bucket --bucket <bucket name> --region <region name>
      ```

3. Create a Terraform file called `main.tf` in your working directory:

    ```
    cd ~/$WORKSPACE
    vi main.tf
    ```

4. Modify the `main.tf` file to configure Terraform and to create Aptos FullNode from the Terraform module. See below example content for `main.tf`:

    ```
    terraform {
      required_version = "~> 1.1.0"
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
      source        = "github.com/aptos-labs/aptos-core.git//terraform/aptos-node/aws?ref=main"
      region        = <aws region>  # Specify the region
      # zone_id     = "<Route53 zone id>"  # zone id for Route53 if you want to use DNS
      era           = 1              # bump era number to wipe the chain
      chain_id      = 23
      image_tag     = "testnet" # Specify the image tag to use
      validator_name = "<Name of your Validator>"
    }
    ```

    For full customization options, see:
      - The Terraform variables file [https://github.com/aptos-labs/aptos-core/blob/main/terraform/aptos-node/aws/variables.tf](https://github.com/aptos-labs/aptos-core/blob/main/terraform/aptos-node/aws/variables.tf), and
      - The values YAML file [https://github.com/aptos-labs/aptos-core/blob/main/terraform/helm/aptos-node/values.yaml](https://github.com/aptos-labs/aptos-core/blob/main/terraform/helm/aptos-node/values.yaml).

5. Initialize Terraform in the `$WORKSPACE` directory where you created the `main.tf` file.

  ```
  terraform init
  ```
This will download all the Terraform dependencies into the `.terraform` folder in your current working directory.

6. Create a new Terraform workspace to isolate your environments:

  ```
  terraform workspace new $WORKSPACE
  # This command will list all workspaces
  terraform workspace list
  ```

7. Apply the configuration.

  ```
  terraform apply
  ```

  This may take a while to finish (~20 minutes). Terraform will create all the resources on your AWS cloud account.

8. After `terraform apply` finishes, you can check if those resources are created:

    - `aws eks update-kubeconfig --name aptos-$WORKSPACE`: To configure access for your k8s cluster.
    - `kubectl get pods`: This should have haproxy, validator and fullnode, with validator and fullnode pod `pending` (require further action in later steps).
    - `kubectl get svc`: This should have `validator-lb` and `fullnode-lb`, with an external IP you can share later for connectivity.

9. Get your node IP information into your environment:

    ```
    export VALIDATOR_ADDRESS="$(kubectl get svc ${WORKSPACE}-aptos-node-validator-lb --output jsonpath='{.status.loadBalancer.ingress[0].hostname}')"

    export FULLNODE_ADDRESS="$(kubectl get svc ${WORKSPACE}-aptos-node-fullnode-lb --output jsonpath='{.status.loadBalancer.ingress[0].hostname}')"
    ```

10. Generate the key pairs (node owner key, consensus key and networking key) in your working directory.

    ```
    aptos genesis generate-keys --output-dir ~/$WORKSPACE
    ```

    This will create three key files:
      - `private-keys.yaml`
      - `validator-identity.yaml`, and
      - `validator-full-node-identity.yaml`.

      :::caution IMPORTANT

       Backup your key files somewhere safe. These key files are important for you to establish ownership of your node, and you will use this information to claim your rewards later if eligible. **Never share these keys with anyone.** :::

11. Configure the Validator information. This is all the information you need to register on Aptos community website later.

    ```
    aptos genesis set-validator-configuration --keys-dir ~/$WORKSPACE --local-repository-dir ~/$WORKSPACE --username <select a username for your node> --validator-host $VALIDATOR_ADDRESS:6180 --full-node-host $FULLNODE_ADDRESS:6182

    ```

    This will create a YAML file in your working directory with your username. For example, for a username `aptosbot` the file will be `aptosbot.yaml`, and the file contents will look like below:

    ```
    ---
    account_address: 7410973313fd0b5c69560fd8cd9c4aaeef873f869d292d1bb94b1872e737d64f
    consensus_key: "0x4e6323a4692866d54316f3b08493f161746fda4daaacb6f0a04ec36b6160fdce"
    account_key: "0x83f090aee4525052f3b504805c2a0b1d37553d611129289ede2fc9ca5f6aed3c"
    network_key: "0xa06381a17b090b8db5ffef97c6e861baad94a1b0e3210e6309de84c15337811d"
    validator_host:
      host: 30247cc34f270cb8.elb.us-west-2.amazonaws.com
      port: 6180
    full_node_host:
      host: abc5b9734d4cc418.elb.us-west-2.amazonaws.com
      port: 6182
    stake_amount: 1
    ```

12. Create a layout YAML file, which defines the node in the Aptos `validatorSet`. For the test mode, it is sufficient to create a genesis blob containing only one node.

  ```
  vi layout.yaml
  ```

  Add the `root_key`, the Validator node username, and `chain_id` in the `layout.yaml` file. For example:

  ```
  ---
  root_key: "0x5243ca72b0766d9e9cbf2debf6153443b01a1e0e6d086c7ea206eaf6f8043956"
  users:
    - <username you specified for the node in step 11>
  chain_id: 23
  ```

13. Download the AptosFramework Move bytecode into a folder named `framework`.

    Download the Aptos Framework zip file from the Aptos release page: https://github.com/aptos-labs/aptos-core/releases/tag/aptos-framework-v0.1.0 and unzip it.

    ```
    wget https://github.com/aptos-labs/aptos-core/releases/download/aptos-framework-v0.1.0/framework.zip
    unzip framework.zip
    ```

    You will now have a folder called `framework`, which contains the Move bytecode with the format `.mv`.

14. Compile the genesis blob and waypoint.

    ```
    aptos genesis generate-genesis --local-repository-dir ~/$WORKSPACE --output-dir ~/$WORKSPACE
    ```

    This will create two files in your working directory: `genesis.blob` and `waypoint.txt`.

15. To summarize, in your working directory you should have a list of files:
    - `main.tf`: The Terraform files to install the `aptos-node` module (from steps 3 and 4).
    - `private-keys.yaml`: Private keys for the owner account, consensus, networking (from step 10).
    - `validator-identity.yaml`: Private keys for setting the Validator identity (from step 10).
    - `validator-full-node-identity.yaml`: Private keys for setting validator full node identity (from step 10).
    - `<username>.yaml`: Node information that will be used for both the Validator and the FullNode (from step 11).
    - `layout.yaml`: The layout file containing the key values for root key, validator user, and chain ID (from step 12).
    - `framework`: The folder that contains all the Move bytecode you downloaded and unzipped (from step 13).
    - `waypoint.txt`: The waypoint for the genesis transaction (from step 14).
    - `genesis.blob` The genesis binary that contains all the information about the framework, validatorSet and more (from step 14).

16. Insert `genesis.blob`, `waypoint.txt` and the identity files as secret into k8s cluster.

    ```
    kubectl create secret generic ${WORKSPACE}-aptos-node-genesis-e1 \
        --from-file=genesis.blob=genesis.blob \
        --from-file=waypoint.txt=waypoint.txt \
        --from-file=validator-identity.yaml=validator-identity.yaml \
        --from-file=validator-full-node-identity.yaml=validator-full-node-identity.yaml
    ```

    :::note

    The `-e1` suffix refers to the era number. If you changed the era number, make sure it matches when creating the secret.

    :::


17. Check that all the pods are running.

    ```
    kubectl get pods

    NAME                                        READY   STATUS    RESTARTS   AGE
    node1-aptos-node-fullnode-e9-0              1/1     Running   0          4h31m
    node1-aptos-node-haproxy-7cc4c5f74c-l4l6n   1/1     Running   0          4h40m
    node1-aptos-node-validator-0                1/1     Running   0          4h30m
    ```

Now you have successfully completed setting up your node in test mode. You can now proceed to the [Aptos community](https://community.aptoslabs.com/) website for registration.

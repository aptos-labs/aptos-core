# Using Terraform

If you're not familar with GCP (Google Cloud Platform), checkout this [tutorial](https://aptos.dev/tutorials/run-a-fullnode-on-gcp#prerequisites) for GCP account setup.

## Run on GCP
This guide assumes you already have GCP account setup, and have created a new project for deploying Aptos node.

Install pre-requisites if needed:

   * Terraform 1.1.7: https://www.terraform.io/downloads.html
   * Kubernetes CLI: https://kubernetes.io/docs/tasks/tools/
   * Google Cloud CLI: https://cloud.google.com/sdk/docs/install-sdk

1. Create a working directory for your configuration.

    * Choose a workspace name e.g. `testnet`. Note: this defines Terraform workspace name, which in turn is used to form resource names.
    ```
    $ export WORKSPACE=testnet
    ```

    * Create a directory for the workspace
    ```
    $ mkdir -p ~/$WORKSPACE
    ```
2. Create a storage bucket for storing the Terraform state on Google Cloud Storage.  Use the GCP UI or Google Cloud Storage command to create the bucket.  The name of the bucket must be unique.  See the Google Cloud Storage documentation here: https://cloud.google.com/storage/docs/creating-buckets#prereq-cli

  ```
  $ gsutil mb gs://BUCKET_NAME
  # for example
  $ gsutil mb gs://<project-name>-aptos-terraform-dev
  ```

3. Create Terraform file called `main.tf` in your working directory:
  ```
  $ cd ~/$WORKSPACE
  $ touch main.tf
  ```

4. Modify `main.tf` file to configure Terraform, and create fullnode from Terraform module. Example content for `main.tf`:
  ```
  terraform {
    required_version = "~> 1.2.0"
    backend "gcs" {
      bucket = "BUCKET_NAME" # bucket name created in step 2
      prefix = "state/aptos-node"
    }
  }

  module "aptos-node" {
    # download Terraform module from aptos-labs/aptos-core repo
    source        = "github.com/aptos-labs/aptos-core.git//terraform/aptos-node/gcp?ref=testnet"
    region        = "us-central1"  # Specify the region
    zone          = "c"            # Specify the zone suffix
    project       = "<GCP Project Name>" # Specify your GCP project name
    era           = 1              # bump era number to wipe the chain
    chain_id      = 5
    image_tag     = "testnet" # Specify the docker image tag to use
    validator_name = "<Name of Your Validator, no space>"
  }
  ```

  For the full customization options, see the variables file [here](https://github.com/aptos-labs/aptos-core/blob/main/terraform/aptos-node/gcp/variables.tf), and the [helm values](https://github.com/aptos-labs/aptos-core/blob/main/terraform/helm/aptos-node/values.yaml).

5. Initialize Terraform in the same directory of your `main.tf` file
  ```
  $ terraform init
  ```
This will download all the Terraform dependencies for you, in the `.terraform` folder in your current working directory.

6. Create a new Terraform workspace to isolate your environments:
  ```
  $ terraform workspace new $WORKSPACE
  # This command will list all workspaces
  $ terraform workspace list
  ```

7. Apply the configuration.

  ```
  $ terraform apply
  ```

  This might take a while to finish (10 - 20 minutes), Terraform will create all the resources on your cloud account. 

8. Once Terraform apply finishes, you can check if those resources are created:

    - `gcloud container clusters get-credentials aptos-$WORKSPACE --zone <region/zone> --project <project>` to configure the access for k8s cluster.
    - `kubectl get pods` this should have haproxy, validator and fullnode. with validator and fullnode pod `pending` (require further action in later steps)
    - `kubectl get svc` this should have `validator-lb` and `fullnode-lb`, with an external-IP you can share later for connectivity.

9. Get your node IP info:

    ```
    $ export VALIDATOR_ADDRESS="$(kubectl get svc ${WORKSPACE}-aptos-node-validator-lb --output jsonpath='{.status.loadBalancer.ingress[0].ip}')"

    $ export FULLNODE_ADDRESS="$(kubectl get svc ${WORKSPACE}-aptos-node-fullnode-lb --output jsonpath='{.status.loadBalancer.ingress[0].ip}')"
    ```

10. Generate key pairs (node owner key, consensus key and networking key) in your working directory.

    ```
    $ aptos genesis generate-keys --output-dir ~/$WORKSPACE
    ```

    This will create three files: `private-keys.yaml`, `validator-identity.yaml`, `validator-full-node-identity.yaml` for you. **IMPORTANT**: Backup your key files somewhere safe. These key files are important for you to establish ownership of your node, and you will use this information to claim your rewards later if eligible.

11. Configure validator information.

    ```
    $ aptos genesis set-validator-configuration --keys-dir ~/$WORKSPACE --local-repository-dir ~/$WORKSPACE --username <pick a username for your node> --validator-host $VALIDATOR_ADDRESS:6180 --full-node-host $FULLNODE_ADDRESS:6182

    ```

    This will create a YAML file in your working directory with your username, e.g. `aptosbot.yaml`, it should looks like:

    ```
    ---
    account_address: 7410973313fd0b5c69560fd8cd9c4aaeef873f869d292d1bb94b1872e737d64f
    consensus_key: "0x4e6323a4692866d54316f3b08493f161746fda4daaacb6f0a04ec36b6160fdce"
    account_key: "0x83f090aee4525052f3b504805c2a0b1d37553d611129289ede2fc9ca5f6aed3c"
    network_key: "0xa06381a17b090b8db5ffef97c6e861baad94a1b0e3210e6309de84c15337811d"
    validator_host:
      host: 35.232.235.205
      port: 6180
    full_node_host:
      host: 34.135.169.144
      port: 6182
    stake_amount: 1
    ```

12. Create layout YAML file, which defines the node in the validatorSet. For test mode, we can create a genesis blob containing only one node. **Note: this step is only needed for starting the node in test mode, for production, it will be generated by Aptos Labs**

    ```
    $ vi layout.yaml
    ```

    Add root key, node username, and chain_id in the `layout.yaml` file, for example:

    ```
    ---
    root_key: "0x5243ca72b0766d9e9cbf2debf6153443b01a1e0e6d086c7ea206eaf6f8043956"
    users:
      - <username you created in step 5>
    chain_id: 5
    ```

13. Download AptosFramework Move bytecode into a folder named `framework`. **Note: this step is only needed for starting the node in test mode, for production, it will be generated by Aptos Labs**

    Download the Aptos Framework from the release page: https://github.com/aptos-labs/aptos-core/releases/tag/aptos-framework-v0.1.0

    ```
    $ unzip framework.zip
    ```

    You should now have a folder called `framework`, which contains move bytecodes with format `.mv`.

14. Compile genesis blob and waypoint. **Note: this step is only needed for starting the node in test mode, for production, it will be generated by Aptos Labs**

    ```
    $ aptos genesis generate-genesis --local-repository-dir ~/$WORKSPACE --output-dir ~/$WORKSPACE
    ``` 

    This will create two files in your working directory, `genesis.blob` and `waypoint.txt`

15. To recap, in your working directory, you should have a list of files:
    - `private-keys.yaml` Private keys for owner account, consensus, networking
    - `validator-identity.yaml` Private keys for setting validator identity
    - `validator-full-node-identity.yaml` Private keys for setting validator full node identity
    - `<username>.yaml` Node info for both validator / fullnode
    - `layout.yaml` layout file to define root key, validator user, and chain ID
    - `framework` folder which contains all the move bytecode for AptosFramework.
    - `waypoint.txt` waypoint for genesis transaction
    - `genesis.blob` genesis binary contains all the info about framework, validatorSet and more.

16. Insert `genesis.blob`, `waypoint.txt` and identity files as secret into k8s cluster.

    ```
    $ kubectl create secret generic ${WORKSPACE}-aptos-node-genesis-e1 \
        --from-file=genesis.blob=genesis.blob \
        --from-file=waypoint.txt=waypoint.txt \
        --from-file=validator-identity.yaml=validator-identity.yaml \
        --from-file=validator-full-node-identity.yaml=validator-full-node-identity.yaml
    ```

    If you changed the era number, make sure it matches when creating the secret.

17. Check all pods running.

    ```
    $ kubectl get pods

    NAME                                        READY   STATUS    RESTARTS   AGE
    node1-aptos-node-fullnode-e9-0              1/1     Running   0          4h31m
    node1-aptos-node-haproxy-7cc4c5f74c-l4l6n   1/1     Running   0          4h40m
    node1-aptos-node-validator-0                1/1     Running   0          4h30m
    ```

# Using Terraform

If you're not familar with GCP (Google Cloud Platform), checkout this [tutorial](https://velor.dev/nodes/full-node/run-a-fullnode-on-gcp#prerequisites) for GCP account setup.

## Run on GCP
This guide assumes you already have GCP account setup, and have created a new project for deploying Velor node.

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
  $ gsutil mb gs://<project-name>-velor-terraform-dev
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
      prefix = "state/velor-node"
    }
  }

  module "velor-node" {
    # download Terraform module from velor-chain/velor-core repo
    source        = "github.com/velor-chain/velor-core.git//terraform/velor-node/gcp?ref=testnet"
    region        = "us-central1"  # Specify the region
    zone          = "c"            # Specify the zone suffix
    project       = "<GCP Project Name>" # Specify your GCP project name
    era           = 1              # bump era number to wipe the chain
    chain_id      = 5
    image_tag     = "testnet" # Specify the docker image tag to use
    validator_name = "<Name of Your Validator, no space>"
  }
  ```

  For the full customization options, see the variables file [here](https://github.com/velor-chain/velor-core/blob/main/terraform/velor-node/gcp/variables.tf), and the [helm values](https://github.com/velor-chain/velor-core/blob/main/terraform/helm/velor-node/values.yaml).

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

    - `gcloud container clusters get-credentials velor-$WORKSPACE --zone <region/zone> --project <project>` to configure the access for k8s cluster.
    - `kubectl get pods` this should have haproxy, validator and fullnode. with validator and fullnode pod `pending` (require further action in later steps)
    - `kubectl get svc` this should have `validator-lb` and `fullnode-lb`, with an external-IP you can share later for connectivity.

  If you have just created your GCP project, you'll need to authorize the service account of your cluster to fetch images from the `velor-global` Docker registry.
  
    - open [the registry](https://console.cloud.google.com/artifacts?referrer=search&project=velor-global)
    - select `velor-internal` then click on `ADD PRINCIPAL` on the right in the `Permissions` tab and assign the `Artifact Registry Reader` role to the service account

9. Get your node IP info:

    ```
    $ export VALIDATOR_ADDRESS="$(kubectl get svc \
        ${WORKSPACE}-velor-node-0-validator-lb \
        --output jsonpath='{.status.loadBalancer.ingress[0].ip}')"

    $ export FULLNODE_ADDRESS="$(kubectl get svc \
        ${WORKSPACE}-velor-node-0-fullnode-lb \
        --output jsonpath='{.status.loadBalancer.ingress[0].ip}')"
    ```

10. Generate key pairs (node owner key, consensus key and networking key) in your working directory.

    ```
    $ velor genesis generate-keys --output-dir ~/$WORKSPACE
    ```

    This will create four files: `public-keys.yaml`, `private-keys.yaml`, `validator-identity.yaml`, `validator-full-node-identity.yaml` for you. **IMPORTANT**: Backup your key files somewhere safe. These key files are important for you to establish ownership of your node, and you will use this information to claim your rewards later if eligible.

11. Configure validator information.

    ```
    $ velor genesis set-validator-configuration \
        --local-repository-dir ~/$WORKSPACE \
        --username <pick a username for your node> \
        --validator-host $VALIDATOR_ADDRESS:6180 \
        --full-node-host $FULLNODE_ADDRESS:6182
    ```

    This will create a directory in your working directory with your username, e.g. `velorbot`, and two files inside it: `operator.yaml` and `owner.yaml`. `operator.yaml` should looks like:

    ```
    ---
    operator_account_address: 2adeace541c3018d1117ae528c95a6cd91d924ab916f6e16d910b0668fe74b34
    operator_account_public_key: "0xb612f2727550042e0f8e3c0525f2b64a01e987598bc17c01167ccc94b30e32b4"
    consensus_public_key: "0x92eed9b185de3745b374200a3bb5e2173573bf8822edcee473a668182a1b1232c692c9a5c008f7425e752bf9aa84e03c"
    consensus_proof_of_possession: "0x810b0d3afb62e9905fcbe215a150d9709bb7c977ceaf05e1ab576c542b087743b35bf655e5db86c5db83ccbacb5926f40bc07e48bd2a00bcedacb43858a7fe3594890abccd03ff1ba340e3fe0e7895a27cdfe8739c16ca75e275af95d026caba"
    validator_network_public_key: "0xe83246a3f3203bb3919621330417243c891e67d8efd3072e237d7d97d4bbe70f"
    validator_host:
      host: xxx.xxx.xxx.xxx
      port: 6180
    full_node_network_public_key: "0x8f385f894027cfaa95c46d8a3c1b50476114a8bcdb62b2c7c07b391509b45717"
    full_node_host:
      host: xxx.xxx.xxx.xxx
      port: 6182
    ```

12. Create layout YAML file, which defines the node in the validatorSet. For test mode, we can create a genesis blob containing only one node. **Note: this step is only needed for starting the node in test mode, for production, it will be generated by Velor Labs**

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

13. Download VelorFramework Move bytecode into a folder named `framework`. **Note: this step is only needed for starting the node in test mode, for production, it will be generated by Velor Labs**

    Download the Velor Framework from the release page: https://github.com/velor-chain/velor-core/releases/tag/velor-framework-v0.1.0

    ```
    $ unzip framework.zip
    ```

    You should now have a folder called `framework`, which contains move bytecodes with format `.mv`.

14. Compile genesis blob and waypoint. **Note: this step is only needed for starting the node in test mode, for production, it will be generated by Velor Labs**

    ```
    $ velor genesis generate-genesis --local-repository-dir ~/$WORKSPACE --output-dir ~/$WORKSPACE
    ``` 

    This will create two files in your working directory, `genesis.blob` and `waypoint.txt`

15. To recap, in your working directory, you should have a list of files:
    - `private-keys.yaml` Private keys for owner account, consensus, networking
    - `validator-identity.yaml` Private keys for setting validator identity
    - `validator-full-node-identity.yaml` Private keys for setting validator full node identity
    - `<username>.yaml` Node info for both validator / fullnode
    - `layout.yaml` layout file to define root key, validator user, and chain ID
    - `framework` folder which contains all the move bytecode for VelorFramework.
    - `waypoint.txt` waypoint for genesis transaction
    - `genesis.blob` genesis binary contains all the info about framework, validatorSet and more.

16. Insert `genesis.blob`, `waypoint.txt` and identity files as secret into k8s cluster.

    ```
    $ kubectl create secret generic ${WORKSPACE}-velor-node-genesis-e1 \
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
    node1-velor-node-fullnode-e9-0              1/1     Running   0          4h31m
    node1-velor-node-haproxy-7cc4c5f74c-l4l6n   1/1     Running   0          4h40m
    node1-velor-node-validator-0                1/1     Running   0          4h30m
    ```

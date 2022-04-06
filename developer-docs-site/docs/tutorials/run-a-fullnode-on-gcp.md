---
title: "Run a FullNode on GCP"
slug: "run-a-fullnode-on-gcp"
sidebar_position: 11
---

# Run a FullNode on GCP

This tutorial explains how to configure and deploy a public FullNode to connect to the Aptos devnet using Google Cloud (GCP). Running Fullnode on Cloud usually provides better stability and avaliability compare to running it on your laptop, if you're looking for deploying a production grade Fullnode, we recommend you to deploy it on the cloud.

> **Note:** Please read [Run a Fullnode](run-a-fullnode) if you want other alternatives for deployment, using Cloud comes with a cost, and it varies depends on how you configure it.
>

## Prerequisites
Before you get started with this tutorial, install the required dependencies and get familiar with the toolings:
* Terraform 1.1.7: https://www.terraform.io/downloads.html
* Kubernetes cli: https://kubernetes.io/docs/tasks/tools/
* Google Cloud cli: https://cloud.google.com/sdk/docs/install-sdk

Once you have installed the gcloud CLI, log into GCP using gcloud (https://cloud.google.com/sdk/gcloud/reference/auth/login)
```
$ gcloud auth login --update-adc
```

## Getting started

You can deploy a public FullNode on GCP by using the Aptos fullnode Terraform module.

1. Create a working directory for your configuration.

    * Choose a workspace name e.g. `devnet`. Note: this defines terraform workspace name, which in turn is used to form resource names.
    ```
    $ export WORKSPACE=devnet
    ```

    * Create a directory for the workspace
    ```
    $ mkdir -p ~/$WORKSPACE
    ```

2. Create a storage bucket for storing the Terraform state on Google Cloud Storage.  Use the console or this gcs command to create the bucket.  See the Google Cloud Storage documentation here: https://cloud.google.com/storage/docs/creating-buckets#prereq-cli
  ```
  $ gsutil mb gs://BUCKET_NAME
  # for example
  $ gsutil mb gs://aptos-terraform-dev
  ```

3. Create Terraform file called `main.tf` in your working directory:
  ```
  $ cd ~/$WORKSPACE
  $ touch main.tf
  ```

4. Modify `main.tf` file to configure Terraform, and create fullnode from Terraform module. Example content for `main.tf`:
  ```
  terraform {
    required_version = "~> 1.1.0"
    backend "gcs" {
      bucket = "BUCKET_NAME" # bucket name created in step 2
      prefix = "state/fullnode"
    }
  }

  module "fullnode" {
    # download Terraform module from aptos-labs/aptos-core repo
    source        = "git@github.com:aptos-labs/aptos-core.git//terraform/fullnode/gcp?ref=main"
    region        = "us-central1"  # Specify the region
    zone          = "c"            # Specify the zone suffix
    project       = "gcp-fullnode" # Specify your GCP project name
    era           = 1              # bump era number to wipe the chain
    image_tag     = "dev_5b525691" # Specify the docker image tag to use
  }
  ```

5. Initialise Terraform in the same diretory of your `main.tf` file
  ```
  $ terraform init
  ```
This should download all the terraform dependencies for you, in the `.terraform` folder.

6. Create a new Terraform workspace to isolate your environments:
  ```
  $ terraform workspace new $WORKSPACE
  ```

7. Apply the configuration.
  ```
  $ terraform apply
  ```
  This might take a while to finish (10 - 20 minutes), Terraform will create all the resources on your cloud account.

## Validation

Once Terraform apply finished, you can follow this section to validate your deployment.

1. Configure your Kubernetes client to access the cluster you just deployed:
  ```
  $ gcloud container clusters get-credentials aptos-$WORKSPACE --zone <region_zone_name> --project <project_name>
  # for example:
  $ gcloud container clusters get-credentials aptos-devnet --zone us-central1-a --project aptos-fullnode
  ```

2. Check that your fullnode pods are now running (this may take a few minutes):
  ```
  $ kubectl get pods -n aptos
  ```

3. Get your fullnode IP:
  ```
  $ kubectl get svc -o custom-columns=IP:status.loadBalancer.ingress -n aptos
  ```

4. Check REST API, make sure the ledge version is increasing.
  ```
  $ curl http://<IP>
  ```

5. To verify the correctness of your FullNode, as outlined in the [fullnode documentation](https://aptos.dev/tutorials/run-a-fullnode/#verify-the-correctness-of-your-fullnode), you will need to set up a port-forwarding mechanism directly to the aptos pod in one ssh terminal and test it in another ssh terminal

   * Set up the port-forwarding to the aptos-fullnode pod.  Use `kubectl get pods -n aptos` to get the name of the pod
      ```
      $ kubectl port-forward -n aptos <pod-name> 9101:9101
      ```

   * Open a new ssh terminal.  Execute the following curl calls to verify the correctness
      ```
      $ curl -v http://0:9101/metrics 2> /dev/null | grep "aptos_state_sync_version{type=\"synced\"}"

      $ curl -v http://0:9101/metrics 2> /dev/null | grep "aptos_connections{direction=\"outbound\""
      ```

   * Exit port-forwarding when you are done by entering control-c in the terminal

## Update Fullnode With New Releases

There could be two types of releasees, one comes with a data wipe to startover the blockchain, one is just a software update.

### Upgrade with data wipe

1. You can increase the `era` number in `main.tf` to trigger a new data volume creation, which will start the node on a new DB.

2. Update `image_tag` in `main.tf`

3. Update Terraform module for fullnode, run this in the same directory of your `main.tf` file
  ```
  $ terraform get -update
  ```

4. Apply Terraform changes
  ```
  $ terraform apply
  ```

### Upgrade without data wipe

1. Update `image_tag` in `main.tf` (if you use `devnet` tag you can skip this step)

2. Update Terraform module for fullnode, run this in the same directory of your `main.tf` file
  ```
  $ terraform get -update
  ```

3. Apply Terraform changes
  ```
  $ terraform apply
  # if you didn't update the image tag, terraform will show nothing to change, in this case, force helm update
  $ terraform apply -var force_helm_update=true
  ```
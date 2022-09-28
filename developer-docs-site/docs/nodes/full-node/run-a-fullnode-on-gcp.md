---
title: "Run a Fullnode on GCP"
slug: "run-a-fullnode-on-gcp"
---

# Run a Fullnode on GCP

This tutorial explains how to configure and deploy a public fullnode to connect to the Aptos devnet using Google Cloud (GCP). Running a fullnode in the cloud usually provides better stability and availability compared to running it on your laptop. If you're looking for deploying a production grade fullnode, we recommend you to deploy it on the cloud.

> **Note:** Please read [Run a Fullnode](/nodes/full-node/public-fullnode) if you want other alternatives for deployment, using Cloud comes with a cost, and it varies depends on how you configure it.
>

## Prerequisites
You can run the commands in this guide to deploy your full node on Google Kubernetes Engine from any machine you want. From a [VM on GCP](https://cloud.google.com/compute), [Google Cloud Shell](https://cloud.google.com/shell), or your personal computer.

The following packages come pre-installed with Cloud Shell. Make sure to review the [documentation around ephermability](https://cloud.google.com/shell/docs/using-cloud-shell#choosing_ephemeral_mode) if you choose to use Cloud Shell. But if you are running the installation from your laptop or another machine, you need to install:
* Terraform 1.1.7: https://www.terraform.io/downloads.html
* Kubernetes cli: https://kubernetes.io/docs/tasks/tools/
* Google Cloud cli: https://cloud.google.com/sdk/docs/install-sdk

Once you have installed the gcloud CLI, log into GCP using gcloud (https://cloud.google.com/sdk/gcloud/reference/auth/login)
```
$ gcloud auth login --update-adc
```

If you already have a GCP account setup, jump right into [Getting Started](#getting-started), if you don't, follow the sections below to create and configure your GCP account.

### GCP Setup

#### Sign Up for the 90 Day Free Trial
Google Cloud offers a [90 day $300 free trial for every new user](https://cloud.google.com/free/docs/gcp-free-tier/#free-trial). These $300 are given as credits to your account and you can use them to get a sense of Google Cloud products. Be aware that you will need to add payment information when signing up for the free trial. This is for identity verification purposes and [will not incur charges until you upgrade to a paid account and run out of credits](https://cloud.google.com/free/docs/gcp-free-tier/#:~:text=Don%27t%20worry%2C%20setting,90%2Dday%20period).). Some GCP feature such as GPUs and Windows servers are not available in the free trial. 

[Sign up for the $300 in credits here.](https://cloud.google.com/free)

#### Create a new GCP Project
You will also need to create a new project on the GCP Console or using the gcloud command from the Google Cloud CLI. Before you do that, however, it may be helpful to familiarize yourself with the [resource hierarchy on GCP](https://cloud.google.com/resource-manager/docs/cloud-platform-resource-hierarchy).

[Follow these instructions to setup a new project.](https://cloud.google.com/resource-manager/docs/creating-managing-projects#creating_a_project)

#### Enable billing / Upgrade your Account
You will still be able to use the free trial credits, but enabling billing allows you to have full access to all the features of GCP and not experience any interruption to your nodes.

[Upgrade your account by following the steps outlined here.](https://cloud.google.com/free/docs/gcp-free-tier#how-to-upgrade)

#### Further GCP Resources
This should be enough to get your GCP setup ready to start deploying your fullnod. But if you are brand new to GCP, you may want to check out some of our [quickstart guides](https://cloud.google.com/docs/get-started/quickstarts) and [Google Cloud Skills Boost](https://www.cloudskillsboost.google/catalog).


## Getting started

You can deploy a public fullnode on GCP by using the Aptos fullnode Terraform module, this guide assumes you already have GCP account setup, and have created a new project for deploying Aptos fullnode. If you don't, check out the instructions above for [GCP Setup](#gcp-setup).

1. Create a working directory for your configuration.

    * Choose a workspace name e.g. `devnet`. Note: this defines terraform workspace name, which in turn is used to form resource names.
    ```
    $ export WORKSPACE=devnet
    ```

    * Create a directory for the workspace
    ```
    $ mkdir -p ~/$WORKSPACE
    ```

2. Create a storage bucket for storing the Terraform state on Google Cloud Storage.  Use the console or this gcs command to create the bucket.  The name of the bucket must be unique.  See the Google Cloud Storage documentation here: https://cloud.google.com/storage/docs/creating-buckets#prereq-cli
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
      prefix = "state/fullnode"
    }
  }

  module "fullnode" {
    # download Terraform module from aptos-labs/aptos-core repo
    source        = "github.com/aptos-labs/aptos-core.git//terraform/fullnode/gcp?ref=main"
    region        = "us-central1"  # Specify the region
    zone          = "c"            # Specify the zone suffix
    project       = "gcp-fullnode" # Specify your GCP project name
    era           = 1              # bump era number to wipe the chain
    image_tag     = "devnet"       # Specify the docker image tag to use, replace to `testnet` or other tag if needed

    fullnode_helm_values = {
      chain = {
      name = "devnet"              # replace with `ait3` or other values if connecting to different networks.
      }
    }
  }
  ```

5. Initialise Terraform in the same directory of your `main.tf` file
  ```
  $ terraform init
  ```
This should download all the terraform dependencies for you, in the `.terraform` folder.

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
  $ curl http://<IP>/v1
  # Example command syntax: curl http://104.198.36.142/v1
  ```

5. To verify the correctness of your fullnode, as outlined in the [fullnode documentation](https://aptos.dev/tutorials/run-a-fullnode/#verify-the-correctness-of-your-fullnode), you will need to set up a port-forwarding mechanism directly to the Aptos pod in one SSH terminal and test it in another ssh terminal

   * Set up the port-forwarding to the aptos-fullnode pod.  Use `kubectl get pods -n aptos` to get the name of the pod
      ```
      $ kubectl port-forward -n aptos <pod-name> 9101:9101
      # for example:
      $ kubectl port-forward -n aptos devnet0-aptos-fullnode-0 9101:9101
      ```

   * Open a new ssh terminal.  Execute the following curl calls to verify the correctness
      ```
      $ curl -v http://0:9101/metrics 2> /dev/null | grep "aptos_state_sync_version{type=\"synced\"}"

      $ curl -v http://0:9101/metrics 2> /dev/null | grep "aptos_connections{direction=\"outbound\""
      ```

   * Exit port-forwarding when you are done by entering control-c in the terminal

## Update fullnode with new releases

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

1. Update `image_tag` in `main.tf`

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

## Configure identity and seed peers

### Static identity

If you want to configure your node with a static identity, check the [fullnode advanced guide](https://aptos.dev/tutorials/run-a-fullnode#advanced-guide) for how to generate keys, and follow the instruction below to configure it in your terraform file.

1. Generate your own private key, and extract peer id, following the guide [here](https://aptos.dev/tutorials/run-a-fullnode#create-a-static-identity-for-a-fullnode)

2. Modify the `main.tf` to add `fullnode_identity` in `fullnode_helm_values`, this will configure the keys for fullnode, for example:
  ```
  module "fullnode" {
    # download Terraform module from aptos-labs/aptos-core repo
    source        = "github.com/aptos-labs/aptos-core.git//terraform/fullnode/gcp?ref=main"
    region        = "us-central1"  # Specify the region
    zone          = "c"            # Specify the zone suffix
    project       = "gcp-fullnode" # Specify your GCP project name
    era           = 1              # bump era number to wipe the chain
    image_tag     = "devnet"       # Specify the docker image tag to use

    fullnode_helm_values = {
      chain = {
        name = "devnet"
      }
      # create fullnode from this identity config, so it will always have same peer id and address
      fullnode_identity = {
        type = "from_config"
        key = "B8BD811A91D8E6E0C6DAC991009F189337378760B55F3AD05580235325615C74"
        peer_id = "ca3579457555c80fc7bb39964eb298c414fd60f81a2f8eedb0244ec07a26e575"
      }
    }
  }
  ```

3. Apply Terraform changes
  ```
  $ terraform apply
  ```

### Add upstream seed peers

You can add upstream seed peers to allow your node state sync from a specific fullnode, this is helpful when the fullnode not able to connect to the network due to congestion.

1. Get Upstream peer id info, you can either use the one we listed in the [fullnode tutorial](https://aptos.dev/tutorials/run-a-fullnode#add-upstream-seed-peers); or grab one from [Aptos Discord](http://discord.gg/aptoslabs), `#advertise-full-node` channel, those are the nodes hosted by our community.

2. Modify the `main.tf` to add seeds for devnet in `fullnode_helm_values`, this will configure the upstream seeds for fullnode, for example:
```
module "fullnode" {
    # download Terraform module from aptos-labs/aptos-core repo
    source        = "github.com/aptos-labs/aptos-core.git//terraform/fullnode/gcp?ref=main"
    region        = "us-central1"  # Specify the region
    zone          = "c"            # Specify the zone suffix
    project       = "gcp-fullnode" # Specify your GCP project name
    era           = 1              # bump era number to wipe the chain
    image_tag     = "dev_5b525691" # Specify the docker image tag to use

    fullnode_helm_values = {
      # add a list of peers as upstream
      aptos_chains = {
        devnet = {
          seeds = {
            "bb14af025d226288a3488b4433cf5cb54d6a710365a2d95ac6ffbd9b9198a86a" = {
            addresses = ["/dns4/pfn0.node.devnet.aptoslabs.com/tcp/6182/noise-ik/bb14af025d226288a3488b4433cf5cb54d6a710365a2d95ac6ffbd9b9198a86a/handshake/0"]
            role = "Upstream"
            },
            "7fe8523388084607cdf78ff40e3e717652173b436ae1809df4a5fcfc67f8fc61" = {
            addresses = ["/dns4/pfn1.node.devnet.aptoslabs.com/tcp/6182/noise-ik/7fe8523388084607cdf78ff40e3e717652173b436ae1809df4a5fcfc67f8fc61/handshake/0"]
            role = "Upstream"
            },
            "f6b135a59591677afc98168791551a0a476222516fdc55869d2b649c614d965b" = {
            addresses = ["/dns4/pfn2.node.devnet.aptoslabs.com/tcp/6182/noise-ik/f6b135a59591677afc98168791551a0a476222516fdc55869d2b649c614d965b/handshake/0"]
            role = "Upstream"
            }
          }
        }
      }
    }
  }
```

Make sure to update `aptos_chains.devnet` to corresponding networks if you're connecting to other networks.

3. Apply Terraform changes
  ```
  $ terraform apply
  ```

## Check Logging

To check the logs of the pod, using the following commands.

  ```
  # Get a list of the pods
  $ kubectl get pods -n aptos

  # Get logs of the pod
  $ kubectl logs <pod-name> -n aptos
  # for example:
  $ kubectl logs devnet0-aptos-fullnode-0 -n aptos
  ```


When using GKE, the logs of the cluster and pod will automatically show up in the Google Cloud console.  From the console menu, choose `Kubernetes Engine`.  From the side menu, choose `Workloads`.  You will see all the pods from the cluster listed.  


![GKE Workloads screenshot](../../../static/img/tutorial-gcp-logging1.png "GKE Workloads screenshot")


The `devnet0-aptos-fullnode` is the pod that is running the aptos fullnode container. Click on the pod to view details.  You will see some metrics and other details about the pod.


![GKE Workloads Pod screenshot](../../../static/img/tutorial-gcp-logging2.png "GKE Workloads Pod screenshot")


Click the `LOGS` tab to view the logs directly from the pod.  If there are errors in the pod, you will see them here.


![GKE Workloads Pod Logs screenshot](../../../static/img/tutorial-gcp-logging3.png "GKE Workloads Pod Logs screenshot")


Click the `open in new window` icon to view the logs in the Log Explorer.  This screen allows advanced searching in the logs.  


![GKE Workloads Pod Logs Explorer screenshot](../../../static/img/tutorial-gcp-logging4.png "GKE Workloads Pod Logs Explorer screenshot")



Other logging insights are available in the Logs Dashboard 


![GKE Workloads Pod Logs Dashboard screenshot](../../../static/img/tutorial-gcp-logging5.png "GKE Workloads Pod Logs Dashboard screenshot")



Additional [features](https://cloud.google.com/logging/docs) are available through [Cloud Logging](https://cloud.google.com/logging), including creating log-based metrics, logging sinks and log buckets. 



## Check Monitoring

Google cloud captures many metrics from the cluster and makes them easily viewable in the console.  From the console menu, choose `Kubernetes Engine`.  Click on the cluster that aptos is deployed to.  Click on the `Operations` link at the top right.  Click on the `Metrics` sub-tab to view specific cluster metrics.


![GKE Monitoring metrics screenshot](../../../static/img/tutorial-gcp-mon1.png "GKE Monitoring metrics screenshot")


Click the `View in Cloud Monitoring` link at the top to view the built-in GKE [dashboard](https://cloud.google.com/stackdriver/docs/solutions/gke/observing) for the cluster.  


![GKE Monitoring dashboard screenshot](../../../static/img/tutorial-gcp-mon2.png "GKE Monitoring dashboard screenshot")


Google Cloud [Monitoring](https://cloud.google.com/monitoring) has many other features to easily monitor the cluster and pods.  You can configure [uptime checks](https://cloud.google.com/monitoring/uptime-checks/introduction) for the services and configure [alerts](https://cloud.google.com/monitoring/alerts/using-alerting-ui) for when the metrics reach a certain [threshold](https://cloud.google.com/stackdriver/docs/solutions/slo-monitoring/sli-metrics/overview).  


## Troubleshooting

Common troubleshooting solutions.

### Terraform "Connection Refused" Error Message

When running terraform, the command errors out with a connection refused error message.

  ```
  Error: Get "http://localhost/api/v1/namespaces/aptos": dial tcp 127.0.0.1:80: connect: connection refused
  ```

This likely means that the state of the install is out of sync with the saved terraform state file located in the storage bucket.  (configured during `terraform init` statement).  This could happen if the cluster or other components were deleted outside of terraform.  Or if terraform had an error and did not finish.  Use the following commands to check the state.  Delete the state that is related to the error message.  You will likely need to run terraform destroy, clean up the environment, and run the terraform script again.  

  ```
  terraform state list

  terraform state rm <state>
  ```

### Fullnode "NoAvailablePeers" Error message

If your node cannot state sync, and the logs are showing "NoAvailablePeers", it's likely due to network congestion. You can try add some extra upstream peers for your fullnode to state sync from. See the guide [Add upstream seed peers](#add-upstream-seed-peers).

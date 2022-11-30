---
title: "Bootstrap a New Fullnode"
slug: "bootstrap-fullnode"
sidebar_position: 14
---

# Bootstrap a New Fullnode

Bootstrapping a new fullnode using [state-sync](../../guides/state-sync.md) might not be an optimal approach after the network has been running for a while; it can either take too much time, or it won't be able to fetch required data since most nodes have already pruned the ledger history. The most effective way for bootstrapping a new fullnode is to use data restore, which attempts to grab the latest snapshot.

## Restore data from a backup

Follow the guide below to build your Aptos database; then you can configure your fullnode binary to start with this restored data directory.

### Use source code or Docker

1. Install the [Amazon Web Services (AWS) CLI](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html).
    
1. Download the restore config file for using AWS Simple Storage Service (S3):    
   ```
   curl https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/data-restore/s3.yaml --output restore.yaml
    ```

1. Use the [Aptos CLI](../../cli-tools/aptos-cli-tool/use-aptos-cli.md) to bootstrap the database into a local directory. Replace the `--target-db-dir` with your data directory for the node if you're not using the default.

    ```yaml
    RUST_LOG=info aptos \
        node bootstrap-db-from-backup \
    --metadata-cache-dir ./mc \ 
    --config-path restore.yaml \
    --target-db-dir /opt/aptos/data/db
    ```

    Note that this command can run for a **few hours** to restore all the data. And if due to network instability or other reasons it’s interrupted, retry with the same command. Use the same `--metadata-cache-dir` parameter so you don’t need to download the metadata files again. In case a resumption keeps failing, delete the DB folder and try again.

1. Follow the rest of the [fullnode guide](fullnode-source-code-or-docker.md) to start the fullnode.

### Use Terraform/Helm

If you use our fullnode helm chart to deploy your node, we have a restore job built in there.

#### GCP fullnode

For Google Cloud Platform (GCP) fullnodes:

1. Modify the `main.tf` to add `restore` config in `fullnode_helm_values`; this will configure where the node should be restoring data from:

    ```
    module "fullnode" {
        # download Terraform module from aptos-labs/aptos-core repo
        source        = "github.com/aptos-labs/aptos-core.git//
        ...
        image_tag     = "testnet"      # Specify the docker image tag to use

        fullnode_helm_values = {
        chain = {
            name = "devnet"
        }
        ...

        restore = {
            config = {
                location = "gcs"
                restore_era = 4
                gcs = {
                bucket = "aptos-sherry-backup-8e146203"
                }
            }
        }
        }
    }
    ```

1. Apply Terraform changes:

        ```
        terraform apply
        ```

1. Take down the fullnode pod and make sure the fullnode pod has stopped running since we have to unmount the storage and mount it to the restore job. Once the pod stops, the storage PVC is automatically detached.
    
        ```
        kubectl scale sts $WORKSPACE0-aptos-fullnode -n aptos --replicas=0
        ```
1. Get the job manifest file from the original never-run job; modify it for restart. (The `create-restore-job.py` script is hosted in the `aptos-core` repo.):
        ```
        kubectl get job -n aptos \
          -l app.kubernetes.io/name=restore \
          -o json \
          | ~/aptos-core/scripts/create-restore-job.py \
          | kubectl apply -n aptos -f -
        ```

1. Check the job is running; it might take a few hours for the data to finish restoring:
        ```
        kubectl get pods
        ```
    
1. Once the job finishes, scale back the fullnode pod:
        ```
        kubectl scale sts $WORKSPACE0-aptos-fullnode -n aptos --replicas=1
        ```

#### AWS fullnode

For AWS or other clouds, modify the restore config in Terraform:

    ```
    restore = {
        config = {
            location = "s3"
            restore_era = 4
            s3 = {
              bucket = "aptos-ait3-data/backups"
            }
        }
    }
    ```

    Follow the GCP instructions above for the remaining steps.

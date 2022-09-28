---
title: "Bootstrap a new Fullnode"
slug: "bootstrap-fullnode"
sidebar_position: 14
---

# Bootstrap a new Fullnode

Bootstrapping a new fullnode using state-sync might not work well after the network is running for a while, it can either take super long time, or won't be able to fetch required data because most of nodes pruned the ledge history. The most effective way for bootstrapping a new fullnode is to use data restore.

## Restore data from a backup

We're hosting the **AIT3 blockchain** (other network coming soon) backup data on AWS S3 and Google Cloud GCS. Following the guide below to build your Aptos DB, then you can point your fullnode binary to start with this restored data directory.

### Using source code or docker

1. Install AWS CLI and download the restore config.
    - follow the instructions below to install the `aws` tool.
        - [https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html)
    
    - download the restore config file for using S3:
        
        ```
        curl https://raw.githubusercontent.com/aptos-labs/aptos-core/main/docker/compose/data-restore/s3.yaml \
        --output restore.yaml
        ```

2. Use  `aptos` CLI to bootstrap a DB into a local directory, replace the `--target-db-dir` with your data directory for the node if you're not using the default.

    ```yaml
    RUST_LOG=info aptos \
        node bootstrap-db-from-backup \
    --metadata-cache-dir ./mc \ 
    --config-path restore.yaml \
    --target-db-dir /opt/aptos/data/db
    ```

    Note that this command can run for **few hours** to restore all the data. And if due to network instability or other reasons it’s interrupted, retry with the same command. Notice to use the same `--metadata-cache-dir` param so you don’t need to download most metadata files again.  In case a resumption keeps failing, delete the DB folder and try again.

3. follow the rest of the [fullnode guide](fullnode-source-code-or-docker.md) to start the fullnode.

### Using Terraform/Helm

If you use our fullnode helm chart to deploy your node, we have a restore job build-in there.

- GCP fullnode

  1. Modify the `main.tf` to add `restore` config in `fullnode_helm_values`, this will configure where the node should be restoring data from:

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

    2. Apply Terraform changes

        ```
        terraform apply
        ```

    3. Take down the fullnode pod and make sure the fullnode pod has stopped running since we have to unmount the storage and mount it to the restore job. Once the pod stops, the storage pvc is automatically detached.
    
        ```
        kubectl scale sts $WORKSPACE0-aptos-fullnode -n aptos --replicas=0
        ```
    4. Get the job manifest file from the original never-run job, modify it for restart (`create-restore-job.py` script is hosted in `aptos-core` repo):
        ```
        kubectl get job -n aptos \
          -l app.kubernetes.io/name=restore \
          -o json \
          | ~/aptos-core/scripts/create-restore-job.py \
          | kubectl apply -n aptos -f -
        ```

    5. Check job is running, it might take few hours for the data to finish restore:
        ```
        kubectl get pods
        ```
    
    6. Once the job finishes, scale back the fullnode pod:
        ```
        kubectl scale sts $WORKSPACE0-aptos-fullnode -n aptos --replicas=1
        ```

- For AWS or other clouds, modify the restore config in Terraform:

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

    All other steps are the same with the GCP instruction above.

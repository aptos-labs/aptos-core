# Aptos DB Restore Tools and Public Backup Files

Since its launch in October 2022, the Aptos community has grown rapidly. As of May 2023, Aptos has 743G and 159G of data in testnet and mainnet, respectively. We expect the data to increase greatly as more transactions are submitted to the blockchain. Facing the large amount of data, we want to provide users with a way to achieve two goals:

- Quickly bootstrap a database to start a new or failed node
- Efficiently recover data from any specific period

Our DB restore tool enables you to use existing public backup files to restore the database on your local machine. These public backup files, which have cryptographic proof, are stored on both AWS and Google Cloud for public download. You can use these backup files, along with our restore tool, to restore your database to any historical range or to the latest version.

## **Restore DB using the Public Backup Files**

Our CLI supports restoring a database using backup files. It reads from the backup files and recreates the Aptos DB. We support two kinds of restore: (1) recreating a DB with minimal transaction history at a user-specified transaction version (or the latest version the backup has), and (2) restoring the database over a specific period. In addition to (1), this option also ensures that the recreated DB carries the ledger history of the user-designated version range.

**Bootstrap DB**

The command restores the database from the closest snapshot to the target version. This command can quickly restore a database to a target version, but it does not restore all the transaction history from the past.

Here is an example command (note: depending on whether you use AWS or Google Cloud, you may need to follow the instructions to install [aws cli](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html) or [gsutil](https://cloud.google.com/storage/docs/gsutil_install) as prerequisites):

```bash
# This requires the follow prerequsites
# 1. aws cli or google gsutil installed
# 2. aptos cli 1.0.14	
# This command syncs to version 500000000 with transaction history from 500000000 onwards
aptos node bootstrap-db \ 
    --target-version 500000000 \
    --command-adapter-config /path/to/s3-public.yaml \
    --target-db-dir /path/to/local/db
```

The s3-public.yaml ([link](https://github.com/aptos-labs/aptos-networks/blob/main/testnet/backups/s3-public.yaml)) used in the command specifies the location of the public backup files mentioned above, as well as the commands used by our backup and restore tool to interact with S3. Additionally, you can use the public Google backup files as shown here [link](https://github.com/aptos-labs/aptos-networks/blob/main/testnet/backups/gcs.yaml).

**Restoring a Database over a Specific Time Period**

We also support restoring the database to a previous period in the past. The command will restore all transaction history (events, write sets, key-value pairs, etc.) within the specified period, along with the state Merkle tree at the target version.

To use this command, you need to specify the `ledger-history-start-version` and `target-version` to indicate the period you are interested in.

```bash
# This requires aws cli installed and aptos cli 1.0.14	
# This command syncs to version 155000000 with transaction history from 150000000 onwards
aptos node bootstrap-db \ 
    --ledger-history-start-version 150000000 \
    --target-version 155000000 
    --command-adapter-config /path/to/s3-public.yaml \
    --target-db-dir /path/to/local/db
```

## **Public Backup Files**

The backup files are created by continuously querying a local full node and storing the backup data in either local files or remote storage (eg: google clound, aws, azure, etc).

The backup files consist of three types of data that can be used to reconstruct the blockchain DB:

- epoch_ending: It contains the ledger_info at the ending block of each epoch since the genesis. This data can be used to prove the epoch's provenance from the genesis and validator set of each epoch
- state_snapshot: It contains a snapshot of the blockchain's state Merkle tree (SMT) and key values at certain version.
- transaction: It contains the raw transaction metadata, payload, the executed outputs of the transaction after VM, as well as the cryptographic proof of the transaction in the ledger history.

Each type of data in the backup storage is organized in the following way. The metadata file in the metadata folder contains the range of each backup and the relative path to the backup folder. The backup contains a manifest file and all the actual chunked data files.

![image.png](./aptos-db-restore-images/image.png)

The Aptos Labs maintains a few publicly accessible database backups in Amazon S3 and Google Cloud Storage. You can access these data files as follows:

|  | AWS Backup Data | Google Cloud Backup Data  |
| --- | --- | --- |
| Testnet | https://github.com/aptos-labs/aptos-networks/blob/main/testnet/backups/s3-public.yaml | https://github.com/aptos-labs/aptos-networks/blob/main/testnet/backups/gcs.yaml |
| Mainnet | https://github.com/aptos-labs/aptos-networks/blob/main/mainnet/backups/s3-public.yaml | https://github.com/aptos-labs/aptos-networks/blob/main/mainnet/backups/gcs.yaml |
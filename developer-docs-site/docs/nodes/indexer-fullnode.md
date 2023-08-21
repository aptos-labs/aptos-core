---
title: "Run an Indexer"
slug: "indexer-fullnode"
---

# Run an Aptos Indexer

This document describes how to operate an indexer fullnode on the Aptos network. To understand and ingest the indexer data in your app, see [Use the Aptos Indexer](../integration/indexing.md).

:::danger On macOS with Apple silicon only
The below installation steps are verified only on macOS with Apple silicon. They might require minor tweaking when running on other builds.
:::

## Summary

To run an indexer fullnode, these are the steps in summary:

1. Make sure that you have all the required tools and packages described below in this document.
1. Follow the instructions to [set up a public fullnode](./full-node/fullnode-source-code-or-docker.md) but do not start the fullnode yet. 
1. Edit the `fullnode.yaml` as described below in this document.
1. Run the indexer fullnode per the instructions below.

## Prerequisites

Install the packages below. Note, you may have already installed many of these while [preparing your development environment](../guides/building-from-source). You can confirm by running `which command-name` and ensuring the package appears in the output (although `libpq` will not be returned even when installed).

> Important: If you are on macOS, you will need to [install Docker following the official guidance](https://docs.docker.com/desktop/install/mac-install/) rather than `brew`.

For an Aptos indexer fullnode, install these packages:

  - [`brew`](https://brew.sh/) - `/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"` Run the commands emitted in the output to add the command to your path and install any dependencies
  - [`cargo` Rust package manager](https://www.rust-lang.org/tools/install) - `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
  - [`docker`](https://docs.docker.com/get-docker/) - `brew install docker`
  - [libpq Postgres C API library containing the `pg_ctl` command](https://formulae.brew.sh/formula/libpq) - `brew install libpq`
    Make sure to perform all export commands after the installation.
  -  [`postgres` PostgreSQL server](https://www.postgresql.org/) - `brew install postgresql`
  - [`diesel`](https://diesel.rs/) - `brew install diesel`

## Set up the database

1. Start the PostgreSQL server: 
   `brew services start postgresql`
1. Ensure you can run `psql postgres` and then exit the prompt by entering: `\q`
1. Create a PostgreSQL user `postgres` with the `createuser` command (find it with `which`):
   ```bash
   /path/to/createuser -s postgres
   ```
1. Clone `aptos-core` repository if you have not already:
    ```bash
    git clone https://github.com/aptos-labs/aptos-core.git
    ```
1. Navigate (or `cd`) into `aptos-core/crates/indexer` directory.
1.  Create the database schema:
    ```bash
    diesel migration run --database-url postgresql://localhost/postgres
    ```
    This will create a database schema with the subdirectory `migrations` located in this `aptos-core/crates/indexer` directory. If for some reason this database is already in use, try a different database. For example: `DATABASE_URL=postgres://postgres@localhost:5432/indexer_v2 diesel database reset`

## Start the fullnode indexer

1. Follow the instructions to set up a [public fullnode](./full-node/fullnode-source-code-or-docker.md) and prepare the setup, but **do not** yet start the indexer (with `cargo run` or `docker run`).
1. Pull the latest indexer Docker image with:
    ```bash
    docker pull aptoslabs/validator:nightly_indexer
    ```
1. Edit the `./fullnode.yaml` and add the following configuration:
    ```yaml
    storage:
        enable_indexer: true
        # This is to avoid the node being pruned
        storage_pruner_config:
            ledger_pruner_config:
                enable: false
    
    indexer:
        enabled: true
        postgres_uri: "postgres://postgres@localhost:5432/postgres"
        processor: "default_processor"
        check_chain_id: true
        emit_every: 500
    ```

:::tip Bootstap the fullnode
Instead of syncing your indexer fullnode from genesis, which may take a long period of time, you can choose to bootstrap your fullnode using backup data before starting it. To do so, follow the instructions to [restore from a backup](../nodes/full-node/aptos-db-restore.md).

Note: indexers cannot be bootstrapped using [a snapshot](../nodes/full-node/bootstrap-fullnode.md) or [fast sync](../guides/state-sync.md#fast-syncing).
:::

1. Run the indexer fullnode with either `cargo run` or `docker run` depending upon your setup. Remember to supply the arguments you need for your specific node:
    ```bash
    docker run -p 8080:8080 \
      -p 9101:9101 -p 6180:6180 \
      -v $(pwd):/opt/aptos/etc -v $(pwd)/data:/opt/aptos/data \
      --workdir /opt/aptos/etc \
      --name=aptos-fullnode aptoslabs/validator:nightly_indexer aptos-node \
      -f /opt/aptos/etc/fullnode.yaml
    ```
    or:
    ```bash
    cargo run -p aptos-node --features "indexer" --release -- -f ./fullnode.yaml
    ```

## Restart the indexer

To restart the PostgreSQL server:

1. [shut down the server](https://www.postgresql.org/docs/8.1/postmaster-shutdown.html) by searching for the `postmaster` process and killing it:
    ```bash
    ps -ef | grep -i postmaster
    ```

1. Copy the process ID (PID) for the process and pass it to the following command to shut it down:
    ```bash
    kill -INT PID
    ```

1. Restart the PostgreSQL server with:
    ```bash
    brew services restart postgresql@14
    ```

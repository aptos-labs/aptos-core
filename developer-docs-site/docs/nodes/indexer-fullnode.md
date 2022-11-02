---
title: "Aptos Indexer Fullnode"
slug: "indexer-fullnode"
---

# Aptos Indexer Fullnode

This document describes how to run an indexer fullnode on the Aptos network. See the [Indexing](/guides/indexing.md) guide that describes the indexing concept and provides the available options for the indexing service on the Aptos blockchain.

:::danger On macOS with Apple silicon only
The below installation steps are verified only on macOS with Apple silicon.
:::

## Summary

To run an indexer fullnode, these are the steps in summary:

1. Make sure that you have all the required tools and packages described below in this document.
1. Follow the instructions to [set up a public fullnode](full-node/fullnode-source-code-or-docker/) but do not start the fullnode yet. 
1. Edit the `fullnode.yaml` as described below in this document.
1. Run the indexer fullnode per the instructions below.

## Prerequisites

Install the packages below. Note, you may have already installed many of these while [preparing your development environment](../guides/getting-started.md#prepare-development-environment). You can confirm by running `which command-name` and ensuring the package appears in the output (although `libpq` will not be returned even when installed).

For an Aptos indexer fullnode, install these packages:

  - [`brew`](https://brew.sh/) - `/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"`
  - [`cargo` Rust package manager](https://www.rust-lang.org/tools/install) - `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
  - [`libpq` Postgres C API library](https://formulae.brew.sh/formula/libpq) - `brew install libpq`
    Make sure to perform all export commands after the installation.
  -  [`postgres` PostgreSQL server](https://www.postgresql.org/) - `brew install postgresql`
  - [`diesel`](https://diesel.rs/) - `brew install diesel`


## Startup

1. Start the PostgreSQL server: 
   - macOS: `brew services start postgresql`
   - Linux: `pg_ctl -D /opt/homebrew/var/postgres start`
1. Create a PostgreSQL user `postgres`:
   ```bash
   /opt/homebrew/bin/createuser -s postgres
   ```
1. Ensure you can run `psql postgres` and then exit the prompt by entering: `\q`
1. Install the Diesel CLI: 
    ```bash
    cargo install diesel_cli --no-default-features --features postgres
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

8. Follow the [method #1 of setting up a public fullnode by using the source code](full-node/fullnode-source-code-or-docker/#method-1-building-and-running-from-source) and prepare the setup, but **do not run** the `cargo run -p aptos-node --release -- -f ./fullnode.yaml` command yet. **Docker is not yet supported for the indexer fullnode.**
9. Edit the `./fullnode.yaml` and add the following configuration:
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

10. Run the indexer fullnode with:
    ```bash
    cargo run --bin aptos-node --features "indexer"  -- --config </path/to/fullnode.yaml>
    ```

## Restart

To restart the PostgreSQL server, first [shut down the server](https://www.postgresql.org/docs/8.1/postmaster-shutdown.html) by searching for the `postmaster` process and killing it:
```bash
ps -ef | grep -i postmaster
```

Copy the process ID (PID) for the process and pass it to the following command to shut it down:
```bash
kill -INT PID
```
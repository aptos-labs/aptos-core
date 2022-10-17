---
title: "Indexer Fullnode"
slug: "indexer-fullnode"
---

# Indexer Fullnode

This document describes how to run an indexer fullnode on the Aptos network. See [Indexing](/guides/indexing.md) guide that describes the basic indexing concept and the available options for indexing service on the Aptos blockchain.

:::danger On macOS with Apple silicon only
The below installation steps are verified only on macOS with Apple silicon.
:::

## Summary

To run an indexer fullnode, these are the steps in summary:

1. Make sure that you have all the required tools and packages described below in this document.
2. Follow the guide [Fullnode Using Aptos Source or Docker](full-node/fullnode-source-code-or-docker.md) and prepare the setup, but **do not run** the `cargo run -p aptos-node --release -- -f ./fullnode.yaml` command yet. 
3. Edit the `fullnode.yaml` as described below in this document.
4. Run the indexer fullnode by executing the command described below in this document.

### Required tools and packages

- Install [Brew](https://brew.sh/).
- Install Cargo and Rust via [Install Rust](https://www.rust-lang.org/tools/install).
- Install [libpq](https://formulae.brew.sh/formula/libpq). This is a Postgres C API library. Make sure to perform all export commands after the installation.
  - macOS: `brew install libpq`
  - Linux: `apt install libpq`
- Install [PostgreSQL](https://www.postgresql.org/):
  - macOS: `brew install postgres`
  - Linux: `apt install postgres`
- Install [Diesel](https://diesel.rs/):
`cargo install diesel_cli --no-default-features --features postgres`.

### Setup

1. Start the PostgreSQL server: 
   - macOS: `brew services start postgresql`
   - Linux: `pg_ctl -D /opt/homebrew/var/postgres start`
2. Run the following command to create a PostgreSQL user `postgres` (macOS commmand example below):
   ```bash
   /opt/homebrew/bin/createuser -s postgres
   ```
3. Ensure you are able to do: `psql postgres `.
4. Install the Diesel CLI: 
    ```bash
    cargo install diesel_cli --no-default-features --features postgres
    ```
5. Clone `aptos-core` repo:
    ```bash
    git clone https://github.com/aptos-labs/aptos-core.git
    ```
6. `cd` into `aptos-core/crates/indexer` directory.
7.  Run the command:
    ```bash
    diesel migration run --database-url postgresql://localhost/postgres
    ```
    This will create a database schema with the subdirectory `migrations` located in this `aptos-core/crates/indexer` directory.
    - If for some reason this database is already being used, try a different database. For example: `DATABASE_URL=postgres://postgres@localhost:5432/indexer_v2 diesel database reset`

8. Follow the guide [Fullnode Using Aptos Source or Docker](full-node/fullnode-source-code-or-docker.md) and prepare the setup, but **do not run** the `cargo run -p aptos-node --release -- -f ./fullnode.yaml` command yet. 
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
    cargo run --bin aptos-node --features "indexer"  -- --config </path/to/fullnode.yaml>`



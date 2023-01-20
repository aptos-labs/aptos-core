---
title: "Run Validator Localnet with Indexer"
slug: "localnet-indexer"
---

# Run a Validator Localnet with Indexer

This document describes how to run an Aptos indexer locally. See [Aptos Indexer Fullnode](../indexer-fullnode.md) to run an indexer fullnode on the Aptos network.

See the [Indexing](../../guides/indexing.md) guide that describes the indexing concept and provides the available options for the indexing service on the Aptos blockchain.

## Set up

1. [Prepare your development environment](../../guides/getting-started.md#prepare-development-environment) if you haven't already.

2. [Install the same prerequisites](../indexer-fullnode.md#prerequisites) as for the indexer on-chain except [Docker](https://docs.docker.com/get-docker/), which is not needed locally. 

3. [Clone the Aptos repository `main` branch and set up the database](../indexer-fullnode.md#set-up-database) as for the indexer on-chain.

4. Create an `indexer_test` database in Postgres.

TODO: How does this reconcilve with? `diesel migration run --database-url postgresql://localhost/postgres` on?
https://aptos.dev/nodes/indexer-fullnode/#set-up-database

5. Install a database management tool, ex. [DBeaver](https://dbeaver.io/).

## Start

1. Ensure you are on the [`main` branch of aptos-core](https://github.com/aptos-labs/aptos-core/tree/main).

2. Start the localnet validator to ensure it is working with:

```shell
cargo run -p aptos -- node run-local-testnet --with-faucet
```

3. Optionally, if you encounter issues, reset localnet with:

```shell
cargo run -p aptos -- node run-local-testnet --with-faucet --force-restart --assume-yes
```

4. Take note of the paths in the output.

## Configure

1. [Shut down the PostgreSQL server](https://www.postgresql.org/docs/8.1/postmaster-shutdown.html).

2. Find the `node.yaml` file in the `Test dir` directory from the paths above.

TODO: Make this mor clear. Ex. Does this `testnet` directory exist in the `main` branch.

3. Edit the `node.yaml` file to change:

  * `enable_indexer` to `true`, like so:
  ```shell
  `enable_indexer`: true
  ```
  * replace the `indexer` section with the following, updating postgres_uri with the URL to your own test database:
  ```shell
  indexer:
    enabled: true
    check_chain_id: true
    emit_every: 1000
    postgres_uri: "postgres://postgres@localhost:5432/indexer_test"
    processor: "default_processor"
    starting_version: 0
    fetch_tasks: 10
    processor_tasks: 10
  ```

  ## Run

  1. Rebuild and run validator with indexer feature:

  ```shell
  RUST_LOG=info cargo run -p aptos --features indexer -- node run-local-testnet --with-faucet
  ```

  2. Validate your setup by connecting to the local database and confirming all tables are created, for instance using DBeaver.

  TODO: List the tables or include the image.

  Although all of the tables will be created, only a few tables will actually have content depending on which processor you’re running, For examples, `default_processor` populates events and transactions, while `token_processor` populates NFT tables and include collections and tokens.

  See the [Indexing](/guides/indexing.md) guide for use.
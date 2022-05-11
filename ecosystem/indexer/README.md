# Aptos Indexer

> Tails the blockchain's transactions and pushes them into a postgres DB

Tails the node utilizing the rest interface/client, and maintains state for each registered `TransactionProcessor`. On
startup, by default, will retry any previously errored versions for each registered processor.

When developing your own, ensure each `TransactionProcessor` is idempotent, and being called with the same input won't
result in an error if some or all of the processing had previously been completed.

Example invocation:

```bash
cargo run -- --pg-uri "postgresql://localhost/postgres" --node-url "https://fullnode.devnet.aptoslabs.com" --emit-every 25 --batch-size 100
```

Try running the indexer with `--help` to get more details

## Requirements

- [Rust](https://rustup.rs/)
- [Diesel](https://diesel.rs/)
- [Postgres](https://www.postgresql.org/)

# Local Development

> *Notes*:
> - Diesel uses the `DATABASE_URL` env var to connect to the database.
> - Diesel CLI can be installed via cargo, e.g., `cargo install diesel_cli --no-default-features --features postgres`.
> - `diesel migration run` sets up the database and runs all available migrations.
> - Aptos tests use the `INDEXER_DATABASE_URL` env var. It needs to be set for the relevant tests to run.
> - Postgres can be [installed and run via brew](https://wiki.postgresql.org/wiki/Homebrew).

## Adding new tables / Updating tables with Diesel

* `diesel migration generate <your_migration_name>` generates a new folder containing `up.sql + down.sql` for your
  migration
* `diesel migration run` to apply the missing migrations. This will re-generate `schema.rs` as required.
* `diesel migration redo` to rollback and apply the last migration
* `diesel database reset` drops the existing database and reruns all the migrations
* You can find more information in the [Diesel](https://diesel.rs/) documentation

# General Flow

The `Tailer` is the central glue that holds all the other components together. It's responsible for the following:

1. Maintaining processor state. The `Tailer` keeps a record of the `Result` of each `TransactionProcessor`'s output for
   each transaction version (eg: transaction). If a `TransactionProcessor` returns a `Result::Err()` for a transaction,
   the `Tailer` will mark that version as failed in the database (along with the stringified error text) and continue
   on.
2. Retry failed versions for each `TransactionProcessor`. By default, when a `Tailer` is started, it will re-fetch the
   versions for all `TransactionProcessor` which have failed, and attempt to re-process them. The `Result::Ok`
   /`Result::Err` returned from the `TransactionProcessor::process_version` replace the state in the DB for the
   given `TransactionProcessor`/version combination.
3. Piping new transactions from the `Fetcher` into each `TransactionProcessor` that was registered to it.
   Each `TransactionProcessor` gets its own copy, in its own `tokio::Task`, for each version. These are done in batches,
   the size of which is specifiable via `--batch-size`. For other tunable parameters, try `cargo run -- --help`.

The `Fetcher` is responsible for fetching transactions from a node in one of two ways:

1. One at a time (used by the `Tailer` when retrying previously errored transactions).
2. In bulk, with an internal buffer. Although the `Tailer` only fetches one transaction at a time from the `Fetcher`,
   internally the `Fetcher` will fetch from the `/transactions` endpoint, which returns potentially hundreds of
   transactions at a time. This is much more efficient than making hundreds of individual HTTP calls. In the future,
   when there is a streaming Node API, that would be the optimal source of transactions.

All the above comes free 'out of the box'. The `TransactionProcessor` is where everything becomes useful for those
writing their own indexers. The trait only has one main method that needs to be implemented: `process_transaction`. You
can do anything you want in a `TransactionProcessor` - write data to Postgres tables like the `DefaultProcessor` does,
make restful HTTP calls to some other service, submit its own transactions to the chain: anything at all. There is just
one note: *transaction processing is guaranteed at least once*. It's possible for a given `TransactionProcessor` to
receive the same transaction more than once: and so your implementation must be idempotent.

To implement your own `TransactionProcessor`, check out the documentation and source code
here: [`./src/indexer/transaction_processor.rs`](./src/indexer/transaction_processor.rs).

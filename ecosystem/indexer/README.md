# Aptos Indexer
> Tails the blockchain's transactions and pushes them into a postgres DB

Tails the node utilizing the rest interface/client, and maintains state for each registered `TransactionProcessor`.
On startup, by default, will retry any previously errored versions for each registered processor.

When developing your own, ensure each `TransactionProcessor` is idempotent, and being called with the same input won't result in an error if
some or all of the processing had previously been completed.


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
> - Diesel uses the `DATABASE_URL` env var to connect to the database
> - `diesel migration run` sets up the database and run all available migrations.

## Adding new tables / Updating tables with Diesel
* `diesel migration generate <your_migration_name>` generates a new folder containing `up.sql + down.sql` for your migration
* `diesel migration run` to apply the missing migrations,
* `diesel migration redo` to rollback and apply the last migration
* `diesel database reset` drops the existing database and reruns all the migration
* You can find more information in the [Diesel](https://diesel.rs/) documentation

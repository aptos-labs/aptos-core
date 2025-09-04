# Velor Indexer

> Tails the blockchain's transactions and pushes them into a postgres DB

A fullnode can run an indexer with the proper configs. If enabled, the indexer will tail
transactions in the fullnode with business logic from  each registered `TransactionProcessor`. On
startup, by default, will restart from the first gap (e.g. version 5 if versions succeeded are 0, 1, 2, 3, 4, 6). 

Each `TransactionProcessor` will need to be run in a separate fullnode. Please note that it may be difficult to run several transaction processors simultaneously in a single machine due to port conflicts. 

When developing your own, ensure each `TransactionProcessor` is idempotent, and being called with the same input won't result in an error if some or all of the processing had previously been completed.

## Requirements

- [Rust](https://rustup.rs/)
- [Diesel](https://diesel.rs/)
- [Postgres](https://www.postgresql.org/)

# Local Development

## Installation Guide (for apple sillicon)
### Postgres 
1. `brew install libpq` ([this is a postgres C API library](https://formulae.brew.sh/formula/libpq)). Also perform all export commands post-installation
2. `brew install postgres`
3. `pg_ctl -D /opt/homebrew/var/postgres start` or `brew services start postgresql`
4. `/opt/homebrew/bin/createuser -s postgres`
5. Ensure you're able to do: `psql postgres`
6. `cargo install diesel_cli --no-default-features --features postgres`
7. Make sure that you're in the indexer folder (run `cd crates/indexer` from base directory), run `diesel migration run --database-url postgresql://localhost/postgres`
   a. If for some reason this database is already being used, try a different db. e.g.
      `DATABASE_URL=postgres://postgres@localhost:5432/indexer_v2 diesel database reset`

### Installing fullnode
Please follow standard fullnode installation guide on velor.dev (https://velor.dev/nodes/full-node/fullnode-source-code-or-docker)

### Running indexer
```bash
cargo run -p velor-node --features "indexer" --release -- -f <some_path>/fullnode.yaml
```
   * Example fullnode.yaml modification
      ```
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

### Optional PgAdmin4
1. Complete Installation Guide above
2. `brew install --cask pgadmin4`
3. Open PgAdmin4
4. Create a master password
5. Right Click Servers > `Register` > `Server`
6. Enter the information in the registration Modal:

```yaml
General:
Name: Indexer

Connection:
Hostname / Address: 127.0.0.1
Port: 5432
Maintenance Database: postgres
Username: postgres
```
7. Save

> *Notes*:
> - Diesel uses the `DATABASE_URL` env var to connect to the database, or the `--database-url` argument.
> - Diesel CLI can be installed via cargo, e.g., `cargo install diesel_cli --no-default-features --features postgres`.
> - `diesel migration run` sets up the database and runs all available migrations.
> - Velor tests use the `INDEXER_DATABASE_URL` env var. It needs to be set for the relevant tests to run.
> - Postgres can be [installed and run via brew](https://wiki.postgresql.org/wiki/Homebrew).

## Adding new tables / Updating tables with Diesel

* `diesel migration generate <your_migration_name>` generates a new folder containing `up.sql + down.sql` for your
  migration
* `diesel migration run` to apply the missing migrations. This will re-generate `schema.rs` as required.
* `diesel migration redo` to rollback and apply the last migration
* `diesel database reset` drops the existing database and reruns all the migrations
* You can find more information in the [Diesel](https://diesel.rs/) documentation

### Miscellaneous
1. If you run into
```bash
  = note: ld: library not found for -lpq
          clang: error: linker command failed with exit code 1 (use -v to see invocation)
```

first make sure you have `postgres` and `libpq` installed via `homebrew`, see installation guide above for more details.
This is complaining about the `libpq` library, a postgres C API library which diesel needs to run, more on this issue [here](https://github.com/diesel-rs/diesel/issues/2612)
2. [Postgresql Mac M1 installation guide](https://gist.github.com/phortuin/2fe698b6c741fd84357cec84219c6667)
3. Stop postgresql: `brew services stop postgresql`
4. Since homebrew installs packages in `/opt/homebrew/bin/postgres`, your `pg_hba.conf` should be in `/opt/homebrew/var/postgres/` for Apple Silicon users
5. Likewise, your `postmaster.pid` should be retrievable via `cat /opt/homebrew/var/postgres/postmaster.pid`. Sometimes you may have to remove this if you are unable to start your server, an error like:
```bash
waiting for server to start....2022-05-17 12:49:42.735 PDT [4936] FATAL:  lock file "postmaster.pid" already exists
2022-05-17 12:49:42.735 PDT [4936] HINT:  Is another postmaster (PID 4885) running in data directory "/opt/homebrew/var/postgres"?
 stopped waiting
pg_ctl: could not start server
```
then run `brew services restart postgresql`
6. Alias for starting testnet (put this in `~/.zshrc`)
```bash
alias testnet="cd ~/Desktop/velor-core; CARGO_NET_GIT_FETCH_WITH_CLI=true cargo run -p velor-node -- --test"
```
Then run `source ~/.zshrc`, and start testnet by running `testnet` in your terminal

# Aptos Indexer

Will fix readme but just remember command for now
```
APTOS_POSTGRES_CONNECTION_STRING_VAR=postgresql://localhost:5432/indexer_v2 APTOS_DATASTREAM_SERVICE_ADDRESS_VAR=http://localhost:50051 STARTING_VERSION=0 cargo run
```


> Tails the blockchain's transactions and pushes them into a postgres DB

A fullnode can run an indexer with the proper configurations. If enabled, the indexer will tail
transactions in the fullnode with business logic from  each registered `TransactionProcessor`. On
startup, by default, the indexer will restart from the first gap (e.g. version 5 if versions succeeded are 0, 1, 2, 3, 4, 6). 

Each `TransactionProcessor` will need to be run in a separate fullnode. Please note that it may be difficult to run several transaction processors simultaneously in a single machine due to port conflicts. 

When developing your own, ensure each `TransactionProcessor` is idempotent, and being called with the same input will not result in an error if some or all of the processing had previously been completed.

## Requirements

- [Rust](https://rustup.rs/)
- [Diesel](https://diesel.rs/)
- [Postgres](https://www.postgresql.org/)

# Local Development

## Installation Guide (for apple sillicon)
### Create a postgres database 
1. Install the `libpq` package ([a postgres C API library](https://formulae.brew.sh/formula/libpq)): `brew install libpq`. Also, perform all export commands post-installation.
2. Install the `postgres` package: `brew install postgres`
3. Start the PostgreSQL server: `pg_ctl -D /opt/homebrew/var/postgres start` or `brew services start postgresql`
4. Create a PostgreSQL user called `postgres`:  
   `/opt/homebrew/bin/createuser -s postgres`
5. Ensure you can run this command: `psql postgres`
6. Install the `diesel_cli` package with no default features enabled except for `postgres`:  
`cargo install diesel_cli --no-default-features --features postgres`
7. Ensure you are in the indexer folder. Run this command from the base directory: `cd crates/indexer` 
8. Create the database schema: `diesel migration run --database-url postgresql://localhost/postgres`  
   a. If for some reason this database is already being used, try a different db. For example:
      `DATABASE_URL=postgres://postgres@localhost:5432/indexer_v2 diesel database reset`

### Setup a fullnode
Please follow the standard [fullnode installation guide](https://aptos.dev/nodes/full-node/fullnode-source-code-or-docker) on aptos.dev, but do not start the fullnode until you finish the instructions for [Running indexer](#start-the-indexer-fullnode).

### Start the indexer fullnode
1. Modify fullnode.yaml with your details. Example fullnode.yaml modification:
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
2. Run the indexer fullnode:
    ```bash
    cargo run -p aptos-node --features "indexer" --release -- -f <some_path>/fullnode.yaml
    ```

### Optional pgadmin4
1. Complete the [steps](#installation-guide-for-apple-sillicon) above.
2. Install pgadmin4: `brew install --cask pgadmin4`
3. Open PgAdmin4.
4. Create a master password.
5. Right Click Servers > `Register` > `Server`
6. Enter the following information in the registration Modal:  
    ```yaml
    General:
    Name: Indexer

    Connection:
    Hostname / Address: 127.0.0.1
    Port: 5432
    Maintenance Database: postgres
    Username: postgres
    ```
7. Save.

> *Notes*:
> - Diesel uses the `DATABASE_URL` env var to connect to the database, or the `--database-url` argument.
> - Diesel CLI can be installed via cargo. For example: `cargo install diesel_cli --no-default-features --features postgres`
> - `diesel migration run` sets up the database and runs all available migrations.
> - Aptos tests use the `INDEXER_DATABASE_URL` env var. It needs to be set for the relevant tests to run.
> - Postgres can be [installed and run via brew](https://wiki.postgresql.org/wiki/Homebrew).

## Adding new tables / updating tables with Diesel

* `diesel migration generate <your_migration_name>` generates a new folder containing `up.sql + down.sql` for your
  migration
* `diesel migration run` to apply the missing migrations. This will re-generate `schema.rs` as required.
* `diesel migration redo` to rollback and apply the last migration
* `diesel database reset` drops the existing database and reruns all the migrations
* You can find more information in the [Diesel](https://diesel.rs/) documentation.

### Miscellaneous
1. If you run into
   ```bash
     = note: ld: library not found for -lpq
             clang: error: linker command failed with exit code 1 (use -v to see invocation)
   ```
first make sure you have `postgres` and `libpq` installed via `homebrew`, see installation guide above for more details.
This error is regarding the `libpq` library, a postgres C API library which diesel needs to run, more on this issue [here](https://github.com/diesel-rs/diesel/issues/2612).  

2. [PostgreSQL Mac M1 installation guide](https://gist.github.com/phortuin/2fe698b6c741fd84357cec84219c6667).
3. To stop the PostgreSQL server: `brew services stop postgresql`
4. Since homebrew installs packages in `/opt/homebrew/bin/postgres`, your `pg_hba.conf` should be in `/opt/homebrew/var/postgres/` for Apple Silicon users.
5. Likewise, your `postmaster.pid` should be retrievable via `cat /opt/homebrew/var/postgres/postmaster.pid`. Sometimes you may have to remove this if you are unable to start your server due to an error like:
   ```bash
   waiting for server to start....2022-05-17 12:49:42.735 PDT [4936] FATAL:  lock file "postmaster.pid" already exists
   2022-05-17 12:49:42.735 PDT [4936] HINT:  Is another postmaster (PID 4885) running in data directory "/opt/homebrew/var/postgres"?
    stopped waiting
   pg_ctl: could not start server
   ```
   Afterward, run `brew services restart postgresql`  
6. Alias for starting testnet (put this in `~/.zshrc`)
```bash
alias testnet="cd ~/Desktop/aptos-core; CARGO_NET_GIT_FETCH_WITH_CLI=true cargo run -p aptos-node -- --test"
```
Then run `source ~/.zshrc`, and start testnet by running `testnet` in your terminal

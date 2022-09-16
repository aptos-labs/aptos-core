> :warning: DO* NOT USE THIS, IT'S NOT READY. Go to crates/indexer for a working indexer
# Aptos Indexer

> Indexes Aptos Blockchain via Substreams connected to Aptos Firehose (https://github.com/streamingfast/firehose-aptos). 

## Requirements

- [Rust](https://rustup.rs/)
- [Diesel](https://diesel.rs/)
- [Postgres](https://www.postgresql.org/)
- [substreams]

# Local Development

## Compiling Protobufs

#### Install `protoc`

protoc is a Protocol Buffer compiler. It is needed to generate code for Rust and other languages, out of the protobuf definitions you will create or get through third-party Substreams packages.

There are multiple ways on how to do it. Here is the official documentation of [protocol buffer compiler](https://grpc.io/docs/protoc-installation/).

#### Install plugins

This tool helps you render Rust structures out of protobuf definitions, for use in your Substreams modules. It is called by protoc following their plugin system.
Install it with:

```bash
  cargo install --version 0.2.0 protoc-gen-prost
  cargo install --version 0.1.3 protoc-gen-tonic
```

> If you forget to install `protoc`, when generating the definitions, you might see error about `cmake` not defined, this is a fallback when `protoc` is not found.

#### Install `buf`

[https://buf.build](https://buf.build) is a tool used to simplify the generation of typed structures in any language. It invokes `protoc` and simplifies a good number of things. Substreams packages are compatible with [buf Images](https://docs.buf.build/reference/images).

See the [installation instructions here](https://docs.buf.build/installation).

#### generating
run `buf generate --include-imports`

### Run Firehose
TBD

### Installation Guide (for apple silicon)
1. `brew install libpq` ([this is a postgres C API library](https://formulae.brew.sh/formula/libpq)). Also perform all export commands post-installation
2. `brew install postgres`
3. `pg_ctl -D /opt/homebrew/var/postgres start` or `brew services start postgresql`
4. `/opt/homebrew/bin/createuser -s postgres`
5. Ensure you're able to do: `psql postgres`
6. `cargo install diesel_cli --no-default-features --features postgres`
7. `DATABASE_URL=postgresql://localhost/postgres diesel migration run`
8. Start indexer
```bash
DATABASE_URL=postgres://postgres@localhost:5432/postgres  cargo run -- --endpoint-url http://localhost:18015 --package-file ../aptos-substreams/aptos-substreams-v0.0.1.spkg --module-name block_to_block_output
```

### Creating/editing substreams
TBD

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
> - Diesel uses the `DATABASE_URL` env var to connect to the database.
> - Diesel CLI can be installed via cargo, e.g., `cargo install diesel_cli --no-default-features --features postgres`.
> - `diesel migration run` sets up the database and runs all available migrations.
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
alias testnet="cd ~/Desktop/aptos-core; CARGO_NET_GIT_FETCH_WITH_CLI=true cargo run -p aptos-node -- --test"
```
Then run `source ~/.zshrc`, and start testnet by running `testnet` in your terminal

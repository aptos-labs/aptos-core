# Experimental

## Step 0: Install Dependencies

- Install Diem dependencies including Rust, Clang, etc, by running the following script in `diem` root directory:
```
./scripts/dev_setup.sh
```

## Usage

Please run `shuffle help`.

## Sample Usage

From the `diem/` base repo directory:

1. `cargo run -p shuffle -- new /tmp/helloblockchain` creates a new shuffle project
1. `cargo run -p shuffle -- node /tmp/helloblockchain` runs node based on project, perform in a different terminal
1. `cargo run -p shuffle -- account /tmp/helloblockchain shuffle/cli/new_account.key` creates an account onchain
1. `cargo run -p shuffle -- deploy /tmp/helloblockchain shuffle/cli/new_account.key` publishes a module to the created node
1. `cargo run -p shuffle -- console /tmp/helloblockchain shuffle/cli/new_account.key` enters a typescript REPL with helpers loaded
1. `cargo run -p shuffle -- test /tmp/helloblockchain` runs end to end tests

## Development

Note that for local development, `shuffle` is replaced with `cargo run -p shuffle --`:

```bash
shuffle new /tmp/helloblockchain # is replaced by
cargo run -p shuffle -- new /tmp/helloblockchain
```

## Testing

```
cd shuffle/cli
cargo test
```

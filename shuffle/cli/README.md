# Experimental

## Step 0: Install Dependencies

- Install Diem dependencies including Rust, Clang, etc, by running the following script in `diem` root directory:
```
./scripts/dev_setup.sh
```

## Usage

Please run `cargo run -p shuffle -- help`. Ultimately, this will become
`shuffle help`.

## Sample Usage

1. `cargo run -p shuffle -- new /tmp/helloblockchain` creates a new shuffle project
2. `cargo run -p shuffle -- node /tmp/helloblockchain` runs node based on project
3. `cargo run -p shuffle -- deploy /tmp/helloblockchain cli/new_account.key` publishes a module to the created node
3. `cargo run -p shuffle -- account /tmp/helloblockchain cli/new_account.key` creates an account onchain

## Development

Note that for local development, `shuffle` is replaced with `cargo run -p shuffle --`:

```bash
shuffle new helloblockchain # is replaced by
cargo run -p shuffle -- new helloblockchain
```

1. `cargo run -p shuffle -- new helloblockchain` creates a new shuffle project for move program development
2. `cargo run -p shuffle -- node helloblockchain` or `cd helloblockchain && cargo run -p shuffle -- node .` runs devnet for said project

See help for other commands and uses.

## Testing

```
cd diem/shuffle/cli
cargo test
```

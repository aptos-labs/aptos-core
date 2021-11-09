# Experimental

## Step 0: Install Dependencies

- Install Diem dependencies including Rust, Clang, Deno, etc, by running the following script in `diem` root directory:
```
./scripts/dev_setup.sh
```

## Usage

Please run `shuffle help`.

## Walkthrough for Move Application Development

From the `diem/` base repo directory:

`cargo install --path shuffle/cli` to install the `shuffle` binary, or replace `shuffle` with `cargo run -p shuffle -- `

### Running a Node

1. `shuffle node` runs local test node
1. `shuffle account` creates accounts on the default localhost network

### Creating and developing a project

1. `shuffle new /tmp/helloblockchain` creates a new shuffle project
1. `cd /tmp/helloblockchain`
1. `shuffle deploy` publishes the move package to the default local node
1. `shuffle console` enters a typescript REPL with helpers loaded
1. `shuffle test all` runs both unit tests and end to end tests
1. Modify `/tmp/helloblockchain/e2e/message.test.ts` and rerun `shuffle test e2e`

## Walkthrough for Genesis Development

### Running a Node with custom genesis

1. `shuffle node --genesis diem-move/diem-framework/experimental` runs local test node with a specific move package as the genesis modules
1. `shuffle account` creates accounts on the default localhost network
1. To pick up modifications to the .move code used in genesis, one has to `rm -rf ~/.shuffle` and restart from step 1.

### REPL console with privileged account access

1. `shuffle new /tmp/helloblockchain` creates a new shuffle project. Unused for genesis but needed for REPL. No need to recreate for node restart.
1. `cd /tmp/helloblockchain`
1. `shuffle console -a 0xB1E55ED -k /Users/username/.shuffle/nodeconfig/mint.key` enters a typescript REPL as a privileged account
1. `await devapi.accountTransactions()` in REPL
1. In REPL:
```
await helpers.invokeScriptFunction("0x1::AccountCreationScripts::create_parent_vasp_account", ["0x1::XUS::XUS"], [
  "0",   // sliding_nonce
  "0x948156f6f1ece3a89f1e4354f7edc5fe", // new_account_address
  "0xe1d06094c9cf29963630053d2f6c54df",  // new account auth_key_prefix
  "0x76617370",  // human_name, "vasp"
  true  // add_all_currencies
]);
```
6. `await devapi.accountTransactions()` in REPL
7. `await devapi.resources("0xdeadbeef")` in REPL

### Extending functionality

1. Freestyle in the typescript REPL console, using [deno libraries](https://deno.land/x)
1. Modify `/tmp/helloblockchain/main/mod.ts` with more functions
1. Run E2E tests against your custom genesis with `shuffle test e2e`

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

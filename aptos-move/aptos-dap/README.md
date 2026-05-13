# aptos-dap

Debug Adapter Protocol (DAP) server for Move on Aptos. Supports two modes:

- **Test** — debug Move unit tests locally
- **Replay** — replay a committed on-chain transaction with full debugger support

## Building

```bash
cargo build --release -p aptos-dap
```

The binary is at `target/release/aptos-dap`.

## VS Code setup

### 1. Configure the extension

Open VS Code settings (JSON) and point `move-on-aptos.dap.path` to the binary:

```jsonc
{
    "move-on-aptos.dap.path": "/path/to/aptos-dap"
}
```

Once `dap.path` is set, a **Debug** code lens appears above `#[test]` functions.

### 2. Launch configurations

Add entries to `.vscode/launch.json`:

#### Debug a Move test

```jsonc
{
    "type": "aptos-move-test",
    "request": "launch",
    "name": "Debug Move Test",
    "testFilter": "test_name",       // substring filter; empty runs all tests
    "packagePath": "${workspaceFolder}"
}
```

#### Replay a transaction with local source packages

```jsonc
{
    "type": "aptos-move-replay",
    "request": "launch",
    "name": "Replay Transaction",
    "network": "testnet",
    "txnId": 8571567156,
    // Adjust all paths below to match your local setup.
    // Include stdlib / framework from the Aptos cached packages
    // so that types like Option, String, etc. display with field names.
    "useLocalPackages": [
        "/home/user/.move/https___github_com_aptos-labs_aptos-framework_git_mainnet/aptos-framework/move-stdlib",
        "/home/user/.move/https___github_com_aptos-labs_aptos-framework_git_mainnet/aptos-framework/aptos-framework",
        "/home/user/code/etna/move/accounts",
        "/home/user/code/etna/move/perp",
        "/home/user/code/etna/move/aptos_market"
    ],
    "namedAddresses": {
        "decibel_dex": "0x4e110a6a81a8ead3a0e3785b9ad7d8d61ec04608697dfa0faafc7a8d16dfa692"
    }
}
```

> **Setting breakpoints for replay:** Open a source file from one of the
> `useLocalPackages` paths and set a breakpoint in VS Code as usual.
> For example, with the config above you could set a breakpoint at
> `/home/user/code/etna/move/perp/sources/perp_engine.move:1440`
> (adjust the path to match your local checkout).

> **Rich variable display:** The debugger resolves struct field names, enum
> variant names, and signer values using source maps from the packages listed
> in `useLocalPackages`. Types defined in dependencies (e.g. `Option` from
> `move-stdlib`) are only shown with their proper names when the dependency
> is included. For example, without `move-stdlib` loaded, an `Option<u64>`
> value of `None` appears as `{ [0]: 0 }` instead of `None`. If you need
> rich display for stdlib or framework types, include the corresponding
> package in `useLocalPackages`.

> **Named addresses and breakpoints:** On-chain packages are often deployed
> with named addresses resolved to specific account addresses (e.g.
> `my_addr = 0xCAFE`). If your local `Move.toml` uses a different value
> (or leaves it as `_`), the compiled module IDs won't match the on-chain
> ones and breakpoints will silently not hit. Use `namedAddresses` in
> `launch.json` (or `--named-address` on the CLI) to supply the correct
> deployment addresses.


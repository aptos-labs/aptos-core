{% if once(name="move_package") %}
## Move Packages

A Move package is a directory with a `Move.toml` manifest and source files. The manifest defines the package name, dependencies, and named addresses.

### Named Addresses

Modules are published at named addresses (e.g., `@my_package`). These must resolve to hex values for compilation.

- **`[addresses]`** — production addresses (may use `_` placeholder for deploy-time assignment)
- **`[dev-addresses]`** — development/test values (used when compiling in dev or test mode)

**Fixing "Unresolved addresses" errors:** For each `Named address 'X' in package 'Y'`, add `X = "0x..."` to `[dev-addresses]` in that package's `Move.toml`. Use `0x100` and up, avoiding reserved addresses (`0x0`=vm_reserved, `0x1`=std/aptos_std/aptos_framework, `0x3`=aptos_token, `0x4`=aptos_token_objects, `0x5`=aptos_trading, `0x7`=aptos_experimental, `0xA`=aptos_fungible_asset, `0xA550C18`=core_resources):

```toml
[dev-addresses]
my_package = "0x100"
other_addr = "0x101"
```
{% endif %}

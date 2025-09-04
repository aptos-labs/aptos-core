# velor-ledger

Provides a set of commands and methods to communicate with Velor app on Ledger

The methods supported by the velor-ledger are:

- `get_app_version()`
- `get_app_name()`
- `get_public_key()`
- `sign_txn()`
- get_public_key()
- sign_txn()

NOTE: All methods and commands requires the user to have Velor ledger app unlocked and open

## Examples

### Get Public Key from your Velor account on Ledger

```rust
use velor_ledger::get_public_key;

let public_key = match get_public_key(false);
println!("Public Key: {:?}", public_key);
// "Public Key: ______"
```

### Sign Transaction

Currently we only support derivative path at index 0, and the transaction has to be serialized using BCS format

```rust
use velor_ledger::sign_txn;
use bcs;

let utf8_str = "my transaction";
let signed_txn = sign_txn(to_bytes(utf8_str)?);
```

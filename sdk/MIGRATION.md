# Migration Guide: aptos-sdk → aptos-rust-sdk

This guide helps you migrate from the legacy `aptos-sdk` crate (this repository,
`aptos-labs/aptos-core`) to the new [`aptos-rust-sdk`](https://github.com/aptos-labs/aptos-rust-sdk)
(`aptos-labs/aptos-rust-sdk`).

## Why Migrate?

The new `aptos-rust-sdk` is the recommended SDK for external developers building on Aptos. It:

- Is maintained in a standalone repository, decoupled from aptos-core internals
- Provides a single, ergonomic `Aptos` entry point instead of multiple disconnected client types
- Is fully async/await (tokio)
- Supports more cryptographic schemes (Ed25519, Secp256k1, Secp256r1/P-256, BLS12-381)
- Provides procedural macros that generate type-safe Rust bindings from Move ABIs
- Handles gas estimation, sequence numbers, and chain IDs automatically
- Follows feature parity with the official TypeScript SDK

## Dependency Change

Remove the old SDK and add the new one:

```toml
# Before (Cargo.toml)
[dependencies]
aptos-sdk = "0.0.3"

# After
[dependencies]
aptos-sdk = { version = "0.4", package = "aptos-sdk", git = "https://github.com/aptos-labs/aptos-rust-sdk" }
# Or once published to crates.io:
# aptos-sdk = "0.4"
```

> **Note:** The new crate also uses the name `aptos-sdk` on crates.io (separate from the legacy
> one). Check [crates.io](https://crates.io/crates/aptos-sdk) or the new repository's README for
> the current published version.

## Client Initialization

### Old SDK

```rust
use aptos_sdk::rest_client::Client;

let rest_client = Client::new("https://fullnode.mainnet.aptoslabs.com".parse()?);
```

### New SDK

```rust
use aptos_sdk::Aptos;

// Named network constructors
let aptos = Aptos::mainnet().await?;
let aptos = Aptos::testnet().await?;
let aptos = Aptos::devnet().await?;
let aptos = Aptos::local().await?;

// Custom URL
use aptos_sdk::config::AptosConfig;
let aptos = Aptos::new(AptosConfig::new("https://fullnode.mainnet.aptoslabs.com")).await?;
```

## Account Creation

### Old SDK

```rust
use aptos_sdk::types::LocalAccount;

// Generate a new random account
let account = LocalAccount::generate(&mut rand::rngs::OsRng);

// From a private key
use aptos_sdk::crypto::ed25519::Ed25519PrivateKey;
let account = LocalAccount::new(address, private_key, sequence_number);
```

### New SDK

```rust
use aptos_sdk::account::Ed25519Account;

// Generate a new random account
let account = Ed25519Account::generate(&mut rand::rngs::OsRng);

// From a private key
use aptos_sdk::crypto::ed25519::Ed25519PrivateKey;
let account = Ed25519Account::new(private_key, address, sequence_number);
```

The new SDK also provides additional account types for other key schemes:

```rust
use aptos_sdk::account::{
    Secp256k1Account,    // secp256k1 (Bitcoin/Ethereum curve)
    Secp256r1Account,    // secp256r1/P-256 (WebAuthn/Passkeys), requires feature "secp256r1"
    MultiKeyAccount,     // multi-key (k-of-n threshold)
    KeylessAccount,      // OIDC-based keyless
};
```

## Funding Accounts (Faucet)

### Old SDK

```rust
use aptos_sdk::rest_client::FaucetClient;

let faucet_client = FaucetClient::new(
    "https://faucet.testnet.aptoslabs.com".parse()?,
    "https://fullnode.testnet.aptoslabs.com".parse()?,
);
faucet_client.fund(account.address(), 100_000_000).await?;
```

### New SDK

The faucet is integrated into the `Aptos` client (requires the `faucet` feature):

```toml
# Cargo.toml
aptos-sdk = { version = "0.4", features = ["faucet"] }
```

```rust
aptos.fund_account(account.address(), 100_000_000).await?;
```

## Reading Account Balance

### Old SDK

```rust
use aptos_sdk::coin_client::CoinClient;

let coin_client = CoinClient::new(&rest_client);
let balance = coin_client.get_account_balance(&account.address()).await?;
```

### New SDK

```rust
let balance = aptos.get_balance(account.address()).await?;
```

## Transferring APT

### Old SDK

```rust
use aptos_sdk::coin_client::{CoinClient, TransferOptions};

let coin_client = CoinClient::new(&rest_client);
let txn_hash = coin_client
    .transfer(&mut sender, recipient_address, amount, None)
    .await?;
rest_client.wait_for_transaction(&txn_hash).await?;
```

### New SDK

```rust
let txn = aptos
    .transfer_apt(&mut sender, recipient_address, amount)
    .await?;
// sign_and_submit_and_wait is also available as a one-liner:
// aptos.sign_submit_and_wait(&mut sender, txn).await?;
```

## Building Custom Transactions

### Old SDK

You had to manually manage sequence numbers, gas prices, chain ID, and expiration times:

```rust
use aptos_sdk::transaction_builder::TransactionFactory;
use aptos_sdk::types::chain_id::ChainId;

// Fetch chain info manually
let chain_id = rest_client.get_ledger_information().await?.inner().chain_id;
let sender_account = rest_client.get_account(sender.address()).await?.into_inner();

let txn = TransactionFactory::new(ChainId::new(chain_id))
    .with_gas_unit_price(100)
    .with_max_gas_amount(10_000)
    .entry_function(EntryFunction::new(
        ModuleId::new(account_address, ident_str!("my_module").to_owned()),
        ident_str!("my_function").to_owned(),
        vec![],
        vec![bcs::to_bytes(&arg)?],
    ))
    .sender(sender.address())
    .sequence_number(sender_account.sequence_number)
    .build();

let signed_txn = sender.sign_transaction(txn);
rest_client.submit(&signed_txn).await?;
rest_client.wait_for_transaction(&signed_txn.committed_hash()).await?;
```

### New SDK

Chain ID, sequence number, and gas parameters are fetched and managed automatically:

```rust
use aptos_sdk::move_types::{ident_str, language_storage::ModuleId};

let txn = aptos
    .build_transaction()
    .sender(&mut sender)
    .entry_function(
        ModuleId::new(module_address, ident_str!("my_module").to_owned()),
        ident_str!("my_function").to_owned(),
        vec![],      // type arguments
        vec![bcs::to_bytes(&arg)?],
    )
    .build()
    .await?;

aptos.sign_submit_and_wait(&mut sender, txn).await?;
```

### Type-Safe Move Bindings (new feature)

The new SDK can generate type-safe Rust bindings from Move ABIs using procedural macros:

```rust
use aptos_sdk_macros::move_contract;

// Generates a Rust module with type-safe builders for all entry functions
move_contract!("path/to/abi.json");

// Then call entry functions by name with type-checked arguments
let txn = my_module::my_function(arg1, arg2);
```

## Simulating Transactions / Estimating Gas

### Old SDK

Gas estimation was done manually by calling the simulate endpoint via the REST client directly:

```rust
let simulated = rest_client.simulate(&unsigned_txn).await?;
let gas_used = simulated.inner()[0].info.gas_used;
```

### New SDK

```rust
let simulation = aptos.simulate_transaction(&txn).await?;
let estimated_gas = aptos.estimate_gas(&txn).await?;
```

## Submitting Batch Transactions

The new SDK adds native batch support (not available in the old SDK):

```rust
let txns = vec![txn1, txn2, txn3];
aptos.submit_batch(txns).await?;
```

## Querying On-Chain Data

### Old SDK

```rust
// View function
let result = rest_client
    .view(&ViewRequest { function, type_arguments, arguments })
    .await?;

// Resource
let resource = rest_client
    .get_account_resource(address, "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>")
    .await?;
```

### New SDK

```rust
// View function
let result = aptos.view(function, type_args, args).await?;

// BCS-decoded view function
let result: MyType = aptos.view_bcs(function, type_args, args).await?;
```

The new SDK also exposes an `IndexerClient` for GraphQL queries against indexed data
(requires the `indexer` feature):

```toml
aptos-sdk = { version = "0.4", features = ["indexer"] }
```

```rust
let indexer = aptos.indexer_client()?;
let response = indexer.query(MY_GRAPHQL_QUERY, variables).await?;
```

## Full Example: Fund, Transfer, and Check Balance

### Old SDK

```rust
use aptos_sdk::{
    coin_client::CoinClient,
    rest_client::{Client, FaucetClient},
    types::LocalAccount,
};

let rest_client = Client::new("https://fullnode.testnet.aptoslabs.com".parse()?);
let faucet_client = FaucetClient::new(
    "https://faucet.testnet.aptoslabs.com".parse()?,
    "https://fullnode.testnet.aptoslabs.com".parse()?,
);
let coin_client = CoinClient::new(&rest_client);

let mut alice = LocalAccount::generate(&mut rand::rngs::OsRng);
let bob = LocalAccount::generate(&mut rand::rngs::OsRng);

faucet_client.fund(alice.address(), 100_000_000).await?;

coin_client.transfer(&mut alice, bob.address(), 1_000, None).await?;

let balance = coin_client.get_account_balance(&bob.address()).await?;
println!("Bob's balance: {}", balance);
```

### New SDK

```rust
use aptos_sdk::{account::Ed25519Account, Aptos};

let aptos = Aptos::testnet().await?;

let mut alice = Ed25519Account::generate(&mut rand::rngs::OsRng);
let bob = Ed25519Account::generate(&mut rand::rngs::OsRng);

aptos.fund_account(alice.address(), 100_000_000).await?;

aptos.transfer_apt(&mut alice, bob.address(), 1_000).await?;

let balance = aptos.get_balance(bob.address()).await?;
println!("Bob's balance: {}", balance);
```

## Crate Feature Flags

The new SDK uses feature flags to keep binary size lean. Enable only what you need:

| Feature | Description |
|---------|-------------|
| `faucet` | Enables `FaucetClient` for testnet/devnet funding |
| `indexer` | Enables `IndexerClient` for GraphQL queries |
| `secp256r1` | Enables Secp256r1/P-256 account support (WebAuthn/Passkeys) |
| `bls12-381` | Enables BLS12-381 key support |

## Summary of Type Renames

| Old (`aptos-sdk` in aptos-core) | New (`aptos-rust-sdk`) |
|--------------------------------|------------------------|
| `aptos_sdk::types::LocalAccount` | `aptos_sdk::account::Ed25519Account` |
| `aptos_sdk::rest_client::Client` | `aptos_sdk::Aptos` (unified client) |
| `aptos_sdk::rest_client::FaucetClient` | `aptos_sdk::Aptos::fund_account()` |
| `aptos_sdk::coin_client::CoinClient` | Methods on `aptos_sdk::Aptos` |
| `aptos_sdk::transaction_builder::TransactionFactory` | `aptos_sdk::Aptos::build_transaction()` |
| `aptos_sdk::crypto::*` | `aptos_sdk::crypto::*` (same names) |
| `aptos_sdk::move_types::*` | `aptos_sdk::move_types::*` (same names) |

## Further Resources

- [aptos-rust-sdk repository](https://github.com/aptos-labs/aptos-rust-sdk)
- [Aptos Developer Documentation](https://aptos.dev)
- [Aptos REST API Specification](https://aptos.dev/nodes/aptos-api-spec)

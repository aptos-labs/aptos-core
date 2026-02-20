# aptos-move-testing-utils

Shared utilities for Aptos Move testing tools, providing common functionality for tools like `aptos-e2e-comparison-testing` and `replay-benchmark`.

## Features

- **Transaction Comparison**: Compare transaction outputs with detailed diff reporting
- **Package Management**: Download and manage Aptos framework packages
- **Compilation Caching**: Cache compiled Move packages to avoid redundant compilation
- **Client Initialization**: Convenient REST client and debugger setup
- **Transaction Persistence**: Serialize and persist transaction blocks to disk
- **State Store Population**: Utilities for populating state stores with compiled packages
- **Flexible Configuration**: Builder patterns for easy customization

## Modules

### `diff` - Transaction Output Comparison

Compare transaction outputs and generate detailed diffs for debugging and testing.

```rust
use aptos_move_testing_utils::TransactionDiffBuilder;

// Create a diff builder with default settings
let builder = TransactionDiffBuilder::new();

// Or allow different gas usage
let builder = TransactionDiffBuilder::with_gas_tolerance();

// Or customize with builder pattern
let builder = TransactionDiffBuilder::new()
    .allow_different_gas_usage(true)
    .still_compare_gas_used(false);

// Compare transaction outputs
let diff = builder.build_from_outputs(output1, output2, fee_payer);

// Print differences
if !diff.is_empty() {
    diff.println();
}
```

**Key Types**:
- `Diff` - Enum representing different types of differences (gas, status, events, write set)
- `TransactionDiff` - Container for all differences between two transaction outputs
- `TransactionDiffBuilder` - Builder for configuring comparison behavior

### `packages` - Aptos Framework Package Management

Detect, download, and manage Aptos framework packages.

```rust
use aptos_move_testing_utils::{
    is_aptos_package, get_aptos_dir, prepare_aptos_packages,
};
use std::path::PathBuf;

// Check if a package is an Aptos framework package
if is_aptos_package("AptosFramework") {
    println!("This is an Aptos framework package");
}

// Get the directory name for a package
let dir = get_aptos_dir("MoveStdlib"); // Some("move-stdlib")

// Download and prepare framework packages
let packages_path = PathBuf::from("/tmp/aptos-packages");
prepare_aptos_packages(packages_path, None, false).await;
```

**Key Functions**:
- `is_aptos_package()` - Check if a package is an Aptos framework package
- `get_aptos_dir()` - Get directory name for a framework package
- `download_aptos_packages()` - Download framework packages from GitHub
- `check_aptos_packages_availability()` - Verify packages exist
- `prepare_aptos_packages()` - Download packages if needed

### `client` - REST Client and Debugger Setup

Convenient initialization of Aptos REST clients and debuggers.

```rust
use aptos_move_testing_utils::{
    ClientConfig, create_rest_client, create_debugger,
    create_client_and_debugger,
};

// Use a preset configuration
let config = ClientConfig::mainnet();
let config = ClientConfig::testnet();
let config = ClientConfig::devnet();
let config = ClientConfig::local();

// Add an API key
let config = ClientConfig::mainnet().with_api_key("your-api-key".to_string());

// Create just a REST client
let client = create_rest_client(&config.endpoint, config.api_key)?;

// Create just a debugger
let debugger = create_debugger(&config.endpoint, config.api_key)?;

// Create both at once
let (client, debugger) = create_client_and_debugger(&config)?;
```

**Key Types**:
- `ClientConfig` - Configuration for REST clients with network presets
- `create_rest_client()` - Create an Aptos REST client
- `create_debugger()` - Create an AptosDebugger
- `create_client_and_debugger()` - Create both client and debugger

### `compilation` - Compilation Caching

Cache compiled Move packages to avoid redundant compilation during testing.

```rust
use aptos_move_testing_utils::{CompilationCache, PackageInfo};
use move_core_types::account_address::AccountAddress;

// Create a new cache
let mut cache = CompilationCache::new();

// Create package info
let package_info = PackageInfo::new(
    AccountAddress::from_hex_literal("0x1")?,
    "my-package".to_string(),
    None,
);

// Check if package failed to compile
if cache.is_failed_base(&package_info) {
    println!("Package failed to compile with base compiler");
}

// Mark package as failed
cache.mark_failed_base(package_info);

// Get cache statistics
let stats = cache.stats();
println!("Compiled packages: {}", stats.compiled_packages);
println!("Failed packages: {}", stats.failed_base);
```

**Key Types**:
- `CompilationCache` - Main cache structure
- `PackageInfo` - Package identification and metadata
- `CacheStats` - Statistics about cache contents

### `persistence` - Transaction Block Serialization

Utilities for persisting and loading transaction blocks to/from disk.

```rust
use aptos_move_testing_utils::{
    TransactionBlock, save_blocks_to_file, load_blocks_from_file,
};
use std::path::Path;

// Create transaction blocks
let blocks = vec![
    TransactionBlock::new(0, vec![/* transactions */]),
    TransactionBlock::new(100, vec![/* transactions */]),
];

// Save blocks to a file
save_blocks_to_file(&blocks, Path::new("blocks.bcs")).await?;

// Load blocks from a file
let loaded_blocks = load_blocks_from_file(Path::new("blocks.bcs")).await?;

// Or work with individual blocks
let block = TransactionBlock::new(0, vec![]);
block.save_to_file(Path::new("single_block.bcs")).await?;
let loaded_block = TransactionBlock::load_from_file(Path::new("single_block.bcs")).await?;

// Serialize/deserialize without file I/O
let bytes = block.serialize_to_bytes()?;
let deserialized = TransactionBlock::deserialize_from_bytes(&bytes)?;
```

**Key Types**:
- `TransactionBlock` - Structure holding a version and list of transactions
- `save_blocks_to_file()` - Save multiple blocks to a file
- `load_blocks_from_file()` - Load multiple blocks from a file
- `serialize_blocks()` - Serialize blocks to bytes
- `deserialize_blocks()` - Deserialize blocks from bytes

### `state_store_utils` - State Store Population

Helper functions for populating state stores with compiled packages.

```rust
use aptos_move_testing_utils::{
    populate_state_with_packages, populate_state_with_aptos_packages,
    StateStorePackageInfo,
};
use aptos_transaction_simulation::InMemoryStateStore;
use move_core_types::account_address::AccountAddress;

let state_store = InMemoryStateStore::new();
let package_info = StateStorePackageInfo {
    address: AccountAddress::ONE,
    package_name: "my-package".to_string(),
    upgrade_number: None,
};

// Populate with a specific package
populate_state_with_packages(&state_store, &package_info, &compiled_cache);

// Populate with all Aptos framework packages
populate_state_with_aptos_packages(&state_store, &compiled_cache);
```

**Key Types**:
- `StateStorePackageInfo` - Package identification for state store population
- `populate_state_with_packages()` - Add a specific package to state store
- `populate_state_with_aptos_packages()` - Add all framework packages

## Usage in Tools

### replay-benchmark

The `replay-benchmark` tool uses this library for:
- Transaction output comparison (`TransactionDiffBuilder`)
- Debugger initialization (`create_debugger`)
- Transaction block persistence (`TransactionBlock`, `save_blocks_to_file`, `load_blocks_from_file`)

### aptos-e2e-comparison-testing

The `aptos-e2e-comparison-testing` tool uses this library for:
- Transaction output comparison (`Diff`, `TransactionDiffBuilder`)
- Aptos framework package management (all `packages` module functions)
- Client initialization (`ClientConfig`, `create_rest_client`)
- State store population (`populate_state_with_packages`, `populate_state_with_aptos_packages`)

## Development

### Running Tests

```bash
cargo test
```

### Building Documentation

```bash
cargo doc --open
```

## Dependencies

This library depends on:
- `aptos-framework` - Framework packages and metadata
- `aptos-move-debugger` - AptosDebugger functionality
- `aptos-rest-client` - REST client for Aptos nodes
- `aptos-types` - Core Aptos types
- `move-core-types` - Move language core types
- `move-package` - Move package compilation
- And other workspace dependencies

## License

Licensed pursuant to the Innovation-Enabling Source Code License.

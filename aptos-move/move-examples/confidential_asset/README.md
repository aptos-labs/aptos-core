# Confidential Asset — Usage Examples

This package contains developer-facing examples showing how to call the confidential asset public API from an external Move package.

Each file in `tests/` is a self-contained, readable walkthrough of one operation:

- `register_example.move` — register a confidential balance
- `deposit_example.move` — deposit tokens into a confidential balance
- `rollover_example.move` — roll over the pending balance into the available balance
- `transfer_example.move` — confidentially transfer tokens between two accounts
- `withdraw_example.move` — withdraw tokens out of a confidential balance
- `normalize_example.move` — normalize a confidential balance
- `rotate_example.move` — rotate the encryption key

## How these differ from `framework/aptos-experimental/tests/`

| | `move-examples/confidential_asset/` | `framework/aptos-experimental/tests/` |
|---|---|---|
| **Purpose** | Developer documentation / usage examples | Internal correctness and regression tests |
| **Package** | Separate (`confidential_asset_example`) — treats `aptos_experimental` as an external dependency, just like a real developer would | Same package as `aptos_experimental` — has access to internal helpers and private types |
| **Scope** | One file per operation, focused on readability | Exhaustive coverage of operations, edge cases, error conditions, ZK proofs, and individual Sigma protocols |
| **Audience** | Developers learning how to use the API | Framework maintainers |

The examples import `aptos_experimental::confidential_asset_tests` for test setup helpers, so they depend on the framework tests package.

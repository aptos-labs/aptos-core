{% if once(name="move_lang") %}
## Move Language

Move on Aptos is a safe, resource-oriented programming language for smart contracts on the
Aptos blockchain. It uses a linear type system to enforce ownership and prevent
double-spending at compile time.

## Move Language Basics

- **Modules** are the unit of code organization, published at an address.
- **Structs** define data types; abilities (`key`, `store`, `copy`, `drop`) control what
  operations are permitted.
- **Entry functions** (`entry fun`) are transaction entry points callable from outside Move.
- **View functions** (`#[view]`) are read-only queries that do not modify state.
- **Global storage** stores resources (structs with `key`) at addresses.
- **Move 2 syntax** (required):
    - Read resource: `&T[addr]` (not `borrow_global<T>(addr)`)
    - Mutate resource: `&mut T[addr]` (not `borrow_global_mut<T>(addr)`)
    - Access field: `T[addr].field` directly (the compiler inserts the ref op)
    - `acquires` annotations are no longer needed — do not add them.
- **Error codes**: Use named constants for abort codes (`const E_NOT_FOUND: u64 = 1;`) and
  document them.
- **Edit hook**: The edit hook auto-runs on `.move` files after edits. If it reports
  compilation errors, fix them before proceeding with further changes.


## Reference

- [The Move Book](https://aptos.dev/move/book/SUMMARY)
- [Aptos Framework Reference](https://aptos.dev/reference/move/?branch=mainnet&page=aptos-framework/doc/overview.md)
{% endif %}

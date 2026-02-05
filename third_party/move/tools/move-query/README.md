# move-query

Programmatic access to Move package introspection via the Move model.

## Usage Modes

### 1. Library API

Use `QueryEngine` to query Move packages programmatically from Rust code.

- **Example**: [`tests/testsuite.rs`](tests/testsuite.rs)

### 2. MCP Server (CLI)

JSON-RPC 2.0 server over stdio for AI agent integration ([MCP spec](https://modelcontextprotocol.io/specification)).

```bash
aptos move query serve
```

- **Example session**: [`src/mcp/server.rs`](src/mcp/server.rs) (module doc comment)
- **Tool schemas**: Call `tools/list` to get input schemas for each tool
- **Available tools**: `build_model`, `rebuild_model`, `get_package`, `get_module`, `get_function`, `get_struct`, `get_constant`, `get_source`, `shutdown`
- **Name format**: `0x1::module` for modules, `0x1::module::item` for functions/structs/constants
- **Response types**: [`src/types.rs`](src/types.rs) defines `Package`, `Module`, `Function`, `Struct`, `Constant`, etc.

# MoveFlow Crate (`aptos-move/flow`)

Claude Code plugin for Move smart contract development on Aptos. Provides an MCP server, plugin generator, and edit hooks.

## Build & Test

```bash
cargo build -p aptos-move-flow              # Build
cargo test -p aptos-move-flow               # Run all tests
cargo test -p aptos-move-flow -- <name>     # Run a specific test
UB=1 cargo test -p aptos-move-flow          # Update .exp baselines
cargo install --path aptos-move/flow --locked --profile ci  # Install binary
```

## Architecture

Three subcommands via `move-flow <subcommand>`:

- **`plugin <dir>`** — Generates plugin files (agents, skills, hooks, `.mcp.json`, `.claude-plugin/plugin.json`) from Tera templates in `cont/`. See `src/plugin/`.
- **`mcp`** — Stdio-based MCP server (rmcp) with tools for Move package analysis. See `src/mcp/`.
- **`hook edit|package-path`** — Hooks called by the AI platform on file edits and prompt submission. See `src/hooks/`.

### Key Modules

| Path | Purpose |
|------|---------|
| `src/mcp/session.rs` | `FlowSession` — MCP server handler, owns package cache |
| `src/mcp/package_data.rs` | `PackageData` — wraps Move compiler's `GlobalEnv` |
| `src/mcp/file_watcher.rs` | OS-native file watching for cache invalidation |
| `src/mcp/tools/` | MCP tool implementations (status, manifest, test, verify, query, spec_infer) |
| `src/mcp/tools/replay_transaction.rs` | Replay tool: fetches a committed tx, runs it locally, optionally with package overrides |
| `src/plugin/render.rs` | Tera template rendering |
| `src/plugin/output.rs` | File output writer |
| `src/hooks/source_check/` | Edit hook: parse checking, AST checks, deprecated syntax detection |
| `src/hooks/package_path.rs` | UserPromptSubmit hook: detects current Move package |
| `cont/` | Source templates: `agents/`, `skills/`, `hooks/`, `templates/` |

### MCP Tools

`move_package_status`, `move_package_manifest`, `move_package_test`, `move_package_coverage`, `move_package_verify`, `move_package_query`, `move_package_spec_infer`, `move_replay_transaction`

## Testing

Tests are end-to-end in `src/tests/`, organized by tool/feature. Each test module has `.exp` baseline files — use `UB=1` to regenerate them. Tests spin up the MCP server as a client and invoke tools against fixture packages.

## Templates

Templates in `cont/` use [Tera](https://keats.github.io/tera/) syntax. A custom `tool(name="...")` function validates that referenced MCP tool names exist at render time. Templates are organized as:
- `cont/agents/` — Agent instruction files
- `cont/skills/` — Skill definitions (each in its own subdirectory with `SKILL.md`)
- `cont/hooks/` — `hooks.json` event hook configuration
- `cont/templates/` — Shared fragments included by agents/skills via Tera `{% include %}`

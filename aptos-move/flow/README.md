# MoveFlow

AI-assisted Move smart contract development for Aptos. Provides a plugin
generator, MCP server, and edit hooks for AI coding assistants. Currently
targets Claude Code, with other platforms planned.

**For users:** install via [aptos-labs/aptos-ai](https://github.com/aptos-labs/aptos-ai).
The rest of this document is for MoveFlow developers.

## Development Setup

Generate a local plugin directory and start Claude with it:

```bash
./scripts/gen-local-for-claude.sh            # builds move-flow, generates plugin at ./gen/claude
claude --plugin-dir ./gen/claude
```

Options: `--debug` (debug build), `--log <file>` (enable MCP server logging).

### Debugging

Logging is controlled via the `MVC_LOG` env var:

```bash
./scripts/gen-local-for-claude.sh --log /tmp/flow.err.log
MVC_LOG=aptos_move_flow=debug claude --plugin-dir ./gen/claude
```

Without the module filter, `debug` produces heavy output from other Move tools.

### Publishing

`scripts/publish-plugin.sh` generates the plugin tree and opens a PR against
[aptos-labs/aptos-ai](https://github.com/aptos-labs/aptos-ai):

```bash
./scripts/publish-plugin.sh           # defaults to ~/aptos-ai
./scripts/publish-plugin.sh /path/to/aptos-ai
```

## Plugin Generator (`src/plugin/`)

Uses [Tera](https://keats.github.io/tera/) templates to produce
platform-specific configuration files.

- **`cont/`** — Source templates: `agents/`, `skills/`, `hooks/`, plus shared
  `templates/` fragments included via Tera `{% include %}`.
- **`render.rs`** — Discovers and renders templates. A custom `tool(name="...")`
  function validates that referenced MCP tool names exist.
- **`output.rs`** — Writes rendered files to the output directory.

Also emits `.mcp.json` (MCP server config) and `.claude-plugin/plugin.json`
(plugin manifest).

## MCP Server (`src/mcp/`)

Stdio-based MCP server built on [rmcp](https://github.com/anthropics/rmcp).

Provides tools for Move package analysis. Packages are identified by path
(`<path>/Move.toml`). Compilation results are cached on demand and invalidated
via OS-native file watchers when sources change.

### Architecture

- **`session.rs`** — `FlowSession`: server handler, package cache, tool router.
  Compilation runs on `spawn_blocking`.
- **`package_data.rs`** — `PackageData`: wraps the Move compiler's `GlobalEnv`.
- **`file_watcher.rs`** — inotify/FSEvents watcher for cache invalidation.

### Tools

| Tool | Description |
|------|-------------|
| `move_package_status` | Compilation errors and warnings |
| `move_package_manifest` | Source file paths and dependency paths |
| `move_package_query` | Structural queries: dependency graph, module summary, call graph, function usage |
| `move_package_test` | Run unit tests, report coverage changes against a baseline |
| `move_package_coverage` | Uncovered source lines |
| `move_package_verify` | Run the Move Prover |
| `move_package_spec_infer` | Infer and inject specifications |

All tools accept a `package_path` parameter.

## Edit Hook (`src/hooks/`)

Runs automatically after `Edit`/`Write` on `.move` files (registered in
`cont/hooks/hooks.json`, invoked as `move-flow hook edit`).

1. **Syntactic checks** — parse errors, AST checks (spec expression issues),
   deprecated Move 1 patterns (`borrow_global`, `acquires`).
2. **Auto-formatting** — runs [movefmt](https://github.com/movebit/movefmt)
   if installed (`$MOVEFMT_EXE` / `~/.local/bin/movefmt` / `PATH`).

# MoveFlow

AI-assisted Move smart contract development for Aptos. Provides a plugin
generator, MCP server, and edit hooks for AI coding assistants. Currently
targets Claude Code, with other platforms planned.

**For users:** install via [aptos-labs/aptos-ai](https://github.com/aptos-labs/aptos-ai).
The rest of this document is for MoveFlow developers.

## Development Setup

**Quick Install (Pre-built Binaries):**

For the easiest installation, use our one-line installer:

```bash
# Unix/Linux/macOS
curl -fsSL https://raw.githubusercontent.com/aptos-labs/aptos-core/main/scripts/binary_release/install_binary.sh | sh -s -- move-flow

# Windows (PowerShell)
iwr https://raw.githubusercontent.com/aptos-labs/aptos-core/main/scripts/binary_release/install_binary.ps1 -OutFile install.ps1; .\install.ps1 -BinaryName move-flow

# Using cargo-binstall (if published to crates.io)
cargo binstall aptos-move-flow
```

**Build from Source:**

```bash
cargo install --path aptos-move/flow --locked --profile ci
```

This puts `move-flow` on your `$PATH`. You can also set `$MOVE_FLOW` to point
to a custom binary location; the generated `.mcp.json` will respect it.

For more installation options and release information, see [RELEASE.md](RELEASE.md).

### Generate a Plugin
Generate a local plugin directory and start Claude with it:

```bash
./scripts/gen-local-for-claude.sh            # builds move-flow, generates plugin at ./gen/claude
claude --plugin-dir ./gen/claude
```

Options: `--debug` (debug build), `--log <file>` (enable MCP server logging).

### Inference Tactics

`/move-inf` follows one of three tactics: `hybrid-guided` (WP diagnostics
drive the invariant work; the default), `hybrid-flexible` (WP is available,
the workflow is the agent's), and `agent-only` (direct reasoning, no WP tool).

The two hybrid tactics share one tool inventory, so a hybrid plugin serves
both and an invocation picks one without regenerating anything:

```text
/move-inf                         # the plugin's default hybrid tactic
/move-inf hybrid-flexible sources/x.move
```

`move-flow plugin ... --inference-tactic <tactic>` (or
`MOVE_FLOW_INFERENCE_TACTIC`) sets the default. The direct tactic is its own
plugin: generated with `--inference-tactic agent-only`, it carries only that
tactic and the WP tool is absent from both the advertised inventory and the
runtime router.

`--evaluation-mode` pins the tactic for a measured session: the skill carries
only that tactic and accepts no override. It requires
`--flow-source-commit COMMIT`; the generated `.mcp.json` records the tactic,
the evaluation flag and the expected tool-inventory hash, and the MCP server
refuses to start if an environment override disagrees. Pass
`--telemetry-jsonl <path>` to forward a JSONL telemetry destination. The
generated `move-flow-manifest.json` records the tactic, evaluation-mode flag,
rendered inference-skill hash, and MCP tool-inventory hash.

```bash
move-flow plugin ./gen/agent --inference-tactic agent-only --evaluation-mode --flow-source-commit COMMIT
```

The paper-evaluation controller, hidden judge, corpus builder, and randomized
schedule tooling live in [`evaluation/spec-inference`](evaluation/spec-inference/README.md).
`move-flow experiment inventory` enumerates the two frozen source frames through
the compiler model AST, while `move-flow experiment compare-implementation`
lets the external judge reject runtime-code changes by comparing compiled Move
modules.

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
| `move_package_wp` | Infer and inject specifications with weakest preconditions (hybrid tactics only) |
| `move_spec_check` | Acceptance check for a specification: compile, admissibility, contract coverage, prover |

All tools accept a `package_path` parameter.

## Edit Hook (`src/hooks/`)

Runs automatically after `Edit`/`Write` on `.move` files (registered in
`cont/hooks/hooks.json`, invoked as `move-flow hook edit`).

1. **Syntactic checks** — parse errors, AST checks (spec expression issues),
   deprecated Move 1 patterns (`borrow_global`, `acquires`).
2. **Auto-formatting** — runs [movefmt](https://github.com/movebit/movefmt)
   if installed (`$MOVEFMT_EXE` / `~/.local/bin/movefmt` / `PATH`).

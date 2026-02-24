# MoveFlow

MoveFlow generates AI platform configurations for Move smart contract development
on Aptos and provides an MCP (Model Context Protocol) server for interactive
package analysis. Currently targets Claude Code, with other platforms planned.

## Usage

### Install

```bash
cargo install --path aptos-move/flow --locked --profile ci
```

This puts `move-flow` on your `$PATH`. You can also set `$MOVE_FLOW` to point
to a custom binary location; the generated `.mcp.json` will respect it.

### Generate a Plugin

```bash
move-flow plugin <plugin_dir>
```

This renders templates and writes them to `<plugin_dir>`:

```
<plugin_dir>/
  agents/         # Agent personality / instruction files
  skills/         # Skill definitions (e.g. move/)
  hooks/          # Event hooks (shell scripts + hooks.json)
  .mcp.json       # MCP server discovery config
```

Use `--platform <target>` to select the AI platform (default: `claude`).

### Run with Claude Code

For now, one can just specify at claude startup time. This is useful for development of flow 
where the plugin tree is not installed in a fixed location:

```bash
claude --plugin-dir <plugin_dir>
```

For more permanent configurations, consult Claude docs.

### Debugging

The MCP server uses the same logging mechanism as the Move compiler and prover, sending
log messages to stderr. Logging can be controlled via the `MVC_LOG` env var as shown below,
where we narrow output to code in the flow crate, and increase level from default 'info' to
'debug' (note that without the module filter, `debug` would create tons of output from other
Move tools).

```bash
move-flow plugin --log /tmp/flow.err.log <plugin_dir>
MVC_LOG=aptos_move_flow=debug claude --plugin-dir ~/plugin/test
```

The `--log <path>` option adds a stderr redirect to the generated `.mcp.json` so output is
appended to the given file. Without it, stderr is not redirected.

## Plugin Generator (`plugin/`)

The plugin generator uses [Tera](https://keats.github.io/tera/) templates
to produce platform-specific configuration files.

- **`cont/`** — Source templates organized by category (`agents/`,
  `skills/`, `hooks/`) plus shared `templates/` containing reusable
  content fragments (language references, workflow descriptions) that
  are included into the agent and skill templates via Tera.
- **`render.rs`** — Discovers all template files under `cont/`, renders each
  with a context containing the platform name, display name, version, and output
  directory. A custom `tool(name="...")` Tera function validates that
  referenced MCP tool names actually exist.
- **`output.rs`** — Writes rendered files to the output directory, creating
  subdirectories as needed.

The generator also emits a `.mcp.json` file so Claude Code can automatically
discover and launch the MCP server. 

The design is flexible enough to support other AI platforms (e.g. Cursor) by allowing
per-platform conditionals in templates and rendering content into a different directory layout.

## MCP Server (`mcp/`)

A stdio-based MCP server built on [rmcp](https://github.com/anthropics/rmcp).

This service provides a set of tools around Move packages. A move package is identified by a 
package path such that `<package-path>/Move.toml` exists. The MCP maintains a cache of compilation 
and analysis results based on `<package-path>`, among those the Move Model for this package and
all of its dependencies. 

When at tool requests information about a package, the cache will be 
populated on demand. Subsequent tool calls can use the cached package and should be very fast.

In order to track whether the cached package is up-to-date wrt sources, the service registers file 
watchers at relevant places in the transient dependency, and if a change is detected, the related 
cache entry is invalidated.

### Architecture

- **`session.rs`** — `FlowSession` implements `ServerHandler` and owns the
  package cache (`BTreeMap<String, Arc<Mutex<PackageData>>>`) and tool router.
  Package compilation is offloaded to `spawn_blocking` to keep the async
  executor responsive.
- **`package_data.rs`** — `PackageData` wraps the Move compiler's `GlobalEnv`,
  which holds the full compilation result (modules, diagnostics, source maps).
- **`file_watcher.rs`** — `FileWatcher` uses OS-native file watching
  (FSEvents / inotify) to monitor source directories and invalidate cached
  packages when files change.

### Package Tools

| Tool                       | Description                                                     |
|----------------------------|-----------------------------------------------------------------|
| `move_package_status`      | Return errors and warnings as formatted diagnostics             |
| `move_package_manifest`    | Return the package's source file paths and dependency paths     |
| `move_package_verify`      | Run the Move Prover on a package and return verification output |
| `move_package_spec_infer`  | Run spec inference and inject inferred specs into source files  |

All tools accept a `package_path` parameter pointing to a Move package
directory.

## Edit Hook (`hooks/`)

The `PostToolUse` edit hook runs automatically after every `Edit` or `Write`
tool call that touches a `.move` file. It provides fast feedback without
waiting for a full package build.

The hook is registered in `cont/hooks/hooks.json` and invoked as:

```
move-flow hook edit
```

It reads the hook JSON from stdin (provided by the AI platform) and performs
two steps:

### 1. Syntactic Checks

Parses the file with the Move v1 parser and runs three layers of checks:

- **Parse errors** — reported immediately if the file doesn't parse.
- **AST checks** — walks the parsed AST to flag spec-expression issues such
  as `old()` in the wrong context, or `*e` / `&e` inside spec blocks.
- **Text checks** — scans source text for deprecated Move 1 patterns
  (`borrow_global<`, `borrow_global_mut<`, `acquires`).

Diagnostics are printed to stdout in `codespan-reporting` format so the AI
assistant can see and fix them.

### 2. Auto-formatting

When the file has no errors, the hook runs
[movefmt](https://github.com/movebit/movefmt) in-place to keep formatting
consistent. The binary is resolved with the same 3-step lookup as the Aptos
CLI: `$MOVEFMT_EXE` → `~/.local/bin/movefmt` → `PATH`. If `movefmt` is not
installed, this step is silently skipped.

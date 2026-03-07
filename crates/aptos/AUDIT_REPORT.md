# Aptos CLI (`crates/aptos`) Audit Report

**Date:** February 13, 2026  
**Scope:** `crates/aptos/` — the main Aptos CLI binary (~24,600 lines of Rust across 70 source files)  
**Auditor:** Automated code review  

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Architecture Overview](#2-architecture-overview)
3. [Security Findings](#3-security-findings)
4. [Performance Findings](#4-performance-findings)
5. [UX / Usability Findings](#5-ux--usability-findings)
6. [Code Quality & Maintainability](#6-code-quality--maintainability)
7. [Prioritized Action Items](#7-prioritized-action-items)

---

## 1. Executive Summary

The Aptos CLI is a well-structured Rust application that serves as the primary interface for developers interacting with the Aptos blockchain. It covers account management, Move smart contract development, node operations, staking, governance, and self-updating.

Overall the CLI is in good shape, with `#![deny(unsafe_code)]` / `#![forbid(unsafe_code)]` at the crate level and solid use of Rust's type system. However, this audit identifies **8 security findings** (2 high, 3 medium, 3 low), **6 performance findings**, and **12 UX findings** that merit attention.

---

## 2. Architecture Overview

### Module Structure

| Module | Purpose | Approx. Lines |
|--------|---------|--------------|
| `common/types.rs` | Core types, `CliError`, `TransactionOptions`, key handling | ~2,750 |
| `move_tool/mod.rs` | Move package compile, test, publish, deploy | ~3,050 |
| `common/init.rs` | `aptos init` command | ~500 |
| `common/utils.rs` | File I/O, telemetry, prompts, helpers | ~710 |
| `common/transactions.rs` | Transaction construction / simulation | ~390 |
| `config/mod.rs` | Global/workspace config management | ~500 |
| `account/` | Account creation, balance, transfer, key rotation, multisig | ~1,800 |
| `node/` | Node management, local testnet (Docker-based) | ~3,500 |
| `genesis/` | Genesis ceremony tools | ~2,000 |
| `governance/` | On-chain governance proposals, delegation pools | ~1,700 |
| `stake/` | Staking operations | ~670 |
| `update/` | Self-update, tool management | ~800 |
| `op/key.rs` | Key generation / extraction | ~200 |

### Dependency Count
- **56 internal crates** (`aptos-*` / `move-*`)
- **~40 external crates** (reqwest, tokio, clap, serde, bollard, diesel, etc.)
- Notable: `self_update` pinned to a forked Git revision
- The `aptos-vm` dependency is pulled with `features = ["testing"]` even in the production binary

---

## 3. Security Findings

### SEC-01 [HIGH] — Private keys can be passed via CLI arguments

**Location:** `common/types.rs:791-793` (`PrivateKeyInputOptions`)

```rust
#[clap(long, group = "private_key_input")]
private_key: Option<String>,
```

**Issue:** Private keys passed as `--private-key <hex>` appear in process listings (`ps aux`), shell history files (`.bash_history`, `.zsh_history`), and system audit logs. This is a significant leakage vector on shared machines.

**Recommendation:**
- Add a warning when `--private-key` is used (not from file) advising users to prefer `--private-key-file` or profile configuration.
- Consider reading from stdin or an environment variable as a safer alternative.
- Document the risk prominently in the `--help` text.

---

### SEC-02 [HIGH] — `self_update` dependency pinned to a personal fork

**Location:** `Cargo.toml:99-102`

```toml
self_update = { git = "https://github.com/banool/self_update.git", rev = "8306158ad0fd5b9d4766a3c6bf967e7ef0ea5c4b", ... }
```

**Issue:** The self-update mechanism — which **downloads and replaces the running binary** — depends on a personal GitHub fork (`banool/self_update`). If that account is compromised, a supply chain attack could distribute malicious CLI binaries to all users running `aptos update`.

**Recommendation:**
- Move to the official `self_update` crate, or fork under the `aptos-labs` organization.
- Implement binary signature verification (e.g., GPG or Sigstore cosign) before applying updates.
- Pin to a hash and audit the forked code.

---

### SEC-03 [MEDIUM] — Config files store private keys in plaintext YAML

**Location:** `common/types.rs:276-282` (`ProfileConfig.private_key`)

The private key is stored as a hex-encoded string in `.aptos/config.yaml`. While the file is created with `0o600` permissions on Unix, there is no encryption at rest, and on non-Unix systems there is no file permission restriction at all.

```rust
#[cfg(unix)]
opts.mode(0o600);
```

**Recommendation:**
- Add encryption at rest (e.g., age/nacl symmetric encryption with a user passphrase).
- On Windows, use ACLs via `std::os::windows` or at minimum warn the user.
- Consider integration with OS keychains (macOS Keychain, Windows Credential Manager, Linux Secret Service/libsecret).

---

### SEC-04 [MEDIUM] — GitHub token read from file without permission check

**Location:** `genesis/git.rs:148-149`

```rust
let token = Token::FromDisk(token_path).read_token()?;
```

The GitHub API token file is read without verifying its file permissions. A world-readable token file could leak credentials.

**Recommendation:**
- Verify the token file has restrictive permissions (`0o600` or `0o400`) before reading, and warn/error if it doesn't.

---

### SEC-05 [MEDIUM] — `node_api_key` exposed via environment variable without documentation

**Location:** `common/types.rs:1108-1109`

```rust
#[clap(long, env)]
pub node_api_key: Option<String>,
```

The `#[clap(long, env)]` attribute causes clap to read from the `NODE_API_KEY` environment variable. While this is actually good practice for secrets, the env var name should be explicitly set and documented, and the value should be masked in `--help` output.

**Recommendation:**
- Use `#[clap(long, env = "APTOS_NODE_API_KEY", hide_env_values = true)]` to explicitly name and hide the value.
- Similarly for `faucet_auth_token` at line 1659-1660.

---

### SEC-06 [LOW] — `RngArgs.from_string_seed` uses `assert!` instead of returning error

**Location:** `common/types.rs:580-581`

```rust
pub fn from_string_seed(str: &str) -> RngArgs {
    assert!(str.len() < 32);
```

This will panic if the string is >= 32 bytes. While this is likely only used internally/for tests, it should return a `Result` for safety.

**Recommendation:** Replace `assert!` with a proper error return.

---

### SEC-07 [LOW] — No TLS certificate verification configuration for REST client

**Location:** `common/types.rs:1148-1156` (`RestOptions::client`)

The REST client is created without explicit TLS configuration. While `reqwest` defaults to system certificate verification, there is no option to pin certificates or control TLS behavior for security-sensitive operations (e.g., submitting transactions on mainnet).

**Recommendation:**
- Consider adding a `--tls-ca-cert` option for environments requiring certificate pinning.
- Document that the CLI relies on system CA trust store.

---

### SEC-08 [LOW] — `ShowPrivateKey` command has no confirmation prompt

**Location:** `config/mod.rs:136-161`

The `config show-private-key` command outputs the private key to stdout without any confirmation prompt or warning. This could accidentally expose keys in shared terminal sessions or logs.

**Recommendation:**
- Add a confirmation prompt: "Are you sure you want to display the private key?"
- Consider copying to clipboard instead of stdout, or masking by default.

---

## 4. Performance Findings

### PERF-01 — Massive dependency tree inflates binary size and compile time

**Location:** `Cargo.toml`

The CLI depends on 56 internal crates plus 40+ external crates, including heavy dependencies like:
- `aptos-node` (the full node implementation)
- `aptos-vm` with `features = ["testing"]`
- `diesel` (ORM, with postgres backend)
- `bollard` (Docker API client)
- `aptos-indexer-*` crates

Many of these are only needed for specific subcommands (e.g., `node run-local-testnet` needs Docker/diesel/indexer, but `account transfer` does not).

**Recommendation:**
- Split heavy dependencies behind Cargo features so only users who need `local-testnet` functionality pay the compile/binary cost.
- The `aptos-vm` `testing` feature should not be enabled in the release binary; guard it behind `#[cfg(any(test, feature = "fuzzing"))]`.

---

### PERF-02 — Repeated config file loading

**Location:** `common/types.rs` (multiple functions)

Several methods in `PrivateKeyInputOptions`, `RestOptions`, `ProfileOptions`, etc. call `CliConfig::load_profile()` separately. For example, `extract_private_key_and_address` and then `extract_public_key` may both load and parse the YAML config file independently in the same command execution.

**Recommendation:**
- Cache the loaded `CliConfig` (e.g., using `OnceCell` or passing it as a parameter) to avoid repeated file I/O and YAML parsing.

---

### PERF-03 — Telemetry blocks command completion

**Location:** `common/utils.rs:102-109`

```rust
if let Err(err) = timeout(
    Duration::from_millis(2000),
    send_telemetry_event(command, latency, error),
).await
```

Every command execution waits up to 2 seconds for telemetry. While there is a timeout, this can add noticeable latency to every CLI invocation.

**Recommendation:**
- Reduce timeout to 500ms or fire-and-forget (spawn the telemetry send and don't await it at all in the main flow).
- Consider batching telemetry events or sending them asynchronously in the background.

---

### PERF-04 — `generate_vanity_account_ed25519` is a brute-force infinite loop

**Location:** `common/utils.rs:369-399`

```rust
loop {
    let private_key = key_generator.generate_ed25519_private_key();
    // ...
    if account_address.short_str_lossless().starts_with(vanity_prefix_ref) {
        return Ok(private_key);
    };
}
```

This function runs an unbounded loop with no progress reporting, timeout, or cancellation mechanism. A long vanity prefix (e.g., 8+ hex characters) could run for hours or indefinitely.

**Recommendation:**
- Add a `--max-attempts` or `--timeout` parameter.
- Print periodic progress (e.g., attempts per second, estimated time).
- Allow graceful cancellation via Ctrl+C handling.

---

### PERF-05 — Multi-threaded tokio runtime always created

**Location:** `main.rs:21-24`

```rust
let runtime = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()
    .unwrap();
```

A multi-threaded runtime is created for every CLI invocation, including simple commands like `aptos info` that don't need async at all.

**Recommendation:**
- For lightweight/synchronous commands, consider using `current_thread` runtime, or lazy-initialize the runtime.

---

### PERF-06 — `clone()` called on large structures unnecessarily

**Location:** Various (e.g., `common/types.rs:982-985`, `move_tool/mod.rs` payload cloning)

Multiple locations clone keys, payloads, and config structures where borrows or moves would suffice. While not critical, this creates unnecessary heap allocations.

**Recommendation:** Audit hot paths for unnecessary clones, particularly for `TransactionPayload`, `PrivateKeyInputOptions`, and `BTreeMap<String, AccountAddressWrapper>`.

---

## 5. UX / Usability Findings

### UX-01 — Inconsistent output format (stdout vs stderr)

**Location:** Throughout the codebase

The CLI mixes `eprintln!` (stderr) for progress messages and `println!` (stdout) for results, but inconsistently. Some informational messages go to stdout (e.g., `common/types.rs:2291-2292`):

```rust
println!();
println!("Simulating transaction locally...");
```

Meanwhile the final JSON result also goes to stdout (`main.rs:35`):

```rust
Ok(inner) => println!("{}", inner),
```

This makes it difficult to pipe JSON output reliably.

**Recommendation:**
- Ensure ALL informational/progress messages use `eprintln!` (stderr).
- Only JSON results should go to `println!` (stdout).
- Audit all 38+ `println!` call sites in the codebase.

---

### UX-02 — No `--output-format` option (JSON-only output)

**Location:** `common/utils.rs:86-127` (`to_common_result`)

All output is JSON. While JSON is machine-parseable, it's not always the best for human reading, especially for simple operations.

**Recommendation:**
- Add `--output-format` flag supporting `json` (default), `text`, and `yaml`.
- For human-friendly output, consider a table format for list operations.

---

### UX-03 — Error messages sometimes lack actionable guidance

Several error paths provide minimal context:

- `common/types.rs:1144`: `"No rest url given. Please add --url or add a rest_url to the .aptos/config.yaml for the current profile"` — Good.
- `common/types.rs:1380`: `"'--account' or '--profile' after using aptos init must be provided"` — Confusing grammar.
- `common/types.rs:871`: `"One of ['--private-key', '--private-key-file'], or ['public_key'] must present in profile"` — Grammar error ("must present" → "must be present").

**Recommendation:**
- Audit all error messages for grammar and clarity.
- Include suggested next steps in error messages.
- Consider linking to relevant documentation URLs in error messages.

---

### UX-04 — `KeyType::from_str` error message is inconsistent with actual options

**Location:** `common/types.rs:493`

```rust
_ => Err("Invalid key type: Must be one of [ed25519, x25519]"),
```

This error message omits `bls12381`, which is actually a valid option.

**Recommendation:** Update to `"Must be one of [ed25519, x25519, bls12381]"`.

---

### UX-05 — No colored/structured error output

**Location:** `main.rs:37`

```rust
Err(inner) => {
    println!("{}", inner);
    exit(1);
}
```

Errors are printed as plain text JSON to stdout with exit code 1. There is no color differentiation, no stderr usage for errors, and no distinction between user errors and internal errors.

**Recommendation:**
- Print errors to stderr.
- Use colored output for error severity (red for errors, yellow for warnings).
- Include exit codes that distinguish error types (e.g., 1 for user error, 2 for network error, 3 for internal error).

---

### UX-06 — Clock skew warning has a typo

**Location:** `common/types.rs:1972`

```rust
eprintln!("Local clock is is skewed from blockchain clock. ...")
```

"is is" should be "is".

**Recommendation:** Fix the typo. This message also appears in `common/transactions.rs:193`.

---

### UX-07 — `aptos init` defaults to devnet, not mainnet

**Location:** `common/init.rs:117-119`

When no network is specified interactively, the CLI defaults to devnet. This is reasonable for development but new users targeting mainnet may not realize they need to explicitly choose mainnet.

**Recommendation:**
- Keep devnet as default but add a prominent note in the prompt: "Note: Use 'mainnet' for production deployments".

---

### UX-08 — Transaction cost prompt shows Octas, not APT

**Location:** `common/types.rs:2035-2039`

```
"Do you want to submit a transaction for a range of [{} - {}] Octas at a gas unit price of {} Octas?"
```

Most users think in APT, not Octas (10^-8 APT). Showing `123456789 Octas` is much less intuitive than `1.23456789 APT`.

**Recommendation:**
- Show both APT and Octas: "Do you want to submit a transaction for approximately 1.23 APT (123000000 Octas)?"

---

### UX-09 — No shell completion out of the box

**Location:** `config/mod.rs:57-77` (`GenerateShellCompletions`)

Shell completions exist but must be manually generated via `aptos config generate-shell-completions`. The command isn't discoverable.

**Recommendation:**
- Document this in `aptos init` output or first-run experience.
- Consider printing a hint after `aptos init`: "Tip: Run `aptos config generate-shell-completions --shell bash --output-file ...` for tab completion."

---

### UX-10 — `move test` failures are a single undifferentiated error

**Location:** `common/types.rs:134`

```rust
#[error("Move unit tests failed")]
MoveTestError,
```

`MoveTestError` carries no information about which tests failed or why. The actual output is printed to the console by the test framework, but the final JSON result just says "Move unit tests failed".

**Recommendation:**
- Include test failure summary in the error (e.g., "3 of 10 tests failed: test_foo, test_bar, test_baz").

---

### UX-11 — `MoveTool` has too many subcommands (29+)

**Location:** `move_tool/mod.rs:113-151`

The `aptos move` subcommand has 29 subcommands, many with overlapping purposes (e.g., `CreateObjectAndPublishPackage` vs `DeployObject` vs `Publish`). This is overwhelming for new users.

**Recommendation:**
- Group related commands under sub-subcommands (e.g., `aptos move package compile`, `aptos move package test`, `aptos move object deploy`).
- Add aliases for common workflows.
- Hide less-used commands behind `--help-all` or mark them as advanced.

---

### UX-12 — `--assume-yes` short flag `-y` but no corresponding `-n` for `--assume-no`

**Location:** `common/types.rs:611-616`

`--assume-yes` has shorthand `-y`, but `--assume-no` does not have a corresponding shorthand.

**Recommendation:** Add `-n` as shorthand for `--assume-no` for symmetry.

---

## 6. Code Quality & Maintainability

### CQ-01 — 44+ TODO/FIXME comments in production code

There are 44+ `TODO`, `FIXME`, `HACK`, and `XXX` comments scattered across the codebase (see findings in analysis). Notable ones:

- `types.rs:2027`: "TODO: remove the hardcoded 530" — hardcoded gas value
- `types.rs:1858`: "TODO: Cache this information" — repeated config loading
- `transactions.rs:59`: "TODO: I know this is a copy..." — duplicated `TxnOptions` struct
- `config/mod.rs:426`: "TODO: When we version up..." — deferred breaking change
- `node/mod.rs:1499`: "FIXME: Remove this test, it's very fragile"

**Recommendation:** Triage all TODOs: convert to GitHub issues with owners, fix, or remove stale ones.

---

### CQ-02 — Duplicated `TransactionOptions` / `TxnOptions`

**Location:** `common/types.rs:1776+` and `common/transactions.rs:60+`

There are two nearly identical transaction options structs (`TransactionOptions` and `TxnOptions`) with the comment "Currently experimental without any worries of backwards compatibility." This creates maintenance burden and inconsistency risk.

**Recommendation:** Consolidate into a single struct, or clearly document the migration path.

---

### CQ-03 — 210+ `unwrap()` calls in non-test code

While many are in test files, there are `unwrap()` calls in production paths (e.g., `move_tool/mod.rs:21`, `node/mod.rs:25`, `genesis/mod.rs:20`). These can cause panics on unexpected input.

**Recommendation:** Replace `unwrap()` with `expect("descriptive message")` or proper error handling. Target zero `unwrap()` in non-test code.

---

### CQ-04 — `base64::decode` / `base64::encode` (deprecated API)

**Location:** `genesis/git.rs:243,259,263`

The code uses `base64::decode` and `base64::encode` which are deprecated in favor of the `base64::Engine` API.

**Recommendation:** Migrate to `base64::engine::general_purpose::STANDARD.decode()` / `.encode()`.

---

## 7. Prioritized Action Items

### Critical (address immediately)

| # | Finding | Type | Effort |
|---|---------|------|--------|
| 1 | SEC-02: Fork `self_update` under aptos-labs org, add binary signature verification | Security | Medium |
| 2 | SEC-01: Add private key CLI argument warnings and prefer safer input methods | Security | Low |

### High Priority (address within 1-2 sprints)

| # | Finding | Type | Effort |
|---|---------|------|--------|
| 3 | SEC-03: Encrypt private keys at rest in config files | Security | High |
| 4 | PERF-01: Feature-gate heavy dependencies (Docker, diesel, indexer, VM testing) | Performance | High |
| 5 | UX-01: Standardize stdout/stderr usage across all commands | UX | Medium |
| 6 | UX-05: Print errors to stderr with structured exit codes | UX | Low |
| 7 | CQ-02: Consolidate `TransactionOptions` / `TxnOptions` | Code Quality | Medium |

### Medium Priority (address within 1-3 months)

| # | Finding | Type | Effort |
|---|---------|------|--------|
| 8 | SEC-04: Validate file permissions on token/key files | Security | Low |
| 9 | SEC-05: Explicitly name env vars and hide values in help | Security | Low |
| 10 | PERF-02: Cache config file loading | Performance | Low |
| 11 | PERF-03: Reduce telemetry timeout / make fire-and-forget | Performance | Low |
| 12 | UX-02: Add `--output-format` option | UX | Medium |
| 13 | UX-03: Audit and improve all error messages | UX | Medium |
| 14 | UX-06: Fix "is is" typo in clock skew warning | UX | Trivial |
| 15 | UX-08: Show APT alongside Octas in cost prompts | UX | Low |
| 16 | UX-04: Fix `KeyType::from_str` error message | UX | Trivial |
| 17 | CQ-01: Triage all TODO/FIXME comments | Code Quality | Medium |
| 18 | CQ-03: Replace all production `unwrap()` calls | Code Quality | Medium |

### Low Priority (backlog)

| # | Finding | Type | Effort |
|---|---------|------|--------|
| 19 | SEC-06: Replace `assert!` with error return in `from_string_seed` | Security | Trivial |
| 20 | SEC-07: Add TLS certificate pinning option | Security | Medium |
| 21 | SEC-08: Add confirmation prompt to `show-private-key` | Security | Low |
| 22 | PERF-04: Add timeout/progress to vanity account generation | Performance | Low |
| 23 | PERF-05: Use single-threaded runtime for simple commands | Performance | Medium |
| 24 | PERF-06: Reduce unnecessary clones | Performance | Low |
| 25 | UX-07: Add mainnet hint in `aptos init` | UX | Trivial |
| 26 | UX-09: Surface shell completions in first-run experience | UX | Low |
| 27 | UX-10: Include test failure details in `MoveTestError` | UX | Low |
| 28 | UX-11: Reorganize `aptos move` subcommand hierarchy | UX | High |
| 29 | UX-12: Add `-n` shorthand for `--assume-no` | UX | Trivial |
| 30 | CQ-04: Migrate deprecated `base64` API usage | Code Quality | Low |

---

## Appendix: Files Analyzed

| File | Lines |
|------|-------|
| `src/lib.rs` | 103 |
| `src/main.rs` | 42 |
| `src/common/types.rs` | 2,745 |
| `src/common/utils.rs` | 710 |
| `src/common/init.rs` | 500 |
| `src/common/transactions.rs` | 391 |
| `src/config/mod.rs` | 497 |
| `src/move_tool/mod.rs` | 3,068 |
| `src/account/key_rotation.rs` | 395 |
| `src/genesis/git.rs` | 265 |
| `src/update/mod.rs` | 132 |
| `src/update/aptos.rs` | 186 |
| `src/node/local_testnet/docker.rs` | 239 |
| `Cargo.toml` | 126 |
| + 56 additional source files | ~15,200 |
| **Total** | **~24,600** |

---

*This report is awaiting instructions on which findings to act on. Please indicate which items to prioritize and I will implement the fixes.*

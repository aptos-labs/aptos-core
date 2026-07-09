# Framework Release Process

`aptos-release-tool` (this directory) is the centerpiece of an improved framework
release process. This document covers that process end to end — the current
process and its friction points (sections 1–2), and the bundle-based design the
tool implements (section 3 onward) — so it serves as both the README for the tool
and the design reference for the release process as a whole.

## Status: Partially implemented

The bundle format and the `aptos-release-tool` CLI (`generate-bundle`,
`verify-bundle`, `simulate`, `verify-framework-deployment`) are implemented. The
GitHub Actions workflows that orchestrate the process (section 3.3) live in
`internal-ops` and are added separately.

---

## 1. Current Process Overview

The framework release process spans two networks (testnet then mainnet), multiple
repositories, and several manual handoffs. A simplified view:

**Testnet:**
1. Checkout release branch
2. Generate gas schedule JSON, PR it to `aptos-networks`
3. Edit `release.yaml` (endpoint, gas refs, name), PR with 2 approvals
4. Run `generate-proposals --simulate testnet`
5. Trigger `release-framework` GH Action (internal-ops)
6. Verify on-chain (Grafana, Explorer)
7. Push framework tag

**Mainnet:**
1. Run `generate-proposals` from release branch
2. Run `simulate --network mainnet`
3. Fork Foundation repo, manually copy artifacts, PR to `mainnet-proposals`
4. Coordinate with operator to create on-chain governance proposal
5. Submit proposal to mainnet for voting (3-day window)
6. Execute proposal (operator or self-serve)
7. Verify on-chain (Explorer: PackageRegistry / GasScheduleV2)
8. Push mainnet branch (internal-ops workflow)

### Repositories involved
| Repo | Role |
|------|------|
| `aptos-labs/aptos-core` | Source of truth: framework code, release.yaml, release-builder CLI |
| `aptos-labs/aptos-networks` | Stores gas schedule snapshots (JSON) in `gas/` directory |
| `aptos-foundation/mainnet-proposals` | Stores mainnet governance proposal artifacts (metadata + sources) |
| `aptos-labs/internal-ops` | GH Actions for `release-framework`, `push-branch`, validator removal |

### Existing tooling (`aptos-release-builder` CLI)
| Subcommand | What it does |
|------------|-------------|
| `generate-proposals` | Generates Move scripts + metadata from release.yaml; optional `--simulate` |
| `simulate` | Runs generated proposals against live network state (in-memory VM) |
| `generate-gas-schedule` | Dumps current gas params as JSON |
| `validate-proposals` | Submits & executes proposals on a test network |
| `write-default` | Emits a template release.yaml |
| `print-configs` | Fetches on-chain configs (consensus, execution, gas, features, etc.) |
| `print-package-metadata` | Inspects on-chain package info |

---

## 2. Identified Friction Points

### A. Scattered, Loosely Coupled Artifacts

A single release produces artifacts across three repos (gas JSON in `aptos-networks`,
release config in `aptos-core`, metadata + sources in `mainnet-proposals`), connected
only by naming convention. No machine-readable link between them — mismatches are
caught only by human review or simulation failures that require tribal knowledge to
debug.

### B. No Self-Verification / Lack of Voter Transparency

No way to verify that metadata matches sources, gas schedules are consistent, or
bytecode was compiled from the claimed revision. Beyond internal correctness, there is
a transparency gap for governance voters: validator operators voting on proposals have
no structured way to trace from the on-chain proposal back to exact source code, build
inputs, and simulation results. They largely have to trust the release captain.

### C. Foundation Repo Fork Dance

Mainnet artifacts must be uploaded to `aptos-foundation/mainnet-proposals` via a
personal fork. The captain must keep the fork in sync, manually recreate the expected
directory layout (`metadata/v1.X.Y/`, `sources/v1.X.Y/proposal_1_.../`), and copy
files from their local machine into the correct structure. The layout is a silent
contract with the governance tooling — no validation that it's correct until someone
tries to use it. The runbook includes steps like "drag folders into terminal to see
the addresses" and hardcoded absolute paths.

### D. Local-Machine-Dependent Steps

Gas generation, proposal generation, and simulation all run on the captain's laptop.
Results depend on local toolchain, checkout state, and network conditions. Builds are
slow (`--profile release`), and simulation can fail from rate limiting without an API
key.

### E. Gas Schedule Upload Ceremony

Gas snapshot must be generated locally, PR'd into `aptos-networks`, merged, then its
raw GitHub URL manually pasted into `release.yaml`. Multiple PRs across repos for one
logical operation.

### F. Manual Post-Release Verification

Verification involves eyeballing Grafana, manually looking up `GasScheduleV2` and
`PackageRegistry` on Explorer, and cross-referencing with source code. Easy to skip,
no automated pass/fail.

### G. Manual `release.yaml` Editing (lower priority)

Each release requires hand-editing `release.yaml` to swap endpoints, update gas URLs,
and toggle commented-out blocks. Annoying but manageable — experienced captains know
the drill. Nice-to-have improvement, not a blocker.

### H. Multi-Channel Coordination (out of scope)

Real friction (multiple Slack channels, async handoffs), but organizational rather
than tooling. Out of scope for this effort.

---

## 3. Proposed Improvements

### 3.1 Release Artifact Bundle

Replace the current scattered-files approach with a single, self-contained **release
bundle** — a directory (or archive) that contains everything needed for a framework
release and can be moved as a unit.

#### Bundle Structure

```
aptos-framework-v1.45.1/
├── bundle.toml                          # Bundle manifest (see below)
├── config.yaml                          # The release config used to generate this bundle
├── metadata.json                        # Proposal metadata (title, description, URLs)
├── gas/                                 # Present only when the release changes the gas schedule
│   ├── old.json                         # Previous gas schedule snapshot
│   └── new.json                         # New gas schedule snapshot
├── scripts/                             # The proposal's multi-step governance scripts
│   ├── 0-....move
│   ├── 1-....move
│   └── ...
└── summary/                             # Human-reviewable change summaries
    ├── gas-schedule-changes.md          # Gas parameter diff with sign-off checkboxes
    └── feature-flags.md                 # Feature flag changes with sign-off checkboxes
```

A bundle holds exactly one governance proposal — a framework release or an ad-hoc
change — emitted as a single multi-step proposal whose steps are the numbered Move
scripts under `scripts/`, with `metadata.json` holding the proposal's metadata.
`config.yaml` is the exact release config the bundle was generated from, copied
verbatim, so the bundle is self-describing.

The `summary/` directory contains auto-generated, human-readable summaries of key
changes. These serve two purposes: (1) giving reviewers a quick overview without
reading raw Move scripts, and (2) optionally acting as a sign-off mechanism (see
below).

#### `bundle.toml` — Manifest

```toml
format_version = 1                       # bundle format version, for forward compatibility

[bundle]
name = "aptos-framework-v1.45.1"         # the bundle's identity; matches config.yaml's name
created_at = "2026-04-01T18:30:00Z"

[source]
branch = "aptos-release-v1.45"           # optional
commit = "abc123def456..."               # the revision the framework was built from

[integrity]
# A single content digest over the whole bundle (see "Bundle Integrity" below).
digest = "deadbeef..."

# SHA-256 of every bundle file (excluding bundle.toml itself), inline for
# self-containment.
[checksums]
"config.yaml" = "a1b2c3..."
"metadata.json" = "def012..."
"gas/old.json" = "d4e5f6..."
"gas/new.json" = "789abc..."
"scripts/0-....move" = "345678..."
# ... etc
```

#### `summary/` — Reviewable Change Summaries with Optional Sign-off

The release tooling auto-generates summary files that highlight what changed in a
human-readable format. For example, `gas-schedule-changes.md`:

```markdown
# Gas Schedule Changes

Gas feature version: 30 -> 31

- [ ] I have reviewed the gas schedule changes below.

## Changes

| change   | parameter             |       old |        new | sign-off |
| -------- | --------------------- | --------: | ---------: | -------- |
| modified | instr.add             |        50 |         65 |          |
| added    | instr.mul             |         / |         90 |          |
| removed  | instr.sub             |        80 |          / |          |
| modified | txn.max_execution_gas | 920000000 | 1000000000 | [ ]      |
```

The top checkbox ensures the reviewer scans the changes before signing off. Critical
gas parameters (e.g. `txn.max_execution_gas`, `txn.max_io_gas`,
`txn.max_transaction_size_in_bytes`) additionally get their own per-change `[ ]` in the
sign-off column, forcing explicit acknowledgment. 

`verify-bundle --require-signoff` optionally enforces that every box is ticked; 
it is opt-in (off by default) to avoid friction. Ticking boxes never invalidates 
the bundle's checksums — summary checkboxes are normalized away before hashing 
(see Bundle Integrity).

#### Bundle Integrity

Integrity is two-layered:

1. **Per-file checksums** — `[checksums]` holds a SHA-256 of every bundle file
   (except `bundle.toml`), catching any changed, added, or removed file.
2. **Global digest** — `integrity.digest` is a single hash over the bundle's identity,
   computed using its name, the source commit, and the sorted list of per-file checksums. 
   It excludes volatile fields (`created_at`, `branch`, `format_version`) so regenerating 
   from the same source reproduces the same digest.

`verify-bundle` checks the bundle is internally self-consistent:

```bash
cargo run -p aptos-release-tool -- verify-bundle --bundle aptos-framework-v1.45.1/
```

It confirms the checksums and global digest match, the manifest and `config.yaml`
agree, and the expected layout is present; `--require-signoff` additionally requires
every summary checkbox to be ticked.

Note this verifies self-consistency, not provenance — it does not by itself confirm the
bundle was built from the commit it claims.

---

### 3.2 New CLI Commands

Every step of the release process maps to a CLI subcommand in a new tool (separate
from `aptos-release-builder`). The GitHub Actions workflows (section 3.3) are
convenience wrappers around these commands — if automation fails, the captain can
fall back to running the same commands manually.

| Command | What it does |
|---------|-------------|
| `generate-bundle` | Builds a complete bundle from a release config, and self-verifies it before returning. |
| `verify-bundle` | Checks the bundle is internally self-consistent; `--require-signoff` also requires every summary checkbox to be ticked. |
| `simulate` | Simulates the bundle's governance proposal against a network (reuses `aptos-release-builder`). |
| `verify-framework-deployment` | Checks a deployed framework release on-chain against the bundle — currently the gas schedule only (bytecode verification is a TODO). |

Example usage:

```bash
# Generate a bundle (it self-verifies before returning)
cargo run -p aptos-release-tool -- generate-bundle \
    --release-config aptos-move/aptos-release-tool/data/framework-release.yaml \
    --bundle "$BUNDLE_DIR"

# Verify it
cargo run -p aptos-release-tool -- verify-bundle \
    --bundle "$BUNDLE_DIR"

# Simulate against testnet
cargo run -p aptos-release-tool -- simulate \
    --bundle "$BUNDLE_DIR" \
    --network testnet

# After deployment, verify the framework release on-chain
cargo run -p aptos-release-tool -- verify-framework-deployment \
    --bundle "$BUNDLE_DIR" \
    --network testnet
```

---

### 3.3 GitHub Actions Workflows

Three workflows (which live in `internal-ops`) automate the release process. All operate
on release bundles committed to `aptos-networks`. Gas schedule snapshots are part of the
bundle — no separate upload required.

#### Workflow 1: Generate Release Bundle

Produces a bundle from a release branch and opens a PR adding it to `aptos-networks`
under `framework-releases/<version>/`.

```yaml
on:
  workflow_dispatch:
    inputs:
      release_branch:   # e.g. aptos-release-v1.45 (required)
      release_config:   # path on that branch; defaults to data/framework-release.yaml
      dry_run:          # generate + verify only, skip the PR
```

The tool is **built from the release branch**, not main: the new ("current") gas
schedule is compiled into the binary, so it must come from the branch being released.
The bundle name — and the `framework-releases/<version>` directory it lands in — is
derived from the config's `name` (with the `aptos-framework-` prefix stripped for the
version).

```
Steps:
  1. Checkout the release branch
  2. Build aptos-release-tool from the branch
  3. generate-bundle --release-config <config> --bundle <name>
     (gas snapshots, scripts, metadata, summaries, bundle.toml; self-verifies)
  4. verify-bundle (dedicated step, so verification is visible in the job UI)
  5. Upload the bundle as a CI artifact
  6. Unless dry_run: PR the bundle into aptos-networks/framework-releases/<version>
```

Ideally the same bundle generated for testnet is reused for mainnet. If new changes are
cherry-picked onto the release branch after testnet deployment (or after initial
generation), the bundle must be regenerated; the PR branch name is deterministic, so
regenerating updates the existing PR rather than opening a new one.

#### Workflow 2: Deploy to Testnet (planned, not yet implemented)

Takes a committed bundle, simulates it against testnet, and if successful, executes
the full testnet release — including the governance proposal, framework tag, branch
update, and post-deploy verification.

```yaml
on:
  workflow_dispatch:
    inputs:
      bundle_name:
        description: "Bundle name (e.g. aptos-framework-v1.45.1)"
        required: true
```

```
Steps:
  1. Fetch the bundle from aptos-networks
  2. Run simulate --network testnet
  3. Run verify-bundle
  4. Trigger release-framework (internal-ops)
  5. Update testnet branch (push-branch workflow)
  6. Verify deployment on-chain (gas version, package registry)
  7. Push framework tag (aptos-framework-vX.Y.Z)
```

This replaces the current process where the captain runs simulation locally, triggers
the internal-ops workflow from the GitHub UI, manually checks Explorer, and pushes the
tag from their laptop.

#### Workflow 3: Simulate Governance Proposals (planned, not yet implemented)

Runs simulation on a committed bundle against a specified network. Does not deploy
anything — purely a verification step.

```yaml
on:
  workflow_dispatch:
    inputs:
      bundle_name:
        description: "Bundle name (e.g. aptos-framework-v1.45.1)"
        required: true
      network:
        description: "Network to simulate against"
        required: true
        type: choice
        options: [testnet, mainnet]
```

```
Steps:
  1. Fetch the bundle from aptos-networks
  2. Run simulate --network $network
  3. Run verify-bundle
  4. Report pass/fail
```

The primary use case is simulating against mainnet before announcing a release to
validator operators. The captain runs this on the same bundle that was deployed to
testnet, confirming it works against current mainnet state before coordinating with
operators and the Foundation.

---

## 4. New Release Process

### 4.1 Automated Release Process (End-to-End)

How a release captain uses the three workflows to complete a full release cycle.

#### Testnet Release

```
┌─────────────────────────────────────┐
│ Trigger "Generate Bundle" workflow  │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│ Review bundle PR in aptos-networks  │
│ (check summary files, sign off,     │
│  merge)                             │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│ Trigger "Deploy to Testnet"         │
│ workflow                            │
│                                     │
│ (automated: simulate, deploy,       │
│  verify, tag, update branch)        │
└─────────────────────────────────────┘
```

#### Mainnet Release

```
┌─────────────────────────────────────┐
│ Cherry-picks since testnet?         │
└──────┬─────────────────┬────────────┘
       │                 │
      yes                no
       │                 │
       ▼                 │
┌──────────────────┐     │
│ Regenerate       │     │
│ bundle           │     │
└───────┬──────────┘     │
        │                │
        ▼                ▼
┌─────────────────────────────────────┐
│ Trigger "Simulate" workflow         │
│ (mainnet)                           │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│ Coordinate with operator &          │
│ Foundation (create on-chain         │
│ proposal, initiate voting,          │
│ 3-day window)                       │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│ Execute proposal (simulation runs   │
│ before executing for real, verify   │
│ on-chain after)                     │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│ Update mainnet branch (push-branch) │
└─────────────────────────────────────┘
```

Note: mainnet deployment still involves manual coordination (operator, Foundation,
voting). The automation covers artifact generation, simulation, and verification.

---

### 4.2 Manual Process (Fallback)

If the GitHub Actions workflows are unavailable or fail partway through, the captain
can complete the release manually using the CLI commands from section 3.2. The bundle
is the same artifact regardless of whether CI or a human drives the process.

#### Testnet Release (manual)

```
1. Generate the bundle (verify-bundle runs automatically):
     generate-bundle --release-config framework-release.yaml --bundle aptos-framework-v1.45.1

2. Review summary files, tick checkboxes if required

3. (Optional) Simulate against testnet:
     simulate --bundle aptos-framework-v1.45.1 --network testnet

4. Commit the bundle to aptos-networks (manual PR)

5. Trigger release-framework workflow (internal-ops GitHub UI)

6. Update testnet branch (push-branch workflow in internal-ops)
   — may be done automatically by the release workflow

7. Verify the framework release on-chain:
     verify-framework-deployment --bundle aptos-framework-v1.45.1 --network testnet

8. Push framework tag:
     git tag -f aptos-framework-v1.45.1 && git push -f origin refs/tags/aptos-framework-v1.45.1
```

#### Mainnet Release (manual)

```
1. If cherry-picks landed since testnet, regenerate:
     generate-bundle --release-config framework-release.yaml --bundle aptos-framework-v1.45.1
   Otherwise, reuse the testnet bundle.

2. Simulate against mainnet:
     simulate --bundle aptos-framework-v1.45.1 --network mainnet

3. Coordinate with operator & Foundation (create on-chain proposal,
   initiate voting, 3-day window)

4. Execute proposal (simulation runs before executing for real,
   verify on-chain after)

5. Update mainnet branch (push-branch workflow in internal-ops)
```

The manual process is intentionally similar to today's, but with less friction: the
bundle eliminates scattered artifacts, `verify-bundle` catches mismatches early,
`verify-framework-deployment` automates the on-chain gas-schedule check, and the summary
files provide a clear review surface.

---

## 5. Migration Path

### Phase 1: Build new tooling alongside existing
- Build a new CLI tool (separate from `aptos-release-builder`) with the new commands
  (`generate-bundle`, `verify-bundle`, `simulate`, `verify-framework-deployment`), and
  the three GitHub Actions workflows (generate, deploy-testnet, simulate)
- Leave `aptos-release-builder` and existing workflows in place — ongoing releases
  continue to use the existing process without disruption

### Phase 2: Trial run
- Use the new tools and workflows for one release end-to-end
- Fall back to the existing process if issues arise

### Phase 3: Deprecate old tooling
- Once confident the new process is stable, remove the old tooling and workflows

---

## 6. Open Questions

1. **Where should bundles live?** Using `aptos-networks` for now.
   `mainnet-proposals` would be a more natural home since it already stores mainnet
   release artifacts, but the name is restrictive — it doesn't fit testnet-only
   releases. A rename of that repo could resolve this.

2. **Operator coordination**: Can governance proposal submission be automated via
   a GitHub workflow, or does it inherently require the operator? What does the
   operator actually do that tooling can't?

3. **Sign-off enforcement**: Should summary checkboxes be enforced by tooling
   (`verify-bundle --require-signoff`) or by process (PR review)? Needs evaluation
   of friction before deciding.

4. **Scope of `internal-ops`**: The `release-framework` and `push-branch` workflows
   live in internal-ops. Should the deploy-testnet workflow call them as-is, or
   should they be migrated?

---

## 7. TODO

1. **Private releases**: How should releases from `aptos-core-private` (security
   fixes) be handled? The bundle and verification tooling should work, but the
   process around visibility, artifact storage, and review needs to be defined.

2. **Ad-hoc governance proposals**: The same release tooling (bundles, governance
   simulation, proposal submission, etc.) needs to work for ad-hoc governance
   proposals that are not part of a regular framework release. However, some of
   the GitHub workflows (e.g., generate bundle from a release branch) may not
   apply. What process should ad-hoc proposals follow?

3. **Framework source diff in summaries**: Add a framework diff (against the
   prior release) under `summary/`, optionally with an AI-generated summary, so
   reviewers can see what Move code changed in each release.

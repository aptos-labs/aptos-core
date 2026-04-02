# Framework Release Process: Friction Analysis & Improvement Proposal

## Status: Early Planning (Draft)

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
├── release.yaml                         # The release config used to generate this bundle
├── gas/
│   ├── old.json                         # Previous gas schedule snapshot
│   └── new.json                         # New gas schedule snapshot
├── proposals/
│   └── proposal_1_upgrade_framework/
│       ├── metadata.json                # Proposal metadata (title, description, URLs)
│       ├── 0-gas-schedule.move
│       ├── 1-move-stdlib.move
│       ├── 2-aptos-stdlib.move
│       ├── 3-aptos-framework.move
│       ├── 4-aptos-token.move
│       └── 5-aptos-token-objects.move
└── summary/                             # Human-reviewable change summaries
    ├── gas-schedule-changes.md          # Gas parameter diff with sign-off checkboxes
    └── feature-flags.md                 # Feature flag changes with sign-off checkboxes
```

Each proposal directory is self-contained: its metadata and Move scripts live
together, so there is no possibility of a metadata/source mismatch when copying files
around.

The `summary/` directory contains auto-generated, human-readable summaries of key
changes. These serve two purposes: (1) giving reviewers a quick overview without
reading raw Move scripts, and (2) optionally acting as a sign-off mechanism (see
below).

#### `bundle.toml` — Manifest

```toml
[release]
name = "v1.45.1"
created_at = "2026-04-01T18:30:00Z"

[source]
branch = "aptos-release-v1.45"
commit = "abc123def456..."
release_tag = "aptos-node-v1.45.1"

[gas]
old_version = "v1.44.0"
new_version = "v1.45.1"
gas_feature_version = 49                 # from ver.rs

[framework]
bytecode_version = 8
packages = ["move-stdlib", "aptos-stdlib", "aptos-framework", "aptos-token", "aptos-token-objects"]

# SHA-256 checksums for all bundle files, inline for self-containment.
[checksums]
"release.yaml" = "a1b2c3..."
"gas/old.json" = "d4e5f6..."
"gas/new.json" = "789abc..."
"proposals/proposal_1_upgrade_framework/metadata.json" = "def012..."
"proposals/proposal_1_upgrade_framework/0-gas-schedule.move" = "345678..."
# ... etc
```

#### `summary/` — Reviewable Change Summaries with Optional Sign-off

The release tooling auto-generates summary files that highlight what changed in a
human-readable format. For example, `gas-schedule-changes.md`:

```markdown
# Gas Schedule Changes: v1.44.0 → v1.45.1

Gas feature version: 48 → 49

- [ ] I have reviewed the gas schedule changes below.

## Changes

+/- txn.max_execution_gas: 1000000 → 1500000          [ ]
+/- storage.per_item_read: 300 → 350
+   instr.vec_push_back_per_elem: 20
```

The checkbox ensures the reviewer at least scans the changes before sign-off. For
particularly important changes (such as certain gas parameters like
`max_execution_gas`, `max_io_gas`, `max_transaction_size_in_bytes`, etc.), the tooling
could generate additional per-change checkboxes to force explicit acknowledgment.
The tooling could optionally enforce that all boxes are checked before allowing the
bundle to proceed (e.g., `verify-bundle --require-signoff`), but this should be
evaluated for how much friction it adds before making it mandatory.

#### Bundle Integrity Verification

A new CLI subcommand verifies the bundle is self-consistent:

```bash
cargo run -p aptos-release-tool -- verify-bundle --path aptos-framework-v1.45.1/
```

Checks performed:
- All files in `[checksums]` exist and their hashes match
- `bundle.toml` fields are consistent with `release.yaml`
- Each proposal directory contains both `metadata.json` and its Move scripts
- `source.commit` matches the git revision the framework was compiled from
- (Optional) Summary files have all checkboxes ticked

---

### 3.2 New CLI Commands

Every step of the release process maps to a CLI subcommand in a new tool (separate
from `aptos-release-builder`). The GitHub Actions workflows (section 3.3) are
convenience wrappers around these commands — if automation fails, the captain can
fall back to running the same commands manually.

| Command | What it does |
|---------|-------------|
| `generate-bundle` | Generates a complete release bundle. Takes a `release.yaml` path and output directory. Produces the full bundle directory: gas snapshots, proposals, metadata, summaries, `bundle.toml` with checksums. Version and gas info are read from the release config. |
| `verify-bundle` | Validates bundle integrity. Checks file checksums, consistency between `bundle.toml` and `release.yaml`, metadata/script pairing, and optionally that summary checkboxes are ticked. |
| `simulate` | Runs governance proposal simulation against a network. Already exists; unchanged, but now operates on a bundle's `proposals/` directory. |
| `verify-deployment` | Checks on-chain state after deployment. Fetches `GasScheduleV2`, `PackageRegistry`, and `Version` from the target network and compares against `bundle.toml` values. Reports pass/fail. |

Example usage:

```bash
# Generate a bundle
cargo run -p aptos-release-tool -- generate-bundle \
    --release-config aptos-move/aptos-release-builder/data/release.yaml \
    --output-dir aptos-framework-v1.45.1

# Verify it
cargo run -p aptos-release-tool -- verify-bundle \
    --path aptos-framework-v1.45.1

# Simulate against testnet
cargo run -p aptos-release-tool -- simulate \
    --path aptos-framework-v1.45.1/proposals \
    --network testnet

# After deployment, verify on-chain
cargo run -p aptos-release-tool -- verify-deployment \
    --bundle aptos-framework-v1.45.1 \
    --network testnet
```

---

### 3.3 GitHub Actions Workflows

Three workflows automate the release process. All operate on release bundles committed
to `aptos-networks` (TBD — see open questions). Gas schedule snapshots are part of the
bundle — no separate upload required.

#### Workflow 1: Generate Release Bundle

Produces a release bundle from a release branch and commits it to `aptos-networks`
via auto-PR.

```yaml
on:
  workflow_dispatch:
    inputs:
      release_branch:
        description: "Release branch (e.g. aptos-release-v1.45)"
        required: true
```

Version and gas old version are read from `release.yaml` on the release branch.

```
Steps:
  1. Checkout release branch
  2. Read version and gas info from release.yaml
  3. Generate gas schedule snapshots (old + new)
  4. Generate proposals + metadata
  5. Generate summary files (gas diff, feature flag changes, etc.)
  6. Populate bundle.toml (commit, branch, checksums, etc.)
  7. Run verify-bundle (integrity check)
  8. PR the bundle to aptos-networks
```

Ideally the same bundle generated for testnet is reused for mainnet. If new changes
are cherry-picked onto the release branch after testnet deployment (or after initial
generation), the bundle must be regenerated.

#### Workflow 2: Deploy to Testnet

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

#### Workflow 3: Simulate Governance Proposals

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
     generate-bundle --release-config release.yaml --output-dir aptos-framework-v1.45.1

2. Review summary files, tick checkboxes if required

3. (Optional) Simulate against testnet:
     simulate --path aptos-framework-v1.45.1/proposals --network testnet

4. Commit the bundle to aptos-networks (manual PR)

5. Trigger release-framework workflow (internal-ops GitHub UI)

6. Update testnet branch (push-branch workflow in internal-ops)
   — may be done automatically by the release workflow

7. Verify deployment:
     verify-deployment --bundle aptos-framework-v1.45.1 --network testnet

8. Push framework tag:
     git tag -f aptos-framework-v1.45.1 && git push -f origin refs/tags/aptos-framework-v1.45.1
```

#### Mainnet Release (manual)

```
1. If cherry-picks landed since testnet, regenerate:
     generate-bundle --release-config release.yaml --output-dir aptos-framework-v1.45.1
   Otherwise, reuse the testnet bundle.

2. Simulate against mainnet:
     simulate --path aptos-framework-v1.45.1/proposals --network mainnet

3. Coordinate with operator & Foundation (create on-chain proposal,
   initiate voting, 3-day window)

4. Execute proposal (simulation runs before executing for real,
   verify on-chain after)

5. Update mainnet branch (push-branch workflow in internal-ops)
```

The manual process is intentionally similar to today's, but with less friction: the
bundle eliminates scattered artifacts, `verify-bundle` catches mismatches early,
`verify-deployment` replaces manual Explorer checks, and the summary files provide a
clear review surface.

---

## 5. Migration Path

### Phase 1: Build new tooling alongside existing
- Build a new CLI tool (separate from `aptos-release-builder`) with the new commands
  (`generate-bundle`, `verify-bundle`, `verify-deployment`), and the three GitHub
  Actions workflows (generate, deploy-testnet, simulate)
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

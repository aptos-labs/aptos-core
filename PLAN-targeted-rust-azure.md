# PLAN: Migrate targeted Rust PR unit tests from AWS to Azure GitHub-hosted runners

Status: **CLOSED — delivered by PR20773 on main; this branch's patch was redundant and is dropped.**

PR20773 **MERGED** 2026-10-26 21:29 UTC (merge commit `a8f8c33289`). Note: an earlier local
`origin/main` was stale (`c29a96451b`) and initially made it look like the targeted job was
unmigrated; after a correct fetch, true main = `a8f8c33289` and **already contains the full
targeted-job migration**, improved beyond this branch's draft (node24 checkout pin, cleaner
`provider == 'aws'` predicate on dev_setup) plus a broader contract test
(`test_remaining_jobs_use_provider_with_aws_fallback`) covering the same guarantees my
targeted-routing test asserted. My local patch added nothing main lacked, so it was dropped and
this branch reset clean to `a8f8c33289`. No independent commit/push remains.

Verified on clean main: contract tests 6/6 OK; actionlint structural clean (only pre-existing
label noise). Main-push run 40404838980: full suite green on `64cpu-aptos-image-ubuntu22-x64`;
`rust-targeted-unit-tests` correctly skipped (push event → full tests, not targeted). Targeted
hosted runs occur on dev PRs; live cold/warm duration for the targeted job is observable on the
next ordinary PR (no synthetic dispatch needed).

Corrections applied at approval:
- **(1) Warm-validation narrowing:** a warm second commit on a PR that still contains `lint-test.yaml` does NOT narrow selection — `lint-test.yaml` is a `global_test_inputs` entry, so full legacy selection holds until the migration merges. Narrowing is validated post-merge on a separate representative diff / explicit pinned comparison, not asserted as pre-merge behavior.
- **(2) GIT_CREDENTIALS scope:** the input is declared but unused by `rust-targeted-unit-tests/action.yaml` steps (confirmed). No claim is made that `rust-lints` or `rust-check-merge-base` consume it without per-file source confirmation.
- **(3) smoke-tests dormancy:** the smoke-tests reusable workflow executes as nested jobs inside other runs, so zero standalone `smoke-tests.yaml` runs does NOT establish dormancy. The §7 'dormant' label is withdrawn; usage must be measured via nested job records, not the standalone-runs API.
- **(4) Cost proxy:** the §4 sample is successful-runs-only and undercounts failed/canceled usage; duration × rate is a proxy, not actual AWS dollars. Treated as an order-of-magnitude signal only.

## 1. Task (restated)

Prepare the next narrowly scoped AWS→Azure CI migration: move **only** the `rust-targeted-unit-tests`
PR job in aptos-labs/aptos-core from AWS self-hosted runners (runs-on,c7) to the existing
GitHub-hosted Azure image runner pool, preserving test selection, the required check name, and
adding an AWS kill switch. Stop at this plan; no code changes yet. Do not copy or stack PR20773's
unmerged implementation.

## 2. Current behavior (verified today, branch e3edc594f9)

- `rust-targeted-unit-tests` (`.github/workflows/lint-test.yaml:116-139`) runs on
  `runs-on,cpu=64,family=c7,disk=large,image=aptos-ubuntu-x64` (AWS, c7 64-vCPU).
  It is gated by `run_targeted_tests == 'true'` (line 117).
- The classifier `.github/actions/lint-test-classifier/classify_lint_event.py` sets
  `run_targeted_tests = development_pr` (pull_request to non-release branch). Verified.
- Checkout pins `ref: github.event.pull_request.head.sha` with `fetch-depth: 0` (line 121-124).
- Setup: `scripts/dev_setup.sh -b -p -r -y -P -t` (installs protoc, Move Prover tools z3/cvc5/
  dotnet/boogie, PostgreSQL, build tools). Then `.github/actions/rust-targeted-unit-tests/action.yaml`.
- The action starts `docker run cimg/postgres:14.2`, then runs
  `cargo x --base "$BASE_REF" targeted-unit-tests -vvv --profile ci --cargo-profile ci --locked --no-fail-fast --retries 3`
  with `APTOS_TEST_DETERMINATOR = DETERMINATOR_MODE`, `TEST_BASE_REF = BASE_REF`.
- Tool env assumed from dev_setup: `Z3_EXE=/home/runner/bin/z3`, `CVC5_EXE`, `DOTNET_ROOT`,
  `BOOGIE_EXE` (action.yaml lines 40-45).
- Selection implementation: `devtools/aptos-cargo-cli/src/test_selection.rs`. Repo variable
  `APTOS_TEST_DETERMINATOR` is currently `subsystem` (verified via `gh variable get`).
  **In subsystem mode, `.github/workflows/lint-test.yaml` is a `global_test_inputs` entry
  (`.config/test-subsystems.toml:14`)**, so editing this workflow forces the full legacy
  package selection on any PR that contains the change. Expected during rollout; not an error.
- **Credentials:** the action's `GIT_CREDENTIALS` input is declared (action.yaml:4-6) but never
  consumed by any step inside `rust-targeted-unit-tests/action.yaml`. No `git config` or
  credential write occurs in this job's steps. This is why the plan can drop the secret from the
  migrated job safely; AWS fallback preserves current behavior. This differs from `rust-lints`/
  `rust-check-merge-base`, which do consume GIT_CREDENTIALS via rust-setup.
- Branch protection requires the check name `rust-targeted-unit-tests` (verified via
  `gh api repos/aptos-labs/aptos-core/branches/main/protection`). The job name must not change
  on the required path.

## 3. Azure pool & image (verified, no new infra needed)

- Org runner group **Default (id 1)** has `visibility: all` → aptos-core covered.
- Pool `64cpu-aptos-image-ubuntu22-x64` = runner id 41, `maximum_runners: 500`, in Default group.
  Sibling pools `32cpu-aptos-image-ubuntu22-x64` (id 40, max 496) and `96cpu-…` (id 42, max 50) exist.
- Image `aptos-runner-image-ubuntu22-x64` v1.0.0 is the backing custom image (per established
  context). PR20773's `hosted-full-suite` checks **pass on this pool today** (e.g. run
  39519698874: unit-test-shards and vm-and-doc-tests green), proving the toolchain image contract.
- Image manifest contract (from PR20773's `rust-prebuilt-setup`): `/opt/aptos-rust-image.json`
  sha256 manifest, plus z3/cvc5/boogie/dotnet/protoc/nextest at `/home/runner/...` paths that
  exactly match the targeted action's env vars. Targeted tests need docker for postgres;
  the hosted images include docker (PR20773 verifies `docker info`).
- GitHub Linux B64 hosted billing meter verified `isazurecrediteligible=True` (established context).

## 4. Evidence — current AWS targeted-test cost attribution

Window: **2026-10-14 → 2026-10-21 (7 days)**, `Lint+Test` workflow, `pull_request` events,
`status=success` runs. Job records pulled via `gh api .../runs/{id}/jobs`.

- 66 successful PR workflow runs; 65 ran `rust-targeted-unit-tests`, 1 skipped it.
  **65/65 executed on AWS runs-on instances; 0 on hosted.** (hosted only appears on PR20773's own
  branch runs for other jobs.)
- Duration distribution (n=65): min 2.0m, p25 13.9m, **median 41.3m**, p75 52.3m, p90 56.3m,
  max 72.1m, mean 33.6m. Total ≈ **2185 job-min over 7 days ≈ 312 job-min/day ≈ 9.3 runs/day.**
- These are 64-vCPU c7 job-minutes. Precise dollar attribution is a **runtime × rate proxy**:
  no per-job AWS billing export was pulled here; the $8k/month aptos-ci figure (established
  context) covers all AWS CI, and the unresolved invoice-vs-export gap means no finer claim
  is made. Targeted PR tests are one of several 64-vCPU consumers (see §7).

## 5. Proposed change (minimal, no broad refactor)

Dependency: **PR20773 must merge first** (it delivers `rust-prebuilt-setup`, the provider
selector pattern, and image-contract verification), **or** the owner explicitly approves
stacking. This plan assumes PR20773 merged; stacking path noted in §8.

Change **one job** in `.github/workflows/lint-test.yaml` (`rust-targeted-unit-tests`, lines
116-139), mirroring PR20773's `rust-lints` pattern:

1. `needs: [file_change_determinator, rust-ci-provider]` and conditional `runs-on`:
   hosted → `64cpu-aptos-image-ubuntu22-x64`; aws → current
   `runs-on,cpu=64,family=c7,disk=large,image=aptos-ubuntu-x64,run-id=${{ github.run_id }}`.
2. Add `rust-prebuilt-setup` step gated to hosted, `cache-key: hosted-rust-targeted`, replacing
   `dev_setup.sh` on hosted only. **Do not run `dev_setup.sh` on hosted** — the image already
   provides all tools; dev_setup's apt/cargo installs are redundant and would reintroduce the
   slow path. Keep `dev_setup.sh -b -p -r -y -P -t` on the AWS path unchanged.
3. Field sources, explicitly (no invented values):
   - `BASE_REF`: unchanged, `origin/${{ github.base_ref || 'main' }}` (workflow line 138).
   - Head SHA: unchanged, `ref: github.event.pull_request.head.sha` (workflow line 122).
   - `DETERMINATOR_MODE`: unchanged, `${{ vars.APTOS_TEST_DETERMINATOR || 'legacy' }}` (line 137).
     Current repo value = `subsystem`.
   - `GIT_CREDENTIALS`: **absent on hosted, rationale:** the targeted action never reads this
     input (§2); the hosted image uses `persist-credentials: false` checkout and needs no
     private-repo credential. AWS path keeps `${{ secrets.GIT_CREDENTIALS }}` byte-identical.
4. Keep the job `name:` and `id` `rust-targeted-unit-tests` unchanged (branch protection).
5. Kill switch: the existing `RUST_CI_PROVIDER` repo variable (set to `aws`) plus the
   `workflow_dispatch` `rust_ci_provider` choice input, both introduced by PR20773 — this
   migration adds no new switch of its own.
6. Add `rust-targeted-unit-tests` to PR20773's `test_hosted_ci_workflows.py` contract test
   (provider gating + check-name preservation assertions), in the same style as existing tests.

Do-not-touch: the action itself (`.github/actions/rust-targeted-unit-tests/action.yaml`),
the classifier, `test_selection.rs`, `test-subsystems.toml`, `nextest.toml`, `dev_setup.sh`,
`rust-lints`, `rust-check-merge-base`, `rust-unit-tests`, and all other workflows.

## 6. Validation design

- **Contract test (free, fast):** extend `test_hosted_ci_workflows.py`; runs on `ubuntu-24.04`.
- **Cold validation (first hosted run, empty rust-cache):** open a **test PR** touching a single
  leaf Rust file. Because `lint-test.yaml` is a `global_test_inputs` entry, that PR's targeted
  job runs the *full legacy* selection — a worst-case cold run that bounds the risk. Verify:
  (a) job schedules on `64cpu-aptos-image-ubuntu22-x64`, (b) postgres container starts,
  (c) `cargo x targeted-unit-tests` selects identical packages vs the same diff on AWS (compare
  "Test packages (N)" line), (d) green result, (e) duration vs the §4 distribution.
- **Warm validation (corrected):** measure warm-cache duration on a second commit, but do NOT
  expect narrowed selection on that PR — it still contains `lint-test.yaml` (a global_test_input).
  Selection narrowing is validated **post-merge** on a separate representative diff via an explicit
  pinned base/head comparison, not asserted as pre-merge behavior.
- **False-success prevention:** the plan fails loudly if the package selection printed in the
  hosted log differs from the AWS log for the identical diff, or if the job exits 0 with an
  empty/unexpected package list. `verify-results`-style aggregate is out of scope (targeted is a
  single job, not sharded).
- **Fallback semantics:** setting `RUST_CI_PROVIDER=aws` (repo admin action, not performed here)
  returns the job to the byte-identical AWS path, including `dev_setup.sh` and GIT_CREDENTIALS.
- **Permission gates:** no repo-variable, branch-protection, runner-group, or image changes are
  made by this task. Any `RUST_CI_PROVIDER` flip is a repo-admin action requiring explicit approval.

## 7. Other cheap portable high-spend follow-ups (evidence)

- **Docker image builds** (`docker-build-test.yaml`, `workflow-run-docker-rust-build.yaml`):
  sampled 5 successful runs → **mean ≈77 AWS job-min/run**; ≥100 runs in the same 7-day window.
  Largest single AWS consumer found; flagged as higher complexity (registry/cache/disk — bake.hcl
  layer caching at docker-build-test.yaml:61-63) and **not** part of this migration.
- `lint-test-gated.yaml`: label/push-gated; 1 of 5 sampled runs used AWS (~94 min); sporadic.
- `rust-client-tests.yaml`: all sampled runs skipped on PRs (non-AWS); low value.
- `smoke-tests.yaml`: usage NOT established. Zero standalone runs is an artifact of it being a
  reusable workflow executed as nested jobs inside other runs; dormancy must be measured via nested
  job records before any claim. Withdrawn as a follow-up candidate pending that measurement.

## 8. Risks & alternatives

- **Rollout skew:** any PR containing this change runs full legacy selection (test-subsystems.toml
  global input). Bounded to the one rollout PR; noted in §5/§6. Not a defect.
- **Stacking instead of waiting:** if PR20773 stalls, an owner-approved alternative is to vendor
  *only* the `rust-prebuilt-setup` action + provider selector into this branch. Not chosen by
  default because it duplicates unmerged code under active review (Stelian's static-group revision).
- **Image staleness:** if the hosted image's `/opt/aptos-rust-image.json` hashes fall behind main's
  toolchain (rust-toolchain.toml currently 1.98.1), `rust-prebuilt-setup` fails the job loudly
  rather than silently mis-testing — this is the intended guard.
- **Disk headroom:** targeted tests build fewer crates than the full suite; hosted `disk=large`
  equivalent on the 64cpu image pool is assumed sufficient but unverified until the cold run.

## 9. Rollback

Pure YAML + contract-test change on one branch. Revert the commit, or set `RUST_CI_PROVIDER=aws`,
or dispatch with `rust_ci_provider=aws`. No runner pools, images, variables, or protection rules
are modified by this change, so nothing outside the workflow file needs unwinding.

## 9a. Implementation outcome (local, this branch)

Dependency base: merged `origin/jon/hosted-rust-ci` @ `854304e1` (PR20773 live head, confirmed
OPEN and matching) into `jon/targeted-rust-azure` — clean merge, no conflicts. PR20773 changes are
excluded from the task diff below; after PR20773 merges, rebase this branch on updated main before
the follow-up PR. No vendored/duplicate shared action; `rust-prebuilt-setup` is reused as-is.

Task diff vs PR20773 base `854304e1` (2 files, +48/-4):
- `.github/workflows/lint-test.yaml` — `rust-targeted-unit-tests`: added `rust-ci-provider` to
  `needs`; conditional `runs-on` (hosted pool `64cpu-aptos-image-ubuntu22-x64` vs unchanged AWS
  `format(...)` label); added `rust-prebuilt-setup` (hosted-only, `cache-key: hosted-rust-targeted`);
  gated `dev_setup.sh` to `provider != 'hosted'`. Classifier, required job name/id, HEAD ref,
  `BASE_REF`, `DETERMINATOR_MODE`, and `GIT_CREDENTIALS` wiring all unchanged.
- `.github/scripts/test_hosted_ci_workflows.py` — removed the stale "targeted is AWS-only"
  assertion from `test_full_suite_checks_and_aggregate`; added
  `test_targeted_unit_tests_provider_routing` (needs set, dual-provider runs-on, mutually
  exclusive setup predicates, preserved BASE_REF/DETERMINATOR/HEAD sources).

Local checks (all PASS, no installs/heavy builds):
- `python3 -m unittest discover -s .github/scripts -p 'test_*.py'` → 5/5 OK.
- `actionlint` 1.7.12 on `lint-test.yaml` + both reusable/validate workflows → zero
  structural/syntax errors; only pre-existing `2cpu-gh-ubuntu24-x64` runner-label noise (the repo's
  `.github/actionlint.yaml` predates this task and doesn't whitelist hosted labels; line 148 reuses
  the same labels as PR20773's line 97, introducing no new unknown). No repo-config edit made.
- comment-trim: 5 introduced comments reviewed, 0 trims (all state non-obvious why/contract).

Live-validation dependency (NOT done — gated on your approval + PR20773 merge): cold/warm hosted
runner runs require pushing this branch + a test PR, which dispatches paid Azure/AWS CI and is not
authorized. Fallback verification requires flipping `RUST_CI_PROVIDER` (repo-admin action).

## 10a. Follow-up cost attribution (7-day window 2026-10-19→26, read-only gh, proxy only)

Method: per-workflow run counts over 7d, then per-run AWS job-minutes (jobs whose runner starts
`runs-on--`) on a bounded sample. **Proxy, not dollars** — runtime × c7 rate; no billing export
joined; samples are small; skips produce 0-min runs. Purpose = rank candidates, not quote cost.

Measured AWS job-min/run (and 7d run volume):
- **nightly-full-suite**: ~223 min/run × 8 runs/wk, 168 AWS jobs in sample — **largest measured
  per-run consumer**; fan-out heavy.
- **coverage**: ~205 min/run × 23 runs/wk — second largest; steady high value.
- **lint-test-gated**: ~15 min/run but **100+ runs/wk** → highest aggregate PR-adjacent spend
  (smoke/encryption/cached-pkgs, label-gated).
- **coverage-move-only**: ~15 min/run × 8/wk. **prebuilt-workloads-check**: ~3 min/run × 38/wk.
- **docker-build-test**: ~10 min/run (sampled) — note the *image-build* reusable
  (`workflow-run-docker-rust-build`) runs nested, not standalone; its spend rides inside
  `docker-build-test` / `nightly-full-suite` parent runs and needs composite measurement.
- **Zero AWS minutes in sample (jobs skip on most runs — low/no value to migrate):**
  mono-move-tests-parity, run-gas-calibration, framework-upgrade-compat, faucet-tests-prod,
  rust-client-tests. High run counts but the c7 jobs are gated off on the sampled runs.
- **0 standalone runs (nested/manual — measure via parent before any claim):** smoke-tests,
  workflow-run-docker-rust-build, prover-daily-test, node-api-compatibility-tests,
  indexer-processor-testing, find-packages-with-undeclared, faucet-tests-main, docker-build-rosetta,
  cli-e2e-tests, lean, mono-move-miri (the last two show 100 runs but the c7 label may not be the
  active path — verify).

Ranked migration candidates (highest $-and-feasibility first): **nightly-full-suite**, **coverage**,
**lint-test-gated**. Deferred/blocked: docker builds (registry/cache/disk complexity), faucet/compat
(live-network/secrets), and the skip-heavy suites (parity/gas/framework — near-zero actual AWS use).

## 11. Phased follow-up migration plan (PLAN ONLY — no implementation until a phase is authorized)

Goal (user directive): move the remaining AWS `runs-on,c7` jobs to the Azure pool **by default**,
keeping them green, and sharding the long suites for wall-clock speedup the way PR20773 did.
Constraint honored: this is **not** a blanket flip — feasibility splits into three buckets.

Shared pattern for every in-scope job (mirror PR20773): conditional `runs-on`
(hosted `64cpu-aptos-image-ubuntu22-x64` vs AWS `format(...)` fallback), `rust-prebuilt-setup` on
hosted, `dev_setup.sh` retained on the AWS leg, provider routed from `rust-ci-provider` (or a
per-workflow equivalent where no provider job exists), required job names unchanged. Kill switch =
existing `RUST_CI_PROVIDER` / dispatch input. Hosted image contract (`/opt/aptos-rust-image.json`)
verified to carry rustc/nextest/protoc/z3/cvc5/boogie/clang; it does **not** carry
`cargo-llvm-cov`/`grcov`, so coverage jobs keep a runtime `taiki-e/install-action` step on hosted.

### Phase A — pure compute, shardable, biggest win (recommend first)
| Workflow | AWS min/run | Change | Speedup lever |
|---|---|---|---|
| nightly-full-suite | ~223 | swap runs-on + prebuilt; **shard the nextest matrix** like PR20773 | yes (4-way) |
| coverage | ~205 | swap runs-on + prebuilt; keep `install-action` llvm-cov; shard unit vs smoke cov | partial |
| lint-test-gated (7 jobs) | ~15 × 100+/wk | swap runs-on + prebuilt on the c7 jobs (doc/batch-enc/all-unit/cached-pkgs/consensus) | no (gated) |

### Phase B — re-scoped after inspection (corrects the earlier blanket bucket)
0 AWS min in the 7d sample for all four (c7 jobs gated off most runs). Closer reads changed the plan:
- `run-gas-calibration` — **DONE (this branch)**: pure compute, routed hosted/AWS with a
  `RUST_CI_PROVIDER`-only selector (this workflow has no `workflow_dispatch`, so no dispatch input).
- `mono-move-tests-parity` — **deferred**: uses RunsOn-only `runs-on/cache/save|restore` for the
  build→smoke cache handoff; needs re-architecting to `actions/cache` (or artifacts) before hosted.
- `framework-upgrade-compat` — **deferred**: runs on a 32-cpu AWS label with no verified matching
  hosted pool label; label-gated + 0 use, low value to chase now.
- `rust-client-tests` — **moved to Phase C**: it is `pull_request_target` + `id-token: write` +
  GCP/AWS `docker-setup` creds, i.e. credentialed, not a pure-compute swap. Earlier mis-bucketed as B.

### Phase C — EXCLUDED from "all"; needs its own design/security review (design below)
- `node-api-compatibility-tests`, `indexer-processor-testing`, `rust-client-tests` (moved from B):
  `id-token: write` + GCP Workload Identity + AWS creds / `docker-setup` — runner-network/identity
  posture changes on Azure. Requires a security decision on federated identity from hosted runners.
- `faucet-tests-prod`, `faucet-tests-main`: hit live devnet/testnet — egress/latency path changes.

#### Docker builds (`docker-build-rosetta`, `workflow-run-docker-rust-build`) — the hard one
Verified coupling in `workflow-run-docker-rust-build.yaml` (the reusable engine):
- `runs-on/snapshot` mounts a **1500 GB** docker volume (`/var/lib/docker`) — RunsOn-only; the hosted
  image's disk is far smaller and not snapshot-backed. Needs an alternative layer-cache strategy.
- Registry fan-out: GCP Artifact Registry (Workload Identity) **and** AWS ECR
  (`AWS_ACCESS_KEY_ID`/`AWS_DOCKER_ARTIFACT_REPO`/`ENV_ECR_AWS_ACCOUNT_NUM`).
- Concurrency control via a **GCS build-lock bucket** (`aptos-docker-build-lock-bucket`) keyed by
  SHA+profile+features, plus `wait-images-ci` polling.
- `docker-build-rosetta` is `id-token: write` + `buildx-setup` + a custom build script.
Migration options to evaluate (not chosen): (a) keep build on AWS but shrink scope; (b) hosted build
with `actions/cache`-backed buildx cache + ACR as the primary registry, dropping the ECR leg if
consumers allow; (c) a larger custom hosted image with bigger disk. Each has registry-cache and
identity implications that need owner sign-off. Attribution note: docker rust-build spend is nested
inside parent runs (nightly etc.), so its true cost needs composite measurement via the parent
`e2e-images` job, not the reusable workflow's standalone runs (which are 0).

### Per-workflow plan fields (to fill before implementing any)
For each: exact `runs-on`/setup/shard diff, secrets/egress check (name the field + source, or mark
deliberately absent), fallback leg, and a "still passes" validation = extend the
`test_hosted_ci_workflows.py` contract test + a hosted dry-run on a canary PR (paid CI — gated).

Resolved during implementation: none of the three workflows had a provider job, so each gained a
per-workflow `rust-ci-provider` selector (defaults hosted, `RUST_CI_PROVIDER=aws` / dispatch input
fallback). The hosted image does NOT carry Node/pnpm, so `lint-test-gated`'s `rust-batch-encryption-tests`
got a hosted-only `actions/setup-node@v4` step (AWS keeps `dev_setup.sh -J`). `coverage` keeps
`rustup component add llvm-tools-preview` + `taiki-e/install-action` (llvm-cov) on BOTH paths.

### Phase A implementation (this branch, local only — no push/PR/CI dispatch)
Files changed (+240/-17 vs main `a8f8c33289`):
- `nightly-full-suite.yaml`: added `rust_ci_provider` dispatch input + `rust-ci-provider` job;
  `cargo-tests` + `batch-encryption` routed hosted/AWS; **workspace suite sharded into PR20773's 4
  static groups** (compiler / mono-move / move-e2e / remaining) with the 4 heavy-package exclusions
  preserved → wall-clock speedup on the ~223 min/run suite. Cloud-cred/docker/forge children untouched.
- `coverage.yaml`: added selector + dispatch input; `rust-unit-coverage` + `rust-smoke-coverage`
  routed; prebuilt on hosted, rust-setup on AWS, llvm-cov install on both.
- `lint-test-gated.yaml`: added selector + dispatch input; all 7 c7 jobs routed; prebuilt on hosted
  (AWS legs keep existing dev_setup/rust-setup); batch-encryption hosted leg adds setup-node.
- `test_hosted_ci_workflows.py`: +2 tests — Phase A routing (single `needs` incl. provider, dual
  runs-on) and nightly shard-coverage integrity.

Validation (local): YAML parse OK ×3; actionlint structural clean; contract tests 8/8 OK.
Caught by actionlint and fixed: a duplicate `needs:` key on two gated jobs that YAML would have
silently resolved by dropping `e2e-test-determinator` — a real regression, now covered by test.

Not done (gated): any push/PR/hosted dry-run (dispatches paid CI), and the nightly/coverage hosted
first-run timing. Phase B (consistency-only) and Phase C (credentialed/docker) remain unimplemented. 

## 10. Explicit non-actions (this task)

No pushes, PRs, CI dispatches, paid CI runs, repo-variable edits, protection edits,
runner/image changes, dependency installs, or credential access. The only commit is the local,
unpushed dependency-base merge of `origin/jon/hosted-rust-ci` into this isolated branch (approved
stacking). All `gh`/`gcloud` calls were read-only. PR20773's code was merged as a base, not
rewritten; no ad-hoc duplicate shared action was created.
